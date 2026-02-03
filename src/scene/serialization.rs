use crate::scene::{
    AssetData, DirectionalLightData, EnvironmentData, SceneObject, SceneObjectKind, SceneState,
};
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
    use super::*;
    use crate::scene::SceneState;

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
            root_entity: None,
            center: [0.0, 0.0, 0.0],
            extent: [0.0, 0.0, 0.0],
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
}
