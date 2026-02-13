use crate::filament::{
    ElementType, Engine, Entity, EntityManager, IndexBuffer, IndexType, Material, MaterialInstance,
    PrimitiveType, RenderableBuilder, Scene, VertexAttribute, VertexBuffer,
};
use crate::render::{PickKey, PickKind};
use glam::{Mat3, Mat4, Vec3};

const MODE_TRANSLATE: i32 = 1;
const MODE_ROTATE: i32 = 2;
const MODE_SCALE: i32 = 3;

const GIZMO_TRANSLATE_X: i32 = 1;
const GIZMO_TRANSLATE_Y: i32 = 2;
const GIZMO_TRANSLATE_Z: i32 = 3;
const GIZMO_TRANSLATE_XY: i32 = 4;
const GIZMO_TRANSLATE_XZ: i32 = 5;
const GIZMO_TRANSLATE_YZ: i32 = 6;
const GIZMO_ROTATE_X: i32 = 11;
const GIZMO_ROTATE_Y: i32 = 12;
const GIZMO_ROTATE_Z: i32 = 13;
const GIZMO_ROTATE_VIEW: i32 = 14;
const GIZMO_ROTATE_ARCBALL: i32 = 15;
const GIZMO_SCALE_X: i32 = 21;
const GIZMO_SCALE_Y: i32 = 22;
const GIZMO_SCALE_Z: i32 = 23;
const GIZMO_SCALE_XY: i32 = 24;
const GIZMO_SCALE_XZ: i32 = 25;
const GIZMO_SCALE_YZ: i32 = 26;
const GIZMO_SCALE_UNIFORM: i32 = 27;
const ROTATE_CLIP_DEBUG_MODE: f32 = 0.0;

#[derive(Clone, Copy)]
pub struct GizmoParams {
    pub visible: bool,
    pub mode: i32,
    pub origin: [f32; 3],
    pub axis_world_len: f32,
    pub camera_forward: [f32; 3],
    pub camera_up: [f32; 3],
    pub highlighted_handle: i32,
    pub selected_object_index: Option<u32>,
}

struct MeshResource {
    vertex: VertexBuffer,
    index: IndexBuffer,
}

struct HandleEntity {
    entity: Entity,
    handle_id: i32,
    kind: PickKind,
    mode_mask: u8,
    base_rotation: Mat3,
    billboard_to_camera: bool,
    pickable: bool,
    uses_rotate_clip: bool,
    base_alpha: f32,
    _mesh: MeshResource,
    material_instance: MaterialInstance,
}

pub struct EditorOverlay {
    _material: Material,
    _rotate_material: Material,
    handles: Vec<HandleEntity>,
    layer_overlay_value: u8,
    layer_hidden_value: u8,
    params: GizmoParams,
}

