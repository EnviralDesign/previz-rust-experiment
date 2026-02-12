mod camera;
pub mod pick;

pub use camera::{CameraController, CameraMovement};
pub use pick::{PickHit, PickKind, PickSystem};

use crate::filament::{
    Backend, Camera, Engine, Entity, ImGuiHelper, IndirectLight, Renderer, Scene, Skybox,
    SwapChain, Texture, View, MaterialInstance,
};
use std::ffi::c_void;
use std::ffi::CString;
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("native window handle unavailable: {0}")]
    NativeHandleUnavailable(String),
    #[error("unsupported window handle type for this platform")]
    UnsupportedWindowHandle,
    #[error("failed to create Filament engine")]
    EngineCreateFailed,
    #[error("failed to create Filament swap chain")]
    SwapChainCreateFailed,
    #[error("failed to create Filament renderer")]
    RendererCreateFailed,
    #[error("failed to create Filament scene")]
    SceneCreateFailed,
    #[error("failed to create Filament view")]
    ViewCreateFailed,
    #[error("failed to create Filament camera")]
    CameraCreateFailed,
    #[error("failed to access Filament entity manager")]
    EntityManagerUnavailable,
    #[error("failed to create UI view")]
    UiViewCreateFailed,
    #[error("failed to create ImGui helper")]
    UiHelperCreateFailed,
}

pub struct RenderContext {
    engine: Engine,
    swap_chain: SwapChain,
    renderer: Renderer,
    view: View,
    ui_view: Option<View>,
    ui_helper: Option<ImGuiHelper>,
    scene: Scene,
    camera: Camera,
    selected_entity: Option<Entity>,
    light_entity: Option<Entity>,
    indirect_light: Option<IndirectLight>,
    indirect_light_texture: Option<Texture>,
    skybox: Option<Skybox>,
    skybox_texture: Option<Texture>,
    material_textures: Vec<Texture>,
    // GPU pick pass
    pick_system: Option<PickSystem>,
    pick_view: Option<View>,
    pending_pick_entities: Option<Vec<(u32, Vec<Entity>)>>,
}

impl RenderContext {
    pub fn new(window: &Window) -> Result<Self, RenderError> {
        let native_handle = get_native_window_handle(window)?;
        let window_size = window.inner_size();

        let mut engine = Engine::create(Backend::OpenGL).ok_or(RenderError::EngineCreateFailed)?;
        let swap_chain = engine
            .create_swap_chain(native_handle)
            .ok_or(RenderError::SwapChainCreateFailed)?;
        let mut renderer = engine
            .create_renderer()
            .ok_or(RenderError::RendererCreateFailed)?;
        renderer.set_clear_options(0.1, 0.1, 0.2, 1.0, true, true);

        let mut scene = engine.create_scene().ok_or(RenderError::SceneCreateFailed)?;
        let mut view = engine.create_view().ok_or(RenderError::ViewCreateFailed)?;

        let mut entity_manager = engine
            .entity_manager()
            .ok_or(RenderError::EntityManagerUnavailable)?;
        let camera_entity = entity_manager.create();
        let mut camera = engine
            .create_camera(camera_entity)
            .ok_or(RenderError::CameraCreateFailed)?;

        view.set_viewport(0, 0, window_size.width, window_size.height);
        view.set_scene(&mut scene);
        view.set_camera(&mut camera);

        // Initialize GPU pick system
        let pick_system = PickSystem::new(&mut engine, window_size.width, window_size.height);
        let mut pick_view = engine.create_view();
        if let Some(pv) = &mut pick_view {
            pv.set_scene(&mut scene);
            pv.set_camera(&mut camera);
            pv.set_viewport(0, 0, window_size.width, window_size.height);
            pv.set_post_processing_enabled(false);
            if let Some(ps) = &pick_system {
                pv.set_render_target(Some(ps.render_target()));
            }
        }
        if pick_system.is_none() {
            log::warn!("GPU pick system failed to initialize; scene picking disabled.");
        }

        // No lights created at startup - user adds them via UI

        Ok(Self {
            engine,
            swap_chain,
            renderer,
            view,
            ui_view: None,
            ui_helper: None,
            scene,
            camera,
            selected_entity: None,
            light_entity: None,
            indirect_light: None,
            indirect_light_texture: None,
            skybox: None,
            skybox_texture: None,
            material_textures: Vec::new(),
            pick_system,
            pick_view,
            pending_pick_entities: None,
        })
    }

