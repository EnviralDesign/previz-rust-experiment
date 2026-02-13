//! GPU Pick Pass system
//!
//! Manages an offscreen render target that renders each pickable entity
//! with a unique flat color encoding a `PickKey`. On click, a 1×1 pixel
//! readback decodes the color back to a `PickKey` and returns a `PickHit`.
//!
//! ## Architecture
//!
//! The pick pass shares the main scene's entities but temporarily swaps
//! their materials to a flat unlit pick material before rendering. After
//! the pick pass, original materials are restored. This avoids duplicating
//! geometry while keeping the pick pass isolated.

#![allow(dead_code)]

use crate::filament::{
    Engine, Entity, Material, MaterialInstance, RenderTarget, Renderer,
    Texture, TextureInternalFormat, TextureUsage, View,
};
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;

// ========================================================================
// PickKey — 32-bit packed identifier for any pickable element
// ========================================================================

/// Classification of pickable element.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PickKind {
    None = 0,
    SceneMesh = 1,
    GizmoAxis = 2,
    GizmoPlane = 3,
    GizmoRing = 4,
    LightHelper = 5,
    CameraWidget = 6,
}

impl PickKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::SceneMesh,
            2 => Self::GizmoAxis,
            3 => Self::GizmoPlane,
            4 => Self::GizmoRing,
            5 => Self::LightHelper,
            6 => Self::CameraWidget,
            _ => Self::None,
        }
    }
}

/// 32-bit packed pick key:
///   R = (kind << 4) | (object_id >> 16) & 0xF
///   G = (object_id >> 8) & 0xFF
///   B = object_id & 0xFF
///   A = sub_id
///
/// Gives: 4-bit kind (16 types), 20-bit object_id (1M ids), 8-bit sub_id (256 sub-parts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PickKey {
    pub kind: PickKind,
    pub object_id: u32,  // 20-bit range
    pub sub_id: u8,
}

impl PickKey {
    pub const NONE: Self = Self {
        kind: PickKind::None,
        object_id: 0,
        sub_id: 0,
    };

    pub fn new(kind: PickKind, object_id: u32, sub_id: u8) -> Self {
        debug_assert!(object_id <= 0xFFFFF, "object_id exceeds 20-bit range");
        Self { kind, object_id, sub_id }
    }

    pub fn scene_mesh(object_id: u32) -> Self {
        Self::new(PickKind::SceneMesh, object_id, 0)
    }

    /// Encode to RGBA8 bytes.
    pub fn to_rgba(&self) -> [u8; 4] {
        let kind_nibble = (self.kind as u8) & 0x0F;
        let obj_hi = ((self.object_id >> 16) & 0x0F) as u8;
        let r = (kind_nibble << 4) | obj_hi;
        let g = ((self.object_id >> 8) & 0xFF) as u8;
        let b = (self.object_id & 0xFF) as u8;
        let a = self.sub_id;
        [r, g, b, a]
    }

    /// Encode to float4 for the pick material's `pickColor` parameter.
    pub fn to_float4(&self) -> [f32; 4] {
        let [r, g, b, a] = self.to_rgba();
        [
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            a as f32 / 255.0,
        ]
    }

    /// Decode from RGBA8 bytes.
    pub fn from_rgba(rgba: [u8; 4]) -> Self {
        let kind_nibble = (rgba[0] >> 4) & 0x0F;
        let obj_hi = (rgba[0] & 0x0F) as u32;
        let obj_mid = rgba[1] as u32;
        let obj_lo = rgba[2] as u32;
        let object_id = (obj_hi << 16) | (obj_mid << 8) | obj_lo;
        let sub_id = rgba[3];
        Self {
            kind: PickKind::from_u8(kind_nibble),
            object_id,
            sub_id,
        }
    }

    pub fn is_none(&self) -> bool {
        self.kind == PickKind::None && self.object_id == 0
    }
}

// ========================================================================
// PickHit — result of a pick operation
// ========================================================================

#[derive(Debug, Clone, Copy)]
pub struct PickHit {
    pub key: PickKey,
    pub screen_x: f32,
    pub screen_y: f32,
}

impl PickHit {
    pub fn none() -> Self {
        Self {
            key: PickKey::NONE,
            screen_x: 0.0,
            screen_y: 0.0,
        }
    }

    pub fn is_none(&self) -> bool {
        self.key.is_none()
    }
}

// ========================================================================
// per-entity saved material state for swap/restore
// ========================================================================

struct SavedMaterials {
    /// (primitive_index, raw_material_instance_pointer)
    entries: Vec<(i32, *mut c_void)>,
}

const LAYER_SCENE: u8 = 0x01;
const LAYER_OVERLAY: u8 = 0x02;
const LAYER_PICK: u8 = 0x04;

// ========================================================================
// PickSystem — manages the offscreen pick pass
// ========================================================================

pub struct PickSystem {
    // Offscreen render resources
    color_texture: Texture,
    depth_texture: Texture,
    render_target: RenderTarget,

