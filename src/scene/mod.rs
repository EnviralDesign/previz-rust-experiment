use crate::assets::LoadedAsset;
use crate::filament::Entity;

#[derive(Debug, Clone)]
pub struct SceneObject {
    pub name: String,
    pub center: [f32; 3],
    pub extent: [f32; 3],
    pub root_entity: Entity,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
}

#[derive(Default)]
pub struct SceneState {
    objects: Vec<SceneObject>,
}

impl SceneState {
    pub fn new() -> Self {
        Self { objects: Vec::new() }
    }

    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub fn object_names(&self) -> Vec<&str> {
        self.objects.iter().map(|object| object.name.as_str()).collect()
    }

    pub fn object_mut(&mut self, index: usize) -> Option<&mut SceneObject> {
        self.objects.get_mut(index)
    }

    pub fn add_asset(&mut self, asset: &LoadedAsset) {
        self.objects.push(SceneObject {
            name: asset.name.clone(),
            center: asset.center,
            extent: asset.extent,
            root_entity: asset.root_entity,
            position: [0.0, 0.0, 0.0],
            rotation_deg: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        });
    }
}

pub fn compose_transform_matrix(
    position: [f32; 3],
    rotation_deg: [f32; 3],
    scale: [f32; 3],
) -> [f32; 16] {
    let (rx, ry, rz) = (
        rotation_deg[0].to_radians(),
        rotation_deg[1].to_radians(),
        rotation_deg[2].to_radians(),
    );
    let (sx, cx) = rx.sin_cos();
    let (sy, cy) = ry.sin_cos();
    let (sz, cz) = rz.sin_cos();

    // Rotation order: Z (roll) * Y (yaw) * X (pitch)
    let r00 = cz * cy;
    let r01 = cz * sy * sx - sz * cx;
    let r02 = cz * sy * cx + sz * sx;
    let r10 = sz * cy;
    let r11 = sz * sy * sx + cz * cx;
    let r12 = sz * sy * cx - cz * sx;
    let r20 = -sy;
    let r21 = cy * sx;
    let r22 = cy * cx;

    let (sx, sy, sz) = (scale[0], scale[1], scale[2]);
    [
        r00 * sx, r10 * sx, r20 * sx, 0.0,
        r01 * sy, r11 * sy, r21 * sy, 0.0,
        r02 * sz, r12 * sz, r22 * sz, 0.0,
        position[0], position[1], position[2], 1.0,
    ]
}
