use crate::scene::SceneState;

pub struct UiState {
    show_asset_panel: bool,
}

impl UiState {
    pub fn new() -> Self {
        Self {
            show_asset_panel: true,
        }
    }

    pub fn update(&mut self, _scene: &SceneState) {
        if self.show_asset_panel {
            // TODO: Replace with filagui/ImGui draw calls once the binding layer exists.
        }
    }
}
