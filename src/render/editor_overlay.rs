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
const GIZMO_TRANSLATE_SCREEN: i32 = 7;
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
const SHAFT_LINE_WIDTH_PX: f32 = 3.0;
const RING_LINE_WIDTH_PX: f32 = 3.0;
const DEFAULT_FOV_Y_DEGREES: f32 = 45.0;
const PICK_WIDTH_EXTRA_PX: f32 = 2.0;

struct LineHandleState {
    segments_local: Vec<(Vec3, Vec3)>,
    pixel_width_visual: f32,
    pixel_width_pick: f32,
    clip_to_front_hemisphere: bool,
    positions: Vec<[f32; 3]>,
}

#[derive(Clone, Copy)]
pub struct GizmoParams {
    pub visible: bool,
    pub mode: i32,
    pub origin: [f32; 3],
    pub axis_world_len: f32,
    pub camera_position: [f32; 3],
    pub camera_forward: [f32; 3],
    pub camera_up: [f32; 3],
    pub viewport_height_px: u32,
    pub camera_fov_y_degrees: f32,
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
    world_space_geometry: bool,
    line_state: Option<LineHandleState>,
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
    pick_width_mode: bool,
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
        handles.extend(Self::create_axis_handles(
            engine,
            scene,
            entity_manager,
            &mut material,
            layer_overlay_value,
        ));
        handles.extend(Self::create_plane_handles(
            engine,
            scene,
            entity_manager,
            &mut material,
            layer_overlay_value,
        ));
        handles.extend(Self::create_rotation_handles(
            engine,
            scene,
            entity_manager,
            &mut material,
            &mut rotate_material,
            layer_overlay_value,
        ));
        handles.extend(Self::create_misc_handles(
            engine,
            scene,
            entity_manager,
            &mut material,
            layer_overlay_value,
        ));

