use crate::filament::{
    Engine, Entity, EntityManager, GltfAsset, GltfAssetLoader, GltfMaterialProvider,
    GltfResourceLoader, GltfTextureProvider, MaterialInstance, Scene,
};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct LoadedAsset {
    pub name: String,
    pub center: [f32; 3],
    pub extent: [f32; 3],
    pub root_entity: Entity,
}

pub struct AssetManager {
    // Store all loaded assets to keep them alive (prevent Drop from destroying entities)
    gltf_assets: Vec<GltfAsset>,
    retired_gltf_assets: Vec<GltfAsset>,
    loaded_assets: Vec<LoadedAsset>,
    material_instances: Vec<MaterialInstance>,
    retired_material_instances: Vec<MaterialInstance>,
    material_names: Vec<String>,
    // glTF providers must outlive loaded assets/material instances.
    material_provider: Option<GltfMaterialProvider>,
    texture_provider: Option<GltfTextureProvider>,
}

#[derive(Debug, thiserror::Error)]
pub enum AssetError {
    #[error("failed to read glTF at {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to create glTF material provider")]
    CreateMaterialProvider,
    #[error("failed to create glTF texture provider")]
    CreateTextureProvider,
    #[error("failed to create glTF asset loader")]
    CreateAssetLoader,
    #[error("failed to create glTF resource loader")]
    CreateResourceLoader,
    #[error("failed to parse glTF JSON: {path}")]
    ParseGltf { path: String },
    #[error("failed to load glTF resources: {path}")]
    LoadResources { path: String },
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            gltf_assets: Vec::new(),
            retired_gltf_assets: Vec::new(),
            loaded_assets: Vec::new(),
            material_instances: Vec::new(),
            retired_material_instances: Vec::new(),
            material_names: Vec::new(),
            material_provider: None,
            texture_provider: None,
        }
    }

    pub fn loaded_assets(&self) -> &[LoadedAsset] {
        &self.loaded_assets
    }

    pub fn material_instances(&self) -> &[MaterialInstance] {
        &self.material_instances
    }

    pub fn material_instances_mut(&mut self) -> &mut [MaterialInstance] {
        &mut self.material_instances
    }

    pub fn material_names(&self) -> &[String] {
        &self.material_names
    }

    /// Prepare for scene rebuild without dropping native glTF/material resources mid-frame.
    /// Old resources are retained until full teardown.
    pub fn prepare_for_scene_rebuild(&mut self) {
        self.retired_material_instances
            .append(&mut self.material_instances);
        self.retired_gltf_assets.append(&mut self.gltf_assets);
        self.loaded_assets.clear();
        self.material_names.clear();
    }

    pub fn load_gltf_from_path(
        &mut self,
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        path: &str,
    ) -> Result<LoadedAsset, AssetError> {
        let gltf_bytes = load_gltf_bytes(path)?;
        if self.material_provider.is_none() {
            self.material_provider = GltfMaterialProvider::create_jit(engine, false);
        }
        if self.texture_provider.is_none() {
            self.texture_provider = GltfTextureProvider::create_stb(engine);
        }
        let material_provider = self
            .material_provider
            .as_mut()
            .ok_or(AssetError::CreateMaterialProvider)?;
        let texture_provider = self
            .texture_provider
            .as_mut()
            .ok_or(AssetError::CreateTextureProvider)?;

        let mut asset_loader = GltfAssetLoader::create(engine, material_provider, entity_manager)
            .ok_or(AssetError::CreateAssetLoader)?;
        let mut resource_loader =
            GltfResourceLoader::create(engine, None, true).ok_or(AssetError::CreateResourceLoader)?;
        resource_loader.add_texture_provider("image/png", texture_provider);
        resource_loader.add_texture_provider("image/jpeg", texture_provider);

        let mut asset = asset_loader
            .create_asset_from_json(&gltf_bytes)
            .ok_or_else(|| AssetError::ParseGltf {
                path: path.to_string(),
            })?;
        let loaded = resource_loader.load_resources(&mut asset);
        if !loaded {
            return Err(AssetError::LoadResources {
                path: path.to_string(),
            });
        }
        asset.release_source_data();
        asset.add_entities_to_scene(scene);

        let (center, extent) = asset.bounding_box();
        let root_entity = asset.root_entity();
        let name = PathBuf::from(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("gltf")
            .to_string();
        let loaded_asset = LoadedAsset {
            name,
            center,
            extent,
            root_entity,
        };

        // Keep asset alive by storing it (prevents Drop from destroying entities)
        let (instances, names) = asset.material_instances();
        self.material_instances.extend(instances);
        self.material_names.extend(names);
        self.gltf_assets.push(asset);
        self.loaded_assets.push(loaded_asset.clone());
        Ok(loaded_asset)
    }
}

impl Drop for AssetManager {
    fn drop(&mut self) {
        // Ensure material instances are dropped before assets/providers.
        self.material_instances.clear();
        self.retired_material_instances.clear();
        self.material_names.clear();
        self.gltf_assets.clear();
        self.retired_gltf_assets.clear();
        self.loaded_assets.clear();
        self.texture_provider = None;
        self.material_provider = None;
    }
}

fn load_gltf_bytes(path: &str) -> Result<Vec<u8>, AssetError> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gltf_path = manifest_dir.join(path);
    std::fs::read(&gltf_path).map_err(|source| AssetError::Read {
        path: gltf_path.display().to_string(),
        source,
    })
}
