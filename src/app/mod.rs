mod input;
mod timing;

use crate::assets::AssetManager;
use crate::filament::Entity;
use crate::render::{CameraController, CameraMovement, RenderContext};
use crate::scene::{
    compose_transform_matrix, DirectionalLightData, EnvironmentData, RuntimeObject, SceneObjectKind,
    SceneRuntime, SceneState,
};
use crate::ui::{MaterialParams, UiState};
use input::{InputAction, InputState};
use timing::FrameTiming;

use std::ffi::CString;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{Modifiers, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

enum SceneCommand {
    AddAsset { path: String },
    SaveScene { path: PathBuf },
    LoadScene { path: PathBuf },
}

enum CommandOutcome {
    None,
    Message(String),
}

pub struct App {
    window: Option<Arc<Window>>,
    assets: AssetManager,
    scene: SceneState,
    scene_runtime: SceneRuntime,
    ui: UiState,
    input: InputState,
    modifiers: Modifiers,
    mouse_pos: Option<(f32, f32)>,
    mouse_buttons: [bool; 5],
    window_focused: bool,
    camera: CameraController,
    timing: FrameTiming,
    target_frame_duration: Duration,
    next_frame_time: Instant,
    close_requested: bool,
    render: Option<RenderContext>,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            assets: AssetManager::new(),
            scene: SceneState::new(),
            scene_runtime: SceneRuntime::new(),
            ui: UiState::new(),
            input: InputState::default(),
            modifiers: Modifiers::default(),
            mouse_pos: None,
            mouse_buttons: [false; 5],
            window_focused: true,
            camera: CameraController::new([0.0, 0.0, 3.0], 0.6, 0.3),
            timing: FrameTiming::new("Previz - Filament v1.69.0 glTF".to_string()),
            target_frame_duration: Duration::from_millis(16),
            next_frame_time: Instant::now(),
            close_requested: false,
            render: None,
        }
    }

    fn init_filament(&mut self, window: &Window) {
        let mut render = RenderContext::new(window);

        // Start with empty scene - no default objects
        self.camera = CameraController::new([0.0, 0.0, 5.0], 0.0, 0.0);
        render.set_projection_for_window(window);
        self.camera.apply(render.camera_mut());
        render.init_ui(window);

        self.render = Some(render);
    }

    fn handle_resize(&mut self, new_size: PhysicalSize<u32>, scale_factor: f64) {
        if let Some(render) = &mut self.render {
            render.resize(new_size, scale_factor);
        }
    }

    fn update_target_frame_duration(&mut self, window: &Window) {
        let mut target = Duration::from_millis(16);
        if let Some(monitor) = window.current_monitor() {
            if let Some(millihz) = monitor.refresh_rate_millihertz() {
                let hz = millihz as f32 / 1000.0;
                if hz > 1.0 {
                    target = Duration::from_secs_f32(1.0 / hz);
                }
            }
        }
        self.target_frame_duration = target;
        self.next_frame_time = Instant::now() + self.target_frame_duration;
    }

    fn update_camera(&mut self) {
        let movement = CameraMovement {
            move_forward: self.input.move_forward,
            move_backward: self.input.move_backward,
            move_left: self.input.move_left,
            move_right: self.input.move_right,
            move_up: self.input.move_up,
            move_down: self.input.move_down,
            aim_left: self.input.aim_left,
            aim_right: self.input.aim_right,
            aim_up: self.input.aim_up,
            aim_down: self.input.aim_down,
        };
        if self.camera.update_movement(&movement, self.timing.frame_dt) {
            if let Some(render) = &mut self.render {
                self.camera.apply(render.camera_mut());
            }
        }
    }

    fn render(&mut self) {
        let frame_start = Instant::now();
        self.ui.update(&self.scene, &self.scene_runtime, &self.assets);
        let ui_text = self.ui.summary().to_string();
        let object_names: Vec<CString> = self
            .scene
            .object_names()
            .into_iter()
            .map(|name| CString::new(name).unwrap_or_else(|_| CString::new("Object").unwrap()))
            .collect();
        let material_names: Vec<CString> = self
            .assets
            .material_names()
            .iter()
            .map(|name| {
                CString::new(name.as_str()).unwrap_or_else(|_| CString::new("Material").unwrap())
            })
            .collect();
        let mut selected_index = self.ui.selected_index();
        let mut position = [0.0f32; 3];
        let mut rotation = [0.0f32; 3];
        let mut scale = [1.0f32; 3];
        let mut can_edit_transform = false;
        let mut selected_kind = -1i32;
        let mut light_settings = self.ui.light_settings();
        let mut environment_intensity = self.ui.environment_intensity();
        let mut selected_light_entity: Option<Entity> = None;

        if selected_index >= 0 {
            if let Some(object) = self.scene.objects().get(selected_index as usize) {
                can_edit_transform = matches!(object.kind, SceneObjectKind::Asset(_));
                selected_kind = match &object.kind {
                    SceneObjectKind::Asset(data) => {
                        position = data.position;
                        rotation = data.rotation_deg;
                        scale = data.scale;
                        0
                    }
                    SceneObjectKind::DirectionalLight(data) => {
                        light_settings.color = data.color;
                        light_settings.intensity = data.intensity;
                        light_settings.direction = data.direction;
                        selected_light_entity = self
                            .scene_runtime
                            .get(selected_index as usize)
                            .and_then(|runtime| runtime.root_entity);
                        1
                    }
                    SceneObjectKind::Environment(data) => {
                        environment_intensity = data.intensity;
                        2
                    }
                };
            }
        }
        let mut selected_material_index = self.ui.selected_material_index();
        let mut material_params = self.ui.material_params();
        let previous_material_selection = selected_material_index;
        let previous_material_params = material_params;
        let previous_environment_intensity = self.ui.environment_intensity();
        let mut environment_apply = false;
        let mut environment_generate = false;
        let mut create_gltf = false;
        let mut create_light = false;
        let mut create_environment = false;
        let mut save_scene = false;
        let mut load_scene = false;
        let (hdr_path_string, ibl_path_string, skybox_path_string) = {
            let (hdr_path, ibl_path, skybox_path) = self.ui.environment_paths_mut();
            if let Some(render) = &mut self.render {
                let (mx, my) = if self.window_focused {
                    self.mouse_pos.unwrap_or((-f32::MAX, -f32::MAX))
                } else {
                    (-f32::MAX, -f32::MAX)
                };
                render.ui_mouse_pos(mx, my);
                for (index, down) in self.mouse_buttons.iter().enumerate() {
                    render.ui_mouse_button(index as i32, *down);
                }
                let render_ms = render.render_scene_ui(
                    "Assets",
                    &ui_text,
                    &object_names,
                    &mut selected_index,
                    &mut selected_kind,
                    &mut can_edit_transform,
                    &mut position,
                    &mut rotation,
                    &mut scale,
                    &mut light_settings.color,
                    &mut light_settings.intensity,
                    &mut light_settings.direction,
                    &material_names,
                    &mut selected_material_index,
                    &mut material_params.base_color_rgba,
                    &mut material_params.metallic,
                    &mut material_params.roughness,
                    &mut material_params.emissive_rgb,
                    hdr_path,
                    ibl_path,
                    skybox_path,
                    &mut environment_intensity,
                    &mut environment_apply,
                    &mut environment_generate,
                    &mut create_gltf,
                    &mut create_light,
                    &mut create_environment,
                    &mut save_scene,
                    &mut load_scene,
                    self.timing.frame_dt,
                );
                self.timing.set_render_ms(render_ms);
            }
            (
                buffer_to_string(hdr_path),
                buffer_to_string(ibl_path),
                buffer_to_string(skybox_path),
            )
        };
        let previous_selection = self.ui.selected_index();
        self.ui.set_selected_index(selected_index);
        self.ui.set_light_settings(light_settings);
        self.ui.set_selected_material_index(selected_material_index);
        self.ui.set_material_params(material_params);
        self.ui.set_environment_intensity(environment_intensity);

        if let Some(render) = &mut self.render {
            if let Some(entity) = selected_light_entity {
                render.set_light_entity(entity);
            }
            render.set_directional_light(
                light_settings.color,
                light_settings.intensity,
                light_settings.direction,
            );

            // Save edited data back to SceneObject
            if selected_index == previous_selection && selected_index >= 0 {
                if let Some(object) = self.scene.object_mut(selected_index as usize) {
                    match &mut object.kind {
                        SceneObjectKind::Asset(data) => {
                            if can_edit_transform {
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
                                    if let Some(entity) = self
                                        .scene_runtime
                                        .get(selected_index as usize)
                                        .and_then(|runtime| runtime.root_entity)
                                    {
                                        let matrix = compose_transform_matrix(
                                            data.position,
                                            data.rotation_deg,
                                            data.scale,
                                        );
                                        render.set_entity_transform(entity, matrix);
                                    }
                                }
                            }
                        }
                        SceneObjectKind::DirectionalLight(data) => {
                            data.color = light_settings.color;
                            data.intensity = light_settings.intensity;
                            data.direction = light_settings.direction;
                        }
                        SceneObjectKind::Environment(data) => {
                            data.intensity = environment_intensity;
                            data.hdr_path = hdr_path_string.clone();
                            data.ibl_path = ibl_path_string.clone();
                            data.skybox_path = skybox_path_string.clone();
                        }
                    }
                }
            }

            if (environment_intensity - previous_environment_intensity).abs() > f32::EPSILON {
                render.set_environment_intensity(environment_intensity);
            }
            if environment_generate {
                match generate_ktx_from_hdr(&hdr_path_string) {
                    Ok((ibl_path, skybox_path)) => {
                        self.ui.set_environment_status(format!(
                            "Generated KTX:\nIBL: {}\nSkybox: {}",
                            ibl_path, skybox_path
                        ));
                        let (hdr_buf, ibl_buf, sky_buf) = self.ui.environment_paths_mut();
                        write_string_to_buffer(&hdr_path_string, hdr_buf);
                        write_string_to_buffer(&ibl_path, ibl_buf);
                        write_string_to_buffer(&skybox_path, sky_buf);
                    }
                    Err(message) => {
                        self.ui.set_environment_status(message);
                    }
                }
            }
            if environment_apply {
                if ibl_path_string.is_empty() && skybox_path_string.is_empty() {
                    self.ui.set_environment_status(
                        "Environment load failed: provide KTX paths or generate from HDR."
                            .to_string(),
                    );
                } else {
                    let ok = render.set_environment(
                        &ibl_path_string,
                        &skybox_path_string,
                        environment_intensity,
                    );
                    if ok {
                        self.scene.set_environment(EnvironmentData {
                            hdr_path: hdr_path_string.clone(),
                            ibl_path: ibl_path_string.clone(),
                            skybox_path: skybox_path_string.clone(),
                            intensity: environment_intensity,
                        });
                        self.ui
                            .set_environment_status("Environment loaded.".to_string());
                    } else {
                        self.ui.set_environment_status(format!(
                            "Environment load failed for:\nIBL: {}\nSkybox: {}",
                            ibl_path_string, skybox_path_string
                        ));
                    }
                }
            }
            apply_material_changes(
                &mut self.assets,
                selected_material_index,
                previous_material_selection,
                previous_material_params,
                material_params,
            );
            if selected_material_index != previous_material_selection {
                if let Some(params) =
                    load_material_params(&mut self.assets, selected_material_index)
                {
                    self.ui.set_material_params(params);
                }
            }

        }
        if create_gltf {
            self.handle_create_gltf_action();
        }
        if create_light {
            self.handle_create_light_action();
        }
        if create_environment {
            let had_environment = self
                .scene
                .objects()
                .iter()
                .any(|object| matches!(object.kind, SceneObjectKind::Environment(_)));
            self.scene.set_environment(EnvironmentData {
                hdr_path: String::new(),
                ibl_path: String::new(),
                skybox_path: String::new(),
                intensity: 30_000.0,
            });
            if !had_environment {
                self.scene_runtime.push(RuntimeObject::default());
            }
        }
        if save_scene {
            self.handle_save_scene_action();
        }
        if load_scene {
            self.handle_load_scene_action();
        }

        self.timing
            .update(self.window.as_ref().map(|w| w.as_ref()), frame_start);
        self.update_camera();
    }

    fn handle_create_gltf_action(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF", &["gltf", "glb"])
            .pick_file()
        else {
            return;
        };
        let Some(path_str) = path.to_str() else {
            return;
        };
        let Some(render) = &mut self.render else {
            return;
        };

        let (engine, scene) = render.engine_scene_mut();
        let mut entity_manager = engine.entity_manager();
        log::info!("Loading glTF: {}", path_str);
        match self
            .assets
            .load_gltf_from_path(engine, scene, &mut entity_manager, path_str)
        {
            Ok(loaded) => {
                log::info!(
                    "Loaded glTF '{}' center={:?} extent={:?}",
                    path_str,
                    loaded.center,
                    loaded.extent
                );
                self.scene.add_asset(loaded.name.clone(), path_str);
                self.scene_runtime.push(RuntimeObject {
                    root_entity: Some(loaded.root_entity),
                    center: loaded.center,
                    extent: loaded.extent,
                });
                self.camera = CameraController::from_bounds(loaded.center, loaded.extent);
                self.camera.apply(render.camera_mut());
            }
            Err(err) => {
                log::warn!("Failed to load glTF {}: {}", path_str, err);
                self.ui
                    .set_environment_status(format!("Failed to load glTF:\n{}", err));
            }
        }
    }

    fn handle_create_light_action(&mut self) {
        let Some(render) = &mut self.render else {
            return;
        };
        let (engine, scene) = render.engine_scene_mut();
        let mut entity_manager = engine.entity_manager();
        let light_entity = engine.create_directional_light(
            &mut entity_manager,
            [1.0, 1.0, 1.0],
            100_000.0,
            [0.0, -1.0, -0.5],
        );
        scene.add_entity(light_entity);
        self.scene.add_directional_light(
            "Directional Light",
            DirectionalLightData {
                color: [1.0, 1.0, 1.0],
                intensity: 100_000.0,
                direction: [0.0, -1.0, -0.5],
            },
        );
        self.scene_runtime.push(RuntimeObject {
            root_entity: Some(light_entity),
            center: [0.0, 0.0, 0.0],
            extent: [0.0, 0.0, 0.0],
        });
        render.set_light_entity(light_entity);
    }

    fn handle_save_scene_action(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene", &["json"])
            .set_file_name("scene.json")
            .save_file()
        {
            if let Err(e) = crate::scene::serialization::save_scene_to_file(&self.scene, &path) {
                log::warn!("Failed to save scene: {}", e);
            }
        }
    }

    fn handle_load_scene_action(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene", &["json"])
            .pick_file()
        else {
            return;
        };

        match crate::scene::serialization::load_scene_from_file(&path) {
            Ok(loaded_scene) => {
                self.scene = loaded_scene;
                match self.rebuild_runtime_scene() {
                    Ok(()) => log::info!("Scene loaded from {:?}", path),
                    Err(err) => {
                        log::warn!("Scene loaded with runtime rebuild errors: {}", err);
                        self.ui
                            .set_environment_status(format!("Scene load warnings:\n{}", err));
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to load scene: {}", e);
            }
        }
    }

    fn rebuild_runtime_scene(&mut self) -> Result<(), String> {
        let Some(render) = &mut self.render else {
            return Ok(());
        };

        render.clear_scene();
        self.assets.clear();
        self.scene_runtime.clear();

        let source_objects = self.scene.objects().to_vec();
        let mut runtime_objects = Vec::with_capacity(source_objects.len());
        let mut transforms_to_apply: Vec<(Entity, [f32; 16])> = Vec::new();
        let mut first_asset_bounds: Option<([f32; 3], [f32; 3])> = None;
        let mut active_light: Option<Entity> = None;
        let mut environment_data: Option<EnvironmentData> = None;
        let mut errors: Vec<String> = Vec::new();

        {
            let (engine, scene) = render.engine_scene_mut();
            let mut entity_manager = engine.entity_manager();
            log::info!(
                "Rebuilding runtime scene from {} serialized objects",
                source_objects.len()
            );
            for object in source_objects {
                match object.kind.clone() {
                    SceneObjectKind::Asset(data) => {
                        log::info!("Rehydrate asset '{}'", data.path);
                        match self
                            .assets
                            .load_gltf_from_path(engine, scene, &mut entity_manager, &data.path)
                        {
                            Ok(loaded) => {
                                transforms_to_apply.push((
                                    loaded.root_entity,
                                    compose_transform_matrix(
                                        data.position,
                                        data.rotation_deg,
                                        data.scale,
                                    ),
                                ));
                                if first_asset_bounds.is_none() {
                                    first_asset_bounds = Some((loaded.center, loaded.extent));
                                }
                                runtime_objects.push(RuntimeObject {
                                    root_entity: Some(loaded.root_entity),
                                    center: loaded.center,
                                    extent: loaded.extent,
                                });
                            }
                            Err(err) => {
                                errors.push(format!("Asset '{}' failed to load: {}", data.path, err));
                                runtime_objects.push(RuntimeObject::default());
                            }
                        }
                    }
                    SceneObjectKind::DirectionalLight(data) => {
                        let light_entity = engine.create_directional_light(
                            &mut entity_manager,
                            data.color,
                            data.intensity,
                            data.direction,
                        );
                        scene.add_entity(light_entity);
                        if active_light.is_none() {
                            active_light = Some(light_entity);
                        }
                        runtime_objects.push(RuntimeObject {
                            root_entity: Some(light_entity),
                            center: [0.0, 0.0, 0.0],
                            extent: [0.0, 0.0, 0.0],
                        });
                    }
                    SceneObjectKind::Environment(data) => {
                        environment_data = Some(data);
                        runtime_objects.push(RuntimeObject::default());
                    }
                }
            }
        }
        self.scene_runtime.replace(runtime_objects);

        for (entity, matrix) in transforms_to_apply {
            render.set_entity_transform(entity, matrix);
        }
        if let Some(light_entity) = active_light {
            render.set_light_entity(light_entity);
        }

        if let Some(environment) = environment_data {
            let env_ok = render.set_environment(
                &environment.ibl_path,
                &environment.skybox_path,
                environment.intensity,
            );
            if env_ok {
                render.set_environment_intensity(environment.intensity);
                let (hdr, ibl, sky) = self.ui.environment_paths_mut();
                write_string_to_buffer(&environment.hdr_path, hdr);
                write_string_to_buffer(&environment.ibl_path, ibl);
                write_string_to_buffer(&environment.skybox_path, sky);
                self.ui.set_environment_intensity(environment.intensity);
                self.ui
                    .set_environment_status("Environment loaded.".to_string());
            } else if !environment.ibl_path.is_empty() || !environment.skybox_path.is_empty() {
                errors.push("Environment failed to load from scene file.".to_string());
            }
        }

        if let Some((center, extent)) = first_asset_bounds {
            self.camera = CameraController::from_bounds(center, extent);
            self.camera.apply(render.camera_mut());
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("\n"))
        }
    }

    fn map_mouse_button(button: MouseButton) -> Option<i32> {
        match button {
            MouseButton::Left => Some(0),
            MouseButton::Right => Some(1),
            MouseButton::Middle => Some(2),
            MouseButton::Other(1) => Some(3),
            MouseButton::Other(2) => Some(4),
            _ => None,
        }
    }

    fn map_imgui_key(code: KeyCode) -> Option<i32> {
        const KEY_BASE: i32 = 512;
        const IMGUI_KEY_TAB: i32 = KEY_BASE + 0;
        const IMGUI_KEY_LEFT_ARROW: i32 = KEY_BASE + 1;
        const IMGUI_KEY_RIGHT_ARROW: i32 = KEY_BASE + 2;
        const IMGUI_KEY_UP_ARROW: i32 = KEY_BASE + 3;
        const IMGUI_KEY_DOWN_ARROW: i32 = KEY_BASE + 4;
        const IMGUI_KEY_PAGE_UP: i32 = KEY_BASE + 5;
        const IMGUI_KEY_PAGE_DOWN: i32 = KEY_BASE + 6;
        const IMGUI_KEY_HOME: i32 = KEY_BASE + 7;
        const IMGUI_KEY_END: i32 = KEY_BASE + 8;
        const IMGUI_KEY_INSERT: i32 = KEY_BASE + 9;
        const IMGUI_KEY_DELETE: i32 = KEY_BASE + 10;
        const IMGUI_KEY_BACKSPACE: i32 = KEY_BASE + 11;
        const IMGUI_KEY_SPACE: i32 = KEY_BASE + 12;
        const IMGUI_KEY_ENTER: i32 = KEY_BASE + 13;
        const IMGUI_KEY_ESCAPE: i32 = KEY_BASE + 14;
        const IMGUI_KEY_LEFT_CTRL: i32 = KEY_BASE + 15;
        const IMGUI_KEY_LEFT_SHIFT: i32 = KEY_BASE + 16;
        const IMGUI_KEY_LEFT_ALT: i32 = KEY_BASE + 17;
        const IMGUI_KEY_LEFT_SUPER: i32 = KEY_BASE + 18;
        const IMGUI_KEY_RIGHT_CTRL: i32 = KEY_BASE + 19;
        const IMGUI_KEY_RIGHT_SHIFT: i32 = KEY_BASE + 20;
        const IMGUI_KEY_RIGHT_ALT: i32 = KEY_BASE + 21;
        const IMGUI_KEY_RIGHT_SUPER: i32 = KEY_BASE + 22;
        const IMGUI_KEY_MENU: i32 = KEY_BASE + 23;
        const IMGUI_KEY_0: i32 = KEY_BASE + 24;
        const IMGUI_KEY_A: i32 = KEY_BASE + 34;
        const IMGUI_KEY_F1: i32 = KEY_BASE + 60;
        const IMGUI_KEY_APOSTROPHE: i32 = KEY_BASE + 84;
        const IMGUI_KEY_COMMA: i32 = KEY_BASE + 85;
        const IMGUI_KEY_MINUS: i32 = KEY_BASE + 86;
        const IMGUI_KEY_PERIOD: i32 = KEY_BASE + 87;
        const IMGUI_KEY_SLASH: i32 = KEY_BASE + 88;
        const IMGUI_KEY_SEMICOLON: i32 = KEY_BASE + 89;
        const IMGUI_KEY_EQUAL: i32 = KEY_BASE + 90;
        const IMGUI_KEY_LEFT_BRACKET: i32 = KEY_BASE + 91;
        const IMGUI_KEY_BACKSLASH: i32 = KEY_BASE + 92;
        const IMGUI_KEY_RIGHT_BRACKET: i32 = KEY_BASE + 93;
        const IMGUI_KEY_GRAVE_ACCENT: i32 = KEY_BASE + 94;
        const IMGUI_KEY_CAPS_LOCK: i32 = KEY_BASE + 95;
        const IMGUI_KEY_SCROLL_LOCK: i32 = KEY_BASE + 96;
        const IMGUI_KEY_NUM_LOCK: i32 = KEY_BASE + 97;
        const IMGUI_KEY_PRINT_SCREEN: i32 = KEY_BASE + 98;
        const IMGUI_KEY_PAUSE: i32 = KEY_BASE + 99;
        const IMGUI_KEY_KEYPAD_0: i32 = KEY_BASE + 100;
        const IMGUI_KEY_KEYPAD_1: i32 = KEY_BASE + 101;
        const IMGUI_KEY_KEYPAD_2: i32 = KEY_BASE + 102;
        const IMGUI_KEY_KEYPAD_3: i32 = KEY_BASE + 103;
        const IMGUI_KEY_KEYPAD_4: i32 = KEY_BASE + 104;
        const IMGUI_KEY_KEYPAD_5: i32 = KEY_BASE + 105;
        const IMGUI_KEY_KEYPAD_6: i32 = KEY_BASE + 106;
        const IMGUI_KEY_KEYPAD_7: i32 = KEY_BASE + 107;
        const IMGUI_KEY_KEYPAD_8: i32 = KEY_BASE + 108;
        const IMGUI_KEY_KEYPAD_9: i32 = KEY_BASE + 109;
        const IMGUI_KEY_KEYPAD_DECIMAL: i32 = KEY_BASE + 110;
        const IMGUI_KEY_KEYPAD_DIVIDE: i32 = KEY_BASE + 111;
        const IMGUI_KEY_KEYPAD_MULTIPLY: i32 = KEY_BASE + 112;
        const IMGUI_KEY_KEYPAD_SUBTRACT: i32 = KEY_BASE + 113;
        const IMGUI_KEY_KEYPAD_ADD: i32 = KEY_BASE + 114;
        const IMGUI_KEY_KEYPAD_ENTER: i32 = KEY_BASE + 115;
        const IMGUI_KEY_KEYPAD_EQUAL: i32 = KEY_BASE + 116;
        const IMGUI_KEY_APP_BACK: i32 = KEY_BASE + 117;
        const IMGUI_KEY_APP_FORWARD: i32 = KEY_BASE + 118;
        const IMGUI_KEY_OEM_102: i32 = KEY_BASE + 119;

        match code {
            KeyCode::Tab => Some(IMGUI_KEY_TAB),
            KeyCode::ArrowLeft => Some(IMGUI_KEY_LEFT_ARROW),
            KeyCode::ArrowRight => Some(IMGUI_KEY_RIGHT_ARROW),
            KeyCode::ArrowUp => Some(IMGUI_KEY_UP_ARROW),
            KeyCode::ArrowDown => Some(IMGUI_KEY_DOWN_ARROW),
            KeyCode::PageUp => Some(IMGUI_KEY_PAGE_UP),
            KeyCode::PageDown => Some(IMGUI_KEY_PAGE_DOWN),
            KeyCode::Home => Some(IMGUI_KEY_HOME),
            KeyCode::End => Some(IMGUI_KEY_END),
            KeyCode::Insert => Some(IMGUI_KEY_INSERT),
            KeyCode::Delete => Some(IMGUI_KEY_DELETE),
            KeyCode::Backspace => Some(IMGUI_KEY_BACKSPACE),
            KeyCode::Space => Some(IMGUI_KEY_SPACE),
            KeyCode::Enter => Some(IMGUI_KEY_ENTER),
            KeyCode::Escape => Some(IMGUI_KEY_ESCAPE),
            KeyCode::ControlLeft => Some(IMGUI_KEY_LEFT_CTRL),
            KeyCode::ShiftLeft => Some(IMGUI_KEY_LEFT_SHIFT),
            KeyCode::AltLeft => Some(IMGUI_KEY_LEFT_ALT),
            KeyCode::SuperLeft => Some(IMGUI_KEY_LEFT_SUPER),
            KeyCode::ControlRight => Some(IMGUI_KEY_RIGHT_CTRL),
            KeyCode::ShiftRight => Some(IMGUI_KEY_RIGHT_SHIFT),
            KeyCode::AltRight => Some(IMGUI_KEY_RIGHT_ALT),
            KeyCode::SuperRight => Some(IMGUI_KEY_RIGHT_SUPER),
            KeyCode::ContextMenu => Some(IMGUI_KEY_MENU),
            KeyCode::Digit0 => Some(IMGUI_KEY_0 + 0),
            KeyCode::Digit1 => Some(IMGUI_KEY_0 + 1),
            KeyCode::Digit2 => Some(IMGUI_KEY_0 + 2),
            KeyCode::Digit3 => Some(IMGUI_KEY_0 + 3),
            KeyCode::Digit4 => Some(IMGUI_KEY_0 + 4),
            KeyCode::Digit5 => Some(IMGUI_KEY_0 + 5),
            KeyCode::Digit6 => Some(IMGUI_KEY_0 + 6),
            KeyCode::Digit7 => Some(IMGUI_KEY_0 + 7),
            KeyCode::Digit8 => Some(IMGUI_KEY_0 + 8),
            KeyCode::Digit9 => Some(IMGUI_KEY_0 + 9),
            KeyCode::KeyA => Some(IMGUI_KEY_A + 0),
            KeyCode::KeyB => Some(IMGUI_KEY_A + 1),
            KeyCode::KeyC => Some(IMGUI_KEY_A + 2),
            KeyCode::KeyD => Some(IMGUI_KEY_A + 3),
            KeyCode::KeyE => Some(IMGUI_KEY_A + 4),
            KeyCode::KeyF => Some(IMGUI_KEY_A + 5),
            KeyCode::KeyG => Some(IMGUI_KEY_A + 6),
            KeyCode::KeyH => Some(IMGUI_KEY_A + 7),
            KeyCode::KeyI => Some(IMGUI_KEY_A + 8),
            KeyCode::KeyJ => Some(IMGUI_KEY_A + 9),
            KeyCode::KeyK => Some(IMGUI_KEY_A + 10),
            KeyCode::KeyL => Some(IMGUI_KEY_A + 11),
            KeyCode::KeyM => Some(IMGUI_KEY_A + 12),
            KeyCode::KeyN => Some(IMGUI_KEY_A + 13),
            KeyCode::KeyO => Some(IMGUI_KEY_A + 14),
            KeyCode::KeyP => Some(IMGUI_KEY_A + 15),
            KeyCode::KeyQ => Some(IMGUI_KEY_A + 16),
            KeyCode::KeyR => Some(IMGUI_KEY_A + 17),
            KeyCode::KeyS => Some(IMGUI_KEY_A + 18),
            KeyCode::KeyT => Some(IMGUI_KEY_A + 19),
            KeyCode::KeyU => Some(IMGUI_KEY_A + 20),
            KeyCode::KeyV => Some(IMGUI_KEY_A + 21),
            KeyCode::KeyW => Some(IMGUI_KEY_A + 22),
            KeyCode::KeyX => Some(IMGUI_KEY_A + 23),
            KeyCode::KeyY => Some(IMGUI_KEY_A + 24),
            KeyCode::KeyZ => Some(IMGUI_KEY_A + 25),
            KeyCode::F1 => Some(IMGUI_KEY_F1 + 0),
            KeyCode::F2 => Some(IMGUI_KEY_F1 + 1),
            KeyCode::F3 => Some(IMGUI_KEY_F1 + 2),
            KeyCode::F4 => Some(IMGUI_KEY_F1 + 3),
            KeyCode::F5 => Some(IMGUI_KEY_F1 + 4),
            KeyCode::F6 => Some(IMGUI_KEY_F1 + 5),
            KeyCode::F7 => Some(IMGUI_KEY_F1 + 6),
            KeyCode::F8 => Some(IMGUI_KEY_F1 + 7),
            KeyCode::F9 => Some(IMGUI_KEY_F1 + 8),
            KeyCode::F10 => Some(IMGUI_KEY_F1 + 9),
            KeyCode::F11 => Some(IMGUI_KEY_F1 + 10),
            KeyCode::F12 => Some(IMGUI_KEY_F1 + 11),
            KeyCode::F13 => Some(IMGUI_KEY_F1 + 12),
            KeyCode::F14 => Some(IMGUI_KEY_F1 + 13),
            KeyCode::F15 => Some(IMGUI_KEY_F1 + 14),
            KeyCode::F16 => Some(IMGUI_KEY_F1 + 15),
            KeyCode::F17 => Some(IMGUI_KEY_F1 + 16),
            KeyCode::F18 => Some(IMGUI_KEY_F1 + 17),
            KeyCode::F19 => Some(IMGUI_KEY_F1 + 18),
            KeyCode::F20 => Some(IMGUI_KEY_F1 + 19),
            KeyCode::F21 => Some(IMGUI_KEY_F1 + 20),
            KeyCode::F22 => Some(IMGUI_KEY_F1 + 21),
            KeyCode::F23 => Some(IMGUI_KEY_F1 + 22),
            KeyCode::F24 => Some(IMGUI_KEY_F1 + 23),
            KeyCode::Quote => Some(IMGUI_KEY_APOSTROPHE),
            KeyCode::Comma => Some(IMGUI_KEY_COMMA),
            KeyCode::Minus => Some(IMGUI_KEY_MINUS),
            KeyCode::Period => Some(IMGUI_KEY_PERIOD),
            KeyCode::Slash => Some(IMGUI_KEY_SLASH),
            KeyCode::Semicolon => Some(IMGUI_KEY_SEMICOLON),
            KeyCode::Equal => Some(IMGUI_KEY_EQUAL),
            KeyCode::BracketLeft => Some(IMGUI_KEY_LEFT_BRACKET),
            KeyCode::Backslash => Some(IMGUI_KEY_BACKSLASH),
            KeyCode::BracketRight => Some(IMGUI_KEY_RIGHT_BRACKET),
            KeyCode::Backquote => Some(IMGUI_KEY_GRAVE_ACCENT),
            KeyCode::CapsLock => Some(IMGUI_KEY_CAPS_LOCK),
            KeyCode::ScrollLock => Some(IMGUI_KEY_SCROLL_LOCK),
            KeyCode::NumLock => Some(IMGUI_KEY_NUM_LOCK),
            KeyCode::PrintScreen => Some(IMGUI_KEY_PRINT_SCREEN),
            KeyCode::Pause => Some(IMGUI_KEY_PAUSE),
            KeyCode::Numpad0 => Some(IMGUI_KEY_KEYPAD_0),
            KeyCode::Numpad1 => Some(IMGUI_KEY_KEYPAD_1),
            KeyCode::Numpad2 => Some(IMGUI_KEY_KEYPAD_2),
            KeyCode::Numpad3 => Some(IMGUI_KEY_KEYPAD_3),
            KeyCode::Numpad4 => Some(IMGUI_KEY_KEYPAD_4),
            KeyCode::Numpad5 => Some(IMGUI_KEY_KEYPAD_5),
            KeyCode::Numpad6 => Some(IMGUI_KEY_KEYPAD_6),
            KeyCode::Numpad7 => Some(IMGUI_KEY_KEYPAD_7),
            KeyCode::Numpad8 => Some(IMGUI_KEY_KEYPAD_8),
            KeyCode::Numpad9 => Some(IMGUI_KEY_KEYPAD_9),
            KeyCode::NumpadDecimal => Some(IMGUI_KEY_KEYPAD_DECIMAL),
            KeyCode::NumpadDivide => Some(IMGUI_KEY_KEYPAD_DIVIDE),
            KeyCode::NumpadMultiply => Some(IMGUI_KEY_KEYPAD_MULTIPLY),
            KeyCode::NumpadSubtract => Some(IMGUI_KEY_KEYPAD_SUBTRACT),
            KeyCode::NumpadAdd => Some(IMGUI_KEY_KEYPAD_ADD),
            KeyCode::NumpadEnter => Some(IMGUI_KEY_KEYPAD_ENTER),
            KeyCode::NumpadEqual => Some(IMGUI_KEY_KEYPAD_EQUAL),
            KeyCode::BrowserBack => Some(IMGUI_KEY_APP_BACK),
            KeyCode::BrowserForward => Some(IMGUI_KEY_APP_FORWARD),
            KeyCode::IntlBackslash => Some(IMGUI_KEY_OEM_102),
            _ => None,
        }
    }

    fn sync_imgui_modifiers(render: &mut RenderContext, modifiers: Modifiers) {
        const IMGUI_MOD_CTRL: i32 = 1 << 12;
        const IMGUI_MOD_SHIFT: i32 = 1 << 13;
        const IMGUI_MOD_ALT: i32 = 1 << 14;
        const IMGUI_MOD_SUPER: i32 = 1 << 15;

        let state = modifiers.state();
        render.ui_key_event(IMGUI_MOD_CTRL, state.control_key());
        render.ui_key_event(IMGUI_MOD_SHIFT, state.shift_key());
        render.ui_key_event(IMGUI_MOD_ALT, state.alt_key());
        render.ui_key_event(IMGUI_MOD_SUPER, state.super_key());
    }
}

