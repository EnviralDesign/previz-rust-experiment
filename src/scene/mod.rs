pub mod serialization;

use crate::assets::LoadedAsset;
use crate::filament::Entity;

/// Asset-specific data - matches what can be edited in UI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssetData {
    pub path: String,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
}

/// Directional light-specific data - matches what can be edited in UI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DirectionalLightData {
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
}

/// Environment-specific data - matches what can be edited in UI
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnvironmentData {
    pub hdr_path: String,
    pub ibl_path: String,
    pub skybox_path: String,
    pub intensity: f32,
}

/// Runtime object in the scene - contains editable parameters + runtime entity reference
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneObject {
    pub name: String,
    pub kind: SceneObjectKind,
    // Runtime entity reference (not serialized - regenerated on load)
    #[serde(skip)]
    pub root_entity: Option<Entity>,
    // Bounding box (for assets, computed at load time)
    #[serde(skip)]
    pub center: [f32; 3],
    #[serde(skip)]
    pub extent: [f32; 3],
}

/// Type-specific editable data - this is what gets saved/loaded
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SceneObjectKind {
    Asset(AssetData),
    DirectionalLight(DirectionalLightData),
    Environment(EnvironmentData),
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SceneState {
    objects: Vec<SceneObject>,
}

impl SceneState {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub fn object_names(&self) -> Vec<&str> {
        self.objects
            .iter()
            .map(|object| object.name.as_str())
            .collect()
    }

    pub fn object_mut(&mut self, index: usize) -> Option<&mut SceneObject> {
        self.objects.get_mut(index)
    }

    pub fn add_asset(&mut self, asset: &LoadedAsset, path: &str) {
        self.objects.push(SceneObject {
            name: asset.name.clone(),
            kind: SceneObjectKind::Asset(AssetData {
                path: path.to_string(),
                position: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }),
            center: asset.center,
            extent: asset.extent,
            root_entity: Some(asset.root_entity),
        });
    }

    pub fn add_directional_light(
        &mut self,
        name: &str,
        entity: Entity,
        data: DirectionalLightData,
    ) {
        self.objects.push(SceneObject {
            name: name.to_string(),
            kind: SceneObjectKind::DirectionalLight(data),
            center: [0.0, 0.0, 0.0],
            extent: [0.0, 0.0, 0.0],
            root_entity: Some(entity),
        });
    }

    pub fn set_environment(&mut self, data: EnvironmentData) {
        let index = self
            .objects
            .iter()
            .position(|object| matches!(object.kind, SceneObjectKind::Environment(_)));
        match index {
            Some(idx) => {
                // Update existing environment
                if let SceneObjectKind::Environment(existing) = &mut self.objects[idx].kind {
                    *existing = data;
                }
            }
            None => {
                // Add new environment
                self.objects.push(SceneObject {
                    name: "Environment".to_string(),
                    kind: SceneObjectKind::Environment(data),
                    center: [0.0, 0.0, 0.0],
                    extent: [0.0, 0.0, 0.0],
                    root_entity: None,
                });
            }
        }
    }

    #[cfg(test)]
    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.push(object);
    }

    pub fn replace_objects(&mut self, objects: Vec<SceneObject>) {
        self.objects = objects;
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
        r00 * sx,
        r10 * sx,
        r20 * sx,
        0.0,
        r01 * sy,
        r11 * sy,
        r21 * sy,
        0.0,
        r02 * sz,
        r12 * sz,
        r22 * sz,
        0.0,
        position[0],
        position[1],
        position[2],
        1.0,
    ]
}
