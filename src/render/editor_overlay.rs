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

#[derive(Clone, Copy)]
pub struct GizmoParams {
    pub visible: bool,
    pub mode: i32,
    pub origin: [f32; 3],
    pub axis_world_len: f32,
    pub camera_forward: [f32; 3],
    pub camera_up: [f32; 3],
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
    _mesh: MeshResource,
    _material_instance: MaterialInstance,
}

pub struct EditorOverlay {
    _material: Material,
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
        let package = include_bytes!(concat!(env!("OUT_DIR"), "/bakedColor.filamat"));
        let mut material = engine.create_material(package)?;

        let mut handles = Vec::new();
        handles.extend(Self::create_axis_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));
        handles.extend(Self::create_plane_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));
        handles.extend(Self::create_rotation_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));
        handles.extend(Self::create_misc_handles(engine, scene, entity_manager, &mut material, layer_overlay_value));

        Some(Self {
            _material: material,
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
            out.push((
                PickKey::new(handle.kind, object_id.min(0xFFFFF), handle.handle_id as u8),
                vec![handle.entity],
            ));
        }
        out
    }

    fn update_handle_visibility(&self, engine: &mut Engine) {
        for handle in &self.handles {
            let visible = self.params.visible && self.is_handle_mode_visible(handle.handle_id);
            let value = if visible {
                self.layer_overlay_value
            } else {
                self.layer_hidden_value
            };
            engine.renderable_set_layer_mask(handle.entity, 0xFF, value);
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
    ) -> Option<HandleEntity> {
        let mut mi = material.create_instance()?;
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
            _mesh: mesh,
            _material_instance: mi,
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
        let axis_mesh_x = create_box_mesh(engine, [0.5, 0.0, 0.0], [1.0, 0.03, 0.03], [255, 80, 80, 255]);
        let axis_mesh_y = create_box_mesh(engine, [0.0, 0.5, 0.0], [0.03, 1.0, 0.03], [80, 255, 80, 255]);
        let axis_mesh_z = create_box_mesh(engine, [0.0, 0.0, 0.5], [0.03, 0.03, 1.0], [80, 160, 255, 255]);
        let data = [
            (GIZMO_TRANSLATE_X, PickKind::GizmoAxis, 0b001, Mat3::IDENTITY, false, true, axis_mesh_x),
            (GIZMO_TRANSLATE_Y, PickKind::GizmoAxis, 0b001, Mat3::IDENTITY, false, true, axis_mesh_y),
            (GIZMO_TRANSLATE_Z, PickKind::GizmoAxis, 0b001, Mat3::IDENTITY, false, true, axis_mesh_z),
            (GIZMO_SCALE_X, PickKind::GizmoAxis, 0b100, Mat3::IDENTITY, false, true, create_box_mesh(engine, [0.5, 0.0, 0.0], [1.0, 0.03, 0.03], [255, 80, 80, 255])),
            (GIZMO_SCALE_Y, PickKind::GizmoAxis, 0b100, Mat3::IDENTITY, false, true, create_box_mesh(engine, [0.0, 0.5, 0.0], [0.03, 1.0, 0.03], [80, 255, 80, 255])),
            (GIZMO_SCALE_Z, PickKind::GizmoAxis, 0b100, Mat3::IDENTITY, false, true, create_box_mesh(engine, [0.0, 0.0, 0.5], [0.03, 0.03, 1.0], [80, 160, 255, 255])),
        ];
        for (id, kind, mask, rot, bb, pickable, mesh) in data {
            if let Some(h) = Self::add_handle(engine, scene, entity_manager, material, layer_overlay_value, mesh, id, kind, mask, rot, bb, pickable) {
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
            (id, PickKind::GizmoPlane, mode, Mat3::IDENTITY, false, true, create_quad_mesh(engine, [0.22, 0.22, 0.0], [0.24, 0.24], color))
        };
        let specs = [
            add(GIZMO_TRANSLATE_XY, [255, 255, 80, 180], 0b001),
            add(GIZMO_SCALE_XY, [255, 255, 80, 180], 0b100),
            (GIZMO_TRANSLATE_XZ, PickKind::GizmoPlane, 0b001, Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_2), false, true, create_quad_mesh(engine, [0.22, 0.0, 0.22], [0.24, 0.24], [255, 120, 80, 180])),
            (GIZMO_SCALE_XZ, PickKind::GizmoPlane, 0b100, Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_2), false, true, create_quad_mesh(engine, [0.22, 0.0, 0.22], [0.24, 0.24], [255, 120, 80, 180])),
            (GIZMO_TRANSLATE_YZ, PickKind::GizmoPlane, 0b001, Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2), false, true, create_quad_mesh(engine, [0.0, 0.22, 0.22], [0.24, 0.24], [80, 255, 255, 180])),
            (GIZMO_SCALE_YZ, PickKind::GizmoPlane, 0b100, Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2), false, true, create_quad_mesh(engine, [0.0, 0.22, 0.22], [0.24, 0.24], [80, 255, 255, 180])),
        ];
        for (id, kind, mask, rot, bb, pickable, mesh) in specs {
            if let Some(h) = Self::add_handle(engine, scene, entity_manager, material, layer_overlay_value, mesh, id, kind, mask, rot, bb, pickable) {
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
        layer_overlay_value: u8,
    ) -> Vec<HandleEntity> {
        let mut out = Vec::new();
        let ring_x = create_ring_mesh(engine, 1.10, 0.012, [255, 80, 80, 220], 64, Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2));
        let ring_y = create_ring_mesh(engine, 1.10, 0.012, [80, 255, 80, 220], 64, Mat3::from_rotation_x(-std::f32::consts::FRAC_PI_2));
        let ring_z = create_ring_mesh(engine, 1.10, 0.012, [80, 160, 255, 220], 64, Mat3::IDENTITY);
        let view_ring = create_ring_mesh(engine, 1.22, 0.01, [230, 230, 230, 200], 64, Mat3::IDENTITY);
        let arcball_disk = create_disk_mesh(engine, 0.86, [255, 255, 255, 28], 48);
        let specs = [
            (GIZMO_ROTATE_X, PickKind::GizmoRing, 0b010, false, true, ring_x),
            (GIZMO_ROTATE_Y, PickKind::GizmoRing, 0b010, false, true, ring_y),
            (GIZMO_ROTATE_Z, PickKind::GizmoRing, 0b010, false, true, ring_z),
            (GIZMO_ROTATE_VIEW, PickKind::GizmoRing, 0b010, true, true, view_ring),
            (GIZMO_ROTATE_ARCBALL, PickKind::GizmoRing, 0b010, true, true, arcball_disk),
        ];
        for (id, kind, mask, billboard, pickable, mesh) in specs {
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
        let mesh = create_box_mesh(engine, [0.0, 0.0, 0.0], [0.10, 0.10, 0.10], [245, 245, 245, 220]);
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
            false,
            true,
        ) {
            out.push(h);
        }
        out
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