fn buffer_to_string(buffer: &[u8]) -> String {
    let end = buffer
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(buffer.len());
    String::from_utf8_lossy(&buffer[..end]).trim().to_string()
}

fn write_string_to_buffer(value: &str, buffer: &mut [u8]) {
    buffer.fill(0);
    let bytes = value.as_bytes();
    if buffer.is_empty() {
        return;
    }
    let max_len = buffer.len().saturating_sub(1);
    let count = bytes.len().min(max_len);
    buffer[..count].copy_from_slice(&bytes[..count]);
}

fn generate_ktx_from_hdr(hdr_path: &str) -> Result<(String, String), String> {
    if hdr_path.trim().is_empty() {
        return Err("Provide an equirect HDR path to generate KTX.".to_string());
    }
    let hdr = PathBuf::from(hdr_path.trim());
    if !hdr.exists() {
        return Err(format!("HDR file not found: {}", hdr.display()));
    }

    let stem = hdr
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("environment");
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_root = manifest_dir.join("assets").join("environments");
    let output_prefix = output_root.join(stem);
    std::fs::create_dir_all(&output_root)
        .map_err(|err| format!("Failed creating environment folder: {}", err))?;

    let cmgen_path = PathBuf::from(env!("FILAMENT_BIN_DIR")).join("cmgen.exe");
    if !cmgen_path.exists() {
        return Err(format!("cmgen not found at {}", cmgen_path.display()));
    }

    let status = Command::new(cmgen_path)
        .args([
            "-x",
            output_prefix.to_string_lossy().as_ref(),
            "--format=ktx",
            "--size=256",
            "--extract-blur=0.1",
        ])
        .arg(&hdr)
        .status()
        .map_err(|err| format!("Failed to run cmgen: {}", err))?;

    if !status.success() {
        return Err(format!("cmgen failed with status {:?}.", status.code()));
    }

    let output_dir = output_root.join(stem);
    let ibl_path = output_dir.join(format!("{stem}_ibl.ktx"));
    let skybox_path = output_dir.join(format!("{stem}_skybox.ktx"));
    Ok((
        ibl_path.to_string_lossy().to_string(),
        skybox_path.to_string_lossy().to_string(),
    ))
}

