mod camera;
mod editor_overlay;
mod light_helpers;
pub mod pick;

pub use camera::{CameraController, CameraMovement};
pub use editor_overlay::GizmoParams;
pub use light_helpers::LightHelperSpec;
pub use pick::{PickHit, PickKey, PickKind, PickSystem};

use crate::filament::{
    Backend, Camera, Engine, Entity, ImGuiHelper, IndirectLight, LightParams, Material,
    MaterialInstance,
    Renderer, Scene, Skybox, SwapChain, Texture, TextureInternalFormat, TextureUsage, View,
};
use std::ffi::c_void;
use std::ffi::CString;
use std::path::{Path, PathBuf};
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
    overlay_view: Option<View>,
    ui_view: Option<View>,
    ui_helper: Option<ImGuiHelper>,
    ui_enabled: bool,
    scene: Scene,
    camera: Camera,
    selected_entity: Option<Entity>,
    selected_outline_params: Option<([f32; 3], f32)>,
    selected_renderables: Vec<Entity>,
    _selection_outline_material: Option<Material>,
    selection_outline_instance: Option<MaterialInstance>,
    selection_outline_last_applied_count: usize,
    selection_outline_unavailable_warned: bool,
    indirect_light: Option<IndirectLight>,
    indirect_light_texture: Option<Texture>,
    skybox: Option<Skybox>,
    skybox_texture: Option<Texture>,
    material_textures: Vec<Texture>,
    // GPU pick pass
    pick_system: Option<PickSystem>,
    pick_view: Option<View>,
    pending_pick_entities: Option<Vec<(PickKey, Vec<Entity>)>>,
    editor_overlay: Option<editor_overlay::EditorOverlay>,
    light_helpers: Option<light_helpers::LightHelperSystem>,
    light_helper_specs: Vec<LightHelperSpec>,
    viewport_width: u32,
    viewport_height: u32,
}

const LAYER_SCENE: u8 = 0x01;
const LAYER_OVERLAY: u8 = 0x02;
const LAYER_PICK: u8 = 0x04;
const LAYER_OUTLINE: u8 = 0x08;
const OUTLINE_EXPAND_WORLD_DEFAULT: f32 = 0.02;

struct SavedSelectionOutlineMaterials {
    entity: Entity,
    entries: Vec<(i32, *mut c_void)>,
}

struct SelectionOutlineRestore {
    saved: Vec<SavedSelectionOutlineMaterials>,
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

        let mut scene = engine
            .create_scene()
            .ok_or(RenderError::SceneCreateFailed)?;
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
        view.set_visible_layers(0xFF, LAYER_SCENE);

        let mut overlay_view = engine.create_view();
        if let Some(ov) = &mut overlay_view {
            ov.set_scene(&mut scene);
            ov.set_camera(&mut camera);
            ov.set_viewport(0, 0, window_size.width, window_size.height);
            ov.set_post_processing_enabled(false);
            ov.set_visible_layers(0xFF, LAYER_OVERLAY);
        }

        // Initialize GPU pick system
        let pick_system = PickSystem::new(&mut engine, window_size.width, window_size.height);
        let mut pick_view = engine.create_view();
        if let Some(pv) = &mut pick_view {
            pv.set_scene(&mut scene);
            pv.set_camera(&mut camera);
            pv.set_viewport(0, 0, window_size.width, window_size.height);
            pv.set_post_processing_enabled(false);
            pv.set_visible_layers(0xFF, LAYER_PICK);
            if let Some(ps) = &pick_system {
                pv.set_render_target(Some(ps.render_target()));
            }
        }
        if pick_system.is_none() {
            log::warn!("GPU pick system failed to initialize; scene picking disabled.");
        }

        // No lights created at startup - user adds them via UI

        let editor_overlay = editor_overlay::EditorOverlay::new(
            &mut engine,
            &mut scene,
            &mut entity_manager,
            LAYER_OVERLAY,
        );
        let light_helpers = light_helpers::LightHelperSystem::new(
            &mut engine,
            &mut scene,
            &mut entity_manager,
            LAYER_OVERLAY,
        );
        let mut selection_outline_material =
            engine.create_material(include_bytes!(concat!(
                env!("OUT_DIR"),
                "/selectionOutline.filamat"
            )));
        let mut selection_outline_instance = selection_outline_material
            .as_mut()
            .and_then(|mat| mat.create_instance());
        if let Some(instance) = &mut selection_outline_instance {
            instance.set_float3("tint", [1.0, 0.68, 0.24]);
            instance.set_float3("center", [0.0, 0.0, 0.0]);
            instance.set_float("expand", OUTLINE_EXPAND_WORLD_DEFAULT);
            log::info!("Selection outline material initialized.");
        } else {
            log::warn!("Selection outline material unavailable; GLTF selection outline disabled.");
        }

