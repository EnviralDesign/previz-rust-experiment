mod input;
mod timing;

use crate::assets::{AssetManager, DEFAULT_GLTF_PATH};
use crate::render::{CameraController, CameraMovement, RenderContext};
use crate::scene::SceneState;
use crate::ui::UiState;
use input::{InputAction, InputState};
use timing::FrameTiming;

use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

pub struct App {
    window: Option<Arc<Window>>,
    assets: AssetManager,
    scene: SceneState,
    ui: UiState,
    input: InputState,
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
            ui: UiState::new(),
            input: InputState::default(),
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
        let (engine, scene) = render.engine_scene_mut();
        let mut entity_manager = engine.entity_manager();
        let loaded =
            self.assets
                .load_gltf_from_path(engine, scene, &mut entity_manager, DEFAULT_GLTF_PATH);
        self.scene.add_asset(&loaded);

        self.camera = CameraController::from_bounds(loaded.center, loaded.extent);
        render.set_projection_for_window(window);
        self.camera.apply(render.camera_mut());
        render.init_ui(window);

        self.render = Some(render);
    }

    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        if let Some(render) = &mut self.render {
            render.resize(new_size);
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
        self.ui.update(&self.scene, &self.assets);
        let ui_text = self.ui.summary();
        if let Some(render) = &mut self.render {
            let render_ms = render.render(ui_text, self.timing.frame_dt);
            self.timing.set_render_ms(render_ms);
        }
        self.timing
            .update(self.window.as_ref().map(|w| w.as_ref()), frame_start);
        self.update_camera();
    }
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
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    self.close_requested = true;
                    event_loop.exit();
                    return;
                }
                let pressed = event.state == winit::event::ElementState::Pressed;
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
            WindowEvent::Resized(new_size) => {
                self.handle_resize(new_size);
                if let Some(window) = self.window.clone() {
                    self.update_target_frame_duration(&window);
                }
            }
            WindowEvent::Moved(_) => {
                if let Some(window) = self.window.clone() {
                    self.update_target_frame_duration(&window);
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