fn apply_material_changes(
    assets: &mut AssetManager,
    selected_index: i32,
    previous_index: i32,
    previous_params: MaterialParams,
    params: MaterialParams,
) {
    if selected_index < 0 || selected_index != previous_index {
        return;
    }
    if previous_params == params {
        return;
    }
    let Some(material_instance) = assets
        .material_instances_mut()
        .get_mut(selected_index as usize)
    else {
        return;
    };

    if material_instance.has_parameter("baseColorFactor") {
        material_instance.set_float4("baseColorFactor", params.base_color_rgba);
    }
    if material_instance.has_parameter("metallicFactor") {
        material_instance.set_float("metallicFactor", params.metallic);
    }
    if material_instance.has_parameter("roughnessFactor") {
        material_instance.set_float("roughnessFactor", params.roughness);
    }
    if material_instance.has_parameter("emissiveFactor") {
        material_instance.set_float3("emissiveFactor", params.emissive_rgb);
    }
}

fn load_material_params(assets: &mut AssetManager, selected_index: i32) -> Option<MaterialParams> {
    if selected_index < 0 {
        return None;
    }
    let material_instance = assets.material_instances().get(selected_index as usize)?;
    let mut params = MaterialParams {
        base_color_rgba: [1.0, 1.0, 1.0, 1.0],
        metallic: 1.0,
        roughness: 1.0,
        emissive_rgb: [0.0, 0.0, 0.0],
    };

    if let Some(value) = material_instance.get_float4("baseColorFactor") {
        params.base_color_rgba = value;
    }
    if let Some(value) = material_instance.get_float("metallicFactor") {
        params.metallic = value;
    }
    if let Some(value) = material_instance.get_float("roughnessFactor") {
        params.roughness = value;
    }
    if let Some(value) = material_instance.get_float3("emissiveFactor") {
        params.emissive_rgb = value;
    }

    Some(params)
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = WindowAttributes::default()
            .with_title("Previz - Filament v1.69.0 glTF")
            .with_inner_size(PhysicalSize::new(1280u32, 720u32))
            .with_resizable(true);

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        self.init_filament(&window);
        self.update_target_frame_duration(&window);
        self.window = Some(window);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.close_requested = true;
                event_loop.exit();
            }
            WindowEvent::Focused(focused) => {
                self.window_focused = focused;
                if !focused {
                    self.mouse_pos = None;
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    self.close_requested = true;
                    event_loop.exit();
                    return;
                }
                let pressed = event.state == winit::event::ElementState::Pressed;
                let mut ui_capture_keyboard = false;

                let modifiers = self.modifiers;
                if let Some(render) = &mut self.render {
                    Self::sync_imgui_modifiers(render, modifiers);
                    if let PhysicalKey::Code(code) = event.physical_key {
                        if let Some(imgui_key) = Self::map_imgui_key(code) {
                            render.ui_key_event(imgui_key, pressed);
                        }
                    }
                    if pressed {
                        if let Some(text) = event.text.as_ref() {
                            for ch in text.chars() {
                                render.ui_add_input_character(ch as u32);
                            }
                        }
                    }
                    ui_capture_keyboard = render.ui_want_capture_keyboard();
                }

                if !ui_capture_keyboard {
                    match self.input.handle_key(event.physical_key, pressed) {
                        InputAction::ZoomIn => {
                            self.camera.nudge(0.0, 0.0, -0.3);
                            if let Some(render) = &mut self.render {
                                self.camera.apply(render.camera_mut());
                            }
                        }
                        InputAction::ZoomOut => {
                            self.camera.nudge(0.0, 0.0, 0.3);
                            if let Some(render) = &mut self.render {
                                self.camera.apply(render.camera_mut());
                            }
                        }
                        InputAction::None => {}
                    }
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers;
                if let Some(render) = &mut self.render {
                    Self::sync_imgui_modifiers(render, modifiers);
                }
            }
            WindowEvent::Resized(new_size) => {
                let scale_factor = self
                    .window
                    .as_ref()
                    .map(|window| window.scale_factor())
                    .unwrap_or(1.0);
                self.handle_resize(new_size, scale_factor);
                if let Some(window) = self.window.clone() {
                    self.update_target_frame_duration(&window);
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(window) = self.window.as_ref() {
                    self.handle_resize(window.inner_size(), scale_factor);
                }
            }
            WindowEvent::Moved(_) => {
                if let Some(window) = self.window.clone() {
                    self.update_target_frame_duration(&window);
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = Some((position.x as f32, position.y as f32));
                if let Some(render) = &mut self.render {
                    render.ui_mouse_pos(position.x as f32, position.y as f32);
                }
            }
            WindowEvent::CursorEntered { .. } => {
                if let Some(render) = &mut self.render {
                    if let Some((mx, my)) = self.mouse_pos {
                        render.ui_mouse_pos(mx, my);
                    }
                }
            }
            WindowEvent::CursorLeft { .. } => {
                self.mouse_pos = None;
                if let Some(render) = &mut self.render {
                    render.ui_mouse_pos(-f32::MAX, -f32::MAX);
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button_index) = Self::map_mouse_button(button) {
                    let pressed = state == winit::event::ElementState::Pressed;
                    if button_index >= 0 && (button_index as usize) < self.mouse_buttons.len() {
                        self.mouse_buttons[button_index as usize] = pressed;
                    }
                    if let Some(render) = &mut self.render {
                        render.ui_mouse_button(button_index, pressed);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (wheel_x, wheel_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x, y),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                if let Some(render) = &mut self.render {
                    render.ui_mouse_wheel(wheel_x, wheel_y);
                }
            }
            WindowEvent::RedrawRequested => {
                self.render();
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        if now >= self.next_frame_time {
            if let Some(window) = &self.window {
                window.request_redraw();
            }
            self.next_frame_time = now + self.target_frame_duration;
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(self.next_frame_time));
    }
}

pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("🚀 Previz - Filament v1.69.0 Renderer POC");
    log::info!("   Press ESC or close window to exit");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");

    log::info!("👋 Goodbye!");
}
