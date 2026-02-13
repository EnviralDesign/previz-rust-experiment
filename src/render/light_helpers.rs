use crate::filament::{
    ElementType, Engine, Entity, EntityManager, IndexBuffer, IndexType, Material, MaterialInstance,
    PrimitiveType, Scene, VertexAttribute, VertexBuffer,
};
use crate::render::{PickKey, PickKind};
use crate::scene::LightType;
use glam::{Mat4, Quat, Vec3};
use std::collections::{HashMap, HashSet};

struct MeshResource {
    vertex: VertexBuffer,
    index: IndexBuffer,
}

struct LightHelperEntry {
    entity: Entity,
    light_type: LightType,
    material_instance: MaterialInstance,
    _mesh: MeshResource,
}

#[derive(Debug, Clone, Copy)]
pub struct LightHelperSpec {
    pub object_id: u64,
    pub object_index: u32,
    pub light_type: LightType,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub selected: bool,
}

pub struct LightHelperSystem {
    _material: Material,
    helpers: HashMap<u64, LightHelperEntry>,
    layer_overlay_value: u8,
    layer_hidden_value: u8,
}

impl LightHelperSystem {
    pub fn new(
        engine: &mut Engine,
        _scene: &mut Scene,
        _entity_manager: &mut EntityManager,
        layer_overlay_value: u8,
    ) -> Option<Self> {
        let base_package = include_bytes!(concat!(env!("OUT_DIR"), "/bakedColor.filamat"));
        let material = engine.create_material(base_package)?;
        Some(Self {
            _material: material,
            helpers: HashMap::new(),
            layer_overlay_value,
            layer_hidden_value: 0x00,
        })
    }

    pub fn sync(
        &mut self,
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        specs: &[LightHelperSpec],
        camera_position: [f32; 3],
    ) {
        let mut seen: HashSet<u64> = HashSet::new();
        for spec in specs {
            seen.insert(spec.object_id);
            let needs_recreate = self
                .helpers
                .get(&spec.object_id)
                .map(|entry| entry.light_type != spec.light_type)
                .unwrap_or(true);
            if needs_recreate {
                if let Some(entry) = self.create_entry(engine, scene, entity_manager, spec.light_type)
                {
                    self.helpers.insert(spec.object_id, entry);
                }
            }
            let Some(entry) = self.helpers.get_mut(&spec.object_id) else {
                continue;
            };
            update_entry(
                engine,
                entry,
                spec,
                camera_position,
                self.layer_overlay_value,
            );
        }
        for (object_id, entry) in &self.helpers {
            if !seen.contains(object_id) {
                engine.renderable_set_layer_mask(entry.entity, 0xFF, self.layer_hidden_value);
            }
        }
    }

    pub fn pickables(&self, specs: &[LightHelperSpec]) -> Vec<(PickKey, Vec<Entity>)> {
        let mut out = Vec::new();
        for spec in specs {
            let Some(entry) = self.helpers.get(&spec.object_id) else {
                continue;
            };
            let sub_id = match spec.light_type {
                LightType::Directional => 1,
                LightType::Sun => 2,
                LightType::Point => 3,
                LightType::Spot => 4,
                LightType::FocusedSpot => 5,
            };
            out.push((
                PickKey::new(PickKind::LightHelper, spec.object_index.min(0xFFFFF), sub_id),
                vec![entry.entity],
            ));
        }
        out
    }

    fn create_entry(
        &mut self,
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        light_type: LightType,
    ) -> Option<LightHelperEntry> {
        let mut instance = self._material.create_instance()?;
        instance.set_float4("tint", [1.0, 1.0, 1.0, 0.88]);
        let mut mesh = create_light_helper_mesh(engine, light_type)?;
        let entity = entity_manager.create();
        let builder = engine
            .renderable_builder(1)
            .bounding_box([0.0, 0.0, 0.0], [2.0, 2.0, 2.0])
            .material(0, &mut instance)
            .geometry(0, PrimitiveType::Triangles, &mut mesh.vertex, &mut mesh.index)
            .layer_mask(0xFF, self.layer_overlay_value)
            .culling(false);
        builder.build(entity);
        scene.add_entity(entity);
        Some(LightHelperEntry {
            entity,
            light_type,
            material_instance: instance,
            _mesh: mesh,
        })
    }
}

fn update_entry(
    engine: &mut Engine,
    entry: &mut LightHelperEntry,
    spec: &LightHelperSpec,
    camera_position: [f32; 3],
    layer_overlay_value: u8,
) {
    let position = Vec3::from_array(spec.position);
    let camera = Vec3::from_array(camera_position);
    let distance = (position - camera).length().max(0.1);
    let scale = (distance * 0.075).clamp(0.12, 3.0);
    let direction = normalized_direction(spec.direction);
    let orientation = if uses_direction(spec.light_type) {
        let forward_axis = match spec.light_type {
            LightType::Spot | LightType::FocusedSpot => Vec3::NEG_Y,
            _ => Vec3::Y,
        };
        Quat::from_rotation_arc(forward_axis, direction)
    } else {
        Quat::IDENTITY
    };
    let world =
        Mat4::from_translation(position) * Mat4::from_quat(orientation) * Mat4::from_scale(Vec3::splat(scale));
    if let Some(mut tm) = engine.transform_manager() {
        tm.set_transform(entry.entity, &world.to_cols_array());
    }
    let base_color = light_type_color(spec.light_type);
    let boost = if spec.selected { 1.20 } else { 1.0 };
    entry.material_instance.set_float4(
        "tint",
        [
            (base_color[0] * boost).clamp(0.0, 1.5),
            (base_color[1] * boost).clamp(0.0, 1.5),
            (base_color[2] * boost).clamp(0.0, 1.5),
            if spec.selected { 1.0 } else { 0.88 },
        ],
    );
    engine.renderable_set_layer_mask(entry.entity, 0xFF, layer_overlay_value);
}