        Ok(Self {
            engine,
            swap_chain,
            renderer,
            view,
            overlay_view,
            ui_view: None,
            ui_helper: None,
            ui_enabled: true,
            scene,
            camera,
            selected_entity: None,
            selected_outline_params: None,
            selected_renderables: Vec::new(),
            _selection_outline_material: selection_outline_material,
            selection_outline_instance,
            selection_outline_last_applied_count: 0,
            selection_outline_unavailable_warned: false,
            indirect_light: None,
            indirect_light_texture: None,
            skybox: None,
            skybox_texture: None,
            material_textures: Vec::new(),
            pick_system,
            pick_view,
            pending_pick_entities: None,
            editor_overlay,
            light_helpers,
            light_helper_specs: Vec::new(),
            viewport_width: window_size.width.max(1),
            viewport_height: window_size.height.max(1),
        })
    }

    pub fn engine_scene_mut(&mut self) -> (&mut Engine, &mut Scene) {
        (&mut self.engine, &mut self.scene)
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>, _scale_factor: f64) {
        let width = new_size.width.max(1);
        let height = new_size.height.max(1);
        self.viewport_width = width;
        self.viewport_height = height;
        self.view
            .set_viewport(0, 0, width, height);
        let aspect = width as f64 / height as f64;
        self.camera
            .set_projection_perspective(45.0, aspect, 0.1, 1000.0);
        if let Some(overlay_view) = &mut self.overlay_view {
            overlay_view.set_viewport(0, 0, width, height);
        }
        if let Some(ui_view) = &mut self.ui_view {
            ui_view.set_viewport(0, 0, width, height);
        }
        // Resize pick system
        if let Some(ps) = &mut self.pick_system {
            ps.resize(&mut self.engine, width, height);
            if let Some(pv) = &mut self.pick_view {
                pv.set_viewport(0, 0, width, height);
                pv.set_render_target(Some(ps.render_target()));
            }
        }
        if let Some(ui_helper) = &mut self.ui_helper {
            ui_helper.set_display_size(
                width as i32,
                height as i32,
                1.0,
                1.0,
                false,
            );
        }
    }

    pub fn set_projection_for_window(&mut self, window: &Window) {
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        let aspect = width as f64 / height as f64;
        self.camera
            .set_projection_perspective(45.0, aspect, 0.1, 1000.0);
    }

    pub fn init_ui(&mut self, window: &Window) -> Result<(), RenderError> {
        let mut ui_view = self
            .engine
            .create_view()
            .ok_or(RenderError::UiViewCreateFailed)?;
        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);
        ui_view.set_viewport(0, 0, width, height);
        let mut helper = ImGuiHelper::create(&mut self.engine, &mut ui_view, None)
            .ok_or(RenderError::UiHelperCreateFailed)?;
        helper.set_display_size(width as i32, height as i32, 1.0, 1.0, false);
        self.ui_view = Some(ui_view);
        self.ui_helper = Some(helper);
        Ok(())
    }

    pub fn set_ui_enabled(&mut self, enabled: bool) {
        self.ui_enabled = enabled;
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
        light_type: &mut i32,
        light_range: &mut f32,
        light_spot_inner_deg: &mut f32,
        light_spot_outer_deg: &mut f32,
        light_sun_angular_radius_deg: &mut f32,
        light_sun_halo_size: &mut f32,
        light_sun_halo_falloff: &mut f32,
        light_cast_shadows: &mut bool,
        light_shadow_map_size: &mut i32,
        light_shadow_cascades: &mut i32,
        light_shadow_far: &mut f32,
        light_shadow_near_hint: &mut f32,
        light_shadow_far_hint: &mut f32,
        material_names: &[CString],
        selected_material_index: &mut i32,
        material_base_color_rgba: &mut [f32; 4],
        material_metallic: &mut f32,
        material_roughness: &mut f32,
        material_emissive_rgb: &mut [f32; 3],
        material_binding_param_names: &[CString],
        material_binding_sources: &mut [u8],
        material_binding_source_stride: i32,
        material_binding_wrap_repeat_u: &mut [u8],
        material_binding_wrap_repeat_v: &mut [u8],
        material_binding_srgb: &mut [u8],
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
        create_light_kind: &mut i32,
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
        if self.ui_enabled {
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
                light_type,
                light_range,
                light_spot_inner_deg,
                light_spot_outer_deg,
                light_sun_angular_radius_deg,
                light_sun_halo_size,
                light_sun_halo_falloff,
                light_cast_shadows,
                light_shadow_map_size,
                light_shadow_cascades,
                light_shadow_far,
                light_shadow_near_hint,
                light_shadow_far_hint,
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
                create_light_kind,
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
        }
        if self.renderer.begin_frame(&mut self.swap_chain) {
            // GPU pick pass — render to offscreen RT before beauty pass
            if let Some(pickable) = self.pending_pick_entities.take() {
                if let (Some(ps), Some(pv)) = (&mut self.pick_system, &self.pick_view) {
                    if let Some(overlay) = &mut self.editor_overlay {
                        overlay.set_pick_width_mode(true);
                    }
                    ps.render_pick_pass(&mut self.engine, &mut self.renderer, pv, &pickable);
                    if let Some(overlay) = &mut self.editor_overlay {
                        overlay.set_pick_width_mode(false);
                    }
                }
            }
            self.renderer.render(&self.view);
            self.render_selection_outline_pass();
            if let Some(overlay_view) = &self.overlay_view {
                self.renderer.render(overlay_view);
            }
            if self.ui_enabled {
                if let Some(ui_view) = &self.ui_view {
                    self.renderer.render(ui_view);
                }
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

    pub fn set_light(&mut self, entity: Entity, params: LightParams) {
        self.engine.set_light(entity, params);
    }

    pub fn set_selected_entity(&mut self, entity: Option<Entity>) {
        self.selected_entity = entity;
    }

    pub fn set_selected_outline_params(&mut self, params: Option<([f32; 3], f32)>) {
        self.selected_outline_params = params;
    }

    pub fn set_selected_renderables(&mut self, entities: &[Entity]) {
        let previous_count = self.selected_renderables.len();
        self.selected_renderables.clear();
        for &entity in entities {
            if self
                .selected_renderables
                .iter()
                .any(|existing| existing.id == entity.id)
            {
                continue;
            }
            self.selected_renderables.push(entity);
        }
        if self.selected_renderables.len() != previous_count {
            log::info!(
                "Outline target renderables updated: {}",
                self.selected_renderables.len()
            );
        }
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
        let Some(texture) = self.engine.bind_material_texture_from_ktx(
            material_instance,
            texture_param,
            ktx_path,
            wrap_repeat_u,
            wrap_repeat_v,
        ) else {
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
            log::error!(
                "Failed to create replacement scene during clear_scene; keeping existing scene."
            );
            return;
        }
        self.view.set_scene(&mut self.scene);
        if let Some(overlay_view) = &mut self.overlay_view {
            overlay_view.set_scene(&mut self.scene);
        }
        if let Some(pick_view) = &mut self.pick_view {
            pick_view.set_scene(&mut self.scene);
        }
        self.pending_pick_entities = None;
        self.selected_entity = None;
        self.selected_outline_params = None;
        self.selected_renderables.clear();
        if let Some(ps) = &mut self.pick_system {
            ps.reset_scene_state();
        }
        if let Some(overlay) = &self.editor_overlay {
            overlay.attach_to_scene(&mut self.scene);
        }
        self.light_helper_specs.clear();
        if let Some(mut entity_manager) = self.engine.entity_manager() {
            self.light_helpers = light_helpers::LightHelperSystem::new(
                &mut self.engine,
                &mut self.scene,
                &mut entity_manager,
                LAYER_OVERLAY,
            );
        } else {
            self.light_helpers = None;
        }

        // Reset environment
        self.scene.set_indirect_light(None);
        self.scene.set_skybox(None);
        self.indirect_light = None;
        self.indirect_light_texture = None;
        self.skybox = None;
        self.skybox_texture = None;

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
    pub fn execute_pick_pass(&mut self, pickable_entities: &[(PickKey, Vec<Entity>)]) {
        let has_pending = self
            .pick_system
            .as_ref()
            .map_or(false, |ps| ps.has_pending_pick());
        if !has_pending {
            return;
        }
        let mut merged = pickable_entities.to_vec();
        if let Some(system) = &self.light_helpers {
            merged.extend(system.pickables(&self.light_helper_specs));
        }
        if let Some(overlay) = &self.editor_overlay {
            // Keep gizmo handles last so they win pick priority over helper geometry.
            merged.extend(overlay.pickable_entities());
        }
        self.pending_pick_entities = Some(merged);
    }

    /// Take the latest pick result, if available.
    pub fn take_pick_hit(&mut self) -> Option<PickHit> {
        self.pick_system.as_mut().and_then(|ps| ps.take_hit())
    }

    /// Whether GPU picking is available.
    pub fn has_pick_system(&self) -> bool {
        self.pick_system.is_some()
    }

    pub fn update_gizmo_overlay(&mut self, params: GizmoParams) {
        if let Some(overlay) = &mut self.editor_overlay {
            overlay.set_params(&mut self.engine, params);
        }
    }

    pub fn sync_light_helpers(&mut self, specs: &[LightHelperSpec], camera_position: [f32; 3]) {
        self.light_helper_specs.clear();
        self.light_helper_specs.extend_from_slice(specs);
        let Some(system) = &mut self.light_helpers else {
            return;
        };
        let Some(mut entity_manager) = self.engine.entity_manager() else {
            log::warn!("Entity manager unavailable; skipping light helper sync.");
            return;
        };
        system.sync(
            &mut self.engine,
            &mut self.scene,
            &mut entity_manager,
            specs,
            camera_position,
        );
    }

    pub fn capture_window_png(&mut self, path: &Path, include_ui: bool) -> Result<(), String> {
        if !self.renderer.begin_frame(&mut self.swap_chain) {
            return Err("capture frame unavailable: begin_frame returned false".to_string());
        }

        let width = self.viewport_width.max(1);
        let height = self.viewport_height.max(1);
        let Some(color) = self.engine.create_texture_2d(
            width,
            height,
            TextureInternalFormat::Rgba8,
            TextureUsage::or3(
                TextureUsage::ColorAttachment,
                TextureUsage::Sampleable,
                TextureUsage::BlitSrc,
            ),
        ) else {
            self.renderer.end_frame();
            return Err("failed to create capture color texture".to_string());
        };
        let Some(depth) = self.engine.create_texture_2d(
            width,
            height,
            TextureInternalFormat::Depth24,
            TextureUsage::DepthAttachment as u32,
        ) else {
            self.renderer.end_frame();
            return Err("failed to create capture depth texture".to_string());
        };
        let Some(render_target) = self.engine.create_render_target(&color, Some(&depth)) else {
            self.renderer.end_frame();
            return Err("failed to create capture render target".to_string());
        };

        self.view.set_render_target(Some(&render_target));
        if let Some(overlay_view) = &mut self.overlay_view {
            overlay_view.set_render_target(Some(&render_target));
        }
        if include_ui {
            if let Some(ui_view) = &mut self.ui_view {
                ui_view.set_render_target(Some(&render_target));
            }
        }

        self.renderer.render(&self.view);
        self.render_selection_outline_pass();
        if let Some(overlay_view) = &self.overlay_view {
            self.renderer.render(overlay_view);
        }
        if include_ui {
            if let Some(ui_view) = &self.ui_view {
                self.renderer.render(ui_view);
            }
        }

        self.view.set_render_target(None);
        if let Some(overlay_view) = &mut self.overlay_view {
            overlay_view.set_render_target(None);
        }
        if include_ui {
            if let Some(ui_view) = &mut self.ui_view {
                ui_view.set_render_target(None);
            }
        }
        self.renderer.end_frame();

        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        if !self
            .renderer
            .read_pixels(&render_target, 0, 0, width, height, &mut pixels)
        {
            return Err("failed scheduling capture readback".to_string());
        }
        self.engine.flush_and_wait();
        Self::save_png(path, width, height, &pixels)
    }

    #[allow(dead_code)]
    fn capture_swap_chain_png(&mut self, path: &Path) -> Result<(), String> {
        let width = self.viewport_width.max(1);
        let height = self.viewport_height.max(1);
        let mut pixels = vec![0u8; (width as usize) * (height as usize) * 4];
        if !self
            .renderer
            .read_pixels_swap_chain(0, 0, width, height, &mut pixels)
        {
            return Err("failed scheduling swap chain capture readback".to_string());
        }
        self.engine.flush_and_wait();
        Self::save_png(path, width, height, &pixels)
    }

    fn save_png(path: &Path, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|err| {
                    format!(
                        "failed creating screenshot directory '{}': {}",
                        parent.display(),
                        err
                    )
                })?;
            }
        }

        image::save_buffer_with_format(
            PathBuf::from(path),
            pixels,
            width,
            height,
            image::ColorType::Rgba8,
            image::ImageFormat::Png,
        )
        .map_err(|err| format!("failed writing screenshot '{}': {}", path.display(), err))
    }

    fn begin_selection_outline_pass(&mut self) -> Option<SelectionOutlineRestore> {
        if self.selection_outline_instance.is_none() {
            if !self.selected_renderables.is_empty() && !self.selection_outline_unavailable_warned {
                log::warn!(
                    "Outline requested for {} renderables, but outline material instance is unavailable.",
                    self.selected_renderables.len()
                );
                self.selection_outline_unavailable_warned = true;
            }
            return None;
        }
        self.selection_outline_unavailable_warned = false;
        if self.selected_renderables.is_empty() {
            if self.selection_outline_last_applied_count != 0 {
                self.selection_outline_last_applied_count = 0;
            }
            return None;
        }

        if let Some(outline) = self.selection_outline_instance.as_mut() {
            let (center, expand) = self
                .selected_outline_params
                .unwrap_or(([0.0, 0.0, 0.0], OUTLINE_EXPAND_WORLD_DEFAULT));
            outline.set_float3("center", center);
            outline.set_float("expand", expand.max(0.0001));
        }
        let Some(outline) = self.selection_outline_instance.as_ref() else {
            return None;
        };

        let mut saved = Vec::new();
        for &entity in &self.selected_renderables {
            let primitive_count = self.engine.renderable_primitive_count(entity);
            if primitive_count <= 0 {
                continue;
            }
            let mut entries = Vec::with_capacity(primitive_count as usize);
            for primitive_index in 0..primitive_count {
                let original = self
                    .engine
                    .renderable_get_material_raw(entity, primitive_index);
                entries.push((primitive_index, original));
                self.engine
                    .renderable_set_material(entity, primitive_index, outline);
            }
            self.engine
                .renderable_set_layer_mask(entity, 0xFF, LAYER_OUTLINE);
            saved.push(SavedSelectionOutlineMaterials { entity, entries });
        }

        if saved.is_empty() {
            log::warn!(
                "Outline pass found selected renderables but no renderable primitives were available."
            );
            return None;
        }
        if self.selection_outline_last_applied_count != saved.len() {
            self.selection_outline_last_applied_count = saved.len();
            log::info!(
                "Outline pass applied to {} renderable entities.",
                self.selection_outline_last_applied_count
            );
        }
        Some(SelectionOutlineRestore { saved })
    }

    fn end_selection_outline_pass(&mut self, state: Option<SelectionOutlineRestore>) {
        let Some(state) = state else {
            return;
        };
        for saved_entity in state.saved {
            for (primitive_index, material_ptr) in saved_entity.entries {
                self.engine.renderable_restore_material_raw(
                    saved_entity.entity,
                    primitive_index,
                    material_ptr,
                );
            }
            self.engine
                .renderable_set_layer_mask(saved_entity.entity, 0xFF, LAYER_SCENE);
        }
    }

    fn render_selection_outline_pass(&mut self) {
        let state = self.begin_selection_outline_pass();
        if state.is_none() {
            return;
        }
        // Keep beauty pass depth/color so the outline shell can test against it.
        self.renderer
            .set_clear_options(0.1, 0.1, 0.2, 1.0, false, false);
        self.view.set_visible_layers(0xFF, LAYER_OUTLINE);
        self.renderer.render(&self.view);
        self.view.set_visible_layers(0xFF, LAYER_SCENE);
        self.renderer
            .set_clear_options(0.1, 0.1, 0.2, 1.0, true, true);
        self.end_selection_outline_pass(state);
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
