mod input;
mod timing;

use crate::assets::AssetManager;
use crate::filament::Entity;
use crate::render::{CameraController, CameraMovement, RenderContext, RenderError};
use crate::scene::{
    compose_transform_matrix, DirectionalLightData, EnvironmentData, MaterialOverrideData,
    MaterialTextureBindingData, MediaSourceKind, RuntimeObject, SceneObjectKind, SceneRuntime,
    SceneState, TextureColorSpace,
};
use crate::ui::{MaterialParams, UiState, MATERIAL_TEXTURE_PARAMS};
use glam::{EulerRot, Mat3, Vec2, Vec3};
use input::InputState;
use sha2::{Digest, Sha256};
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
    AddDirectionalLight {
        name: String,
        data: DirectionalLightData,
    },
    UpdateDirectionalLight {
        index: usize,
        data: DirectionalLightData,
    },
    SetEnvironment {
        data: EnvironmentData,
        apply_runtime: bool,
    },
    SetMaterialParam {
        object_id: u64,
        asset_path: String,
        material_slot: usize,
        material_name: String,
        data: MaterialOverrideData,
    },
    #[allow(dead_code)]
    SetMaterialTextureBinding {
        object_id: u64,
        material_slot: usize,
        binding: MaterialTextureBindingData,
    },
    TransformNode {
        index: usize,
        position: [f32; 3],
        rotation_deg: [f32; 3],
        scale: [f32; 3],
    },
    DeleteObject {
        index: usize,
    },
    SaveScene { path: PathBuf },
    LoadScene { path: PathBuf },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraDragMode {
    Orbit,
    Pan,
    Dolly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CameraControlProfile {
    Blender,
    #[allow(dead_code)]
    FpsLike,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TransformToolMode {
    Select = 0,
    Translate = 1,
    Rotate = 2,
    Scale = 3,
}

const GIZMO_NONE: i32 = 0;
const GIZMO_TRANSLATE_X: i32 = 1;
const GIZMO_TRANSLATE_Y: i32 = 2;
const GIZMO_TRANSLATE_Z: i32 = 3;
const GIZMO_TRANSLATE_XY: i32 = 4;
const GIZMO_TRANSLATE_XZ: i32 = 5;
const GIZMO_TRANSLATE_YZ: i32 = 6;
const GIZMO_TRANSLATE_SCREEN: i32 = 7;
const GIZMO_ROTATE_X: i32 = 11;
const GIZMO_ROTATE_Y: i32 = 12;
const GIZMO_ROTATE_Z: i32 = 13;
const GIZMO_ROTATE_VIEW: i32 = 14;
const GIZMO_ROTATE_ARCBALL: i32 = 15;
const GIZMO_SCALE_X: i32 = 21;
const GIZMO_SCALE_Y: i32 = 22;
const GIZMO_SCALE_Z: i32 = 23;
const GIZMO_SCALE_XY: i32 = 24;
const GIZMO_SCALE_XZ: i32 = 25;
const GIZMO_SCALE_YZ: i32 = 26;
const GIZMO_SCALE_UNIFORM: i32 = 27;
const GIZMO_BASE_DISTANCE_FACTOR: f32 = 0.18;
const GIZMO_BASE_MIN_WORLD_LEN: f32 = 0.15;
const GIZMO_GLOBAL_SCALE: f32 = 0.5;

#[derive(Debug, Clone, Copy)]
struct GizmoDragState {
    mode: TransformToolMode,
    handle: i32,
    gizmo_origin: [f32; 3],
    start_position: [f32; 3],
    start_rotation_deg: [f32; 3],
    start_scale: [f32; 3],
    start_axis_param: f32,
    start_hit_world: [f32; 3],
    drag_plane_normal: [f32; 3],
    uniform_scale_start_radius: f32,
    axis_world_length: f32,
    arcball_radius_px: f32,
    arcball_last_mouse: (f32, f32),
}

enum CommandSeverity {
    Info,
    Warning,
}

struct CommandNotice {
    severity: CommandSeverity,
    message: String,
}

enum CommandOutcome {
    None,
    Notice(CommandNotice),
}

#[derive(Debug, thiserror::Error)]
enum CommandError {
    #[error("render context not initialized")]
    RenderNotInitialized,
    #[error(transparent)]
    Asset(#[from] crate::assets::AssetError),
    #[error(transparent)]
    SceneIo(#[from] crate::scene::serialization::SerializationError),
    #[error("environment load failed: provide KTX paths or generate from HDR")]
    EnvironmentPathsMissing,
    #[error("environment load failed for IBL '{ibl}' and skybox '{skybox}' (check file paths)")]
    EnvironmentLoadFailed { ibl: String, skybox: String },
    #[error("scene object at index {index} not found")]
    SceneObjectNotFound { index: usize },
    #[error("scene object at index {index} is not transformable")]
    SceneObjectNotTransformable { index: usize },
    #[error("scene object at index {index} is not a directional light")]
    SceneObjectNotDirectionalLight { index: usize },
    #[error("render entity manager unavailable")]
    RenderEntityManagerUnavailable,
    #[error("render transform manager unavailable")]
    RenderTransformManagerUnavailable,
    #[error("texture binding source path is empty")]
    TextureBindingSourceEmpty,
}

pub struct App {
    window: Option<Arc<Window>>,
    assets: AssetManager,
    scene: SceneState,
    scene_runtime: SceneRuntime,
    selection_id: Option<u64>,
    ui: UiState,
    input: InputState,
    modifiers: Modifiers,
    mouse_pos: Option<(f32, f32)>,
    mouse_buttons: [bool; 5],
    pending_click_select: bool,
    camera_drag_mode: Option<CameraDragMode>,
    camera_control_profile: CameraControlProfile,
    transform_tool_mode: TransformToolMode,
    gizmo_active_axis: i32,
    gizmo_hover_axis: i32,
    gizmo_drag_state: Option<GizmoDragState>,
    delete_selection_requested: bool,
    orbit_pivot: [f32; 3],
    window_focused: bool,
    camera: CameraController,
    timing: FrameTiming,
    target_frame_duration: Duration,
    next_frame_time: Instant,
    close_requested: bool,
    render: Option<RenderContext>,
}

impl Drop for App {
    fn drop(&mut self) {
        // Drop asset-owned material instances before render-owned textures.
        self.assets = AssetManager::new();
        if let Some(render) = &mut self.render {
            render.flush_and_wait();
        }
        self.render = None;
    }
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            assets: AssetManager::new(),
            scene: SceneState::new(),
            scene_runtime: SceneRuntime::new(),
            selection_id: None,
            ui: UiState::new(),
            input: InputState::default(),
            modifiers: Modifiers::default(),
            mouse_pos: None,
            mouse_buttons: [false; 5],
            pending_click_select: false,
            camera_drag_mode: None,
            camera_control_profile: CameraControlProfile::Blender,
            transform_tool_mode: TransformToolMode::Translate,
            gizmo_active_axis: 0,
            gizmo_hover_axis: 0,
            gizmo_drag_state: None,
            delete_selection_requested: false,
            orbit_pivot: [0.0, 0.0, 0.0],
            window_focused: true,
            camera: CameraController::new([0.0, 0.0, 3.0], 0.6, 0.3),
            timing: FrameTiming::new("Previz - Filament v1.69.0 glTF".to_string()),
            target_frame_duration: Duration::from_millis(16),
            next_frame_time: Instant::now(),
            close_requested: false,
            render: None,
        }
    }

    fn init_filament(&mut self, window: &Window) -> Result<(), RenderError> {
        let mut render = RenderContext::new(window)?;

        // Start with empty scene - no default objects
        self.camera = CameraController::new([0.0, 0.0, 5.0], 0.0, 0.0);
        self.orbit_pivot = [0.0, 0.0, 0.0];
        render.set_projection_for_window(window);
        self.camera.apply(render.camera_mut());
        render.init_ui(window)?;

        self.render = Some(render);
        Ok(())
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

    fn mouse_over_sidebar_ui(&self) -> bool {
        let Some((mx, my)) = self.mouse_pos else {
            return false;
        };
        let Some(window) = self.window.as_ref() else {
            return false;
        };
        let size = window.inner_size();
        let width = size.width as f32;
        let height = size.height as f32;
        if mx < 0.0 || my < 0.0 || mx > width || my > height {
            return false;
        }

        // Keep in sync with side-pane layout in build_support/bindings.cpp.
        let left_width = width * 0.22;
        let right_width = width * 0.30;
        let gutter = 12.0;
        mx <= left_width || mx >= (width - right_width - gutter)
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
            self.apply_camera_to_render();
        }
    }

    fn sanitize_camera_state(&mut self) {
        let valid_position = self
            .camera
            .position
            .iter()
            .all(|value| value.is_finite() && value.abs() < 1_000_000.0);
        let valid_angles = self.camera.yaw.is_finite() && self.camera.pitch.is_finite();
        if !(valid_position && valid_angles) {
            log::warn!("Camera state invalid; resetting to safe defaults.");
            self.camera = CameraController::new([0.0, 0.0, 5.0], 0.0, 0.0);
        }
    }

    fn apply_camera_to_render(&mut self) {
        self.sanitize_camera_state();
        if let Some(render) = &mut self.render {
            self.camera.apply(render.camera_mut());
        }
    }

    fn nudge_camera(&mut self, yaw_delta: f32, pitch_delta: f32, zoom_delta: f32) {
        self.camera.nudge(yaw_delta, pitch_delta, zoom_delta);
        self.apply_camera_to_render();
    }

    fn orbit_camera(&mut self, dx: f32, dy: f32) {
        let orbit_speed = 0.008;
        self.camera
            .orbit_around(self.orbit_pivot, dx * orbit_speed, -dy * orbit_speed);
        self.apply_camera_to_render();
    }

    fn pan_camera(&mut self, dx: f32, dy: f32) {
        let pan_speed = 0.004;
        let right_amount = -dx * pan_speed;
        let up_amount = dy * pan_speed;
        let (_forward_dir, right_dir, up_dir) = self.camera.basis();
        let delta = [
            right_dir[0] * right_amount + up_dir[0] * up_amount,
            right_dir[1] * right_amount + up_dir[1] * up_amount,
            right_dir[2] * right_amount + up_dir[2] * up_amount,
        ];
        self.camera.position[0] += delta[0];
        self.camera.position[1] += delta[1];
        self.camera.position[2] += delta[2];
        self.orbit_pivot[0] += delta[0];
        self.orbit_pivot[1] += delta[1];
        self.orbit_pivot[2] += delta[2];
        self.apply_camera_to_render();
    }

    fn dolly_camera(&mut self, delta: f32) {
        self.nudge_camera(0.0, 0.0, delta);
    }

    fn focus_selected(&mut self) -> bool {
        let Some(selected) = self.current_selection_index() else {
            return false;
        };
        let Some(runtime) = self.scene_runtime.get(selected) else {
            return false;
        };

        let extent = runtime.extent;
        if extent[0] <= 0.0 && extent[1] <= 0.0 && extent[2] <= 0.0 {
            return false;
        }
        self.orbit_pivot = runtime.center;
        self.camera
            .frame_bounds_preserve_orientation(runtime.center, extent);
        self.apply_camera_to_render();
        true
    }

    fn selected_asset_transform(&self) -> Option<(usize, [f32; 3], [f32; 3], [f32; 3])> {
        let selected = self.current_selection_index()?;
        let object = self.scene.objects().get(selected)?;
        match &object.kind {
            SceneObjectKind::Asset(data) => {
                Some((selected, data.position, data.rotation_deg, data.scale))
            }
            _ => None,
        }
    }

    fn axis_unit(axis: i32) -> [f32; 3] {
        match axis {
            1 => [1.0, 0.0, 0.0],
            2 => [0.0, 1.0, 0.0],
            3 => [0.0, 0.0, 1.0],
            _ => [0.0, 0.0, 0.0],
        }
    }

    fn gizmo_axis_from_handle(handle: i32) -> Option<i32> {
        match handle {
            GIZMO_TRANSLATE_X | GIZMO_ROTATE_X | GIZMO_SCALE_X => Some(1),
            GIZMO_TRANSLATE_Y | GIZMO_ROTATE_Y | GIZMO_SCALE_Y => Some(2),
            GIZMO_TRANSLATE_Z | GIZMO_ROTATE_Z | GIZMO_SCALE_Z => Some(3),
            _ => None,
        }
    }

    fn gizmo_plane_normal_from_handle(handle: i32) -> Option<[f32; 3]> {
        match handle {
            GIZMO_TRANSLATE_XY | GIZMO_SCALE_XY => Some([0.0, 0.0, 1.0]),
            GIZMO_TRANSLATE_XZ | GIZMO_SCALE_XZ => Some([0.0, 1.0, 0.0]),
            GIZMO_TRANSLATE_YZ | GIZMO_SCALE_YZ => Some([1.0, 0.0, 0.0]),
            _ => None,
        }
    }

    fn gizmo_plane_axes_from_handle(handle: i32) -> Option<([f32; 3], [f32; 3])> {
        match handle {
            GIZMO_TRANSLATE_XY | GIZMO_SCALE_XY => Some(([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])),
            GIZMO_TRANSLATE_XZ | GIZMO_SCALE_XZ => Some(([1.0, 0.0, 0.0], [0.0, 0.0, 1.0])),
            GIZMO_TRANSLATE_YZ | GIZMO_SCALE_YZ => Some(([0.0, 1.0, 0.0], [0.0, 0.0, 1.0])),
            _ => None,
        }
    }

    fn closest_axis_param_from_screen(
        &self,
        mouse: (f32, f32),
        axis_origin: [f32; 3],
        axis_unit: [f32; 3],
    ) -> Option<f32> {
        let (ray_origin, ray_dir) = self.viewport_ray(mouse.0, mouse.1)?;
        closest_line_line_param(ray_origin, ray_dir, axis_origin, axis_unit)
    }

    fn ray_plane_hit(
        &self,
        mouse: (f32, f32),
        plane_origin: [f32; 3],
        plane_normal: [f32; 3],
    ) -> Option<[f32; 3]> {
        let (ray_origin, ray_dir) = self.viewport_ray(mouse.0, mouse.1)?;
        ray_plane_intersection(ray_origin, ray_dir, plane_origin, plane_normal)
    }

    fn gizmo_axis_world_length(&self, origin: [f32; 3]) -> f32 {
        let to_camera = [
            origin[0] - self.camera.position[0],
            origin[1] - self.camera.position[1],
            origin[2] - self.camera.position[2],
        ];
        let distance = (to_camera[0] * to_camera[0]
            + to_camera[1] * to_camera[1]
            + to_camera[2] * to_camera[2])
            .sqrt()
            .max(0.1);
        (distance * GIZMO_BASE_DISTANCE_FACTOR).max(GIZMO_BASE_MIN_WORLD_LEN) * GIZMO_GLOBAL_SCALE
    }

    fn apply_transform_tool_drag(&mut self, mouse: (f32, f32)) {
        let Some(state_snapshot) = self.gizmo_drag_state else {
            return;
        };
        let Some(index) = self.current_selection_index() else {
            return;
        };
        let mut position = state_snapshot.start_position;
        let mut rotation_deg = state_snapshot.start_rotation_deg;
        let mut scale = state_snapshot.start_scale;

        match state_snapshot.mode {
            TransformToolMode::Select => return,
            TransformToolMode::Translate => {
                if let Some(axis) = Self::gizmo_axis_from_handle(state_snapshot.handle) {
                    let axis_unit = Self::axis_unit(axis);
                    if let Some(t) =
                        self.closest_axis_param_from_screen(mouse, state_snapshot.gizmo_origin, axis_unit)
                    {
                        let delta = t - state_snapshot.start_axis_param;
                        position[0] += axis_unit[0] * delta;
                        position[1] += axis_unit[1] * delta;
                        position[2] += axis_unit[2] * delta;
                    }
                } else if let Some(hit) = self.ray_plane_hit(
                    mouse,
                    state_snapshot.gizmo_origin,
                    state_snapshot.drag_plane_normal,
                ) {
                    let delta = [
                        hit[0] - state_snapshot.start_hit_world[0],
                        hit[1] - state_snapshot.start_hit_world[1],
                        hit[2] - state_snapshot.start_hit_world[2],
                    ];
                    position[0] += delta[0];
                    position[1] += delta[1];
                    position[2] += delta[2];
                }
            }
            TransformToolMode::Rotate => {
                let mut axis = [0.0f32, 0.0, 0.0];
                if state_snapshot.handle == GIZMO_ROTATE_ARCBALL {
                    // Delta-based trackball: constant angular gain regardless of
                    // cursor distance from the arcball center.  Each pixel of mouse
                    // motion produces the same amount of rotation (≈ 1/radius_px rad).
                    let prev_mouse = if let Some(state_mut) = self.gizmo_drag_state.as_mut() {
                        let pm = state_mut.arcball_last_mouse;
                        state_mut.arcball_last_mouse = mouse;
                        pm
                    } else {
                        return;
                    };

                    let dx = mouse.0 - prev_mouse.0;
                    let dy = mouse.1 - prev_mouse.1;
                    let delta_len_sq = dx * dx + dy * dy;
                    if delta_len_sq < 0.25 {
                        // Sub-pixel motion – skip to avoid jitter.
                        return;
                    }
                    let delta_len = delta_len_sq.sqrt();
                    let r = state_snapshot.arcball_radius_px.max(1.0);
                    let angle = delta_len / r;

                    // Rotation axis in camera space: perpendicular to the screen-
                    // space displacement direction.  Convention matches the classic
                    // arcball cross-product at the sphere center:
                    //   mouse right  → rotate around camera-up   (+Y_cam)
                    //   mouse up     → rotate around camera-right (-X_cam)
                    let axis_cam = Vec3::new(dy / delta_len, dx / delta_len, 0.0);
                    let axis_world = self.camera_vec_to_world(axis_cam.to_array());
                    let axis_world_v = Vec3::from_array(axis_world).normalize_or_zero();
                    if axis_world_v.length_squared() <= 1e-10 {
                        return;
                    }

                    // Read the *current* rotation for incremental accumulation.
                    if let Some(object) = self.scene.objects().get(index) {
                        if let SceneObjectKind::Asset(data) = &object.kind {
                            rotation_deg = data.rotation_deg;
                        }
                    }
                    let start_mat = euler_deg_to_mat3(rotation_deg);
                    let delta_mat = Mat3::from_axis_angle(axis_world_v, angle);
                    let out_mat = delta_mat * start_mat;
                    rotation_deg = mat3_to_euler_deg(out_mat);
                    let result = self.execute_scene_command(SceneCommand::TransformNode {
                        index,
                        position,
                        rotation_deg,
                        scale,
                    });
                    self.apply_command_feedback("Failed to transform via tool drag", result);
                    return;
                } else if state_snapshot.handle == GIZMO_ROTATE_VIEW {
                    let (forward, _, _) = self.camera.basis();
                    axis = forward;
                } else if let Some(axis_id) = Self::gizmo_axis_from_handle(state_snapshot.handle) {
                    axis = Self::axis_unit(axis_id);
                }
                if axis == [0.0, 0.0, 0.0] {
                    return;
                }
                let Some(hit) = self.ray_plane_hit(mouse, state_snapshot.gizmo_origin, axis) else {
                    return;
                };
                let start_vec = Vec3::from_array(state_snapshot.start_hit_world) - Vec3::from_array(state_snapshot.gizmo_origin);
                let cur_vec = Vec3::from_array(hit) - Vec3::from_array(state_snapshot.gizmo_origin);
                if start_vec.length_squared() <= 1e-10 || cur_vec.length_squared() <= 1e-10 {
                    return;
                }
                let v0 = start_vec.normalize();
                let v1 = cur_vec.normalize();
                let n = Vec3::from_array(axis).normalize_or_zero();
                if n.length_squared() <= 1e-10 {
                    return;
                }
                let cross = v0.cross(v1);
                let sin_v = n.dot(cross);
                let cos_v = v0.dot(v1).clamp(-1.0, 1.0);
                let angle = sin_v.atan2(cos_v);
                let start_mat = euler_deg_to_mat3(state_snapshot.start_rotation_deg);
                let delta_mat = Mat3::from_axis_angle(n, angle);
                let out_mat = delta_mat * start_mat;
                rotation_deg = mat3_to_euler_deg(out_mat);
            }
            TransformToolMode::Scale => {
                if let Some(axis_id) = Self::gizmo_axis_from_handle(state_snapshot.handle) {
                    let axis_unit = Self::axis_unit(axis_id);
                    if let Some(t) =
                        self.closest_axis_param_from_screen(mouse, state_snapshot.gizmo_origin, axis_unit)
                    {
                        let delta = t - state_snapshot.start_axis_param;
                        let factor = (1.0 + (delta / state_snapshot.axis_world_length.max(0.001))).max(0.01);
                        match axis_id {
                            1 => scale[0] = state_snapshot.start_scale[0] * factor,
                            2 => scale[1] = state_snapshot.start_scale[1] * factor,
                            3 => scale[2] = state_snapshot.start_scale[2] * factor,
                            _ => {}
                        }
                    }
                } else if state_snapshot.handle == GIZMO_SCALE_UNIFORM {
                    let Some(center_screen) = self.world_to_screen(state_snapshot.gizmo_origin) else {
                        return;
                    };
                    let radius = Vec2::new(mouse.0 - center_screen[0], mouse.1 - center_screen[1]).length();
                    if state_snapshot.uniform_scale_start_radius > 1e-4 && radius.is_finite() {
                        let factor = (radius / state_snapshot.uniform_scale_start_radius).clamp(0.01, 100.0);
                        scale = [
                            state_snapshot.start_scale[0] * factor,
                            state_snapshot.start_scale[1] * factor,
                            state_snapshot.start_scale[2] * factor,
                        ];
                    }
                } else if let Some((a_axis, b_axis)) = Self::gizmo_plane_axes_from_handle(state_snapshot.handle) {
                    let Some(hit) = self.ray_plane_hit(
                        mouse,
                        state_snapshot.gizmo_origin,
                        state_snapshot.drag_plane_normal,
                    ) else {
                        return;
                    };
                    let delta = [
                        hit[0] - state_snapshot.start_hit_world[0],
                        hit[1] - state_snapshot.start_hit_world[1],
                        hit[2] - state_snapshot.start_hit_world[2],
                    ];
                    let da = dot3(delta, a_axis);
                    let db = dot3(delta, b_axis);
                    let fa = (1.0 + (da / state_snapshot.axis_world_length.max(0.001))).max(0.01);
                    let fb = (1.0 + (db / state_snapshot.axis_world_length.max(0.001))).max(0.01);
                    if a_axis[0] > 0.5 || b_axis[0] > 0.5 {
                        scale[0] = state_snapshot.start_scale[0] * if a_axis[0] > 0.5 { fa } else { fb };
                    }
                    if a_axis[1] > 0.5 || b_axis[1] > 0.5 {
                        scale[1] = state_snapshot.start_scale[1] * if a_axis[1] > 0.5 { fa } else { fb };
                    }
                    if a_axis[2] > 0.5 || b_axis[2] > 0.5 {
                        scale[2] = state_snapshot.start_scale[2] * if a_axis[2] > 0.5 { fa } else { fb };
                    }
                }
            }
        }

        let result = self.execute_scene_command(SceneCommand::TransformNode {
            index,
            position,
            rotation_deg,
            scale,
        });
        self.apply_command_feedback("Failed to transform via tool drag", result);
    }

    fn begin_gizmo_drag_if_needed(&mut self, mouse: (f32, f32)) {
        if self.gizmo_drag_state.is_some() || self.gizmo_active_axis == GIZMO_NONE {
            return;
        }
        let Some((_, start_position, start_rotation_deg, start_scale)) = self.selected_asset_transform()
        else {
            return;
        };
        let gizmo_origin = start_position;
        let handle = self.gizmo_active_axis;
        let mut start_axis_param = 0.0;
        let mut drag_plane_normal = [0.0, 0.0, 0.0];
        let mut start_hit_world = gizmo_origin;
        let mut uniform_scale_start_radius = 1.0;
        let mut arcball_radius_px = 64.0f32;
        let arcball_last_mouse = mouse;
        let axis_world_length = self.gizmo_axis_world_length(gizmo_origin);

        if let Some(axis) = Self::gizmo_axis_from_handle(handle) {
            let axis_unit = Self::axis_unit(axis);
            start_axis_param = self
                .closest_axis_param_from_screen(mouse, gizmo_origin, axis_unit)
                .unwrap_or(0.0);
            if self.transform_tool_mode == TransformToolMode::Rotate {
                drag_plane_normal = axis_unit;
                if let Some(hit) = self.ray_plane_hit(mouse, gizmo_origin, drag_plane_normal) {
                    start_hit_world = hit;
                } else {
                    start_hit_world = [
                        gizmo_origin[0] + axis_unit[0] * axis_world_length,
                        gizmo_origin[1] + axis_unit[1] * axis_world_length,
                        gizmo_origin[2] + axis_unit[2] * axis_world_length,
                    ];
                }
            }
        } else if handle == GIZMO_ROTATE_VIEW {
            let (forward, _, _) = self.camera.basis();
            drag_plane_normal = forward;
            if let Some(hit) = self.ray_plane_hit(mouse, gizmo_origin, drag_plane_normal) {
                start_hit_world = hit;
            }
        } else if handle == GIZMO_ROTATE_ARCBALL {
            if let Some(reference) = self.gizmo_axis_screen_reference_len(gizmo_origin) {
                arcball_radius_px = (reference * 0.86).max(20.0);
            }
        } else if handle == GIZMO_TRANSLATE_SCREEN {
            let (forward, _, _) = self.camera.basis();
            drag_plane_normal = forward;
            if let Some(hit) = self.ray_plane_hit(mouse, gizmo_origin, drag_plane_normal) {
                start_hit_world = hit;
            }
        } else if handle == GIZMO_SCALE_UNIFORM {
            if let Some(center_screen) = self.world_to_screen(gizmo_origin) {
                uniform_scale_start_radius =
                    Vec2::new(mouse.0 - center_screen[0], mouse.1 - center_screen[1]).length();
            }
        } else if let Some(normal) = Self::gizmo_plane_normal_from_handle(handle) {
            drag_plane_normal = normal;
            if let Some(hit) = self.ray_plane_hit(mouse, gizmo_origin, drag_plane_normal) {
                start_hit_world = hit;
            }
        }

        self.gizmo_drag_state = Some(GizmoDragState {
            mode: self.transform_tool_mode,
            handle,
            gizmo_origin,
            start_position,
            start_rotation_deg,
            start_scale,
            start_axis_param,
            start_hit_world,
            drag_plane_normal,
            uniform_scale_start_radius,
            axis_world_length,
            arcball_radius_px,
            arcball_last_mouse,
        });
    }

    fn camera_vec_to_world(&self, v: [f32; 3]) -> [f32; 3] {
        let (_, right, up) = self.camera.basis();
        let (forward, _, _) = self.camera.basis();
        [
            right[0] * v[0] + up[0] * v[1] + forward[0] * v[2],
            right[1] * v[0] + up[1] * v[1] + forward[1] * v[2],
            right[2] * v[0] + up[2] * v[1] + forward[2] * v[2],
        ]
    }

    fn gizmo_axis_screen_reference_len(&self, origin: [f32; 3]) -> Option<f32> {
        let center = self.world_to_screen(origin)?;
        let axis_world_len = self.gizmo_axis_world_length(origin);
        let x = self.world_to_screen([origin[0] + axis_world_len, origin[1], origin[2]])?;
        let y = self.world_to_screen([origin[0], origin[1] + axis_world_len, origin[2]])?;
        let z = self.world_to_screen([origin[0], origin[1], origin[2] + axis_world_len])?;
        let dx = ((x[0] - center[0]).powi(2) + (x[1] - center[1]).powi(2)).sqrt();
        let dy = ((y[0] - center[0]).powi(2) + (y[1] - center[1]).powi(2)).sqrt();
        let dz = ((z[0] - center[0]).powi(2) + (z[1] - center[1]).powi(2)).sqrt();
        Some(dx.max(dy).max(dz).max(1.0))
    }

    fn viewport_ray(&self, screen_x: f32, screen_y: f32) -> Option<([f32; 3], [f32; 3])> {
        let window = self.window.as_ref()?;
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return None;
        }
        let width = size.width as f32;
        let height = size.height as f32;
        let ndc_x = (2.0 * screen_x / width) - 1.0;
        let ndc_y = 1.0 - (2.0 * screen_y / height);
        let aspect = width / height;
        let tan_half_fov = (45.0f32.to_radians() * 0.5).tan();
        let view_x = ndc_x * aspect * tan_half_fov;
        let view_y = ndc_y * tan_half_fov;
        let (forward, right, up) = self.camera.basis();
        let mut dir = [
            right[0] * view_x + up[0] * view_y + forward[0],
            right[1] * view_x + up[1] * view_y + forward[1],
            right[2] * view_x + up[2] * view_y + forward[2],
        ];
        let len = (dir[0] * dir[0] + dir[1] * dir[1] + dir[2] * dir[2]).sqrt();
        if len <= 1e-6 {
            return None;
        }
        dir[0] /= len;
        dir[1] /= len;
        dir[2] /= len;
        Some((self.camera.position, dir))
    }

    fn world_to_screen(&self, world: [f32; 3]) -> Option<[f32; 2]> {
        let window = self.window.as_ref()?;
        let size = window.inner_size();
        if size.width == 0 || size.height == 0 {
            return None;
        }
        let width = size.width as f32;
        let height = size.height as f32;
        let aspect = width / height;
        let tan_half_fov = (45.0f32.to_radians() * 0.5).tan();
        let rel = [
            world[0] - self.camera.position[0],
            world[1] - self.camera.position[1],
            world[2] - self.camera.position[2],
        ];
        let (forward, right, up) = self.camera.basis();
        let view_x = rel[0] * right[0] + rel[1] * right[1] + rel[2] * right[2];
        let view_y = rel[0] * up[0] + rel[1] * up[1] + rel[2] * up[2];
        let view_z = rel[0] * forward[0] + rel[1] * forward[1] + rel[2] * forward[2];
        if view_z <= 0.001 {
            return None;
        }
        let ndc_x = view_x / (view_z * tan_half_fov * aspect);
        let ndc_y = view_y / (view_z * tan_half_fov);
        if !ndc_x.is_finite() || !ndc_y.is_finite() {
            return None;
        }
        let screen_x = (ndc_x + 1.0) * 0.5 * width;
        let screen_y = (1.0 - ndc_y) * 0.5 * height;
        Some([screen_x, screen_y])
    }

    fn selection_to_ui_index(selection: Option<usize>) -> i32 {
        selection
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(-1)
    }

    fn normalize_selection(raw_index: i32, len: usize) -> Option<usize> {
        if raw_index < 0 {
            return None;
        }
        let index = usize::try_from(raw_index).ok()?;
        if index < len {
            Some(index)
        } else {
            None
        }
    }

    fn current_selection_index(&self) -> Option<usize> {
        let selection_id = self.selection_id?;
        self.scene
            .objects()
            .iter()
            .position(|object| object.id == selection_id)
    }

    fn set_selection_from_index(&mut self, index: Option<usize>) {
        self.selection_id = index.and_then(|idx| self.scene.objects().get(idx).map(|object| object.id));
    }

    fn render(&mut self) {
        let frame_start = Instant::now();
        self.ui.update(&self.scene, &self.scene_runtime, &self.assets);
        let ui_text = self.ui.summary().to_string();
        let object_names: Vec<CString> = self
            .scene
            .object_names()
            .into_iter()
            .map(sanitize_cstring)
            .collect();
        let mut selected_index = Self::selection_to_ui_index(self.current_selection_index());
        let mut position = [0.0f32; 3];
        let mut rotation = [0.0f32; 3];
        let mut scale = [1.0f32; 3];
        let mut can_edit_transform = false;
        let mut selected_kind = -1i32;
        let mut light_settings = self.ui.light_settings();
        let mut environment_intensity = self.ui.environment_intensity();
        let mut selected_light_entity: Option<Entity> = None;
        let mut original_asset_transform: Option<([f32; 3], [f32; 3], [f32; 3])> = None;
        let mut original_light_data: Option<DirectionalLightData> = None;
        let mut original_environment_data: Option<EnvironmentData> = None;

        if let Some(selected) = Self::normalize_selection(selected_index, self.scene.objects().len()) {
            if let Some(object) = self.scene.objects().get(selected) {
                can_edit_transform = matches!(object.kind, SceneObjectKind::Asset(_));
                selected_kind = match &object.kind {
                    SceneObjectKind::Asset(data) => {
                        position = data.position;
                        rotation = data.rotation_deg;
                        scale = data.scale;
                        original_asset_transform = Some((data.position, data.rotation_deg, data.scale));
                        0
                    }
                    SceneObjectKind::DirectionalLight(data) => {
                        light_settings.color = data.color;
                        light_settings.intensity = data.intensity;
                        light_settings.direction = data.direction;
                        original_light_data = Some(data.clone());
                        selected_light_entity = self
                            .scene_runtime
                            .get(selected)
                            .and_then(|runtime| runtime.root_entity);
                        1
                    }
                    SceneObjectKind::Environment(data) => {
                        environment_intensity = data.intensity;
                        original_environment_data = Some(data.clone());
                        2
                    }
                };
            }
        }
        let mut gizmo_screen_points_xy = [f32::NAN; 8];
        let mut gizmo_visible = false;
        let mut gizmo_origin_world_xyz = [f32::NAN; 3];
        let camera_world_xyz = self.camera.position;
        let mut gizmo_axis_world_len = 1.0f32;
        if let Some(selected) = Self::normalize_selection(selected_index, self.scene.objects().len()) {
            if let Some(object) = self.scene.objects().get(selected) {
                let world = match &object.kind {
                    SceneObjectKind::Asset(data) => data.position,
                    SceneObjectKind::DirectionalLight(_) => [0.0, 0.0, 0.0],
                    SceneObjectKind::Environment(_) => self.orbit_pivot,
                };
                gizmo_origin_world_xyz = world;
                if let Some(center_screen) = self.world_to_screen(world) {
                    gizmo_screen_points_xy[0] = center_screen[0];
                    gizmo_screen_points_xy[1] = center_screen[1];
                    let axis_world_len = self.gizmo_axis_world_length(world);
                    gizmo_axis_world_len = axis_world_len;
                    let x_world = [world[0] + axis_world_len, world[1], world[2]];
                    let y_world = [world[0], world[1] + axis_world_len, world[2]];
                    let z_world = [world[0], world[1], world[2] + axis_world_len];
                    if let Some(p) = self.world_to_screen(x_world) {
                        gizmo_screen_points_xy[2] = p[0];
                        gizmo_screen_points_xy[3] = p[1];
                    }
                    if let Some(p) = self.world_to_screen(y_world) {
                        gizmo_screen_points_xy[4] = p[0];
                        gizmo_screen_points_xy[5] = p[1];
                    }
                    if let Some(p) = self.world_to_screen(z_world) {
                        gizmo_screen_points_xy[6] = p[0];
                        gizmo_screen_points_xy[7] = p[1];
                    }
                    gizmo_visible = true;
                }
            }
        }
        let scoped_material_indices = scoped_material_indices_for_selection(
            &self.scene,
            &self.assets,
            self.current_selection_index(),
        );
        let material_names: Vec<CString> = scoped_material_indices
            .iter()
            .filter_map(|index| self.assets.material_binding(*index).map(|binding| binding.material_name.as_str()))
            .map(sanitize_cstring)
            .collect();

        let previous_material_global_index = self.ui.selected_material_index();
        let mut selected_material_index =
            global_material_index_to_ui_index(&scoped_material_indices, previous_material_global_index);
        let mut material_params = self.ui.material_params();
        let previous_material_selection = previous_material_global_index;
        let previous_material_params = material_params;
        let original_material_binding = if previous_material_global_index >= 0 {
            self.assets
                .material_binding(previous_material_global_index as usize)
                .cloned()
        } else {
            None
        };
        let previous_environment_intensity = self.ui.environment_intensity();
        let mut environment_apply = false;
        let mut environment_generate = false;
        let mut environment_pick_hdr = false;
        let mut environment_pick_ibl = false;
        let mut environment_pick_skybox = false;
        let mut create_gltf = false;
        let mut create_light = false;
        let mut create_environment = false;
        let mut save_scene = false;
        let mut load_scene = false;
        let mut transform_tool_mode = self.transform_tool_mode as i32;
        let mut gizmo_active_axis = self.gizmo_active_axis;
        let mut delete_selected = false;
        let mut material_binding_pick_index = -1i32;
        let mut material_binding_apply_index = -1i32;
        let material_binding_param_names: Vec<CString> = MATERIAL_TEXTURE_PARAMS
            .iter()
            .map(|param| sanitize_cstring(param))
            .collect();
        let mut material_binding_sources =
            vec![0u8; MATERIAL_TEXTURE_PARAMS.len() * 260];
        let mut material_binding_wrap_repeat_u = vec![true; MATERIAL_TEXTURE_PARAMS.len()];
        let mut material_binding_wrap_repeat_v = vec![true; MATERIAL_TEXTURE_PARAMS.len()];
        let mut material_binding_srgb = vec![true; MATERIAL_TEXTURE_PARAMS.len()];
        let mut material_binding_uv_offset = vec![0.0f32; MATERIAL_TEXTURE_PARAMS.len() * 2];
        let mut material_binding_uv_scale = vec![1.0f32; MATERIAL_TEXTURE_PARAMS.len() * 2];
        let mut material_binding_uv_rotation_deg = vec![0.0f32; MATERIAL_TEXTURE_PARAMS.len()];
        {
            let rows = self.ui.material_binding_rows();
            for (row_index, row) in rows.iter().enumerate() {
                let start = row_index * 260;
                let end = start + 260;
                material_binding_sources[start..end].copy_from_slice(&row.source);
                material_binding_wrap_repeat_u[row_index] = row.wrap_repeat_u;
                material_binding_wrap_repeat_v[row_index] = row.wrap_repeat_v;
                material_binding_srgb[row_index] = row.srgb;
                material_binding_uv_offset[row_index * 2] = row.uv_offset[0];
                material_binding_uv_offset[row_index * 2 + 1] = row.uv_offset[1];
                material_binding_uv_scale[row_index * 2] = row.uv_scale[0];
                material_binding_uv_scale[row_index * 2 + 1] = row.uv_scale[1];
                material_binding_uv_rotation_deg[row_index] = row.uv_rotation_deg;
            }
        }
        let mut pending_transform_command: Option<SceneCommand> = None;
        let mut pending_update_light_command: Option<SceneCommand> = None;
        let mut pending_update_environment_command: Option<SceneCommand> = None;
        let mut pending_set_material_command: Option<SceneCommand> = None;
        let mut pick_hit: Option<crate::render::PickHit> = None;
        let has_active_selection = self.current_selection_index().is_some();
        let (mut hdr_path_string, mut ibl_path_string, mut skybox_path_string) = {
            let (hdr_path, ibl_path, skybox_path) = self.ui.environment_paths_mut();
            if let Some(render) = &mut self.render {
                let (mx, my) = if self.window_focused {
                    self.mouse_pos.unwrap_or((-f32::MAX, -f32::MAX))
                } else {
                    (-f32::MAX, -f32::MAX)
                };
                let (camera_forward, _camera_right, camera_up) = self.camera.basis();
                let highlighted_handle = if self.gizmo_active_axis != GIZMO_NONE {
                    self.gizmo_active_axis
                } else {
                    self.gizmo_hover_axis
                };
                render.update_gizmo_overlay(crate::render::GizmoParams {
                    visible: gizmo_visible && can_edit_transform,
                    mode: self.transform_tool_mode as i32,
                    origin: gizmo_origin_world_xyz,
                    axis_world_len: gizmo_axis_world_len,
                    camera_position: camera_world_xyz,
                    camera_forward,
                    camera_up,
                    viewport_height_px: self
                        .window
                        .as_ref()
                        .map(|w| w.inner_size().height)
                        .unwrap_or(1),
                    camera_fov_y_degrees: 45.0,
                    highlighted_handle,
                    selected_object_index: Self::normalize_selection(
                        selected_index,
                        self.scene.objects().len(),
                    )
                    .and_then(|idx| u32::try_from(idx).ok()),
                });
                render.ui_mouse_pos(mx, my);
                for (index, down) in self.mouse_buttons.iter().enumerate() {
                    render.ui_mouse_button(index as i32, *down);
                }

                // GPU pick pass — execute before the beauty pass
                if render.has_pick_system() {
                    // In transform modes with an active selection, stage only overlay keys so
                    // gizmo handles are not occluded by scene mesh hits.
                    let include_scene_keys = !matches!(
                        self.transform_tool_mode,
                        TransformToolMode::Translate | TransformToolMode::Rotate | TransformToolMode::Scale
                    ) || !has_active_selection;
                    let pickable_entities: Vec<(crate::render::PickKey, Vec<crate::filament::Entity>)> =
                        if include_scene_keys {
                            self.scene
                                .objects()
                                .iter()
                                .enumerate()
                                .filter_map(|(index, obj)| {
                                    if let SceneObjectKind::Asset(_) = &obj.kind {
                                        let loaded = self.assets.loaded_assets().iter().find(|a| {
                                            self.scene_runtime.get(index)
                                                .and_then(|rt| rt.root_entity)
                                                .map_or(false, |re| re == a.root_entity)
                                        });
                                        loaded.map(|a| {
                                            (
                                                crate::render::PickKey::scene_mesh(index as u32),
                                                a.renderable_entities.clone(),
                                            )
                                        })
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        } else {
                            Vec::new()
                        };
                    render.execute_pick_pass(&pickable_entities);
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
                    &material_binding_param_names,
                    &mut material_binding_sources,
                    260,
                    &mut material_binding_wrap_repeat_u,
                    &mut material_binding_wrap_repeat_v,
                    &mut material_binding_srgb,
                    &mut material_binding_uv_offset,
                    &mut material_binding_uv_scale,
                    &mut material_binding_uv_rotation_deg,
                    &mut material_binding_pick_index,
                    &mut material_binding_apply_index,
                    hdr_path,
                    ibl_path,
                    skybox_path,
                    &mut environment_pick_hdr,
                    &mut environment_pick_ibl,
                    &mut environment_pick_skybox,
                    &mut environment_intensity,
                    &mut environment_apply,
                    &mut environment_generate,
                    &mut create_gltf,
                    &mut create_light,
                    &mut create_environment,
                    &mut save_scene,
                    &mut load_scene,
                    &mut transform_tool_mode,
                    &mut delete_selected,
                    &gizmo_screen_points_xy,
                    gizmo_visible,
                    &gizmo_origin_world_xyz,
                    &camera_world_xyz,
                    &mut gizmo_active_axis,
                    self.timing.frame_dt,
                );
                self.timing.set_render_ms(render_ms);

                // Capture GPU pick result (processed after borrow scope)
                pick_hit = render.take_pick_hit();
            }
            (
                buffer_to_string(hdr_path),
                buffer_to_string(ibl_path),
                buffer_to_string(skybox_path),
            )
        };
        // Process GPU pick result (outside borrow scope)
        if let Some(hit) = pick_hit {
            if hit.is_none() {
                if self.transform_tool_mode == TransformToolMode::Select {
                    selected_index = -1;
                } else {
                    if self.mouse_buttons[0] {
                        self.gizmo_active_axis = GIZMO_NONE;
                        gizmo_active_axis = GIZMO_NONE;
                    } else {
                        self.gizmo_hover_axis = GIZMO_NONE;
                    }
                }
            } else if hit.key.kind == crate::render::PickKind::SceneMesh {
                let index = hit.key.object_id as usize;
                selected_index = i32::try_from(index).unwrap_or(-1);
                self.gizmo_active_axis = GIZMO_NONE;
                gizmo_active_axis = GIZMO_NONE;
                self.gizmo_hover_axis = GIZMO_NONE;
            } else if matches!(
                hit.key.kind,
                crate::render::PickKind::GizmoAxis
                    | crate::render::PickKind::GizmoPlane
                    | crate::render::PickKind::GizmoRing
            ) {
                if self.mouse_buttons[0] {
                    self.gizmo_active_axis = hit.key.sub_id as i32;
                    gizmo_active_axis = self.gizmo_active_axis;
                } else {
                    self.gizmo_hover_axis = hit.key.sub_id as i32;
                }
            }
        }
        // Hover highlight: while idle in transform modes, pick continuously under cursor.
        if self.transform_tool_mode != TransformToolMode::Select
            && !self.mouse_buttons[0]
            && self.gizmo_drag_state.is_none()
            && has_active_selection
            && !self.mouse_over_sidebar_ui()
        {
            if let (Some((mx, my)), Some(render)) = (self.mouse_pos, &mut self.render) {
                render.request_pick(mx, my);
            }
        } else if self.transform_tool_mode == TransformToolMode::Select || !has_active_selection {
            self.gizmo_hover_axis = GIZMO_NONE;
        }
        // Do not acquire a gizmo after mouse-down. Engagement is decided strictly
        // at the press event's pick location to prevent drag-into-handle locking.
        if self.mouse_buttons[0] && self.gizmo_active_axis != GIZMO_NONE && self.gizmo_drag_state.is_none() {
            if let Some(mouse) = self.mouse_pos {
                self.begin_gizmo_drag_if_needed(mouse);
            }
        }
        let previous_selection_id = self.selection_id;
        self.set_selection_from_index(Self::normalize_selection(selected_index, self.scene.objects().len()));
        selected_index = Self::selection_to_ui_index(self.current_selection_index());
        self.ui.set_selected_index(selected_index);
        self.ui.set_light_settings(light_settings);
        let selected_material_global_index =
            ui_material_index_to_global_index(&scoped_material_indices, selected_material_index);
        self.ui
            .set_selected_material_index(selected_material_global_index);
        self.ui.set_material_params(material_params);
        {
            let rows = self.ui.material_binding_rows_mut();
            for (row_index, row) in rows.iter_mut().enumerate() {
                let start = row_index * 260;
                let end = start + 260;
                row.source.copy_from_slice(&material_binding_sources[start..end]);
                row.wrap_repeat_u = material_binding_wrap_repeat_u[row_index];
                row.wrap_repeat_v = material_binding_wrap_repeat_v[row_index];
                row.srgb = material_binding_srgb[row_index];
                row.uv_offset = [
                    material_binding_uv_offset[row_index * 2],
                    material_binding_uv_offset[row_index * 2 + 1],
                ];
                row.uv_scale = [
                    material_binding_uv_scale[row_index * 2],
                    material_binding_uv_scale[row_index * 2 + 1],
                ];
                row.uv_rotation_deg = material_binding_uv_rotation_deg[row_index];
            }
        }
        self.ui.set_environment_intensity(environment_intensity);
        self.transform_tool_mode = match transform_tool_mode {
            0 => TransformToolMode::Select,
            2 => TransformToolMode::Rotate,
            3 => TransformToolMode::Scale,
            _ => TransformToolMode::Translate,
        };
        self.gizmo_active_axis = gizmo_active_axis;
        let selected_runtime_entity = self
            .current_selection_index()
            .and_then(|index| self.scene_runtime.get(index))
            .and_then(|runtime| runtime.root_entity);
        let current_selection_index = self.current_selection_index();
        if self.selection_id != previous_selection_id {
            if let Some(selected) = current_selection_index {
                if let Some(runtime) = self.scene_runtime.get(selected) {
                    if runtime.extent[0] > 0.0 || runtime.extent[1] > 0.0 || runtime.extent[2] > 0.0
                    {
                        self.orbit_pivot = runtime.center;
                    }
                }
            }
        }

        if let Some(render) = &mut self.render {
            render.set_selected_entity(selected_runtime_entity);
            if let Some(entity) = selected_light_entity {
                render.set_light_entity(entity);
                render.set_directional_light(
                    light_settings.color,
                    light_settings.intensity,
                    light_settings.direction,
                );
            }
            if self.selection_id == previous_selection_id {
                if let Some(selected) = current_selection_index {
                    if can_edit_transform {
                        if let Some((old_position, old_rotation, old_scale)) = original_asset_transform
                        {
                            if old_position != position
                                || old_rotation != rotation
                                || old_scale != scale
                            {
                                pending_transform_command = Some(SceneCommand::TransformNode {
                                    index: selected,
                                    position,
                                    rotation_deg: rotation,
                                    scale,
                                });
                            }
                        }
                    }

                    if let Some(old_light) = &original_light_data {
                        let new_light = DirectionalLightData {
                            color: light_settings.color,
                            intensity: light_settings.intensity,
                            direction: light_settings.direction,
                        };
                        if old_light != &new_light {
                            pending_update_light_command = Some(SceneCommand::UpdateDirectionalLight {
                                index: selected,
                                data: new_light,
                            });
                        }
                    }

                    if let Some(old_environment) = &original_environment_data {
                        let new_environment = EnvironmentData {
                            hdr_path: hdr_path_string.clone(),
                            ibl_path: ibl_path_string.clone(),
                            skybox_path: skybox_path_string.clone(),
                            intensity: environment_intensity,
                        };
                        if old_environment != &new_environment {
                            pending_update_environment_command = Some(SceneCommand::SetEnvironment {
                                data: new_environment,
                                apply_runtime: false,
                            });
                        }
                    }
                }
            }

            if (environment_intensity - previous_environment_intensity).abs() > f32::EPSILON {
                render.set_environment_intensity(environment_intensity);
            }
            if selected_material_global_index == previous_material_selection
                && previous_material_params != material_params
            {
                if let Some(binding) = original_material_binding.clone() {
                    pending_set_material_command = Some(SceneCommand::SetMaterialParam {
                        object_id: binding.object_id,
                        asset_path: binding.asset_path,
                        material_slot: binding.material_slot,
                        material_name: binding.material_name,
                        data: material_params_to_override(material_params),
                    });
                }
            }
            if selected_material_global_index != previous_material_selection {
                if let Some(params) =
                    load_material_params(&mut self.assets, selected_material_global_index)
                {
                    self.ui.set_material_params(params);
                }
                sync_material_binding_rows_from_scene(
                    &mut self.ui,
                    &self.scene,
                    &self.assets,
                    selected_material_global_index,
                );
            }

        }
        if let Some(command) = pending_transform_command {
            let result = self.execute_scene_command(command);
            self.apply_command_feedback("Failed to transform node", result);
        }
        if let Some(command) = pending_update_light_command {
            let result = self.execute_scene_command(command);
            self.apply_command_feedback("Failed to update directional light", result);
        }
        if let Some(command) = pending_update_environment_command {
            let result = self.execute_scene_command(command);
            self.apply_command_feedback("Failed to update environment", result);
        }
        if let Some(command) = pending_set_material_command {
            let result = self.execute_scene_command(command);
            self.apply_command_feedback("Failed to update material parameters", result);
        }
        if delete_selected || self.delete_selection_requested {
            self.delete_selection_requested = false;
            if let Some(index) = self.current_selection_index() {
                let result = self.execute_scene_command(SceneCommand::DeleteObject { index });
                self.apply_command_feedback("Failed to delete selected object", result);
            }
        }
        let mut effective_apply_index = material_binding_apply_index;
        if material_binding_pick_index >= 0 {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Texture", &["ktx", "png", "jpg", "jpeg"])
                .pick_file()
            {
                if let Some(path_string) = path.to_str() {
                    let picked_index = material_binding_pick_index as usize;
                    if picked_index < MATERIAL_TEXTURE_PARAMS.len() {
                        if let Some(row) = self.ui.material_binding_rows_mut().get_mut(picked_index)
                        {
                            write_string_to_buffer(path_string, &mut row.source);
                            effective_apply_index = material_binding_pick_index;
                        }
                    }
                }
            }
        }
        if effective_apply_index >= 0 && selected_material_global_index >= 0 {
            let apply_index = effective_apply_index as usize;
            if apply_index < MATERIAL_TEXTURE_PARAMS.len() {
                if let Some(binding) = self
                    .assets
                    .material_binding(selected_material_global_index as usize)
                    .cloned()
                {
                    let row = self.ui.material_binding_rows()[apply_index];
                    let source_path = buffer_to_string(&row.source);
                    let color_space = if row.srgb {
                        TextureColorSpace::Srgb
                    } else {
                        TextureColorSpace::Linear
                    };
                    match prepare_texture_binding_data(
                        MATERIAL_TEXTURE_PARAMS[apply_index],
                        &source_path,
                        row.wrap_repeat_u,
                        row.wrap_repeat_v,
                        color_space,
                        row.uv_offset,
                        row.uv_scale,
                        row.uv_rotation_deg,
                    ) {
                        Ok(texture_binding) => {
                            let result = self.execute_scene_command(
                                SceneCommand::SetMaterialTextureBinding {
                                    object_id: binding.object_id,
                                    material_slot: binding.material_slot,
                                    binding: texture_binding,
                                },
                            );
                            self.apply_command_feedback(
                                "Failed to set material texture binding",
                                result,
                            );
                        }
                        Err(message) => {
                            self.ui.set_environment_status(format!(
                                "Failed to prepare texture binding:\n{}",
                                message
                            ));
                        }
                    }
                }
            }
        }
        if environment_pick_hdr {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("HDR", &["hdr"])
                .pick_file()
            {
                if let Some(path_string) = path.to_str() {
                    hdr_path_string = path_string.to_string();
                    let (_tex_param, _tex_source, hdr_buf, _ibl_buf, _sky_buf) =
                        self.ui.texture_and_environment_paths_mut();
                    write_string_to_buffer(path_string, hdr_buf);
                    environment_apply = true;
                }
            }
        }
        if environment_apply {
            match generate_ktx_from_hdr(&hdr_path_string) {
                Ok((resolved_ibl_path, resolved_skybox_path)) => {
                    ibl_path_string = resolved_ibl_path;
                    skybox_path_string = resolved_skybox_path;
                    let (_tex_param, _tex_source, _hdr_buf, ibl_buf, sky_buf) =
                        self.ui.texture_and_environment_paths_mut();
                    write_string_to_buffer(&ibl_path_string, ibl_buf);
                    write_string_to_buffer(&skybox_path_string, sky_buf);
                    let result = self.execute_scene_command(SceneCommand::SetEnvironment {
                        data: EnvironmentData {
                            hdr_path: hdr_path_string.clone(),
                            ibl_path: ibl_path_string.clone(),
                            skybox_path: skybox_path_string.clone(),
                            intensity: environment_intensity,
                        },
                        apply_runtime: true,
                    });
                    self.apply_command_feedback(
                        "Failed to apply environment",
                        result,
                    );
                }
                Err(message) => {
                    self.ui.set_environment_status(message);
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
            let result = self.execute_scene_command(SceneCommand::SetEnvironment {
                data: EnvironmentData {
                    hdr_path: String::new(),
                    ibl_path: String::new(),
                    skybox_path: String::new(),
                    intensity: 30_000.0,
                },
                apply_runtime: false,
            });
            self.apply_command_feedback(
                "Failed to initialize environment object",
                result,
            );
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

    fn execute_scene_command(&mut self, command: SceneCommand) -> Result<CommandOutcome, CommandError> {
        match command {
            SceneCommand::AddAsset { path } => self.command_add_asset(&path),
            SceneCommand::AddDirectionalLight { name, data } => {
                self.command_add_directional_light(&name, data)
            }
            SceneCommand::UpdateDirectionalLight { index, data } => {
                self.command_update_directional_light(index, data)
            }
            SceneCommand::SetEnvironment {
                data,
                apply_runtime,
            } => self.command_set_environment(data, apply_runtime),
            SceneCommand::SetMaterialParam {
                object_id,
                asset_path,
                material_slot,
                material_name,
                data,
            } => self.command_set_material_param(
                object_id,
                &asset_path,
                material_slot,
                &material_name,
                data,
            ),
            SceneCommand::SetMaterialTextureBinding {
                object_id,
                material_slot,
                binding,
            } => self.command_set_material_texture_binding(object_id, material_slot, binding),
            SceneCommand::TransformNode {
                index,
                position,
                rotation_deg,
                scale,
            } => self.command_transform_node(index, position, rotation_deg, scale),
            SceneCommand::DeleteObject { index } => self.command_delete_object(index),
            SceneCommand::SaveScene { path } => self.command_save_scene(&path),
            SceneCommand::LoadScene { path } => self.command_load_scene(&path),
        }
    }

    fn apply_command_feedback(
        &mut self,
        context: &str,
        result: Result<CommandOutcome, CommandError>,
    ) {
        match result {
            Ok(CommandOutcome::None) => {}
            Ok(CommandOutcome::Notice(notice)) => {
                match notice.severity {
                    CommandSeverity::Info => log::info!("{}", notice.message),
                    CommandSeverity::Warning => log::warn!("{}", notice.message),
                }
                self.ui.set_environment_status(notice.message);
            }
            Err(err) => {
                log::warn!("{}: {}", context, err);
                self.ui
                    .set_environment_status(format!("{}:\n{}", context, err));
            }
        }
    }

    fn command_add_asset(&mut self, path: &str) -> Result<CommandOutcome, CommandError> {
        let Some(render) = &mut self.render else {
            return Err(CommandError::RenderNotInitialized);
        };

        let (engine, scene) = render.engine_scene_mut();
        let mut entity_manager = engine
            .entity_manager()
            .ok_or(CommandError::RenderEntityManagerUnavailable)?;
        let object_id = self.scene.reserve_object_id();
        log::info!("Loading glTF: {}", path);
        let loaded = self
            .assets
            .load_gltf_from_path(engine, scene, &mut entity_manager, path, object_id)
            ?;
        for entity in &loaded.renderable_entities {
            engine.renderable_set_layer_mask(*entity, 0xFF, 0x01);
        }

        log::info!(
            "Loaded glTF '{}' center={:?} extent={:?}",
            path,
            loaded.center,
            loaded.extent
        );
        self.scene
            .add_asset_with_id(object_id, loaded.name.clone(), path);
        self.scene_runtime.push(RuntimeObject {
            root_entity: Some(loaded.root_entity),
            center: loaded.center,
            extent: loaded.extent,
        });
        apply_scene_material_overrides_to_runtime(&self.scene, &mut self.assets);
        self.orbit_pivot = loaded.center;
        self.camera = CameraController::from_bounds(loaded.center, loaded.extent);
        self.camera.apply(render.camera_mut());

        Ok(CommandOutcome::None)
    }

    fn command_add_directional_light(
        &mut self,
        name: &str,
        data: DirectionalLightData,
    ) -> Result<CommandOutcome, CommandError> {
        let Some(render) = &mut self.render else {
            return Err(CommandError::RenderNotInitialized);
        };
        let (engine, scene) = render.engine_scene_mut();
        let mut entity_manager = engine
            .entity_manager()
            .ok_or(CommandError::RenderEntityManagerUnavailable)?;
        let light_entity = engine.create_directional_light(
            &mut entity_manager,
            data.color,
            data.intensity,
            data.direction,
        );
        scene.add_entity(light_entity);
        self.scene.add_directional_light(name, data);
        self.scene_runtime.push(RuntimeObject {
            root_entity: Some(light_entity),
            center: [0.0, 0.0, 0.0],
            extent: [0.0, 0.0, 0.0],
        });
        render.set_light_entity(light_entity);

        Ok(CommandOutcome::None)
    }

    fn command_update_directional_light(
        &mut self,
        index: usize,
        data: DirectionalLightData,
    ) -> Result<CommandOutcome, CommandError> {
        let object = self
            .scene
            .object_mut(index)
            .ok_or(CommandError::SceneObjectNotFound { index })?;
        match &mut object.kind {
            SceneObjectKind::DirectionalLight(existing) => {
                *existing = data.clone();
            }
            _ => return Err(CommandError::SceneObjectNotDirectionalLight { index }),
        }

        let Some(render) = &mut self.render else {
            return Err(CommandError::RenderNotInitialized);
        };
        if let Some(light_entity) = self
            .scene_runtime
            .get(index)
            .and_then(|runtime| runtime.root_entity)
        {
            render.set_light_entity(light_entity);
        }
        render.set_directional_light(data.color, data.intensity, data.direction);
        Ok(CommandOutcome::None)
    }

    fn command_set_environment(
        &mut self,
        data: EnvironmentData,
        apply_runtime: bool,
    ) -> Result<CommandOutcome, CommandError> {
        let had_environment = self
            .scene
            .objects()
            .iter()
            .any(|object| matches!(object.kind, SceneObjectKind::Environment(_)));

        if apply_runtime {
            if data.ibl_path.is_empty() && data.skybox_path.is_empty() {
                return Err(CommandError::EnvironmentPathsMissing);
            }
            let Some(render) = &mut self.render else {
                return Err(CommandError::RenderNotInitialized);
            };
            let ok = render.set_environment(&data.ibl_path, &data.skybox_path, data.intensity);
            if !ok {
                return Err(CommandError::EnvironmentLoadFailed {
                    ibl: data.ibl_path.clone(),
                    skybox: data.skybox_path.clone(),
                });
            }
            render.set_environment_intensity(data.intensity);
        }

        self.scene.set_environment(data);
        if !had_environment {
            self.scene_runtime.push(RuntimeObject::default());
        }

        if apply_runtime {
            Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Info,
                message: "Environment loaded.".to_string(),
            }))
        } else {
            Ok(CommandOutcome::None)
        }
    }

    fn command_set_material_param(
        &mut self,
        object_id: u64,
        asset_path: &str,
        material_slot: usize,
        material_name: &str,
        data: MaterialOverrideData,
    ) -> Result<CommandOutcome, CommandError> {
        self.scene.set_material_override(
            object_id,
            asset_path.to_string(),
            material_slot,
            material_name.to_string(),
            data.clone(),
        );

        let mut applied_any = false;
        let binding_len = self.assets.material_instances().len();
        for index in 0..binding_len {
            let Some(binding) = self.assets.material_binding(index) else {
                continue;
            };
            if binding.object_id != object_id || binding.material_slot != material_slot {
                continue;
            }
            if let Some(material_instance) = self.assets.material_instances_mut().get_mut(index) {
                apply_material_override(material_instance, &data);
                applied_any = true;
            }
            break;
        }

        if applied_any {
            Ok(CommandOutcome::None)
        } else {
            Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!(
                    "Material override saved but active slot not found for '{}[{}]'.",
                    asset_path, material_slot
                ),
            }))
        }
    }

    fn command_set_material_texture_binding(
        &mut self,
        object_id: u64,
        material_slot: usize,
        binding: MaterialTextureBindingData,
    ) -> Result<CommandOutcome, CommandError> {
        if binding.source_path.trim().is_empty() {
            return Err(CommandError::TextureBindingSourceEmpty);
        }
        self.scene
            .set_texture_binding(object_id, material_slot, binding.clone());

        let Some(render) = &mut self.render else {
            return Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Info,
                message: format!(
                    "Stored texture binding '{}' for object {} slot {}.",
                    binding.texture_param, object_id, material_slot
                ),
            }));
        };

        let Some(runtime_path) = texture_binding_runtime_path(&binding) else {
            return Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!(
                    "Stored texture binding '{}' but no runtime .ktx path is available.",
                    binding.texture_param
                ),
            }));
        };

        let (assets, render_ref) = (&mut self.assets, render);
        let index_opt = (0..assets.material_instances().len()).find(|index| {
            assets
                .material_binding(*index)
                .map(|entry| entry.object_id == object_id && entry.material_slot == material_slot)
                .unwrap_or(false)
        });
        let Some(index) = index_opt else {
            return Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!(
                    "Stored texture binding '{}' but target material slot was not active in runtime.",
                    binding.texture_param
                ),
            }));
        };
        let Some(material_instance) = assets.material_instances_mut().get_mut(index) else {
            return Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!(
                    "Stored texture binding '{}' but material instance is unavailable.",
                    binding.texture_param
                ),
            }));
        };
        let applied = render_ref.bind_material_texture_from_ktx(
            material_instance,
            &binding.texture_param,
            &runtime_path,
            binding.wrap_repeat_u,
            binding.wrap_repeat_v,
        );
        if applied {
            Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Info,
                message: format!(
                    "Applied texture binding '{}' for object {} slot {}.",
                    binding.texture_param, object_id, material_slot
                ),
            }))
        } else {
            Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!(
                    "Stored texture binding '{}' but runtime apply failed for '{}'.",
                    binding.texture_param, runtime_path
                ),
            }))
        }
    }

    fn command_transform_node(
        &mut self,
        index: usize,
        position: [f32; 3],
        rotation_deg: [f32; 3],
        scale: [f32; 3],
    ) -> Result<CommandOutcome, CommandError> {
        let object = self
            .scene
            .object_mut(index)
            .ok_or(CommandError::SceneObjectNotFound { index })?;
        match &mut object.kind {
            SceneObjectKind::Asset(data) => {
                data.position = position;
                data.rotation_deg = rotation_deg;
                data.scale = scale;
            }
            _ => return Err(CommandError::SceneObjectNotTransformable { index }),
        }

        let Some(entity) = self
            .scene_runtime
            .get(index)
            .and_then(|runtime| runtime.root_entity)
        else {
            return Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!(
                    "Transform updated for object {} but runtime entity is unavailable.",
                    index
                ),
            }));
        };

        let Some(render) = &mut self.render else {
            return Err(CommandError::RenderNotInitialized);
        };
        let matrix = compose_transform_matrix(position, rotation_deg, scale);
        if !render.set_entity_transform(entity, matrix) {
            return Err(CommandError::RenderTransformManagerUnavailable);
        }
        Ok(CommandOutcome::None)
    }

    fn command_delete_object(&mut self, index: usize) -> Result<CommandOutcome, CommandError> {
        let Some(removed) = self.scene.remove_object(index) else {
            return Err(CommandError::SceneObjectNotFound { index });
        };
        self.selection_id = None;
        match self.rebuild_runtime_scene() {
            Ok(()) => Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Info,
                message: format!("Deleted object '{}'.", removed.name),
            })),
            Err(err) => Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!("Deleted object '{}' with warnings:\n{}", removed.name, err),
            })),
        }
    }

    fn command_save_scene(&mut self, path: &std::path::Path) -> Result<CommandOutcome, CommandError> {
        crate::scene::serialization::save_scene_to_file(&self.scene, path)?;
        Ok(CommandOutcome::Notice(CommandNotice {
            severity: CommandSeverity::Info,
            message: format!("Scene saved: {}", path.display()),
        }))
    }

    fn command_load_scene(&mut self, path: &std::path::Path) -> Result<CommandOutcome, CommandError> {
        let loaded_scene = crate::scene::serialization::load_scene_from_file(path)?;
        self.scene = loaded_scene;
        match self.rebuild_runtime_scene() {
            Ok(()) => Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Info,
                message: format!("Scene loaded: {}", path.display()),
            })),
            Err(err) => Ok(CommandOutcome::Notice(CommandNotice {
                severity: CommandSeverity::Warning,
                message: format!("Scene loaded with warnings:\n{}", err),
            })),
        }
    }

    fn handle_create_gltf_action(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("glTF", &["gltf", "glb"])
            .pick_file()
        else {
            return;
        };
        let Some(path_str) = path.to_str().map(|value| value.to_string()) else {
            return;
        };
        let result = self.execute_scene_command(SceneCommand::AddAsset {
            path: path_str.clone(),
        });
        self.apply_command_feedback(&format!("Failed to load glTF {}", path_str), result);
    }

    fn handle_create_light_action(&mut self) {
        let result = self.execute_scene_command(SceneCommand::AddDirectionalLight {
            name: "Directional Light".to_string(),
            data: DirectionalLightData {
                color: [1.0, 1.0, 1.0],
                intensity: 100_000.0,
                direction: [0.0, -1.0, -0.5],
            },
        });
        self.apply_command_feedback("Failed to create directional light", result);
    }

    fn handle_save_scene_action(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene", &["json"])
            .set_file_name("scene.json")
            .save_file()
        {
            let result = self.execute_scene_command(SceneCommand::SaveScene { path: path.clone() });
            self.apply_command_feedback("Failed to save scene", result);
        }
    }

    fn handle_load_scene_action(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Scene", &["json"])
            .pick_file()
        else {
            return;
        };

        let result = self.execute_scene_command(SceneCommand::LoadScene { path: path.clone() });
        self.apply_command_feedback("Failed to load scene", result);
    }

    fn rebuild_runtime_scene(&mut self) -> Result<(), String> {
        let Some(render) = &mut self.render else {
            return Ok(());
        };
        self.scene.ensure_object_ids();

        // Conservative teardown order for native resource safety:
        // 1) drain GPU work
        // 2) detach/replace scene references
        // 3) drop asset-owned resources
        // 4) drain again before rebuilding
        log::info!("Runtime rebuild: flush before teardown");
        render.flush_and_wait();
        log::info!("Runtime rebuild: clear scene references");
        render.clear_scene();
        render.flush_and_wait();
        log::info!("Runtime rebuild: rotate asset generations");
        self.assets.prepare_for_scene_rebuild();
        render.flush_and_wait();
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
            let Some(mut entity_manager) = engine.entity_manager() else {
                return Err("Entity manager unavailable during runtime rebuild.".to_string());
            };
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
                            .load_gltf_from_path(
                                engine,
                                scene,
                                &mut entity_manager,
                                &data.path,
                                object.id,
                            )
                        {
                            Ok(loaded) => {
                                for entity in &loaded.renderable_entities {
                                    engine.renderable_set_layer_mask(*entity, 0xFF, 0x01);
                                }
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
        apply_scene_material_overrides_to_runtime(&self.scene, &mut self.assets);
        apply_scene_texture_bindings_to_runtime(&self.scene, &mut self.assets, render, &mut errors);

        for (entity, matrix) in transforms_to_apply {
            if !render.set_entity_transform(entity, matrix) {
                errors.push(format!(
                    "Transform manager unavailable while applying transform to entity {}.",
                    entity.id
                ));
            }
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
            self.orbit_pivot = center;
            self.camera = CameraController::from_bounds(center, extent);
            self.camera.apply(render.camera_mut());
        }

        match format_rebuild_errors(&errors) {
            Some(message) => Err(message),
            None => Ok(()),
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

fn format_rebuild_errors(errors: &[String]) -> Option<String> {
    if errors.is_empty() {
        None
    } else {
        Some(errors.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::{format_rebuild_errors, App, CommandError, SceneCommand};
    use crate::scene::{MaterialTextureBindingData, MediaSourceKind, TextureColorSpace};

    #[test]
    fn rebuild_error_format_empty_is_none() {
        assert!(format_rebuild_errors(&[]).is_none());
    }

    #[test]
    fn rebuild_error_format_preserves_order() {
        let errors = vec![
            "Asset 'a.glb' failed".to_string(),
            "Environment failed".to_string(),
        ];
        let formatted = format_rebuild_errors(&errors).unwrap();
        assert_eq!(formatted, "Asset 'a.glb' failed\nEnvironment failed");
    }

    #[test]
    fn set_material_texture_binding_requires_source_path() {
        let mut app = App::new();
        let result = app.execute_scene_command(SceneCommand::SetMaterialTextureBinding {
            object_id: 1,
            material_slot: 0,
            binding: MaterialTextureBindingData {
                texture_param: "baseColorMap".to_string(),
                source_kind: MediaSourceKind::Image,
                source_path: "  ".to_string(),
                runtime_ktx_path: None,
                source_hash: None,
                wrap_repeat_u: true,
                wrap_repeat_v: true,
                color_space: TextureColorSpace::Srgb,
                uv_offset: [0.0, 0.0],
                uv_scale: [1.0, 1.0],
                uv_rotation_deg: 0.0,
            },
        });
        assert!(matches!(result, Err(CommandError::TextureBindingSourceEmpty)));
    }

    #[test]
    fn set_material_texture_binding_persists_in_scene_state() {
        let mut app = App::new();
        let result = app.execute_scene_command(SceneCommand::SetMaterialTextureBinding {
            object_id: 5,
            material_slot: 2,
            binding: MaterialTextureBindingData {
                texture_param: "normalMap".to_string(),
                source_kind: MediaSourceKind::Image,
                source_path: "assets/textures/normal.png".to_string(),
                runtime_ktx_path: None,
                source_hash: None,
                wrap_repeat_u: true,
                wrap_repeat_v: true,
                color_space: TextureColorSpace::Srgb,
                uv_offset: [0.0, 0.0],
                uv_scale: [1.0, 1.0],
                uv_rotation_deg: 0.0,
            },
        });
        assert!(result.is_ok());
        assert_eq!(app.scene.texture_bindings().len(), 1);
        let binding = &app.scene.texture_bindings()[0];
        assert_eq!(binding.object_id, 5);
        assert_eq!(binding.material_slot, 2);
        assert_eq!(binding.binding.texture_param, "normalMap");
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
    let hdr = resolve_path_for_read(hdr_path.trim())?;
    if !hdr.exists() {
        return Err(format!("HDR file not found: {}", hdr.display()));
    }
    let hdr_hash = hash_file_bytes(&hdr)?;
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let output_root = manifest_dir.join("assets").join("cache").join("environments");
    let output_prefix = output_root.join(&hdr_hash);
    let output_dir = output_root.join(&hdr_hash);
    let ibl_path = output_dir.join(format!("{hdr_hash}_ibl.ktx"));
    let skybox_path = output_dir.join(format!("{hdr_hash}_skybox.ktx"));
    std::fs::create_dir_all(&output_root)
        .map_err(|err| format!("Failed creating environment folder: {}", err))?;
    if ibl_path.exists() && skybox_path.exists() {
        return Ok((
            display_path_for_scene(&ibl_path),
            display_path_for_scene(&skybox_path),
        ));
    }

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
    Ok((
        display_path_for_scene(&ibl_path),
        display_path_for_scene(&skybox_path),
    ))
}

fn prepare_texture_binding_data(
    texture_param: &str,
    source_path: &str,
    wrap_repeat_u: bool,
    wrap_repeat_v: bool,
    color_space: TextureColorSpace,
    uv_offset: [f32; 2],
    uv_scale: [f32; 2],
    uv_rotation_deg: f32,
) -> Result<MaterialTextureBindingData, String> {
    let texture_param = texture_param.trim();
    if texture_param.is_empty() {
        return Err("Texture parameter is empty.".to_string());
    }
    let source_path = source_path.trim();
    if source_path.is_empty() {
        return Err("Texture source path is empty.".to_string());
    }

    let source_kind = MediaSourceKind::Image;
    let (runtime_ktx_path, source_hash) = resolve_runtime_texture_cache(source_path, color_space)?;

    Ok(MaterialTextureBindingData {
        texture_param: texture_param.to_string(),
        source_kind,
        source_path: source_path.to_string(),
        runtime_ktx_path: Some(runtime_ktx_path),
        source_hash: Some(source_hash),
        wrap_repeat_u,
        wrap_repeat_v,
        color_space,
        uv_offset,
        uv_scale,
        uv_rotation_deg,
    })
}

fn sync_material_binding_rows_from_scene(
    ui: &mut UiState,
    scene: &SceneState,
    assets: &AssetManager,
    selected_material_global_index: i32,
) {
    let rows = ui.material_binding_rows_mut();
    for row in rows.iter_mut() {
        row.source.fill(0);
        row.wrap_repeat_u = true;
        row.wrap_repeat_v = true;
        row.srgb = true;
        row.uv_offset = [0.0, 0.0];
        row.uv_scale = [1.0, 1.0];
        row.uv_rotation_deg = 0.0;
    }
    if selected_material_global_index < 0 {
        return;
    }
    let Some(binding_key) = assets.material_binding(selected_material_global_index as usize) else {
        return;
    };
    for (row_index, texture_param) in MATERIAL_TEXTURE_PARAMS.iter().enumerate() {
        let Some(entry) = scene.texture_bindings().iter().find(|entry| {
            entry.object_id == binding_key.object_id
                && entry.material_slot == binding_key.material_slot
                && entry.binding.texture_param == *texture_param
        }) else {
            continue;
        };
        write_string_to_buffer(&entry.binding.source_path, &mut rows[row_index].source);
        rows[row_index].wrap_repeat_u = entry.binding.wrap_repeat_u;
        rows[row_index].wrap_repeat_v = entry.binding.wrap_repeat_v;
        rows[row_index].srgb = entry.binding.color_space == TextureColorSpace::Srgb;
        rows[row_index].uv_offset = entry.binding.uv_offset;
        rows[row_index].uv_scale = entry.binding.uv_scale;
        rows[row_index].uv_rotation_deg = entry.binding.uv_rotation_deg;
    }
}

fn resolve_runtime_texture_cache(
    source_path: &str,
    color_space: TextureColorSpace,
) -> Result<(String, String), String> {
    let source = resolve_path_for_read(source_path)?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if extension == "ktx" {
        let hash = hash_file_bytes(&source)?;
        return Ok((display_path_for_scene(&source), hash));
    }
    if extension != "png" && extension != "jpg" && extension != "jpeg" {
        return Err(format!(
            "Unsupported texture source extension '{}'. Use .ktx/.png/.jpg/.jpeg.",
            extension
        ));
    }

    let base_hash = hash_file_bytes(&source)?;
    let color_tag = match color_space {
        TextureColorSpace::Srgb => "srgb",
        TextureColorSpace::Linear => "linear",
    };
    let source_hash = format!("{base_hash}_{color_tag}");
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let cache_dir = manifest_dir.join("assets").join("cache").join("textures");
    std::fs::create_dir_all(&cache_dir)
        .map_err(|err| format!("Failed to create texture cache folder: {}", err))?;
    let ktx_path = cache_dir.join(format!("{source_hash}.ktx"));
    if !ktx_path.exists() {
        let normalized_png_path = ensure_normalized_png_for_mipgen(&source, &cache_dir, &source_hash)?;
        let mipgen_path = PathBuf::from(env!("FILAMENT_BIN_DIR")).join("mipgen.exe");
        if !mipgen_path.exists() {
            return Err(format!("mipgen not found at {}", mipgen_path.display()));
        }
        let status = Command::new(&mipgen_path)
            .args(["-q", "-f", "ktx"])
            .args(match color_space {
                TextureColorSpace::Srgb => Vec::<&str>::new(),
                TextureColorSpace::Linear => vec!["--linear"],
            })
            .arg(&normalized_png_path)
            .arg(&ktx_path)
            .status()
            .map_err(|err| format!("Failed to run mipgen: {}", err))?;
        if !status.success() {
            return Err(format!("mipgen failed with status {:?}", status.code()));
        }
    }
    Ok((display_path_for_scene(&ktx_path), source_hash))
}

fn ensure_normalized_png_for_mipgen(
    source: &std::path::Path,
    cache_dir: &std::path::Path,
    source_hash: &str,
) -> Result<PathBuf, String> {
    let normalized_png_path = cache_dir.join(format!("{source_hash}.normalized.png"));
    if normalized_png_path.exists() {
        return Ok(normalized_png_path);
    }
    let image = image::ImageReader::open(source)
        .map_err(|err| format!("Failed to open source image '{}': {}", source.display(), err))?
        .decode()
        .map_err(|err| format!("Failed to decode source image '{}': {}", source.display(), err))?;
    image
        .to_rgba8()
        .save_with_format(&normalized_png_path, image::ImageFormat::Png)
        .map_err(|err| {
            format!(
                "Failed to write normalized PNG '{}': {}",
                normalized_png_path.display(),
                err
            )
        })?;
    Ok(normalized_png_path)
}

fn resolve_path_for_read(path: &str) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path);
    if raw.exists() {
        return Ok(raw);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let joined = manifest_dir.join(path);
    if joined.exists() {
        return Ok(joined);
    }
    Err(format!("File not found: {}", path))
}

fn hash_file_bytes(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path)
        .map_err(|err| format!("Failed reading '{}': {}", path.display(), err))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{:02x}", byte);
    }
    Ok(hex)
}

fn display_path_for_scene(path: &std::path::Path) -> String {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Ok(relative) = path.strip_prefix(&manifest_dir) {
        return relative.to_string_lossy().to_string();
    }
    path.to_string_lossy().to_string()
}

fn closest_line_line_param(
    line_a_origin: [f32; 3],
    line_a_dir: [f32; 3],
    line_b_origin: [f32; 3],
    line_b_dir: [f32; 3],
) -> Option<f32> {
    let w0 = [
        line_a_origin[0] - line_b_origin[0],
        line_a_origin[1] - line_b_origin[1],
        line_a_origin[2] - line_b_origin[2],
    ];
    let a = dot3(line_a_dir, line_a_dir);
    let b = dot3(line_a_dir, line_b_dir);
    let c = dot3(line_b_dir, line_b_dir);
    let d = dot3(line_a_dir, w0);
    let e = dot3(line_b_dir, w0);
    let denom = a * c - b * b;
    if denom.abs() <= 1e-6 {
        return Some(e / c.max(1e-6));
    }
    Some((a * e - b * d) / denom)
}

fn dot3(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn ray_plane_intersection(
    ray_origin: [f32; 3],
    ray_dir: [f32; 3],
    plane_origin: [f32; 3],
    plane_normal: [f32; 3],
) -> Option<[f32; 3]> {
    let denom = dot3(ray_dir, plane_normal);
    if denom.abs() <= 1e-6 {
        return None;
    }
    let rel = [
        plane_origin[0] - ray_origin[0],
        plane_origin[1] - ray_origin[1],
        plane_origin[2] - ray_origin[2],
    ];
    let t = dot3(rel, plane_normal) / denom;
    if t < 0.0 || !t.is_finite() {
        return None;
    }
    Some([
        ray_origin[0] + ray_dir[0] * t,
        ray_origin[1] + ray_dir[1] * t,
        ray_origin[2] + ray_dir[2] * t,
    ])
}

fn euler_deg_to_mat3(rotation_deg: [f32; 3]) -> Mat3 {
    let rx = rotation_deg[0].to_radians();
    let ry = rotation_deg[1].to_radians();
    let rz = rotation_deg[2].to_radians();
    Mat3::from_euler(EulerRot::ZYX, rz, ry, rx)
}

fn mat3_to_euler_deg(mat: Mat3) -> [f32; 3] {
    let (rz, ry, rx) = mat.to_euler(EulerRot::ZYX);
    [
        normalize_angle_deg(rx.to_degrees()),
        normalize_angle_deg(ry.to_degrees()),
        normalize_angle_deg(rz.to_degrees()),
    ]
}

fn normalize_angle_deg(mut a: f32) -> f32 {
    while a > 180.0 {
        a -= 360.0;
    }
    while a < -180.0 {
        a += 360.0;
    }
    a
}

#[allow(dead_code)]
fn map_arcball_vector(
    mouse: (f32, f32),
    center_screen: [f32; 2],
    radius_px: f32,
) -> [f32; 3] {
    let r = radius_px.max(1.0);
    let x = (mouse.0 - center_screen[0]) / r;
    let y = (center_screen[1] - mouse.1) / r;
    let d2 = x * x + y * y;
    let z = if d2 <= 0.5 {
        (1.0 - d2).sqrt()
    } else {
        0.5 / d2.sqrt()
    };
    let mut v = Vec3::new(x, y, z);
    if v.length_squared() <= 1e-10 {
        return [0.0, 0.0, 1.0];
    }
    v = v.normalize();
    v.to_array()
}

fn apply_material_override(
    material_instance: &mut crate::filament::MaterialInstance,
    data: &MaterialOverrideData,
) {
    if material_instance.has_parameter("baseColorFactor") {
        material_instance.set_float4("baseColorFactor", data.base_color_rgba);
    }
    if material_instance.has_parameter("metallicFactor") {
        material_instance.set_float("metallicFactor", data.metallic);
    }
    if material_instance.has_parameter("roughnessFactor") {
        material_instance.set_float("roughnessFactor", data.roughness);
    }
    if material_instance.has_parameter("emissiveFactor") {
        material_instance.set_float3("emissiveFactor", data.emissive_rgb);
    }
}

fn apply_scene_material_overrides_to_runtime(scene: &SceneState, assets: &mut AssetManager) {
    if scene.material_overrides().is_empty() {
        return;
    }
    let binding_count = assets.material_instances().len();
    for override_entry in scene.material_overrides() {
        let target_object_id = override_entry.object_id;
        let target_asset_path = override_entry.asset_path.as_deref();
        let target_slot = override_entry.material_slot;
        for index in 0..binding_count {
            let Some(binding) = assets.material_binding(index) else {
                continue;
            };
            let matches_object_slot = target_object_id == Some(binding.object_id)
                && target_slot == Some(binding.material_slot);
            let matches_path_slot = target_object_id.is_none()
                && target_asset_path == Some(binding.asset_path.as_str())
                && target_slot == Some(binding.material_slot);
            let matches_legacy_name = target_asset_path.is_none()
                && target_object_id.is_none()
                && target_slot.is_none()
                && !override_entry.material_name.is_empty()
                && override_entry.material_name == binding.material_name;
            if !(matches_object_slot || matches_path_slot || matches_legacy_name) {
                continue;
            }
            if let Some(material_instance) = assets.material_instances_mut().get_mut(index) {
                apply_material_override(material_instance, &override_entry.data);
            }
        }
    }
}

fn apply_scene_texture_bindings_to_runtime(
    scene: &SceneState,
    assets: &mut AssetManager,
    render: &mut RenderContext,
    errors: &mut Vec<String>,
) {
    if scene.texture_bindings().is_empty() {
        return;
    }
    for entry in scene.texture_bindings() {
        let Some(runtime_path) = texture_binding_runtime_path(&entry.binding) else {
            errors.push(format!(
                "Texture binding '{}' for object {} slot {} has no runtime .ktx path.",
                entry.binding.texture_param,
                entry.object_id,
                entry.material_slot,
            ));
            continue;
        };
        let index_opt = (0..assets.material_instances().len()).find(|index| {
            assets
                .material_binding(*index)
                .map(|binding| {
                    binding.object_id == entry.object_id
                        && binding.material_slot == entry.material_slot
                })
                .unwrap_or(false)
        });
        let Some(index) = index_opt else {
            errors.push(format!(
                "Texture binding '{}' target object {} slot {} is unavailable in runtime.",
                entry.binding.texture_param, entry.object_id, entry.material_slot
            ));
            continue;
        };
        let Some(material_instance) = assets.material_instances_mut().get_mut(index) else {
            errors.push(format!(
                "Texture binding '{}' target material instance index {} unavailable.",
                entry.binding.texture_param, index
            ));
            continue;
        };
        let applied = render.bind_material_texture_from_ktx(
            material_instance,
            &entry.binding.texture_param,
            &runtime_path,
            entry.binding.wrap_repeat_u,
            entry.binding.wrap_repeat_v,
        );
        if !applied {
            errors.push(format!(
                "Texture binding '{}' failed to apply from '{}'.",
                entry.binding.texture_param, runtime_path
            ));
        }
    }
}

fn texture_binding_runtime_path(binding: &MaterialTextureBindingData) -> Option<String> {
    if let Some(path) = &binding.runtime_ktx_path {
        return Some(path.clone());
    }
    if binding.source_path.trim().to_ascii_lowercase().ends_with(".ktx") {
        return Some(binding.source_path.clone());
    }
    None
}

fn material_params_to_override(params: MaterialParams) -> MaterialOverrideData {
    MaterialOverrideData {
        base_color_rgba: params.base_color_rgba,
        metallic: params.metallic,
        roughness: params.roughness,
        emissive_rgb: params.emissive_rgb,
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

        let window = match event_loop.create_window(window_attrs) {
            Ok(window) => Arc::new(window),
            Err(err) => {
                let message = format!("Failed to create window: {err}");
                log::error!("{message}");
                let _ = rfd::MessageDialog::new()
                    .set_title("Previz Startup Error")
                    .set_description(&message)
                    .show();
                self.close_requested = true;
                event_loop.exit();
                return;
            }
        };

        if let Err(err) = self.init_filament(&window) {
            let message = format!("Failed to initialize renderer: {err}");
            log::error!("{message}");
            let _ = rfd::MessageDialog::new()
                .set_title("Previz Startup Error")
                .set_description(&message)
                .show();
            self.close_requested = true;
            event_loop.exit();
            return;
        }
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
                    if pressed && event.physical_key == PhysicalKey::Code(KeyCode::KeyF) {
                        if !self.focus_selected() {
                            self.ui.set_environment_status(
                                "Focus selected unavailable: select an asset first.".to_string(),
                            );
                        }
                        return;
                    }
                    if pressed && event.physical_key == PhysicalKey::Code(KeyCode::Delete) {
                        self.delete_selection_requested = true;
                        return;
                    }
                    if pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::KeyQ) => {
                                self.transform_tool_mode = TransformToolMode::Select;
                                return;
                            }
                            PhysicalKey::Code(KeyCode::KeyW) => {
                                self.transform_tool_mode = TransformToolMode::Translate;
                                return;
                            }
                            PhysicalKey::Code(KeyCode::KeyE) => {
                                self.transform_tool_mode = TransformToolMode::Rotate;
                                return;
                            }
                            PhysicalKey::Code(KeyCode::KeyR) => {
                                self.transform_tool_mode = TransformToolMode::Scale;
                                return;
                            }
                            _ => {}
                        }
                    }
                    self.input.handle_key(event.physical_key, pressed);
                    if pressed {
                        match event.physical_key {
                            PhysicalKey::Code(KeyCode::Equal) => self.nudge_camera(0.0, 0.0, -0.3),
                            PhysicalKey::Code(KeyCode::Minus) => self.nudge_camera(0.0, 0.0, 0.3),
                            _ => {}
                        }
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
                let new_pos = (position.x as f32, position.y as f32);
                let prev_pos = self.mouse_pos;
                self.mouse_pos = Some(new_pos);
                let over_sidebar_ui = self.mouse_over_sidebar_ui();
                if let Some(render) = &mut self.render {
                    render.ui_mouse_pos(new_pos.0, new_pos.1);
                    let _ = render.ui_want_capture_mouse();
                }
                if !over_sidebar_ui {
                    if self.mouse_buttons[0] && self.gizmo_active_axis != 0 {
                        self.begin_gizmo_drag_if_needed(new_pos);
                        self.apply_transform_tool_drag(new_pos);
                    } else if let (Some((px, py)), Some(mode)) = (prev_pos, self.camera_drag_mode)
                    {
                        let dx = new_pos.0 - px;
                        let dy = new_pos.1 - py;
                        match mode {
                            CameraDragMode::Orbit => self.orbit_camera(dx, dy),
                            CameraDragMode::Pan => self.pan_camera(dx, dy),
                            CameraDragMode::Dolly => self.dolly_camera(-dy * 0.02),
                        }
                    }
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
                self.camera_drag_mode = None;
                self.gizmo_drag_state = None;
                self.gizmo_hover_axis = GIZMO_NONE;
                self.pending_click_select = false;
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
                    let over_sidebar_ui = self.mouse_over_sidebar_ui();
                    if let Some(render) = &mut self.render {
                        render.ui_mouse_button(button_index, pressed);
                        let _ = render.ui_want_capture_mouse();
                    }
                    if !over_sidebar_ui {
                        match self.camera_control_profile {
                            CameraControlProfile::Blender => match (button, pressed) {
                                (MouseButton::Left, true) => {
                                    if self.transform_tool_mode == TransformToolMode::Select {
                                        self.pending_click_select = true;
                                    } else {
                                        self.pending_click_select = false;
                                        if let (Some((mx, my)), Some(render)) =
                                            (self.mouse_pos, &mut self.render)
                                        {
                                            render.request_pick(mx, my);
                                        }
                                    }
                                }
                                (MouseButton::Left, false) => {
                                    if self.pending_click_select && self.gizmo_active_axis == 0 {
                                        if let (Some((mx, my)), Some(render)) = (self.mouse_pos, &mut self.render) {
                                            render.request_pick(mx, my);
                                        }
                                    }
                                    self.pending_click_select = false;
                                    self.gizmo_drag_state = None;
                                    self.gizmo_active_axis = GIZMO_NONE;
                                }
                                (MouseButton::Middle, true) => {
                                    self.pending_click_select = false;
                                    let state = self.modifiers.state();
                                    if state.control_key() {
                                        self.camera_drag_mode = Some(CameraDragMode::Dolly);
                                    } else if state.shift_key() {
                                        self.camera_drag_mode = Some(CameraDragMode::Pan);
                                    } else {
                                        self.camera_drag_mode = Some(CameraDragMode::Orbit);
                                    }
                                }
                                (MouseButton::Middle, false) => {
                                    self.camera_drag_mode = None;
                                }
                                _ => {}
                            },
                            CameraControlProfile::FpsLike => {
                                // Reserved for future alternate camera controls.
                            }
                        }
                    }
                    if !pressed && button == MouseButton::Left {
                        self.pending_click_select = false;
                        self.gizmo_drag_state = None;
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let (wheel_x, wheel_y) = match delta {
                    MouseScrollDelta::LineDelta(x, y) => (x, y),
                    MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                };
                let mut ui_capture_mouse = false;
                if let Some(render) = &mut self.render {
                    render.ui_mouse_wheel(wheel_x, wheel_y);
                    ui_capture_mouse = render.ui_want_capture_mouse();
                }
                if !ui_capture_mouse && !self.mouse_over_sidebar_ui() {
                    self.dolly_camera(wheel_y * 0.15);
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

    let event_loop = match EventLoop::new() {
        Ok(loop_) => loop_,
        Err(err) => {
            let message = format!("Failed to create event loop: {err}");
            log::error!("{message}");
            let _ = rfd::MessageDialog::new()
                .set_title("Previz Startup Error")
                .set_description(&message)
                .show();
            return;
        }
    };
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    if let Err(err) = event_loop.run_app(&mut app) {
        let message = format!("Event loop error: {err}");
        log::error!("{message}");
        let _ = rfd::MessageDialog::new()
            .set_title("Previz Runtime Error")
            .set_description(&message)
            .show();
    }

    log::info!("👋 Goodbye!");
}

fn sanitize_cstring(value: &str) -> CString {
    let cleaned = value.replace('\0', " ");
    CString::new(cleaned).unwrap_or_default()
}

fn scoped_material_indices_for_selection(
    scene: &SceneState,
    assets: &AssetManager,
    selection: Option<usize>,
) -> Vec<usize> {
    let Some(selected_index) = selection else {
        return Vec::new();
    };
    let Some(selected_object) = scene.objects().get(selected_index) else {
        return Vec::new();
    };
    if !matches!(selected_object.kind, SceneObjectKind::Asset(_)) {
        return Vec::new();
    }
    let object_id = selected_object.id;
    (0..assets.material_instances().len())
        .filter(|index| {
            assets
                .material_binding(*index)
                .map(|binding| binding.object_id == object_id)
                .unwrap_or(false)
        })
        .collect()
}

fn global_material_index_to_ui_index(scoped_indices: &[usize], global_index: i32) -> i32 {
    if global_index < 0 {
        return -1;
    }
    let Some(global_usize) = usize::try_from(global_index).ok() else {
        return -1;
    };
    scoped_indices
        .iter()
        .position(|index| *index == global_usize)
        .and_then(|idx| i32::try_from(idx).ok())
        .unwrap_or(-1)
}

fn ui_material_index_to_global_index(scoped_indices: &[usize], ui_index: i32) -> i32 {
    if ui_index < 0 {
        return -1;
    }
    let Some(ui_usize) = usize::try_from(ui_index).ok() else {
        return -1;
    };
    scoped_indices
        .get(ui_usize)
        .and_then(|value| i32::try_from(*value).ok())
        .unwrap_or(-1)
}
