pub mod serialization;

use crate::filament::Entity;

/// Asset-specific data - matches what can be edited in UI
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AssetData {
    pub path: String,
    pub position: [f32; 3],
    pub rotation_deg: [f32; 3],
    pub scale: [f32; 3],
}

/// Directional light-specific data - matches what can be edited in UI
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DirectionalLightData {
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LightType {
    Directional,
    Sun,
    Point,
    Spot,
    FocusedSpot,
}

impl LightType {
    pub fn name_prefix(self) -> &'static str {
        match self {
            Self::Directional => "Directional Light",
            Self::Sun => "Sun Light",
            Self::Point => "Point Light",
            Self::Spot => "Spot Light",
            Self::FocusedSpot => "Focused Spot Light",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LightShadowData {
    #[serde(default = "default_true")]
    pub cast_shadows: bool,
    #[serde(default = "default_shadow_map_size")]
    pub map_size: u32,
    #[serde(default = "default_shadow_cascades")]
    pub cascades: u8,
    #[serde(default)]
    pub shadow_far: f32,
    #[serde(default = "default_shadow_near_hint")]
    pub near_hint: f32,
    #[serde(default = "default_shadow_far_hint")]
    pub far_hint: f32,
}

impl Default for LightShadowData {
    fn default() -> Self {
        Self {
            cast_shadows: true,
            map_size: default_shadow_map_size(),
            cascades: default_shadow_cascades(),
            shadow_far: 0.0,
            near_hint: default_shadow_near_hint(),
            far_hint: default_shadow_far_hint(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LightData {
    pub light_type: LightType,
    pub color: [f32; 3],
    pub intensity: f32,
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation_deg: [f32; 3],
    pub direction: [f32; 3],
    #[serde(default = "default_light_range")]
    pub range: f32,
    #[serde(default = "default_spot_inner_deg")]
    pub spot_inner_deg: f32,
    #[serde(default = "default_spot_outer_deg")]
    pub spot_outer_deg: f32,
    #[serde(default = "default_sun_angular_radius_deg")]
    pub sun_angular_radius_deg: f32,
    #[serde(default = "default_sun_halo_size")]
    pub sun_halo_size: f32,
    #[serde(default = "default_sun_halo_falloff")]
    pub sun_halo_falloff: f32,
    #[serde(default)]
    pub shadow: LightShadowData,
}

impl LightData {
    pub fn default_for(light_type: LightType) -> Self {
        let mut data = Self {
            light_type,
            color: [1.0, 1.0, 1.0],
            intensity: 100_000.0,
            position: [0.0, 2.0, 2.0],
            rotation_deg: [0.0, 0.0, 0.0],
            direction: [0.0, -1.0, -0.5],
            range: 10.0,
            spot_inner_deg: 25.0,
            spot_outer_deg: 35.0,
            sun_angular_radius_deg: 0.545,
            sun_halo_size: 10.0,
            sun_halo_falloff: 80.0,
            shadow: LightShadowData::default(),
        };
        if matches!(light_type, LightType::Point) {
            data.shadow.cast_shadows = false;
        }
        data
    }

    pub fn from_legacy_directional(legacy: DirectionalLightData) -> Self {
        let mut light = Self::default_for(LightType::Directional);
        light.color = legacy.color;
        light.intensity = legacy.intensity;
        light.direction = legacy.direction;
        light
    }
}

/// Environment-specific data - matches what can be edited in UI
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EnvironmentData {
    pub hdr_path: String,
    pub ibl_path: String,
    pub skybox_path: String,
    pub intensity: f32,
}

/// Material override data persisted in scene JSON.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterialOverrideData {
    pub base_color_rgba: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive_rgb: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MediaSourceKind {
    Image,
    #[allow(dead_code)]
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TextureColorSpace {
    #[serde(rename = "srgb")]
    Srgb,
    #[serde(rename = "linear")]
    Linear,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterialTextureBindingData {
    pub texture_param: String,
    pub source_kind: MediaSourceKind,
    pub source_path: String,
    #[serde(default)]
    pub runtime_ktx_path: Option<String>,
    #[serde(default)]
    pub source_hash: Option<String>,
    #[serde(default = "default_true")]
    pub wrap_repeat_u: bool,
    #[serde(default = "default_true")]
    pub wrap_repeat_v: bool,
    #[serde(default = "default_texture_color_space")]
    pub color_space: TextureColorSpace,
    #[serde(default)]
    pub uv_offset: [f32; 2],
    #[serde(default = "default_uv_scale")]
    pub uv_scale: [f32; 2],
    #[serde(default)]
    pub uv_rotation_deg: f32,
}

/// Maps a material identity to user-authored override values.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterialOverrideEntry {
    #[serde(default)]
    pub object_id: Option<u64>,
    #[serde(default)]
    pub asset_path: Option<String>,
    #[serde(default)]
    pub material_slot: Option<usize>,
    #[serde(default)]
    pub material_name: String,
    pub data: MaterialOverrideData,
}

/// Serializable scene object.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SceneObject {
    #[serde(default)]
    pub id: u64,
    pub name: String,
    pub kind: SceneObjectKind,
}

/// Type-specific editable data - this is what gets saved/loaded
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum SceneObjectKind {
    Asset(AssetData),
    Light(LightData),
    #[serde(alias = "LegacyDirectionalLight")]
    DirectionalLight(DirectionalLightData),
    Environment(EnvironmentData),
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct SceneState {
    objects: Vec<SceneObject>,
    #[serde(default)]
    material_overrides: Vec<MaterialOverrideEntry>,
    #[serde(default)]
    texture_bindings: Vec<MaterialTextureBindingEntry>,
    #[serde(default = "default_next_object_id")]
    next_object_id: u64,
}

fn default_next_object_id() -> u64 {
    1
}

fn default_true() -> bool {
    true
}

fn default_texture_color_space() -> TextureColorSpace {
    TextureColorSpace::Srgb
}

fn default_uv_scale() -> [f32; 2] {
    [1.0, 1.0]
}

fn default_shadow_map_size() -> u32 {
    1024
}

fn default_shadow_cascades() -> u8 {
    1
}

fn default_shadow_near_hint() -> f32 {
    1.0
}

fn default_shadow_far_hint() -> f32 {
    100.0
}

fn default_light_range() -> f32 {
    10.0
}

fn default_spot_inner_deg() -> f32 {
    25.0
}

fn default_spot_outer_deg() -> f32 {
    35.0
}

fn default_sun_angular_radius_deg() -> f32 {
    0.545
}

fn default_sun_halo_size() -> f32 {
    10.0
}

fn default_sun_halo_falloff() -> f32 {
    80.0
}

#[derive(Debug, Clone, Copy, Default)]
pub struct RuntimeObject {
    pub root_entity: Option<Entity>,
    pub center: [f32; 3],
    pub extent: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MaterialTextureBindingEntry {
    pub object_id: u64,
    pub material_slot: usize,
    pub binding: MaterialTextureBindingData,
}

#[derive(Default)]
pub struct SceneRuntime {
    objects: Vec<RuntimeObject>,
}

impl SceneRuntime {
    pub fn new() -> Self {
        Self { objects: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.objects.clear();
    }

    pub fn push(&mut self, object: RuntimeObject) {
        self.objects.push(object);
    }

    pub fn get(&self, index: usize) -> Option<&RuntimeObject> {
        self.objects.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut RuntimeObject> {
        self.objects.get_mut(index)
    }

    pub fn replace(&mut self, objects: Vec<RuntimeObject>) {
        self.objects = objects;
    }
}

impl SceneState {
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            material_overrides: Vec::new(),
            texture_bindings: Vec::new(),
            next_object_id: default_next_object_id(),
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

    pub fn reserve_object_id(&mut self) -> u64 {
        let id = self.next_object_id.max(1);
        self.next_object_id = id.saturating_add(1);
        id
    }

    pub fn ensure_object_ids(&mut self) {
        let mut next_id = self.next_object_id.max(1);
        let mut max_id = 0u64;
        for object in &mut self.objects {
            if object.id == 0 {
                object.id = next_id;
                next_id = next_id.saturating_add(1);
            }
            max_id = max_id.max(object.id);
        }
        self.next_object_id = next_id.max(max_id.saturating_add(1));
    }

    pub fn material_overrides(&self) -> &[MaterialOverrideEntry] {
        &self.material_overrides
    }

    pub fn texture_bindings(&self) -> &[MaterialTextureBindingEntry] {
        &self.texture_bindings
    }

    pub fn set_material_override(
        &mut self,
        object_id: u64,
        asset_path: String,
        material_slot: usize,
        material_name: String,
        data: MaterialOverrideData,
    ) {
        if let Some(existing) = self
            .material_overrides
            .iter_mut()
            .find(|entry| {
                entry.object_id == Some(object_id) && entry.material_slot == Some(material_slot)
            })
        {
            existing.object_id = Some(object_id);
            existing.asset_path = Some(asset_path);
            existing.material_name = material_name;
            existing.data = data;
            return;
        }
        self.material_overrides.push(MaterialOverrideEntry {
            object_id: Some(object_id),
            asset_path: Some(asset_path),
            material_slot: Some(material_slot),
            material_name,
            data,
        });
    }

    pub fn set_texture_binding(
        &mut self,
        object_id: u64,
        material_slot: usize,
        binding: MaterialTextureBindingData,
    ) {
        if let Some(existing) = self.texture_bindings.iter_mut().find(|entry| {
            entry.object_id == object_id
                && entry.material_slot == material_slot
                && entry.binding.texture_param == binding.texture_param
        }) {
            existing.binding = binding;
            return;
        }
        self.texture_bindings.push(MaterialTextureBindingEntry {
            object_id,
            material_slot,
            binding,
        });
    }

    pub fn add_asset_with_id(&mut self, id: u64, name: String, path: &str) {
        self.objects.push(SceneObject {
            id,
            name,
            kind: SceneObjectKind::Asset(AssetData {
                path: path.to_string(),
                position: [0.0, 0.0, 0.0],
                rotation_deg: [0.0, 0.0, 0.0],
                scale: [1.0, 1.0, 1.0],
            }),
        });
    }

    pub fn add_light(&mut self, name: &str, data: LightData) {
        let id = self.reserve_object_id();
        self.objects.push(SceneObject {
            id,
            name: name.to_string(),
            kind: SceneObjectKind::Light(data),
        });
    }

    pub fn migrate_legacy_light_objects(&mut self) {
        for object in &mut self.objects {
            if let SceneObjectKind::DirectionalLight(legacy) = &object.kind {
                let legacy_data = legacy.clone();
                object.kind = SceneObjectKind::Light(LightData::from_legacy_directional(legacy_data));
                if object.name.trim().is_empty() || object.name == "Light" {
                    object.name = LightType::Directional.name_prefix().to_string();
                }
            }
        }
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
                let id = self.reserve_object_id();
                self.objects.push(SceneObject {
                    id,
                    name: "Environment".to_string(),
                    kind: SceneObjectKind::Environment(data),
                });
            }
        }
    }

    pub fn remove_object(&mut self, index: usize) -> Option<SceneObject> {
        if index >= self.objects.len() {
            return None;
        }
        let removed = self.objects.remove(index);
        let removed_id = removed.id;
        self.material_overrides
            .retain(|entry| entry.object_id != Some(removed_id));
        self.texture_bindings
            .retain(|entry| entry.object_id != removed_id);
        Some(removed)
    }

    #[cfg(test)]
    pub fn add_object(&mut self, object: SceneObject) {
        self.objects.push(object);
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