        Some(Self {
            _material: material,
            _rotate_material: rotate_material,
            handles,
            layer_overlay_value,
            layer_hidden_value: 0x00,
            pick_width_mode: false,
            params: GizmoParams {
                visible: false,
                mode: MODE_TRANSLATE,
                origin: [0.0, 0.0, 0.0],
                axis_world_len: 1.0,
                camera_position: [0.0, 0.0, 1.0],
                camera_forward: [0.0, 0.0, -1.0],
                camera_up: [0.0, 1.0, 0.0],
                viewport_height_px: 720,
                camera_fov_y_degrees: DEFAULT_FOV_Y_DEGREES,
                highlighted_handle: 0,
                selected_object_index: None,
            },
        })
    }

    pub fn set_params(&mut self, engine: &mut Engine, params: GizmoParams) {
        self.params = params;
        self.update_handle_visibility(engine);
        self.update_line_geometry();
        self.update_handle_transforms(engine);
    }

    pub fn set_pick_width_mode(&mut self, enabled: bool) {
        if self.pick_width_mode == enabled {
            return;
        }
        self.pick_width_mode = enabled;
        self.update_line_geometry();
    }

    pub fn attach_to_scene(&self, scene: &mut Scene) {
        for handle in &self.handles {
            scene.add_entity(handle.entity);
        }
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
            let visible = self.params.visible && mode_visible && fade > 0.001;
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
                && (handle.handle_id == GIZMO_ROTATE_VIEW
                    || handle.handle_id == GIZMO_SCALE_UNIFORM)
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
                handle
                    .material_instance
                    .set_float3("clipCenter", self.params.origin);
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

    fn update_line_geometry(&mut self) {
        let origin = Vec3::from_array(self.params.origin);
        let axis_len = self.params.axis_world_len.max(0.0001);
        let camera_position = Vec3::from_array(self.params.camera_position);
        let mut camera_forward = Vec3::from_array(self.params.camera_forward).normalize_or_zero();
        let mut camera_up = Vec3::from_array(self.params.camera_up).normalize_or_zero();
        if camera_forward.length_squared() <= 1e-8 {
            camera_forward = Vec3::new(0.0, 0.0, -1.0);
        }
        if camera_up.length_squared() <= 1e-8 {
            camera_up = Vec3::Y;
        }
        let mut camera_right = camera_forward.cross(camera_up).normalize_or_zero();
        if camera_right.length_squared() <= 1e-8 {
            camera_right = Vec3::X;
        }
        camera_up = camera_right.cross(camera_forward).normalize_or_zero();
        if camera_up.length_squared() <= 1e-8 {
            camera_up = Vec3::Y;
        }
        let billboard_basis = Mat3::from_cols(camera_right, camera_up, camera_forward);

        let fov_y_radians = self
            .params
            .camera_fov_y_degrees
            .to_radians()
            .clamp(10.0f32.to_radians(), 140.0f32.to_radians());
        let viewport_height = self.params.viewport_height_px.max(1) as f32;
        let clip_center = origin;
        let to_camera = (camera_position - clip_center).normalize_or_zero();

        for handle in &mut self.handles {
            let Some(line_state) = &mut handle.line_state else {
                continue;
            };
            let basis = if handle.billboard_to_camera {
                billboard_basis
            } else {
                handle.base_rotation
            };

            let segment_capacity = line_state.positions.len() / 4;
            let mut slot = 0usize;
            for (local_a, local_b) in &line_state.segments_local {
                if slot >= segment_capacity {
                    break;
                }
                let mut a = origin + basis * (*local_a * axis_len);
                let mut b = origin + basis * (*local_b * axis_len);
                if line_state.clip_to_front_hemisphere
                    && !clip_segment_to_halfspace(&mut a, &mut b, clip_center, to_camera, 0.0)
                {
                    continue;
                }
                let line_width = if self.pick_width_mode && handle.pickable {
                    line_state.pixel_width_pick
                } else {
                    line_state.pixel_width_visual
                };
                if write_segment_quad(
                    &mut line_state.positions,
                    slot,
                    a,
                    b,
                    camera_position,
                    camera_up,
                    camera_right,
                    fov_y_radians,
                    viewport_height,
                    line_width,
                ) {
                    slot += 1;
                }
            }

            let collapse = origin.to_array();
            for segment_index in slot..segment_capacity {
                let base = segment_index * 4;
                line_state.positions[base] = collapse;
                line_state.positions[base + 1] = collapse;
                line_state.positions[base + 2] = collapse;
                line_state.positions[base + 3] = collapse;
            }
            handle
                ._mesh
                .vertex
                .set_buffer_at(0, &line_state.positions, 0);
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

        let identity = Mat4::IDENTITY.to_cols_array();
        for handle in &self.handles {
            if handle.world_space_geometry {
                tm.set_transform(handle.entity, &identity);
                continue;
            }
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
            .geometry(
                0,
                PrimitiveType::Triangles,
                &mut mesh.vertex,
                &mut mesh.index,
            )
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
            world_space_geometry: false,
            line_state: None,
            base_alpha,
            _mesh: mesh,
            material_instance: mi,
        })
    }

    fn add_line_handle(
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        material: &mut Material,
        layer_overlay_value: u8,
        handle_id: i32,
        kind: PickKind,
        mode_mask: u8,
        base_rotation: Mat3,
        billboard_to_camera: bool,
        pickable: bool,
        uses_rotate_clip: bool,
        base_alpha: f32,
        color: [u8; 4],
        segments_local: Vec<(Vec3, Vec3)>,
        pixel_width: f32,
        clip_to_front_hemisphere: bool,
    ) -> Option<HandleEntity> {
        if segments_local.is_empty() {
            return None;
        }
        let segment_count = segments_local.len();
        let mesh = create_line_quad_mesh(engine, segment_count, color);
        let mut handle = Self::add_handle(
            engine,
            scene,
            entity_manager,
            material,
            layer_overlay_value,
            mesh,
            handle_id,
            kind,
            mode_mask,
            base_rotation,
            billboard_to_camera,
            pickable,
            uses_rotate_clip,
            base_alpha,
        )?;
        handle.world_space_geometry = true;
        handle.line_state = Some(LineHandleState {
            segments_local,
            pixel_width_visual: pixel_width,
            pixel_width_pick: pixel_width + PICK_WIDTH_EXTRA_PX,
            clip_to_front_hemisphere,
            positions: vec![[0.0, 0.0, 0.0]; segment_count * 4],
        });
        Some(handle)
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
        let shaft_specs = [
            (
                GIZMO_TRANSLATE_X,
                PickKind::GizmoAxis,
                0b001,
                x_col,
                Vec3::X,
            ),
            (
                GIZMO_TRANSLATE_Y,
                PickKind::GizmoAxis,
                0b001,
                y_col,
                Vec3::Y,
            ),
            (
                GIZMO_TRANSLATE_Z,
                PickKind::GizmoAxis,
                0b001,
                z_col,
                Vec3::Z,
            ),
            (GIZMO_SCALE_X, PickKind::GizmoAxis, 0b100, x_col, Vec3::X),
            (GIZMO_SCALE_Y, PickKind::GizmoAxis, 0b100, y_col, Vec3::Y),
            (GIZMO_SCALE_Z, PickKind::GizmoAxis, 0b100, z_col, Vec3::Z),
        ];
        for (id, kind, mask, color, axis) in shaft_specs {
            let segments = vec![(axis * 0.0, axis * 0.94)];
            if let Some(h) = Self::add_line_handle(
                engine,
                scene,
                entity_manager,
                material,
                layer_overlay_value,
                id,
                kind,
                mask,
                Mat3::IDENTITY,
                false,
                true,
                false,
                0.95,
                color,
                segments,
                SHAFT_LINE_WIDTH_PX,
                false,
            ) {
                out.push(h);
            }
        }
        let head_specs = [
            (
                GIZMO_TRANSLATE_X,
                PickKind::GizmoAxis,
                0b001,
                false,
                true,
                0.95,
                create_pyramid_mesh(engine, [1.0, 0.0, 0.0], 1, 0.12, 0.045, x_col),
            ),
            (
                GIZMO_TRANSLATE_Y,
                PickKind::GizmoAxis,
                0b001,
                false,
                true,
                0.95,
                create_pyramid_mesh(engine, [0.0, 1.0, 0.0], 2, 0.12, 0.045, y_col),
            ),
            (
                GIZMO_TRANSLATE_Z,
                PickKind::GizmoAxis,
                0b001,
                false,
                true,
                0.95,
                create_pyramid_mesh(engine, [0.0, 0.0, 1.0], 3, 0.12, 0.045, z_col),
            ),
            (
                GIZMO_SCALE_X,
                PickKind::GizmoAxis,
                0b100,
                false,
                true,
                0.95,
                create_box_mesh(engine, [0.98, 0.0, 0.0], [0.08, 0.08, 0.08], x_col),
            ),
            (
                GIZMO_SCALE_Y,
                PickKind::GizmoAxis,
                0b100,
                false,
                true,
                0.95,
                create_box_mesh(engine, [0.0, 0.98, 0.0], [0.08, 0.08, 0.08], y_col),
            ),
            (
                GIZMO_SCALE_Z,
                PickKind::GizmoAxis,
                0b100,
                false,
                true,
                0.95,
                create_box_mesh(engine, [0.0, 0.0, 0.98], [0.08, 0.08, 0.08], z_col),
            ),
        ];
        for (id, kind, mask, bb, pickable, base_alpha, mesh) in head_specs {
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
                bb,
                pickable,
                false,
                base_alpha,
            ) {
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
            (
                id,
                PickKind::GizmoPlane,
                mode,
                Mat3::IDENTITY,
                false,
                true,
                0.85,
                create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], color),
            )
        };
        let specs = [
            add(GIZMO_TRANSLATE_XY, [255, 255, 80, 180], 0b001),
            add(GIZMO_SCALE_XY, [255, 255, 80, 180], 0b100),
            (
                GIZMO_TRANSLATE_XZ,
                PickKind::GizmoPlane,
                0b001,
                Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2),
                false,
                true,
                0.85,
                create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [255, 120, 80, 180]),
            ),
            (
                GIZMO_SCALE_XZ,
                PickKind::GizmoPlane,
                0b100,
                Mat3::from_rotation_x(std::f32::consts::FRAC_PI_2),
                false,
                true,
                0.85,
                create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [255, 120, 80, 180]),
            ),
            (
                GIZMO_TRANSLATE_YZ,
                PickKind::GizmoPlane,
                0b001,
                Mat3::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                false,
                true,
                0.85,
                create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [80, 255, 255, 180]),
            ),
            (
                GIZMO_SCALE_YZ,
                PickKind::GizmoPlane,
                0b100,
                Mat3::from_rotation_y(-std::f32::consts::FRAC_PI_2),
                false,
                true,
                0.85,
                create_quad_mesh(engine, [0.34, 0.34, 0.0], [0.20, 0.20], [80, 255, 255, 180]),
            ),
        ];
        for (id, kind, mask, rot, bb, pickable, base_alpha, mesh) in specs {
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
                rot,
                bb,
                pickable,
                false,
                base_alpha,
            ) {
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
        let clipped_specs = [
            (
                GIZMO_ROTATE_X,
                PickKind::GizmoRing,
                0b010,
                false,
                true,
                1.0,
                [214, 128, 128, 200],
                create_ring_segments(
                    1.10,
                    128,
                    Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2),
                ),
            ),
            (
                GIZMO_ROTATE_Y,
                PickKind::GizmoRing,
                0b010,
                false,
                true,
                1.0,
                [142, 196, 142, 200],
                create_ring_segments(
                    1.10,
                    128,
                    Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_2),
                ),
            ),
            (
                GIZMO_ROTATE_Z,
                PickKind::GizmoRing,
                0b010,
                false,
                true,
                1.0,
                [132, 162, 206, 200],
                create_ring_segments(1.10, 128, Mat3::IDENTITY),
            ),
        ];
        for (id, kind, mask, billboard, pickable, base_alpha, color, segments) in clipped_specs {
            if let Some(h) = Self::add_line_handle(
                engine,
                scene,
                entity_manager,
                rotate_material,
                layer_overlay_value,
                id,
                kind,
                mask,
                Mat3::IDENTITY,
                billboard,
                pickable,
                true,
                base_alpha,
                color,
                segments,
                RING_LINE_WIDTH_PX,
                true,
            ) {
                out.push(h);
            }
        }
        let regular_specs = [(
            GIZMO_ROTATE_VIEW,
            PickKind::GizmoRing,
            0b010,
            true,
            true,
            0.75,
            [150, 150, 150, 170],
            create_ring_segments(1.22, 128, Mat3::IDENTITY),
        )];
        for (id, kind, mask, billboard, pickable, base_alpha, color, segments) in regular_specs {
            if let Some(h) = Self::add_line_handle(
                engine,
                scene,
                entity_manager,
                material,
                layer_overlay_value,
                id,
                kind,
                mask,
                Mat3::IDENTITY,
                billboard,
                pickable,
                false,
                base_alpha,
                color,
                segments,
                RING_LINE_WIDTH_PX,
                false,
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
        let center_move_mesh = create_box_mesh(
            engine,
            [0.0, 0.0, 0.0],
            [0.09, 0.09, 0.09],
            [236, 236, 236, 220],
        );
        if let Some(h) = Self::add_handle(
            engine,
            scene,
            entity_manager,
            material,
            layer_overlay_value,
            center_move_mesh,
            GIZMO_TRANSLATE_SCREEN,
            PickKind::GizmoPlane,
            0b001,
            Mat3::IDENTITY,
            true,
            true,
            false,
            0.90,
        ) {
            out.push(h);
        }
        if let Some(h) = Self::add_line_handle(
            engine,
            scene,
            entity_manager,
            material,
            layer_overlay_value,
            GIZMO_SCALE_UNIFORM,
            PickKind::GizmoRing,
            0b100,
            Mat3::IDENTITY,
            true,
            true,
            false,
            0.75,
            [150, 150, 150, 170],
            create_ring_segments(1.22, 128, Mat3::IDENTITY),
            RING_LINE_WIDTH_PX,
            false,
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
        let plane_start_dot = 20.0f32.to_radians().sin();
        let plane_end_dot = 10.0f32.to_radians().sin();
        let fade_from_plane_normal_dot = |dot_abs: f32| -> f32 {
            if dot_abs >= plane_start_dot {
                1.0
            } else if dot_abs <= plane_end_dot {
                0.0
            } else {
                (dot_abs - plane_end_dot) / (plane_start_dot - plane_end_dot)
            }
        };
        let fade_others_from_head_on = |dot_abs: f32| -> f32 {
            if dot_abs <= start_dot {
                1.0
            } else if dot_abs >= end_dot {
                0.0
            } else {
                1.0 - ((dot_abs - start_dot) / (end_dot - start_dot))
            }
        };
        // Rotate ring clipping becomes degenerate near exact axis alignment.
        // Fade the "winner" ring out in a narrow band to avoid a hard pop.
        let rotate_fade_start_dot = 10.0f32.to_radians().cos();
        let rotate_fade_end_dot = 2.0f32.to_radians().cos();
        let fade_winner_near_axis = |dot_abs: f32| -> f32 {
            if dot_abs <= rotate_fade_start_dot {
                1.0
            } else if dot_abs >= rotate_fade_end_dot {
                0.0
            } else {
                1.0 - ((dot_abs - rotate_fade_start_dot)
                    / (rotate_fade_end_dot - rotate_fade_start_dot))
            }
        };

        if matches!(handle_id, GIZMO_ROTATE_X | GIZMO_ROTATE_Y | GIZMO_ROTATE_Z) {
            let dx = view_dir.dot(Vec3::X).abs();
            let dy = view_dir.dot(Vec3::Y).abs();
            let dz = view_dir.dot(Vec3::Z).abs();
            let max_dot = dx.max(dy).max(dz);
            let winner = match handle_id {
                GIZMO_ROTATE_X => dx >= dy && dx >= dz,
                GIZMO_ROTATE_Y => dy >= dx && dy >= dz,
                GIZMO_ROTATE_Z => dz >= dx && dz >= dy,
                _ => false,
            };
            if winner {
                let winner_dot = match handle_id {
                    GIZMO_ROTATE_X => dx,
                    GIZMO_ROTATE_Y => dy,
                    GIZMO_ROTATE_Z => dz,
                    _ => 0.0,
                };
                return fade_winner_near_axis(winner_dot);
            }
            return fade_others_from_head_on(max_dot);
        }
        if let Some(axis) = Self::handle_axis(handle_id) {
            let dot_abs = view_dir.dot(axis).abs();
            return fade_from_dot(dot_abs);
        }
        if let Some(normal) = Self::handle_plane_normal(handle_id) {
            let dot_abs = view_dir.dot(normal).abs();
            return fade_from_plane_normal_dot(dot_abs);
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

    fn handle_plane_normal(handle_id: i32) -> Option<Vec3> {
        match handle_id {
            GIZMO_TRANSLATE_XY | GIZMO_SCALE_XY => Some(Vec3::Z),
            GIZMO_TRANSLATE_XZ | GIZMO_SCALE_XZ => Some(Vec3::Y),
            GIZMO_TRANSLATE_YZ | GIZMO_SCALE_YZ => Some(Vec3::X),
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
    Some(MeshResource {
        vertex: vb,
        index: ib,
    })
}

fn create_box_mesh(
    engine: &mut Engine,
    center: [f32; 3],
    size: [f32; 3],
    color: [u8; 4],
) -> MeshResource {
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
        0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7, 3,
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
    let indices: [u16; 18] = [0, 1, 2, 0, 2, 3, 0, 3, 4, 0, 4, 1, 1, 4, 3, 1, 3, 2];
    create_mesh(engine, &positions, &colors, &indices).expect("pyramid mesh")
}

fn create_quad_mesh(
    engine: &mut Engine,
    center: [f32; 3],
    size: [f32; 2],
    color: [u8; 4],
) -> MeshResource {
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

fn create_line_quad_mesh(engine: &mut Engine, segments: usize, color: [u8; 4]) -> MeshResource {
    let count = segments.max(1);
    let positions = vec![[0.0, 0.0, 0.0]; count * 4];
    let colors = vec![color; count * 4];
    let mut indices = Vec::with_capacity(count * 6);
    for i in 0..count {
        let base = (i * 4) as u16;
        indices.extend_from_slice(&[base, base + 2, base + 3, base, base + 3, base + 1]);
    }
    create_mesh(engine, &positions, &colors, &indices).expect("ring mesh")
}

fn create_ring_segments(radius: f32, segments: usize, rotation: Mat3) -> Vec<(Vec3, Vec3)> {
    let n = segments.max(16);
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let t = (i as f32 / n as f32) * std::f32::consts::TAU;
        let p = rotation * Vec3::new(radius * t.cos(), radius * t.sin(), 0.0);
        points.push(p);
    }
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push((points[i], points[(i + 1) % n]));
    }
    out
}

fn clip_segment_to_halfspace(
    a: &mut Vec3,
    b: &mut Vec3,
    center: Vec3,
    normal: Vec3,
    bias: f32,
) -> bool {
    if normal.length_squared() <= 1e-8 {
        return true;
    }
    let d0 = (*a - center).dot(normal);
    let d1 = (*b - center).dot(normal);
    if d0 < bias && d1 < bias {
        return false;
    }
    if d0 >= bias && d1 >= bias {
        return true;
    }
    let denom = d1 - d0;
    if denom.abs() <= 1e-8 {
        return d0 >= bias || d1 >= bias;
    }
    let t = ((bias - d0) / denom).clamp(0.0, 1.0);
    let p = *a + (*b - *a) * t;
    if d0 < bias {
        *a = p;
    } else {
        *b = p;
    }
    true
}

fn write_segment_quad(
    positions: &mut [[f32; 3]],
    slot: usize,
    a: Vec3,
    b: Vec3,
    camera_position: Vec3,
    camera_up: Vec3,
    camera_right: Vec3,
    fov_y_radians: f32,
    viewport_height: f32,
    pixel_width: f32,
) -> bool {
    let segment = b - a;
    let segment_length = segment.length();
    if segment_length <= 1e-6 {
        return false;
    }
    let segment_dir = segment / segment_length;
    let midpoint = (a + b) * 0.5;
    let view_dir = (camera_position - midpoint).normalize_or_zero();
    let mut side = view_dir.cross(segment_dir);
    if side.length_squared() <= 1e-8 {
        side = camera_up.cross(segment_dir);
    }
    if side.length_squared() <= 1e-8 {
        side = camera_right.cross(segment_dir);
    }
    if side.length_squared() <= 1e-8 {
        return false;
    }
    side = side.normalize();

    let world_per_pixel_a =
        2.0 * (camera_position - a).length().max(0.01) * (fov_y_radians * 0.5).tan()
            / viewport_height.max(1.0);
    let world_per_pixel_b =
        2.0 * (camera_position - b).length().max(0.01) * (fov_y_radians * 0.5).tan()
            / viewport_height.max(1.0);
    let offset_a = side * (pixel_width * 0.5 * world_per_pixel_a);
    let offset_b = side * (pixel_width * 0.5 * world_per_pixel_b);

    let base = slot * 4;
    positions[base] = (a + offset_a).to_array();
    positions[base + 1] = (a - offset_a).to_array();
    positions[base + 2] = (b + offset_b).to_array();
    positions[base + 3] = (b - offset_b).to_array();
    true
}
