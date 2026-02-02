use crate::assets::AssetManager;
use crate::scene::SceneState;

pub struct UiState {
    show_asset_panel: bool,
    asset_summary: String,
    selected_index: i32,
    light_settings: LightSettings,
    selected_material_index: i32,
    material_params: MaterialParams,
    environment_hdr_path: [u8; 260],
    environment_ibl_path: [u8; 260],
    environment_skybox_path: [u8; 260],
    environment_intensity: f32,
    environment_status: String,
}

#[derive(Debug, Clone, Copy)]
pub struct LightSettings {
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
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
                color: [1.0, 1.0, 1.0],
                intensity: 100_000.0,
                direction: [0.0, -1.0, -0.5],
            },
            selected_material_index: -1,
            material_params: MaterialParams {
                base_color_rgba: [1.0, 1.0, 1.0, 1.0],
                metallic: 1.0,
                roughness: 1.0,
                emissive_rgb: [0.0, 0.0, 0.0],
            },
            environment_hdr_path: [0u8; 260],
            environment_ibl_path: [0u8; 260],
            environment_skybox_path: [0u8; 260],
            environment_intensity: 30_000.0,
            environment_status: String::new(),
        }
    }

    pub fn update(&mut self, scene: &SceneState, assets: &AssetManager) {
        if self.show_asset_panel {
            let mut summary = String::new();
            for object in scene.objects() {
                summary.push_str(&format!(
                    "{} (center {:.2}, {:.2}, {:.2}, extent {:.2}, {:.2}, {:.2})\n",
                    object.name,
                    object.center[0],
                    object.center[1],
                    object.center[2],
                    object.extent[0],
                    object.extent[1],
                    object.extent[2]
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

    pub fn selected_index(&self) -> i32 {
        self.selected_index
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