    // Pick material (compiled from pickId.mat)
    pick_material: Material,

    // Per-pick-key material instances keyed by RGBA packing.
    pick_instances: HashMap<u32, MaterialInstance>,

    // Viewport size (for coordinate transform and resize)
    width: u32,
    height: u32,

    // Readback buffer (reused each frame)
    readback_buffer: Vec<u8>,

    // Pending pick request: screen coordinates to read
    pending_pick: Option<(f32, f32)>,

    // Latest pick result
    last_hit: Option<PickHit>,
    // Metadata for an in-flight GPU readback that must be decoded after flush.
    pending_readback: Option<(f32, f32, u32, u32)>,
    // Valid packed pick keys staged for the latest pick pass.
    staged_keys: HashSet<u32>,
}

impl PickSystem {
    /// Create the pick system. Must be called after engine + scene are initialized.
    pub fn new(engine: &mut Engine, width: u32, height: u32) -> Option<Self> {
        // Clamp to minimum 1×1
        let w = width.max(1);
        let h = height.max(1);

        let color_texture = engine.create_texture_2d(
            w, h,
            TextureInternalFormat::Rgba8,
            TextureUsage::or3(TextureUsage::ColorAttachment, TextureUsage::Sampleable, TextureUsage::BlitSrc),
        )?;
        let depth_texture = engine.create_texture_2d(
            w, h,
            TextureInternalFormat::Depth24,
            TextureUsage::DepthAttachment as u32,
        )?;
        let render_target = engine.create_render_target(&color_texture, Some(&depth_texture))?;

        // Load pick material from compiled package
        let pick_material_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/pickId.filamat"));
        let pick_material = engine.create_material(pick_material_bytes)?;

        log::info!("PickSystem initialized ({}×{})", w, h);

        Some(Self {
            color_texture,
            depth_texture,
            render_target,
            pick_material,
            pick_instances: HashMap::new(),
            width: w,
            height: h,
            readback_buffer: vec![0u8; 4], // 1×1 RGBA
            pending_pick: None,
            last_hit: None,
            pending_readback: None,
            staged_keys: HashSet::new(),
        })
    }

    /// Access the offscreen render target (for binding to a View).
    pub fn render_target(&self) -> &RenderTarget {
        &self.render_target
    }

    /// Resize the pick buffer. Call when the viewport resizes.
    pub fn resize(&mut self, engine: &mut Engine, width: u32, height: u32) -> bool {
        let w = width.max(1);
        let h = height.max(1);
        if w == self.width && h == self.height {
            return true;
        }

        // Create new textures + render target
        let color = match engine.create_texture_2d(
            w, h,
            TextureInternalFormat::Rgba8,
            TextureUsage::or3(TextureUsage::ColorAttachment, TextureUsage::Sampleable, TextureUsage::BlitSrc),
        ) {
            Some(t) => t,
            None => return false,
        };
        let depth = match engine.create_texture_2d(
            w, h,
            TextureInternalFormat::Depth24,
            TextureUsage::DepthAttachment as u32,
        ) {
            Some(t) => t,
            None => return false,
        };
        let rt = match engine.create_render_target(&color, Some(&depth)) {
            Some(rt) => rt,
            None => return false,
        };

        // Swap in new resources (old ones drop automatically via RAII)
        self.color_texture = color;
        self.depth_texture = depth;
        self.render_target = rt;
        self.width = w;
        self.height = h;

        log::info!("PickSystem resized to {}×{}", w, h);
        true
    }

    /// Request a pick at the given screen coordinates.
    /// The result will be available after the next render.
    pub fn request_pick(&mut self, screen_x: f32, screen_y: f32) {
        self.pending_pick = Some((screen_x, screen_y));
    }

    /// Check if there is a pending pick request.
    pub fn has_pending_pick(&self) -> bool {
        self.pending_pick.is_some()
    }

    /// Take the latest pick result (if any). Consumes it.
    pub fn take_hit(&mut self) -> Option<PickHit> {
        self.last_hit.take()
    }

    /// Get or create a MaterialInstance for a given pick object_id.
    fn ensure_pick_instance(&mut self, key: PickKey) -> &MaterialInstance {
        let rgba = key.to_rgba();
        let packed = u32::from_be_bytes(rgba);
        if !self.pick_instances.contains_key(&packed) {
            let color = key.to_float4();
            let mut mi = self.pick_material.create_instance()
                .expect("Failed to create pick material instance");
            mi.set_float4("pickColor", color);
            self.pick_instances.insert(packed, mi);
        }
        &self.pick_instances[&packed]
    }

