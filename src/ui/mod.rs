use crate::assets::AssetManager;
use crate::scene::SceneState;

pub struct UiState {
    show_asset_panel: bool,
    asset_summary: String,
    selected_index: i32,
    light_settings: LightSettings,
}

#[derive(Debug, Clone, Copy)]
pub struct LightSettings {
    pub color: [f32; 3],
    pub intensity: f32,
    pub direction: [f32; 3],
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
}
