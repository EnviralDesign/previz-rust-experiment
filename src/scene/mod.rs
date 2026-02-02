use crate::assets::LoadedAsset;

#[derive(Debug, Clone)]
pub struct SceneObject {
    pub name: String,
    pub center: [f32; 3],
    pub extent: [f32; 3],
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

    pub fn add_asset(&mut self, asset: &LoadedAsset) {
        self.objects.push(SceneObject {
            name: asset.name.clone(),
            center: asset.center,
            extent: asset.extent,
        });
    }
}