    /// Execute the pick pass within a frame.
    ///
    /// This must be called between begin_frame() and end_frame().
    ///
    /// The flow:
    /// 1. For each pickable entity: save original materials, set pick materials
    /// 2. Render pick view to offscreen target
    /// 3. Restore all original materials
    ///
    /// `pickable_entities` maps pick key -> list of filament entity IDs.
    pub fn render_pick_pass(
        &mut self,
        engine: &mut Engine,
        renderer: &mut Renderer,
        pick_view: &View,
        pickable_entities: &[(PickKey, Vec<Entity>)],
    ) {
        self.staged_keys.clear();
        // 1. Save and swap materials
        let mut saved: Vec<(Entity, SavedMaterials)> = Vec::new();
        let mut layer_restore: Vec<(Entity, u8)> = Vec::new();

        for (key, entities) in pickable_entities {
            // Pre-bake the pick instance. We need the pick instance pointer
            // so that we can set it on each primitive.
            self.ensure_pick_instance(*key);
            let packed = u32::from_be_bytes(key.to_rgba());
            self.staged_keys.insert(packed);
            let pick_mi = &self.pick_instances[&packed];
            let restore_layer = match key.kind {
                PickKind::SceneMesh => LAYER_SCENE,
                PickKind::GizmoAxis
                | PickKind::GizmoPlane
                | PickKind::GizmoRing
                | PickKind::LightHelper => LAYER_OVERLAY,
                _ => LAYER_SCENE,
            };

            for &entity in entities {
                engine.renderable_set_layer_mask(entity, 0xFF, LAYER_PICK);
                layer_restore.push((entity, restore_layer));
                let prim_count = engine.renderable_primitive_count(entity);
                let mut entries = Vec::with_capacity(prim_count as usize);
                for p in 0..prim_count {
                    let original = engine.renderable_get_material_raw(entity, p);
                    entries.push((p, original));
                    engine.renderable_set_material(entity, p, pick_mi);
                }
                saved.push((entity, SavedMaterials { entries }));
            }
        }
        // 2. Render pick view
        renderer.render(pick_view);

        // 3. Restore original materials
        for (entity, saved_mats) in &saved {
            for &(prim_idx, raw_ptr) in &saved_mats.entries {
                engine.renderable_restore_material_raw(*entity, prim_idx, raw_ptr);
            }
        }
        for (entity, restore_layer) in &layer_restore {
            engine.renderable_set_layer_mask(*entity, 0xFF, *restore_layer);
        }
    }

    /// Schedule a pixel readback at the pending pick location.
    /// Caller must call `engine.flush_and_wait()` and then `complete_readback()`.
    pub fn schedule_readback(
        &mut self,
        renderer: &mut Renderer,
    ) -> bool {
        let Some((sx, sy)) = self.pending_pick.take() else {
            return false;
        };

        // Convert screen coords to pixel coords in the pick buffer.
        // Screen coordinates are top-left origin, OpenGL is bottom-left.
        let px = (sx as u32).min(self.width.saturating_sub(1));
        let py_flipped = self.height.saturating_sub(1).saturating_sub(sy as u32);

        self.readback_buffer.fill(0);
        let ok = renderer.read_pixels(
            &self.render_target,
            px,
            py_flipped,
            1, 1,
            &mut self.readback_buffer,
        );

        if ok {
            // Buffer is filled asynchronously by Filament; decode only after flush.
            self.pending_readback = Some((sx, sy, px, py_flipped));
        } else {
            log::warn!("Pick readback failed at ({},{})", px, py_flipped);
            self.last_hit = Some(PickHit::none());
        }

        ok
    }

    /// Decode the last scheduled readback after `engine.flush_and_wait()`.
    pub fn complete_readback(&mut self) {
        let Some((sx, sy, _px, _py)) = self.pending_readback.take() else {
            return;
        };
        let rgba = [
            self.readback_buffer[0],
            self.readback_buffer[1],
            self.readback_buffer[2],
            self.readback_buffer[3],
        ];
        let key = PickKey::from_rgba(rgba);
        let packed = u32::from_be_bytes(rgba);
        if !self.staged_keys.contains(&packed) {
            self.last_hit = Some(PickHit::none());
            return;
        }
        self.last_hit = Some(PickHit {
            key,
            screen_x: sx,
            screen_y: sy,
        });
    }
}

// ========================================================================
// Tests
// ========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pick_key_roundtrip() {
        let key = PickKey::new(PickKind::SceneMesh, 0x12345, 42);
        let rgba = key.to_rgba();
        let decoded = PickKey::from_rgba(rgba);
        assert_eq!(key, decoded);
    }

    #[test]
    fn pick_key_none_roundtrip() {
        let key = PickKey::NONE;
        let rgba = key.to_rgba();
        let decoded = PickKey::from_rgba(rgba);
        assert!(decoded.is_none());
    }

    #[test]
    fn pick_key_max_values() {
        let key = PickKey::new(PickKind::CameraWidget, 0xFFFFF, 255);
        let rgba = key.to_rgba();
        let decoded = PickKey::from_rgba(rgba);
        assert_eq!(decoded.kind, PickKind::CameraWidget);
        assert_eq!(decoded.object_id, 0xFFFFF);
        assert_eq!(decoded.sub_id, 255);
    }

    #[test]
    fn pick_key_float4_normalized() {
        let key = PickKey::scene_mesh(1);
        let f4 = key.to_float4();
        for v in f4 {
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
