import re

with open('src/app/mod.rs', 'r') as f:
    content = f.read()

# Edit 1: Fix the data reading section
old_reading_code = '''                position = object.position;
                rotation = object.rotation_deg;
                scale = object.scale;
                can_edit_transform = object.kind == SceneObjectKind::Asset;
                selected_kind = match object.kind {
                    SceneObjectKind::Asset => 0,
                    SceneObjectKind::DirectionalLight => 1,
                    SceneObjectKind::Environment => 2,
                };'''

new_reading_code = '''                can_edit_transform = matches!(object.kind, SceneObjectKind::Asset(_));
                selected_kind = match &object.kind {
                    SceneObjectKind::Asset(data) => {
                        position = data.position;
                        rotation = data.rotation_deg;
                        scale = data.scale;
                        0
                    }
                    SceneObjectKind::DirectionalLight(_) => 1,
                    SceneObjectKind::Environment(_) => 2,
                };'''

if old_reading_code in content:
    content = content.replace(old_reading_code, new_reading_code)
    print("Edit 1 applied successfully")
else:
    print("Edit 1: Could not find the old code pattern")

# Edit 2: Fix the data writing section
old_writing_code = '''                    if object.kind == SceneObjectKind::Asset {
                        let mut changed = false;
                        if object.position != position {
                            object.position = position;
                            changed = true;
                        }
                        if object.rotation_deg != rotation {
                            object.rotation_deg = rotation;
                            changed = true;
                        }
                        if object.scale != scale {
                            object.scale = scale;
                            changed = true;
                        }
                        if changed {
                            if let Some(entity) = object.root_entity {
                                let matrix = compose_transform_matrix(
                                    object.position,
                                    object.rotation_deg,
                                    object.scale,
                                );
                                render.set_entity_transform(entity, matrix);
                            }
                        }
                    }'''

new_writing_code = '''                    if let SceneObjectKind::Asset(ref mut data) = object.kind {
                        let mut changed = false;
                        if data.position != position {
                            data.position = position;
                            changed = true;
                        }
                        if data.rotation_deg != rotation {
                            data.rotation_deg = rotation;
                            changed = true;
                        }
                        if data.scale != scale {
                            data.scale = scale;
                            changed = true;
                        }
                        if changed {
                            if let Some(entity) = object.root_entity {
                                let matrix = compose_transform_matrix(
                                    data.position,
                                    data.rotation_deg,
                                    data.scale,
                                );
                                render.set_entity_transform(entity, matrix);
                            }
                        }
                    }'''

if old_writing_code in content:
    content = content.replace(old_writing_code, new_writing_code)
    print("Edit 2 applied successfully")
else:
    print("Edit 2: Could not find the old code pattern")

with open('src/app/mod.rs', 'w') as f:
    f.write(content)

print("Done!")
