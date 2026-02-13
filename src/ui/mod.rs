use crate::assets::AssetManager;
use crate::scene::{SceneRuntime, SceneState};

pub const MATERIAL_TEXTURE_PARAMS: [&str; 5] = [
    "baseColorMap",
    "normalMap",
    "metallicRoughnessMap",
    "occlusionMap",
    "emissiveMap",
];

#[derive(Debug, Clone, Copy)]
pub struct MaterialBindingUiRow {
    pub source: [u8; 260],
    pub wrap_repeat_u: bool,
    pub wrap_repeat_v: bool,
    pub srgb: bool,
    pub uv_offset: [f32; 2],
    pub uv_scale: [f32; 2],
    pub uv_rotation_deg: f32,
}

impl Default for MaterialBindingUiRow {
    fn default() -> Self {
        Self {
            source: [0u8; 260],
            wrap_repeat_u: true,
            wrap_repeat_v: true,
            srgb: true,
            uv_offset: [0.0, 0.0],
            uv_scale: [1.0, 1.0],
            uv_rotation_deg: 0.0,
        }
    }
}

pub struct UiState {
    show_asset_panel: bool,
    asset_summary: String,
    selected_index: i32,
    light_settings: LightSettings,
    selected_material_index: i32,
    material_params: MaterialParams,
    material_texture_param: [u8; 128],
    material_texture_source: [u8; 260],
    material_binding_rows: [MaterialBindingUiRow; 5],
    environment_hdr_path: [u8; 260],
    environment_ibl_path: [u8; 260],
    environment_skybox_path: [u8; 260],
    environment_intensity: f32,
    environment_status: String,
}

#[derive(Debug, Clone, Copy)]
pub struct LightSettings {
    pub light_type: i32,
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
    pub range: f32,
    pub spot_inner_deg: f32,
    pub spot_outer_deg: f32,
    pub sun_angular_radius_deg: f32,
    pub sun_halo_size: f32,
    pub sun_halo_falloff: f32,
    pub cast_shadows: bool,
    pub shadow_map_size: i32,
    pub shadow_cascades: i32,
    pub shadow_far: f32,
    pub shadow_near_hint: f32,
    pub shadow_far_hint: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MaterialParams {
    pub base_color_rgba: [f32; 4],
    pub metallic: f32,
    pub roughness: f32,
    pub emissive_rgb: [f32; 3],
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_asset_panel: true,
            asset_summary: String::new(),
            selected_index: -1,
            light_settings: LightSettings {
                light_type: 0,
                color: [1.0, 1.0, 1.0],
                intensity: 100_000.0,
                direction: [0.0, -1.0, -0.5],
                range: 10.0,
                spot_inner_deg: 25.0,
                spot_outer_deg: 35.0,
                sun_angular_radius_deg: 0.545,
                sun_halo_size: 10.0,
                sun_halo_falloff: 80.0,
                cast_shadows: true,
                shadow_map_size: 1024,
                shadow_cascades: 1,
                shadow_far: 0.0,
                shadow_near_hint: 1.0,
                shadow_far_hint: 100.0,
            },
            selected_material_index: -1,
            material_params: MaterialParams {
                base_color_rgba: [1.0, 1.0, 1.0, 1.0],
                metallic: 1.0,
                roughness: 1.0,
                emissive_rgb: [0.0, 0.0, 0.0],
            },
            material_texture_param: {
                let mut buf = [0u8; 128];
                let value = b"baseColorMap";
                buf[..value.len()].copy_from_slice(value);
                buf
            },
            material_texture_source: [0u8; 260],
            material_binding_rows: [MaterialBindingUiRow::default(); 5],
            environment_hdr_path: [0u8; 260],
            environment_ibl_path: [0u8; 260],
            environment_skybox_path: [0u8; 260],
            environment_intensity: 30_000.0,
            environment_status: String::new(),
        }
    }

    pub fn update(&mut self, scene: &SceneState, runtime: &SceneRuntime, assets: &AssetManager) {
        if self.show_asset_panel {
            let mut summary = String::new();
            for (index, object) in scene.objects().iter().enumerate() {
                let runtime_object = runtime.get(index).copied().unwrap_or_default();
                summary.push_str(&format!(
                    "{} (center {:.2}, {:.2}, {:.2}, extent {:.2}, {:.2}, {:.2})\n",
                    object.name,
                    runtime_object.center[0],
                    runtime_object.center[1],
                    runtime_object.center[2],
                    runtime_object.extent[0],
                    runtime_object.extent[1],
                    runtime_object.extent[2]
                ));
            }
            summary.push_str(&format!("Loaded assets: {}", assets.loaded_assets().len()));
            if !self.environment_status.is_empty() {
                summary.push_str("\n");
                summary.push_str(&self.environment_status);
            }
            self.asset_summary = summary;

            // TODO: Replace with filagui/ImGui draw calls once the binding layer exists.
        }
    }

    pub fn summary(&self) -> &str {
        &self.asset_summary
    }

    pub fn set_selected_index(&mut self, index: i32) {
        self.selected_index = index;
    }

    pub fn light_settings(&self) -> LightSettings {
        self.light_settings
    }

    pub fn set_light_settings(&mut self, settings: LightSettings) {
        self.light_settings = settings;
    }

    pub fn selected_material_index(&self) -> i32 {
        self.selected_material_index
    }

    pub fn set_selected_material_index(&mut self, index: i32) {
        self.selected_material_index = index;
    }

    pub fn material_params(&self) -> MaterialParams {
        self.material_params
    }

    pub fn set_material_params(&mut self, params: MaterialParams) {
        self.material_params = params;
    }

    pub fn texture_and_environment_paths_mut(
        &mut self,
    ) -> (
        &mut [u8; 128],
        &mut [u8; 260],
        &mut [u8; 260],
        &mut [u8; 260],
        &mut [u8; 260],
    ) {
        (
            &mut self.material_texture_param,
            &mut self.material_texture_source,
            &mut self.environment_hdr_path,
            &mut self.environment_ibl_path,
            &mut self.environment_skybox_path,
        )
    }

    pub fn material_binding_rows_mut(&mut self) -> &mut [MaterialBindingUiRow; 5] {
        &mut self.material_binding_rows
    }

    pub fn material_binding_rows(&self) -> &[MaterialBindingUiRow; 5] {
        &self.material_binding_rows
    }

    pub fn environment_paths_mut(
        &mut self,
    ) -> (&mut [u8; 260], &mut [u8; 260], &mut [u8; 260]) {
        (
            &mut self.environment_hdr_path,
            &mut self.environment_ibl_path,
            &mut self.environment_skybox_path,
        )
    }

    pub fn environment_intensity(&self) -> f32 {
        self.environment_intensity
    }

    pub fn set_environment_intensity(&mut self, intensity: f32) {
        self.environment_intensity = intensity;
    }

    pub fn set_environment_status(&mut self, status: String) {
        self.environment_status = status;
    }
}
