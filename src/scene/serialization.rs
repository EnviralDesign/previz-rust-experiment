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
    let scene: SceneState = serde_json::from_str(&json)?;
    Ok(scene)
}

#[cfg(test)]
mod tests {
    use crate::scene::{AssetData, DirectionalLightData, EnvironmentData, SceneObject, SceneObjectKind, SceneState};

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
            name: "Light".to_string(),
            kind: SceneObjectKind::DirectionalLight(DirectionalLightData {
                color: [1.0, 1.0, 1.0],
                intensity: 100_000.0,
                direction: [0.0, -1.0, -0.5],
            }),
        });

        let json = serde_json::to_string_pretty(&scene).unwrap();
        println!("Serialized scene:\n{}", json);

        let loaded: SceneState = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.objects().len(), 1);

        match &loaded.objects()[0].kind {
            SceneObjectKind::DirectionalLight(data) => {
                assert_eq!(data.color, [1.0, 1.0, 1.0]);
                assert_eq!(data.intensity, 100_000.0);
                assert_eq!(data.direction, [0.0, -1.0, -0.5]);
            }
            _ => panic!("Expected DirectionalLight"),
        }
    }

    #[test]
    fn test_runtime_fields_are_not_serialized() {
        let mut scene = SceneState::new();
        scene.add_object(SceneObject {
            name: "Helmet".to_string(),
            kind: SceneObjectKind::Asset(AssetData {
                path: "assets/gltf/DamagedHelmet.gltf".to_string(),
                position: [1.0, 2.0, 3.0],
                rotation_deg: [10.0, 20.0, 30.0],
                scale: [1.0, 1.0, 1.0],
            }),
        });
        scene.add_object(SceneObject {
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
            name: "Helmet".to_string(),
            kind: SceneObjectKind::Asset(AssetData {
                path: "assets/gltf/DamagedHelmet.gltf".to_string(),
                position: [1.0, 2.0, 3.0],
                rotation_deg: [10.0, 20.0, 30.0],
                scale: [1.0, 1.0, 1.0],
            }),
        });
        scene.add_object(SceneObject {
            name: "Light".to_string(),
            kind: SceneObjectKind::DirectionalLight(DirectionalLightData {
                color: [1.0, 1.0, 1.0],
                intensity: 100_000.0,
                direction: [0.0, -1.0, -0.5],
            }),
        });
        scene.add_object(SceneObject {
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
}