    pub fn engine_scene_mut(&mut self) -> (&mut Engine, &mut Scene) {
        (&mut self.engine, &mut self.scene)
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>, _scale_factor: f64) {
        self.view
            .set_viewport(0, 0, new_size.width, new_size.height);
        let aspect = new_size.width as f64 / new_size.height as f64;
        self.camera
            .set_projection_perspective(45.0, aspect, 0.1, 1000.0);
        if let Some(ui_view) = &mut self.ui_view {
            ui_view.set_viewport(0, 0, new_size.width, new_size.height);
        }
        // Resize pick system
        if let Some(ps) = &mut self.pick_system {
            ps.resize(&mut self.engine, new_size.width, new_size.height);
            if let Some(pv) = &mut self.pick_view {
                pv.set_viewport(0, 0, new_size.width, new_size.height);
                pv.set_render_target(Some(ps.render_target()));
            }
        }
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.set_display_size(
                new_size.width as i32,
                new_size.height as i32,
                1.0,
                1.0,
                false,
            );
        }
    }

    pub fn set_projection_for_window(&mut self, window: &Window) {
        let size = window.inner_size();
        let aspect = size.width as f64 / size.height as f64;
        self.camera
            .set_projection_perspective(45.0, aspect, 0.1, 1000.0);
    }

    pub fn init_ui(&mut self, window: &Window) -> Result<(), RenderError> {
        let mut ui_view = self
            .engine
            .create_view()
            .ok_or(RenderError::UiViewCreateFailed)?;
        ui_view.set_viewport(0, 0, window.inner_size().width, window.inner_size().height);
        let mut helper = ImGuiHelper::create(&mut self.engine, &mut ui_view, None)
            .ok_or(RenderError::UiHelperCreateFailed)?;
        let size = window.inner_size();
        helper.set_display_size(size.width as i32, size.height as i32, 1.0, 1.0, false);
        self.ui_view = Some(ui_view);
        self.ui_helper = Some(helper);
        Ok(())
    }

    pub fn ui_mouse_pos(&mut self, x: f32, y: f32) {
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.add_mouse_pos(x, y);
        }
    }

    pub fn ui_mouse_button(&mut self, button: i32, down: bool) {
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.add_mouse_button(button, down);
        }
    }

    pub fn ui_mouse_wheel(&mut self, wheel_x: f32, wheel_y: f32) {
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.add_mouse_wheel(wheel_x, wheel_y);
        }
    }

    pub fn ui_key_event(&mut self, key: i32, down: bool) {
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.add_key_event(key, down);
        }
    }

    pub fn ui_add_input_character(&mut self, codepoint: u32) {
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.add_input_character(codepoint);
        }
    }

    #[allow(dead_code)]
    pub fn ui_want_capture_mouse(&mut self) -> bool {
        self.ui_helper
            .as_mut()
            .map(|helper| helper.want_capture_mouse())
            .unwrap_or(false)
    }

    pub fn ui_want_capture_keyboard(&mut self) -> bool {
        self.ui_helper
            .as_mut()
            .map(|helper| helper.want_capture_keyboard())
            .unwrap_or(false)
    }

    pub fn render_scene_ui(
        &mut self,
        assets_title: &str,
        assets_body: &str,
        object_names: &[CString],
        selected_index: &mut i32,
        selected_kind: &mut i32,
        can_edit_transform: &mut bool,
        position_xyz: &mut [f32; 3],
        rotation_deg_xyz: &mut [f32; 3],
        scale_xyz: &mut [f32; 3],
        light_color_rgb: &mut [f32; 3],
        light_intensity: &mut f32,
        light_dir_xyz: &mut [f32; 3],
        material_names: &[CString],
        selected_material_index: &mut i32,
        material_base_color_rgba: &mut [f32; 4],
        material_metallic: &mut f32,
        material_roughness: &mut f32,
        material_emissive_rgb: &mut [f32; 3],
        material_binding_param_names: &[CString],
        material_binding_sources: &mut [u8],
        material_binding_source_stride: i32,
        material_binding_wrap_repeat_u: &mut [bool],
        material_binding_wrap_repeat_v: &mut [bool],
        material_binding_srgb: &mut [bool],
        material_binding_uv_offset: &mut [f32],
        material_binding_uv_scale: &mut [f32],
        material_binding_uv_rotation_deg: &mut [f32],
        material_binding_pick_index: &mut i32,
        material_binding_apply_index: &mut i32,
        hdr_path: &mut [u8],
        ibl_path: &mut [u8],
        skybox_path: &mut [u8],
        environment_pick_hdr: &mut bool,
        environment_pick_ibl: &mut bool,
        environment_pick_skybox: &mut bool,
        environment_intensity: &mut f32,
        environment_apply: &mut bool,
        environment_generate: &mut bool,
        create_gltf: &mut bool,
        create_light: &mut bool,
        create_environment: &mut bool,
        save_scene: &mut bool,
        load_scene: &mut bool,
        transform_tool_mode: &mut i32,
        delete_selected: &mut bool,
        gizmo_screen_points_xy: &[f32; 8],
        gizmo_visible: bool,
        gizmo_origin_world_xyz: &[f32; 3],
        camera_world_xyz: &[f32; 3],
        gizmo_active_axis: &mut i32,
        delta_seconds: f32,
    ) -> f32 {
        let frame_start = std::time::Instant::now();
        if let Some(ui_helper) = &mut self.ui_helper {
            let name_ptrs: Vec<*const std::ffi::c_char> =
                object_names.iter().map(|name| name.as_ptr()).collect();
            let material_ptrs: Vec<*const std::ffi::c_char> =
                material_names.iter().map(|name| name.as_ptr()).collect();
            let texture_param_ptrs: Vec<*const std::ffi::c_char> = material_binding_param_names
                .iter()
                .map(|name| name.as_ptr())
                .collect();
            ui_helper.render_scene_ui(
                delta_seconds,
                assets_title,
                assets_body,
                &name_ptrs,
                selected_index,
                selected_kind,
                can_edit_transform,
                position_xyz,
                rotation_deg_xyz,
                scale_xyz,
                light_color_rgb,
                light_intensity,
                light_dir_xyz,
                &material_ptrs,
                selected_material_index,
                material_base_color_rgba,
                material_metallic,
                material_roughness,
                material_emissive_rgb,
                &texture_param_ptrs,
                material_binding_sources,
                material_binding_source_stride,
                material_binding_wrap_repeat_u,
                material_binding_wrap_repeat_v,
                material_binding_srgb,
                material_binding_uv_offset,
                material_binding_uv_scale,
                material_binding_uv_rotation_deg,
                material_binding_pick_index,
                material_binding_apply_index,
                hdr_path,
                ibl_path,
                skybox_path,
                environment_pick_hdr,
                environment_pick_ibl,
                environment_pick_skybox,
                environment_intensity,
                environment_apply,
                environment_generate,
                create_gltf,
                create_light,
                create_environment,
                save_scene,
                load_scene,
                transform_tool_mode,
                delete_selected,
                gizmo_screen_points_xy,
                gizmo_visible,
                gizmo_origin_world_xyz,
                camera_world_xyz,
                gizmo_active_axis,
            );
        }
        if self.renderer.begin_frame(&mut self.swap_chain) {
            // GPU pick pass — render to offscreen RT before beauty pass
            if let Some(pickable) = self.pending_pick_entities.take() {
                if let (Some(ps), Some(pv)) = (&mut self.pick_system, &self.pick_view) {
                    ps.render_pick_pass(
                        &mut self.engine,
                        &mut self.renderer,
                        pv,
                        &pickable,
                    );
                }
            }
            self.renderer.render(&self.view);
            if let Some(ui_view) = &self.ui_view {
                self.renderer.render(ui_view);
            }
            self.renderer.end_frame();
        }
        // Pick readback — after endFrame, before next beginFrame
        if let Some(ps) = &mut self.pick_system {
            if ps.has_pending_pick() {
                if ps.schedule_readback(&mut self.renderer) {
                    self.engine.flush_and_wait();
                    ps.complete_readback();
                }
            }
        }

        let render_end = std::time::Instant::now();
        render_end
            .saturating_duration_since(frame_start)
            .as_secs_f32()
            * 1000.0
    }

    pub fn set_directional_light(&mut self, color: [f32; 3], intensity: f32, direction: [f32; 3]) {
        if let Some(entity) = self.light_entity {
            self.engine
                .set_directional_light(entity, color, intensity, direction);
        }
    }

    pub fn set_light_entity(&mut self, entity: Entity) {
        self.light_entity = Some(entity);
    }

    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        // Placeholder hook for upcoming viewport highlighting/picking.
        self.selected_entity = entity;
    }

    pub fn set_entity_transform(&mut self, entity: Entity, matrix4x4: [f32; 16]) -> bool {
        let Some(mut tm) = self.engine.transform_manager() else {
            log::warn!("Transform manager unavailable; skipping entity transform update.");
            return false;
        };
        tm.set_transform(entity, &matrix4x4);
        true
    }

    pub fn bind_material_texture_from_ktx(
        &mut self,
        material_instance: &mut MaterialInstance,
        texture_param: &str,
        ktx_path: &str,
        wrap_repeat_u: bool,
        wrap_repeat_v: bool,
    ) -> bool {
        let Some(texture) = self
            .engine
            .bind_material_texture_from_ktx(
                material_instance,
                texture_param,
                ktx_path,
                wrap_repeat_u,
                wrap_repeat_v,
            )
        else {
            return false;
        };
        self.material_textures.push(texture);
        true
    }

    pub fn set_environment(&mut self, ibl_path: &str, skybox_path: &str, intensity: f32) -> bool {
        if ibl_path.is_empty() && skybox_path.is_empty() {
            return false;
        }

        self.scene.set_indirect_light(None);
        self.scene.set_skybox(None);
        self.indirect_light = None;
        self.indirect_light_texture = None;
        self.skybox = None;
        self.skybox_texture = None;
        self.material_textures.clear();

        if !ibl_path.is_empty() {
            if let Some((light, texture)) = self
                .engine
                .create_indirect_light_from_ktx(ibl_path, intensity)
            {
                self.scene.set_indirect_light(Some(&light));
                self.indirect_light = Some(light);
                self.indirect_light_texture = Some(texture);
            } else {
                return false;
            }
        }

        if !skybox_path.is_empty() {
            if let Some((skybox, texture)) = self.engine.create_skybox_from_ktx(skybox_path) {
                self.scene.set_skybox(Some(&skybox));
                self.skybox = Some(skybox);
                self.skybox_texture = Some(texture);
            } else {
                return false;
            }
        }

        true
    }

    pub fn set_environment_intensity(&mut self, intensity: f32) {
        if let Some(light) = &mut self.indirect_light {
            light.set_intensity(intensity);
        }
    }

    pub fn clear_scene(&mut self) {
        // Remove all entities from the Filament scene except camera
        // Note: In a full implementation, we'd track all entities and remove them properly
        // For now, we create a fresh scene
        if let Some(new_scene) = self.engine.create_scene() {
            self.scene = new_scene;
        } else {
            log::error!("Failed to create replacement scene during clear_scene; keeping existing scene.");
            return;
        }
        self.view.set_scene(&mut self.scene);
        if let Some(pick_view) = &mut self.pick_view {
            pick_view.set_scene(&mut self.scene);
        }

        // Reset environment
        self.scene.set_indirect_light(None);
        self.scene.set_skybox(None);
        self.indirect_light = None;
        self.indirect_light_texture = None;
        self.skybox = None;
        self.skybox_texture = None;

        // Reset light entity reference (it will be recreated if needed)
        self.light_entity = None;
    }

    pub fn flush_and_wait(&mut self) {
        self.engine.flush_and_wait();
    }

    // ====================================================================
    // GPU Pick Pass public API
    // ====================================================================

    /// Request a GPU pick at the given screen coordinates (top-left origin).
    /// The pick will be executed on the next call to `execute_pick_pass`.
    pub fn request_pick(&mut self, screen_x: f32, screen_y: f32) {
        if let Some(ps) = &mut self.pick_system {
            ps.request_pick(screen_x, screen_y);
        }
    }

    /// Stage pickable entities for the GPU pick pass.
    /// The actual rendering happens inside render_scene_ui's frame.
    ///
    /// `pickable_entities` maps (object_id, entities) for each pickable scene object.
    pub fn execute_pick_pass(&mut self, pickable_entities: &[(u32, Vec<Entity>)]) {
        let has_pending = self.pick_system.as_ref().map_or(false, |ps| ps.has_pending_pick());
        if !has_pending {
            return;
        }
        self.pending_pick_entities = Some(pickable_entities.to_vec());
    }

    /// Take the latest pick result, if available.
    pub fn take_pick_hit(&mut self) -> Option<PickHit> {
        self.pick_system.as_mut().and_then(|ps| ps.take_hit())
    }

    /// Whether GPU picking is available.
    pub fn has_pick_system(&self) -> bool {
        self.pick_system.is_some()
    }
}

/// Get the native window handle (HWND) on Windows
#[cfg(target_os = "windows")]
fn get_native_window_handle(window: &Window) -> Result<*mut c_void, RenderError> {
    let handle = window
        .window_handle()
        .map_err(|err| RenderError::NativeHandleUnavailable(err.to_string()))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get() as *mut c_void),
        _ => Err(RenderError::UnsupportedWindowHandle),
    }
}

#[cfg(not(target_os = "windows"))]
fn get_native_window_handle(_window: &Window) -> Result<*mut c_void, RenderError> {
    Err(RenderError::UnsupportedWindowHandle)
}
