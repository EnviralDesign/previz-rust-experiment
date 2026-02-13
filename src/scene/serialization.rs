use crate::scene::SceneState;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum SerializationError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, SerializationError>;

pub fn save_scene_to_file(scene: &SceneState, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(scene)?;
    std::fs::write(path, json)?;
    Ok(())
}

pub fn load_scene_from_file(path: &Path) -> Result<SceneState> {
    let json = std::fs::read_to_string(path)?;
    let mut scene: SceneState = serde_json::from_str(&json)?;
    scene.migrate_legacy_light_objects();
    scene.ensure_object_ids();
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use crate::scene::{
        AssetData, EnvironmentData, LightData, LightType, MaterialOverrideData,
        MaterialTextureBindingData, MediaSourceKind, SceneObject,
        SceneObjectKind, SceneState, TextureColorSpace,
    };

    #[test]
    fn test_empty_scene_serialization() {
        let scene = SceneState::new();
        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: SceneState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.objects().len(), 0);
    }

    #[test]
    fn test_scene_with_light() {
        let mut scene = SceneState::new();
        scene.add_object(SceneObject {
            id: 1,
            name: "Directional Light".to_string(),
            kind: SceneObjectKind::Light(LightData {
                light_type: LightType::Directional,
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
                shadow: Default::default(),
            }),
        });

        let json = serde_json::to_string_pretty(&scene).unwrap();
        println!("Serialized scene:\n{}", json);

        let loaded: SceneState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.objects().len(), 1);

        match &loaded.objects()[0].kind {
            SceneObjectKind::Light(data) => {
                assert_eq!(data.light_type, LightType::Directional);
                assert_eq!(data.color, [1.0, 1.0, 1.0]);
                assert_eq!(data.intensity, 100_000.0);
                assert_eq!(data.direction, [0.0, -1.0, -0.5]);
            }
            _ => panic!("Expected Light"),
        }
    }

    #[test]
    fn test_legacy_directional_light_migrates_on_load() {
        let json = r#"
{
  "objects": [
    {
      "id": 1,
      "name": "Light",
      "kind": {
        "DirectionalLight": {
          "color": [1.0, 1.0, 1.0],
          "intensity": 100000.0,
          "direction": [0.0, -1.0, -0.5]
        }
      }
    }
  ]
}
        "#;
        let mut path = std::env::temp_dir();
        path.push("previz_legacy_light_scene.json");
        std::fs::write(&path, json).unwrap();
        let loaded = super::load_scene_from_file(&path).unwrap();
        let _ = std::fs::remove_file(&path);
        match &loaded.objects()[0].kind {
            SceneObjectKind::Light(data) => {
                assert_eq!(data.light_type, LightType::Directional);
                assert_eq!(data.direction, [0.0, -1.0, -0.5]);
            }
            _ => panic!("Expected migrated light"),
        }
    }

    #[test]
    fn test_runtime_fields_are_not_serialized() {
        let mut scene = SceneState::new();
        scene.add_object(SceneObject {
            id: 1,
            name: "Helmet".to_string(),
            kind: SceneObjectKind::Asset(AssetData {
                path: "assets/gltf/DamagedHelmet.gltf".to_string(),
                position: [1.0, 2.0, 3.0],
                rotation_deg: [10.0, 20.0, 30.0],
                scale: [1.0, 1.0, 1.0],
            }),
        });
        scene.add_object(SceneObject {
            id: 2,
            name: "Environment".to_string(),
            kind: SceneObjectKind::Environment(EnvironmentData {
                hdr_path: "hdr.hdr".to_string(),
                ibl_path: "ibl.ktx".to_string(),
                skybox_path: "sky.ktx".to_string(),
                intensity: 12000.0,
            }),
        });

        let json = serde_json::to_string_pretty(&scene).unwrap();
        assert!(!json.contains("root_entity"));
        assert!(!json.contains("\"center\""));
        assert!(!json.contains("\"extent\""));

        let loaded: SceneState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.objects().len(), 2);
    }

    #[test]
    fn test_save_load_stress_loop_via_file() {
        let mut scene = SceneState::new();
        scene.add_object(SceneObject {
            id: 1,
            name: "Helmet".to_string(),
            kind: SceneObjectKind::Asset(AssetData {
                path: "assets/gltf/DamagedHelmet.gltf".to_string(),
                position: [1.0, 2.0, 3.0],
                rotation_deg: [10.0, 20.0, 30.0],
                scale: [1.0, 1.0, 1.0],
            }),
        });
        scene.add_object(SceneObject {
            id: 2,
            name: "Directional Light".to_string(),
            kind: SceneObjectKind::Light(LightData {
                light_type: LightType::Directional,
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
                shadow: Default::default(),
            }),
        });
        scene.add_object(SceneObject {
            id: 3,
            name: "Environment".to_string(),
            kind: SceneObjectKind::Environment(EnvironmentData {
                hdr_path: "hdr.hdr".to_string(),
                ibl_path: "ibl.ktx".to_string(),
                skybox_path: "sky.ktx".to_string(),
                intensity: 12000.0,
            }),
        });

        let mut path = std::env::temp_dir();
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!(
            "previz_scene_stress_{}_{}.json",
            std::process::id(),
            nonce
        ));

        for _ in 0..50 {
            super::save_scene_to_file(&scene, &path).unwrap();
            scene = super::load_scene_from_file(&path).unwrap();
            assert_eq!(scene.objects().len(), 3);
            match &scene.objects()[0].kind {
                SceneObjectKind::Asset(asset) => {
                    assert_eq!(asset.path, "assets/gltf/DamagedHelmet.gltf");
                }
                _ => panic!("Expected first object to be Asset"),
            }
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn test_material_overrides_roundtrip() {
        let mut scene = SceneState::new();
        scene.set_material_override(
            42,
            "assets/gltf/DamagedHelmet.gltf".to_string(),
            0,
            "Material_MR".to_string(),
            MaterialOverrideData {
                base_color_rgba: [0.2, 0.3, 0.4, 1.0],
                metallic: 0.8,
                roughness: 0.25,
                emissive_rgb: [0.1, 0.0, 0.2],
            },
        );

        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: SceneState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.material_overrides().len(), 1);
        let entry = &loaded.material_overrides()[0];
        assert_eq!(entry.object_id, Some(42));
        assert_eq!(
            entry.asset_path.as_deref(),
            Some("assets/gltf/DamagedHelmet.gltf")
        );
        assert_eq!(entry.material_slot, Some(0));
        assert_eq!(entry.material_name, "Material_MR");
        assert_eq!(entry.data.base_color_rgba, [0.2, 0.3, 0.4, 1.0]);
        assert_eq!(entry.data.metallic, 0.8);
        assert_eq!(entry.data.roughness, 0.25);
        assert_eq!(entry.data.emissive_rgb, [0.1, 0.0, 0.2]);
    }

    #[test]
    fn test_texture_bindings_roundtrip() {
        let mut scene = SceneState::new();
        scene.set_texture_binding(
            7,
            0,
            MaterialTextureBindingData {
                texture_param: "baseColorMap".to_string(),
                source_kind: MediaSourceKind::Image,
                source_path: "assets/textures/albedo.png".to_string(),
                runtime_ktx_path: Some("assets/cache/textures/albedo_hash.ktx".to_string()),
                source_hash: Some("abc123".to_string()),
                wrap_repeat_u: true,
                wrap_repeat_v: false,
                color_space: TextureColorSpace::Srgb,
                uv_offset: [0.0, 0.0],
                uv_scale: [1.0, 1.0],
                uv_rotation_deg: 0.0,
            },
        );

        let json = serde_json::to_string_pretty(&scene).unwrap();
        let loaded: SceneState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.texture_bindings().len(), 1);
        let entry = &loaded.texture_bindings()[0];
        assert_eq!(entry.object_id, 7);
        assert_eq!(entry.material_slot, 0);
        assert_eq!(entry.binding.texture_param, "baseColorMap");
        assert_eq!(entry.binding.source_kind, MediaSourceKind::Image);
        assert_eq!(entry.binding.source_path, "assets/textures/albedo.png");
        assert_eq!(
            entry.binding.runtime_ktx_path.as_deref(),
            Some("assets/cache/textures/albedo_hash.ktx")
        );
        assert_eq!(entry.binding.source_hash.as_deref(), Some("abc123"));
        assert!(entry.binding.wrap_repeat_u);
        assert!(!entry.binding.wrap_repeat_v);
    }
}
