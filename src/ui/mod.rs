use crate::assets::AssetManager;
use crate::scene::SceneState;

pub struct UiState {
    show_asset_panel: bool,
    asset_summary: String,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_asset_panel: true,
            asset_summary: String::new(),
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
}