impl EditorOverlay {
    pub fn new(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        layer_overlay_value: u8,
    ) -> Option<Self> {
        let base_package = include_bytes!(concat!(env!("OUT_DIR"), "/bakedColor.filamat"));
        let rotate_package = include_bytes!(concat!(env!("OUT_DIR"), "/gizmoRotate.filamat"));
        let mut material = engine.create_material(base_package)?;
        let mut rotate_material = engine.create_material(rotate_package)?;

        let mut handles = Vec::new();
        handles.extend(Self::create_axis_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));
        handles.extend(Self::create_plane_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));
        handles.extend(Self::create_rotation_handles(
            engine,
            scene,
            entity_manager,
            &mut material,
            &mut rotate_material,
            layer_overlay_value,
        ));
        handles.extend(Self::create_misc_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));

        Some(Self {
            _material: material,
            _rotate_material: rotate_material,
            handles,
            layer_overlay_value,
            layer_hidden_value: 0x00,
            params: GizmoParams {
                visible: false,
                mode: MODE_TRANSLATE,
                origin: [0.0, 0.0, 0.0],
                axis_world_len: 1.0,
                camera_forward: [0.0, 0.0, -1.0],
                camera_up: [0.0, 1.0, 0.0],
                highlighted_handle: 0,
                selected_object_index: None,
            },
        })
    }

    pub fn set_params(&mut self, engine: &mut Engine, params: GizmoParams) {
        self.params = params;
        self.update_handle_visibility(engine);
        self.update_handle_transforms(engine);
    }

    pub fn pickable_entities(&self) -> Vec<(PickKey, Vec<Entity>)> {
        let Some(object_id) = self.params.selected_object_index else {
            return Vec::new();
        };
        if !self.params.visible {
            return Vec::new();
        }
        let mut out = Vec::new();
        for handle in &self.handles {
            if !handle.pickable || !self.is_handle_mode_visible(handle.handle_id) {
                continue;
            }
            let fade = self.handle_fade_factor(handle.handle_id);
            if fade <= 0.75 {
                continue;
            }
            out.push((
                PickKey::new(handle.kind, object_id.min(0xFFFFF), handle.handle_id as u8),
                vec![handle.entity],
            ));
        }
        out
    }

    fn update_handle_visibility(&mut self, engine: &mut Engine) {
        let active_mode_mask = match self.params.mode {
            MODE_TRANSLATE => 0b001,
            MODE_ROTATE => 0b010,
            MODE_SCALE => 0b100,
            _ => 0,
        };
        let camera_forward = self.params.camera_forward;
        for handle in &mut self.handles {
            let fade = Self::handle_fade_factor_for(camera_forward, handle.handle_id);
            let mode_visible = (handle.mode_mask & active_mode_mask) != 0;
            let visible = self.params.visible
                && mode_visible
                && fade > 0.001;
            let value = if visible {
                self.layer_overlay_value
            } else {
                self.layer_hidden_value
            };
            engine.renderable_set_layer_mask(handle.entity, 0xFF, value);
            let is_highlighted = self.params.highlighted_handle == handle.handle_id;
            let (rgb, alpha_mult) = if is_highlighted {
                ([1.30, 1.26, 1.08], 1.05)
            } else {
                ([1.0, 1.0, 1.0], 1.0)
            };
            let (rgb, alpha_mult) = if is_highlighted
                && (handle.handle_id == GIZMO_ROTATE_VIEW || handle.handle_id == GIZMO_SCALE_UNIFORM)
            {
                ([1.50, 1.50, 1.50], 1.20)
            } else {
                (rgb, alpha_mult)
            };
            let mut alpha = (handle.base_alpha * fade * alpha_mult).clamp(0.0, 1.0);
            // Inner arcball disk should always stay subtle.
            if handle.handle_id == GIZMO_ROTATE_ARCBALL {
                alpha = alpha.min(0.12);
            }
            let rgba = [rgb[0], rgb[1], rgb[2], alpha];
            handle.material_instance.set_float4("tint", rgba);
            if handle.uses_rotate_clip {
                handle.material_instance.set_float3("clipCenter", self.params.origin);
                handle.material_instance.set_float("clipBias", 0.0);
                handle
                    .material_instance
                    .set_float("debugMode", ROTATE_CLIP_DEBUG_MODE);
                handle
                    .material_instance
                    .set_float("debugScale", (self.params.axis_world_len * 1.1).max(0.001));
            }
        }
    }

    fn update_handle_transforms(&self, engine: &mut Engine) {
        let origin = Vec3::from_array(self.params.origin);
        let axis_len = self.params.axis_world_len.max(0.0001);
        let camera_forward = Vec3::from_array(self.params.camera_forward).normalize_or_zero();
        let camera_up = Vec3::from_array(self.params.camera_up).normalize_or_zero();
        let camera_right = camera_forward.cross(camera_up).normalize_or_zero();
        let billboard_basis = Mat3::from_cols(camera_right, camera_up, camera_forward);

        let Some(mut tm) = engine.transform_manager() else {
            return;
        };

        for handle in &self.handles {
            let basis = if handle.billboard_to_camera {
                billboard_basis
            } else {
                handle.base_rotation
            };
            let world = Mat4::from_translation(origin)
                * Mat4::from_mat3(basis)
                * Mat4::from_scale(Vec3::splat(axis_len));
            tm.set_transform(handle.entity, &world.to_cols_array());
        }
    }

    fn is_handle_mode_visible(&self, handle_id: i32) -> bool {
        let mode_mask = match self.params.mode {
            MODE_TRANSLATE => 0b001,
            MODE_ROTATE => 0b010,
            MODE_SCALE => 0b100,
            _ => 0,
        };
        self.handles
            .iter()
            .find(|h| h.handle_id == handle_id)
            .map(|h| (h.mode_mask & mode_mask) != 0)
            .unwrap_or(false)
    }

    fn add_handle(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        material: &mut Material,
        layer_overlay_value: u8,
        mut mesh: MeshResource,
        handle_id: i32,
        kind: PickKind,
        mode_mask: u8,
        base_rotation: Mat3,
        billboard_to_camera: bool,
        pickable: bool,
        uses_rotate_clip: bool,
        base_alpha: f32,
    ) -> Option<HandleEntity> {
        let mut mi = material.create_instance()?;
        mi.set_float4("tint", [1.0, 1.0, 1.0, base_alpha.clamp(0.0, 1.0)]);
        if uses_rotate_clip {
            mi.set_float3("clipCenter", [0.0, 0.0, 0.0]);
            mi.set_float("clipBias", 0.0);
            mi.set_float("debugMode", ROTATE_CLIP_DEBUG_MODE);
            mi.set_float("debugScale", 1.0);
        }
        let entity = entity_manager.create();
        let builder: RenderableBuilder = engine
            .renderable_builder(1)
            .bounding_box([0.0, 0.0, 0.0], [2.0, 2.0, 2.0])
            .material(0, &mut mi)
            .geometry(0, PrimitiveType::Triangles, &mut mesh.vertex, &mut mesh.index)
            .layer_mask(0xFF, layer_overlay_value)
            .culling(false);
        builder.build(entity);
        scene.add_entity(entity);

        Some(HandleEntity {
            entity,
            handle_id,
            kind,
            mode_mask,
            base_rotation,
            billboard_to_camera,
            pickable,
            uses_rotate_clip,
            base_alpha,
            _mesh: mesh,
            material_instance: mi,
        })
    }

    fn create_axis_handles(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        material: &mut Material,
        layer_overlay_value: u8,
    ) -> Vec<HandleEntity> {
        let mut out = Vec::new();
        let x_col = [214, 128, 128, 255];
        let y_col = [142, 196, 142, 255];
        let z_col = [132, 162, 206, 255];
        let data = [
            // Translate shafts + arrow heads.
            (GIZMO_TRANSLATE_X, PickKind::GizmoAxis, 0b001, false, true, 0.95, create_box_mesh(engine, [0.47, 0.0, 0.0], [0.94, 0.017, 0.017], x_col)),
            (GIZMO_TRANSLATE_Y, PickKind::GizmoAxis, 0b001, false, true, 0.95, create_box_mesh(engine, [0.0, 0.47, 0.0], [0.017, 0.94, 0.017], y_col)),
            (GIZMO_TRANSLATE_Z, PickKind::GizmoAxis, 0b001, false, true, 0.95, create_box_mesh(engine, [0.0, 0.0, 0.47], [0.017, 0.017, 0.94], z_col)),
            (GIZMO_TRANSLATE_X, PickKind::GizmoAxis, 0b001, false, true, 0.95, create_pyramid_mesh(engine, [1.0, 0.0, 0.0], 1, 0.12, 0.045, x_col)),
            (GIZMO_TRANSLATE_Y, PickKind::GizmoAxis, 0b001, false, true, 0.95, create_pyramid_mesh(engine, [0.0, 1.0, 0.0], 2, 0.12, 0.045, y_col)),
            (GIZMO_TRANSLATE_Z, PickKind::GizmoAxis, 0b001, false, true, 0.95, create_pyramid_mesh(engine, [0.0, 0.0, 1.0], 3, 0.12, 0.045, z_col)),
            // Scale shafts + square heads.
            (GIZMO_SCALE_X, PickKind::GizmoAxis, 0b100, false, true, 0.95, create_box_mesh(engine, [0.47, 0.0, 0.0], [0.94, 0.017, 0.017], x_col)),
            (GIZMO_SCALE_Y, PickKind::GizmoAxis, 0b100, false, true, 0.95, create_box_mesh(engine, [0.0, 0.47, 0.0], [0.017, 0.94, 0.017], y_col)),
            (GIZMO_SCALE_Z, PickKind::GizmoAxis, 0b100, false, true, 0.95, create_box_mesh(engine, [0.0, 0.0, 0.47], [0.017, 0.017, 0.94], z_col)),
            (GIZMO_SCALE_X, PickKind::GizmoAxis, 0b100, false, true, 0.95, create_box_mesh(engine, [0.98, 0.0, 0.0], [0.08, 0.08, 0.08], x_col)),
            (GIZMO_SCALE_Y, PickKind::GizmoAxis, 0b100, false, true, 0.95, create_box_mesh(engine, [0.0, 0.98, 0.0], [0.08, 0.08, 0.08], y_col)),
            (GIZMO_SCALE_Z, PickKind::GizmoAxis, 0b100, false, true, 0.95, create_box_mesh(engine, [0.0, 0.0, 0.98], [0.08, 0.08, 0.08], z_col)),
        ];
        for (id, kind, mask, bb, pickable, base_alpha, mesh) in data {
            if let Some(h) = Self::add_handle(engine, scene, entity_manager, material, layer_overlay_value, mesh, id, kind, mask, Mat3::IDENTITY, bb, pickable, false, base_alpha) {
                out.push(h);
            }
        }
        out
    }

    fn create_plane_handles(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        material: &mut Material,
        layer_overlay_value: u8,
    ) -> Vec<HandleEntity> {
        let mut out = Vec::new();
        let mut add = |id, color: [u8; 4], mode| {
            (id, PickKind::GizmoPlane, mode, Mat3::IDENTITY, false, true, 0.85, create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], color))
        };
        let specs = [
            add(GIZMO_TRANSLATE_XY, [255, 255, 80, 180], 0b001),
            add(GIZMO_SCALE_XY, [255, 255, 80, 180], 0b100),
            (GIZMO_TRANSLATE_XZ, PickKind::GizmoPlane, 0b001, Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2), false, true, 0.85, create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [255, 120, 80, 180])),
            (GIZMO_SCALE_XZ, PickKind::GizmoPlane, 0b100, Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2), false, true, 0.85, create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [255, 120, 80, 180])),
            (GIZMO_TRANSLATE_YZ, PickKind::GizmoPlane, 0b001, Mat3::from_rotation_y(-std::f32::consts::FRAC_PI_2), false, true, 0.85, create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [80, 255, 255, 180])),
            (GIZMO_SCALE_YZ, PickKind::GizmoPlane, 0b100, Mat3::from_rotation_y(-std::f32::consts::FRAC_PI_2), false, true, 0.85, create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [80, 255, 255, 180])),
        ];
        for (id, kind, mask, rot, bb, pickable, base_alpha, mesh) in specs {
            if let Some(h) = Self::add_handle(engine, scene, entity_manager, material, layer_overlay_value, mesh, id, kind, mask, rot, bb, pickable, false, base_alpha) {
                out.push(h);
            }
        }
        out
    }

    fn create_rotation_handles(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        material: &mut Material,
        rotate_material: &mut Material,
        layer_overlay_value: u8,
    ) -> Vec<HandleEntity> {
        let mut out = Vec::new();
        let ring_x = create_ring_mesh(engine, 1.10, 0.008, [214, 128, 128, 200], 64, Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let ring_y = create_ring_mesh(engine, 1.10, 0.008, [142, 196, 142, 200], 64, Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        let ring_z = create_ring_mesh(engine, 1.10, 0.008, [132, 162, 206, 200], 64, Mat3::IDENTITY);
        let view_ring = create_ring_mesh(engine, 1.22, 0.007, [150, 150, 150, 170], 64, Mat3::IDENTITY);
        let clipped_specs = [
            (GIZMO_ROTATE_X, PickKind::GizmoRing, 0b010, false, true, 1.0, ring_x),
            (GIZMO_ROTATE_Y, PickKind::GizmoRing, 0b010, false, true, 1.0, ring_y),
            (GIZMO_ROTATE_Z, PickKind::GizmoRing, 0b010, false, true, 1.0, ring_z),
        ];
        for (id, kind, mask, billboard, pickable, base_alpha, mesh) in clipped_specs {
            if let Some(h) = Self::add_handle(
                engine,
                scene,
                entity_manager,
                rotate_material,
                layer_overlay_value,
                mesh,
                id,
                kind,
                mask,
                Mat3::IDENTITY,
                billboard,
                pickable,
                true,
                base_alpha,
            ) {
                out.push(h);
            }
        }
        let regular_specs = [(GIZMO_ROTATE_VIEW, PickKind::GizmoRing, 0b010, true, true, 0.75, view_ring)];
        for (id, kind, mask, billboard, pickable, base_alpha, mesh) in regular_specs {
            if let Some(h) = Self::add_handle(
                engine,
                scene,
                entity_manager,
                material,
                layer_overlay_value,
                mesh,
                id,
                kind,
                mask,
                Mat3::IDENTITY,
                billboard,
                pickable,
                false,
                base_alpha,
            ) {
                out.push(h);
            }
        }
        out
    }

    fn create_misc_handles(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        material: &mut Material,
        layer_overlay_value: u8,
    ) -> Vec<HandleEntity> {
        let mut out = Vec::new();
        let mesh = create_ring_mesh(
            engine,
            1.22,
            0.009,
            [150, 150, 150, 170],
            64,
            Mat3::IDENTITY,
        );
        if let Some(h) = Self::add_handle(
            engine,
            scene,
            entity_manager,
            material,
            layer_overlay_value,
            mesh,
            GIZMO_SCALE_UNIFORM,
            PickKind::GizmoRing,
            0b100,
            Mat3::IDENTITY,
            true,
            true,
            false,
            0.75,
        ) {
            out.push(h);
        }
        out
    }

    fn handle_fade_factor(&self, handle_id: i32) -> f32 {
        Self::handle_fade_factor_for(self.params.camera_forward, handle_id)
    }

    fn handle_fade_factor_for(camera_forward: [f32; 3], handle_id: i32) -> f32 {
        let view_dir = Vec3::from_array(camera_forward).normalize_or_zero();
        if view_dir.length_squared() <= 1e-8 {
            return 1.0;
        }
        let start_dot = 20.0f32.to_radians().cos();
        let end_dot = 10.0f32.to_radians().cos();
        let fade_from_dot = |dot_abs: f32| -> f32 {
            if dot_abs <= start_dot {
                1.0
            } else if dot_abs >= end_dot {
                0.0
            } else {
                1.0 - ((dot_abs - start_dot) / (end_dot - start_dot))
            }
        };

        if let Some(axis) = Self::handle_axis(handle_id) {
            let dot_abs = view_dir.dot(axis).abs();
            return fade_from_dot(dot_abs);
        }
        if let Some((axis_a, axis_b)) = Self::handle_plane_axes(handle_id) {
            let max_dot = view_dir.dot(axis_a).abs().max(view_dir.dot(axis_b).abs());
            return fade_from_dot(max_dot);
        }
        1.0
    }

    fn handle_axis(handle_id: i32) -> Option<Vec3> {
        match handle_id {
            GIZMO_TRANSLATE_X | GIZMO_SCALE_X | GIZMO_ROTATE_X => Some(Vec3::X),
            GIZMO_TRANSLATE_Y | GIZMO_SCALE_Y | GIZMO_ROTATE_Y => Some(Vec3::Y),
            GIZMO_TRANSLATE_Z | GIZMO_SCALE_Z | GIZMO_ROTATE_Z => Some(Vec3::Z),
            _ => None,
        }
    }

    fn handle_plane_axes(handle_id: i32) -> Option<(Vec3, Vec3)> {
        match handle_id {
            GIZMO_TRANSLATE_XY | GIZMO_SCALE_XY => Some((Vec3::X, Vec3::Y)),
            GIZMO_TRANSLATE_XZ | GIZMO_SCALE_XZ => Some((Vec3::X, Vec3::Z)),
            GIZMO_TRANSLATE_YZ | GIZMO_SCALE_YZ => Some((Vec3::Y, Vec3::Z)),
            _ => None,
        }
    }
}

fn create_mesh(
    engine: &mut Engine,
    positions: &[[f32; 3]],
    colors: &[[u8; 4]],
    indices: &[u16],
) -> Option<MeshResource> {
    if positions.is_empty() || positions.len() != colors.len() || indices.is_empty() {
        return None;
    }
    let mut vb = engine
        .vertex_buffer_builder()
        .vertex_count(positions.len() as u32)
        .buffer_count(2)
        .attribute(VertexAttribute::Position, 0, ElementType::Float3, 0, 12)
        .attribute(VertexAttribute::Color, 1, ElementType::UByte4, 0, 4)
        .normalized(VertexAttribute::Color, true)
        .build()?;
    vb.set_buffer_at(0, positions, 0);
    vb.set_buffer_at(1, colors, 0);
    let mut ib = engine
        .index_buffer_builder()
        .index_count(indices.len() as u32)
        .buffer_type(IndexType::UShort)
        .build()?;
    ib.set_buffer(indices, 0);
    Some(MeshResource { vertex: vb, index: ib })
}

fn create_box_mesh(engine: &mut Engine, center: [f32; 3], size: [f32; 3], color: [u8; 4]) -> MeshResource {
    let cx = center[0];
    let cy = center[1];
    let cz = center[2];
    let sx = size[0] * 0.5;
    let sy = size[1] * 0.5;
    let sz = size[2] * 0.5;
    let p = [
        [cx - sx, cy - sy, cz - sz],
        [cx + sx, cy - sy, cz - sz],
        [cx + sx, cy + sy, cz - sz],
        [cx - sx, cy + sy, cz - sz],
        [cx - sx, cy - sy, cz + sz],
        [cx + sx, cy - sy, cz + sz],
        [cx + sx, cy + sy, cz + sz],
        [cx - sx, cy + sy, cz + sz],
    ];
    let c = [color; 8];
    let idx: [u16; 36] = [
        0, 1, 2, 0, 2, 3,
        4, 6, 5, 4, 7, 6,
        0, 4, 5, 0, 5, 1,
        1, 5, 6, 1, 6, 2,
        2, 6, 7, 2, 7, 3,
        3, 7, 4, 3, 4, 0,
    ];
    create_mesh(engine, &p, &c, &idx).expect("box mesh")
}

fn create_pyramid_mesh(
    engine: &mut Engine,
    tip: [f32; 3],
    axis_id: i32,
    length: f32,
    base_half: f32,
    color: [u8; 4],
) -> MeshResource {
    let axis = match axis_id {
        1 => Vec3::X,
        2 => Vec3::Y,
        3 => Vec3::Z,
        _ => Vec3::X,
    };
    let (u, v) = match axis_id {
        1 => (Vec3::Y, Vec3::Z),
        2 => (Vec3::X, Vec3::Z),
        3 => (Vec3::X, Vec3::Y),
        _ => (Vec3::Y, Vec3::Z),
    };
    let tip_v = Vec3::from_array(tip);
    let base_center = tip_v - axis * length.max(0.001);
    let p0 = tip_v;
    let p1 = base_center + u * base_half + v * base_half;
    let p2 = base_center + u * base_half - v * base_half;
    let p3 = base_center - u * base_half - v * base_half;
    let p4 = base_center - u * base_half + v * base_half;
    let positions = [
        p0.to_array(),
        p1.to_array(),
        p2.to_array(),
        p3.to_array(),
        p4.to_array(),
    ];
    let colors = [color; 5];
    let indices: [u16; 18] = [
        0, 1, 2,
        0, 2, 3,
        0, 3, 4,
        0, 4, 1,
        1, 4, 3,
        1, 3, 2,
    ];
    create_mesh(engine, &positions, &colors, &indices).expect("pyramid mesh")
}

fn create_quad_mesh(engine: &mut Engine, center: [f32; 3], size: [f32; 2], color: [u8; 4]) -> MeshResource {
    let cx = center[0];
    let cy = center[1];
    let cz = center[2];
    let sx = size[0] * 0.5;
    let sy = size[1] * 0.5;
    let p = [
        [cx - sx, cy - sy, cz],
        [cx + sx, cy - sy, cz],
        [cx + sx, cy + sy, cz],
        [cx - sx, cy + sy, cz],
    ];
    let c = [color; 4];
    let idx: [u16; 6] = [0, 1, 2, 0, 2, 3];
    create_mesh(engine, &p, &c, &idx).expect("quad mesh")
}

fn create_ring_mesh(
    engine: &mut Engine,
    radius: f32,
    thickness: f32,
    color: [u8; 4],
    segments: usize,
    rotation: Mat3,
) -> MeshResource {
    let n = segments.max(16);
    let mut positions = Vec::with_capacity(n * 2);
    let mut colors = Vec::with_capacity(n * 2);
    let mut indices = Vec::with_capacity(n * 6);
    for i in 0..n {
        let t = (i as f32 / n as f32) * std::f32::consts::TAU;
        let dir = Vec3::new(t.cos(), t.sin(), 0.0);
        let p_outer = rotation * (dir * (radius + thickness));
        let p_inner = rotation * (dir * (radius - thickness));
        positions.push(p_outer.to_array());
        positions.push(p_inner.to_array());
        colors.push(color);
        colors.push(color);
    }
    for i in 0..n {
        let i0 = (i * 2) as u16;
        let i1 = ((i * 2 + 1) % (n * 2)) as u16;
        let j0 = (((i + 1) % n) * 2) as u16;
        let j1 = ((((i + 1) % n) * 2 + 1) % (n * 2)) as u16;
        indices.extend_from_slice(&[i0, j0, j1, i0, j1, i1]);
    }
    create_mesh(engine, &positions, &colors, &indices).expect("ring mesh")
}

fn create_disk_mesh(engine: &mut Engine, radius: f32, color: [u8; 4], segments: usize) -> MeshResource {
    let n = segments.max(12);
    let mut positions = Vec::with_capacity(n + 1);
    let mut colors = Vec::with_capacity(n + 1);
    let mut indices = Vec::with_capacity(n * 3);
    positions.push([0.0, 0.0, 0.0]);
    colors.push(color);
    for i in 0..n {
        let t = (i as f32 / n as f32) * std::f32::consts::TAU;
        positions.push([radius * t.cos(), radius * t.sin(), 0.0]);
        colors.push(color);
    }
    for i in 0..n {
        let a = 0u16;
        let b = (i + 1) as u16;
        let c = if i + 1 == n { 1u16 } else { (i + 2) as u16 };
        indices.extend_from_slice(&[a, b, c]);
    }
    create_mesh(engine, &positions, &colors, &indices).expect("disk mesh")
}
