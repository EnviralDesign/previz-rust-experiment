use crate::filament::{
    Engine, Entity, EntityManager, GltfAsset, GltfAssetLoader, GltfMaterialProvider,
    GltfResourceLoader, GltfTextureProvider, MaterialInstance, Scene,
};
use std::path::PathBuf;

pub const DEFAULT_GLTF_PATH: &str = "assets/gltf/DamagedHelmet.gltf";

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
    loaded_assets: Vec<LoadedAsset>,
    material_instances: Vec<MaterialInstance>,
    material_names: Vec<String>,
}

impl AssetManager {
    pub fn new() -> Self {
        Self {
            gltf_assets: Vec::new(),
            loaded_assets: Vec::new(),
            material_instances: Vec::new(),
            material_names: Vec::new(),
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

    pub fn clear(&mut self) {
        self.gltf_assets.clear();
        self.loaded_assets.clear();
        self.material_instances.clear();
        self.material_names.clear();
    }

    pub fn load_gltf_from_path(
        &mut self,
        engine: &mut Engine,
        scene: &mut Scene,
        entity_manager: &mut EntityManager,
        path: &str,
    ) -> LoadedAsset {
        let gltf_bytes = load_gltf_bytes(path);
        let mut material_provider = GltfMaterialProvider::create_jit(engine, false)
            .expect("Failed to create gltf material provider");
        let mut texture_provider =
            GltfTextureProvider::create_stb(engine).expect("Failed to create stb texture provider");
        let mut asset_loader =
            GltfAssetLoader::create(engine, &mut material_provider, entity_manager)
                .expect("Failed to create gltf asset loader");
        let mut resource_loader = GltfResourceLoader::create(engine, None, true)
            .expect("Failed to create gltf resource loader");
        resource_loader.add_texture_provider("image/png", &mut texture_provider);
        resource_loader.add_texture_provider("image/jpeg", &mut texture_provider);

        let mut asset = asset_loader
            .create_asset_from_json(&gltf_bytes)
            .expect("Failed to parse gltf");
        let loaded = resource_loader.load_resources(&mut asset);
        if !loaded {
            panic!("Failed to load gltf resources");
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
        loaded_asset
    }
}

fn load_gltf_bytes(path: &str) -> Vec<u8> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gltf_path = manifest_dir.join(path);
    std::fs::read(&gltf_path).unwrap_or_else(|err| {
        panic!(
            "Failed to read glTF asset at {}: {}",
            gltf_path.display(),
            err
        )
    })
}