fn uses_direction(light_type: LightType) -> bool {
    !matches!(light_type, LightType::Point)
}

fn light_type_color(light_type: LightType) -> [f32; 3] {
    match light_type {
        LightType::Directional => [1.0, 0.92, 0.50],
        LightType::Sun => [1.0, 0.78, 0.45],
        LightType::Point => [0.45, 0.90, 1.0],
        LightType::Spot => [0.95, 0.65, 0.40],
        LightType::FocusedSpot => [0.55, 1.0, 0.60],
    }
}

fn normalized_direction(direction: [f32; 3]) -> Vec3 {
    let v = Vec3::from_array(direction);
    if v.length_squared() <= 1e-10 {
        Vec3::NEG_Y
    } else {
        v.normalize()
    }
}

fn create_light_helper_mesh(engine: &mut Engine, light_type: LightType) -> Option<MeshResource> {
    match light_type {
        LightType::Directional | LightType::Sun => create_arrow_mesh(engine),
        LightType::Point => create_octahedron_mesh(engine),
        LightType::Spot => create_cone_mesh(engine, 0.35, 1.0, 16),
        LightType::FocusedSpot => create_cone_mesh(engine, 0.22, 1.0, 16),
    }
}

fn create_mesh(engine: &mut Engine, positions: &[[f32; 3]], indices: &[u16]) -> Option<MeshResource> {
    if positions.is_empty() || indices.is_empty() {
        return None;
    }
    let colors = vec![[255u8, 255u8, 255u8, 255u8]; positions.len()];
    let mut vb = engine
        .vertex_buffer_builder()
        .vertex_count(positions.len() as u32)
        .buffer_count(2)
        .attribute(VertexAttribute::Position, 0, ElementType::Float3, 0, 12)
        .attribute(VertexAttribute::Color, 1, ElementType::UByte4, 0, 4)
        .normalized(VertexAttribute::Color, true)
        .build()?;
    vb.set_buffer_at(0, positions, 0);
    vb.set_buffer_at(1, &colors, 0);

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

fn create_octahedron_mesh(engine: &mut Engine) -> Option<MeshResource> {
    let positions: [[f32; 3]; 6] = [
        [0.0, 0.6, 0.0],
        [0.0, -0.6, 0.0],
        [0.6, 0.0, 0.0],
        [-0.6, 0.0, 0.0],
        [0.0, 0.0, 0.6],
        [0.0, 0.0, -0.6],
    ];
    let indices: [u16; 24] = [
        0, 2, 4, 0, 4, 3, 0, 3, 5, 0, 5, 2, 1, 4, 2, 1, 3, 4, 1, 5, 3, 1, 2, 5,
    ];
    create_mesh(engine, &positions, &indices)
}

fn create_cone_mesh(
    engine: &mut Engine,
    base_radius: f32,
    height: f32,
    segments: usize,
) -> Option<MeshResource> {
    let n = segments.max(8);
    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n + 2);
    let mut indices: Vec<u16> = Vec::with_capacity(n * 6);

    // Place tip at origin so gizmo transform origin matches spotlight source position.
    let tip_index = 0u16;
    positions.push([0.0, 0.0, 0.0]);
    let base_center_index = 1u16;
    positions.push([0.0, -height, 0.0]);

    for i in 0..n {
        let t = (i as f32 / n as f32) * std::f32::consts::TAU;
        positions.push([base_radius * t.cos(), -height, base_radius * t.sin()]);
    }

    for i in 0..n {
        let next = (i + 1) % n;
        let a = (2 + i) as u16;
        let b = (2 + next) as u16;
        indices.extend_from_slice(&[tip_index, a, b]);
        indices.extend_from_slice(&[base_center_index, b, a]);
    }
    create_mesh(engine, &positions, &indices)
}

fn create_arrow_mesh(engine: &mut Engine) -> Option<MeshResource> {
    let shaft_half = 0.06f32;
    let shaft_height = 0.65f32;
    let head_radius = 0.18f32;
    let head_height = 0.35f32;

    let mut positions: Vec<[f32; 3]> = vec![
        [-shaft_half, 0.0, -shaft_half],
        [shaft_half, 0.0, -shaft_half],
        [shaft_half, shaft_height, -shaft_half],
        [-shaft_half, shaft_height, -shaft_half],
        [-shaft_half, 0.0, shaft_half],
        [shaft_half, 0.0, shaft_half],
        [shaft_half, shaft_height, shaft_half],
        [-shaft_half, shaft_height, shaft_half],
    ];
    let mut indices: Vec<u16> = vec![
        0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7,
        3, 3, 7, 4, 3, 4, 0,
    ];

    let start_index = positions.len() as u16;
    positions.push([0.0, shaft_height + head_height, 0.0]);
    let tip = start_index;
    let base_center = start_index + 1;
    positions.push([0.0, shaft_height, 0.0]);
    let segments = 12usize;
    for i in 0..segments {
        let t = (i as f32 / segments as f32) * std::f32::consts::TAU;
        positions.push([
            head_radius * t.cos(),
            shaft_height,
            head_radius * t.sin(),
        ]);
    }
    for i in 0..segments {
        let next = (i + 1) % segments;
        let a = start_index + 2 + i as u16;
        let b = start_index + 2 + next as u16;
        indices.extend_from_slice(&[tip, a, b]);
        indices.extend_from_slice(&[base_center, b, a]);
    }
    create_mesh(engine, &positions, &indices)
}
