//! Previz - Minimal Filament Renderer Proof of Concept
//!
//! This is a super minimal "hello world" that demonstrates:
//! - Creating a native Windows window with winit 0.30
//! - Initializing the Filament engine (v1.69.0)
//! - Rendering a simple colored triangle
//!
//! The triangle uses vertex colors (baked color material) and renders
//! directly to the window's swap chain.

mod ffi;
mod filament;

use filament::{Backend, Engine, Entity};
use std::ffi::c_void;
use std::path::PathBuf;
use std::time::Instant;
use std::time::Duration;
use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const GLTF_PATH: &str = "assets/gltf/DamagedHelmet.gltf";

/// Get the native window handle (HWND) on Windows
#[cfg(target_os = "windows")]
fn get_native_window_handle(window: &Window) -> *mut c_void {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut c_void,
        _ => panic!("Expected Win32 window handle"),
    }
}

/// Application state
struct App {
    window: Option<Arc<Window>>,
    swap_chain: Option<filament::SwapChain>,
    renderer: Option<filament::Renderer>,
    view: Option<filament::View>,
    scene: Option<filament::Scene>,
    camera: Option<filament::Camera>,
    gltf_asset: Option<filament::GltfAsset>,
    gltf_asset_loader: Option<filament::GltfAssetLoader>,
    gltf_resource_loader: Option<filament::GltfResourceLoader>,
    gltf_texture_provider: Option<filament::GltfTextureProvider>,
    gltf_material_provider: Option<filament::GltfMaterialProvider>,
    directional_light: Option<Entity>,
    camera_position: [f32; 3],
    camera_yaw: f32,
    camera_pitch: f32,
    move_forward: bool,
    move_backward: bool,
    move_left: bool,
    move_right: bool,
    move_up: bool,
    move_down: bool,
    aim_left: bool,
    aim_right: bool,
    aim_up: bool,
    aim_down: bool,
    last_frame_time: Option<Instant>,
    last_fps_time: Instant,
    frame_count: u32,
    frame_dt: f32,
    render_ms: f32,
    base_title: String,
    target_frame_duration: Duration,
    next_frame_time: Instant,
    engine: Option<Engine>,
    close_requested: bool,
}

impl App {
    fn new() -> Self {
        Self {
            window: None,
            swap_chain: None,
            renderer: None,
            view: None,
            scene: None,
            camera: None,
            gltf_asset: None,
            gltf_asset_loader: None,
            gltf_resource_loader: None,
            gltf_texture_provider: None,
            gltf_material_provider: None,
            directional_light: None,
            camera_position: [0.0, 0.0, 3.0],
            camera_yaw: 0.6,
            camera_pitch: 0.3,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
            aim_left: false,
            aim_right: false,
            aim_up: false,
            aim_down: false,
            last_frame_time: None,
            last_fps_time: Instant::now(),
            frame_count: 0,
            frame_dt: 1.0 / 60.0,
            render_ms: 0.0,
            base_title: "Previz - Filament v1.69.0 glTF".to_string(),
            target_frame_duration: Duration::from_millis(16),
            next_frame_time: Instant::now(),
            engine: None,
            close_requested: false,
        }
    }

    /// Initialize Filament after window is created
    fn init_filament(&mut self, window: &Window) {
        let native_handle = get_native_window_handle(window);
        let window_size = window.inner_size();

        log::info!(
            "Initializing Filament for window {}x{}",
            window_size.width,
            window_size.height
        );

        // Create Filament engine with OpenGL backend
        let mut engine = Engine::create(Backend::OpenGL).expect("Failed to create Filament engine");
        log::info!("Filament engine created (OpenGL backend)");

        // Create swap chain from native window handle
        log::info!("Creating swap chain with native handle: {:?}", native_handle);
        let swap_chain = engine
            .create_swap_chain(native_handle)
            .expect("Failed to create swap chain");
        log::info!("Swap chain created");

        // Create renderer
        log::info!("Creating renderer...");
        let mut renderer = engine.create_renderer().expect("Failed to create renderer");
        log::info!("Renderer created");

        // Set clear color (dark blue background)
        log::info!("Setting clear options...");
        renderer.set_clear_options(0.1, 0.1, 0.2, 1.0, true, true);
        log::info!("Clear options set");

        // Create scene
        log::info!("Creating scene...");
        let scene_result = engine.create_scene();
        log::info!("create_scene returned: {:?}", scene_result.is_some());
        let mut scene = scene_result.expect("Failed to create scene");
        log::info!("Scene created successfully");

        // Create view
        log::info!("Creating view...");
        let view_result = engine.create_view();
        log::info!("create_view returned: {:?}", view_result.is_some());
        let mut view = view_result.expect("Failed to create view");
        log::info!("View created successfully");

        // Create camera
        log::info!("Getting entity manager...");
        let mut entity_manager = engine.entity_manager();
        log::info!("Entity manager obtained");
        let camera_entity = entity_manager.create();
        log::info!("Camera entity created: {:?}", camera_entity);
        let mut camera = engine
            .create_camera(camera_entity)
            .expect("Failed to create camera");
        log::info!("Camera created successfully");

        // Configure viewport
        view.set_viewport(0, 0, window_size.width, window_size.height);
        view.set_scene(&mut scene);
        view.set_camera(&mut camera);

        // Directional light for PBR shading
        let light_entity = engine.create_directional_light(
            &mut entity_manager,
            [1.0, 1.0, 1.0],
            100_000.0,
            [0.0, -1.0, -0.5],
        );
        scene.add_entity(light_entity);

        // Load glTF asset
        let gltf_bytes = load_gltf_bytes();
        let mut material_provider =
            filament::GltfMaterialProvider::create_jit(&mut engine, false)
                .expect("Failed to create gltf material provider");
        let mut texture_provider =
            filament::GltfTextureProvider::create_stb(&mut engine)
                .expect("Failed to create stb texture provider");
        let mut asset_loader = filament::GltfAssetLoader::create(
            &mut engine,
            &mut material_provider,
            &mut entity_manager,
        )
        .expect("Failed to create gltf asset loader");
        let mut resource_loader =
            filament::GltfResourceLoader::create(&mut engine, None, true)
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
        asset.add_entities_to_scene(&mut scene);

        let aspect = window_size.width as f64 / window_size.height as f64;
        let (center, extent) = asset.bounding_box();
        let radius = extent[0].max(extent[1]).max(extent[2]);
        let distance = if radius > 0.0 { radius * 3.0 } else { 3.0 };
        self.camera_position = [
            center[0] + distance,
            center[1] + distance * 0.4,
            center[2] + distance,
        ];
        let forward = [
            center[0] - self.camera_position[0],
            center[1] - self.camera_position[1],
            center[2] - self.camera_position[2],
        ];
        (self.camera_yaw, self.camera_pitch) = forward_to_yaw_pitch(forward);
        camera.set_projection_perspective(45.0, aspect, 0.1, 1000.0);
        self.update_camera_look_at();
        log::info!("glTF asset loaded and added to scene");

        // Store everything
        self.engine = Some(engine);
        self.swap_chain = Some(swap_chain);
        self.renderer = Some(renderer);
        self.view = Some(view);
        self.scene = Some(scene);
        self.camera = Some(camera);
        self.gltf_asset = Some(asset);
        self.gltf_asset_loader = Some(asset_loader);
        self.gltf_resource_loader = Some(resource_loader);
        self.gltf_texture_provider = Some(texture_provider);
        self.gltf_material_provider = Some(material_provider);
        self.directional_light = Some(light_entity);

        log::info!("Filament initialization complete!");
    }

    /// Handle window resize
    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        if let (Some(view), Some(camera)) = (&mut self.view, &mut self.camera) {
            view.set_viewport(0, 0, new_size.width, new_size.height);

            let aspect = new_size.width as f64 / new_size.height as f64;
            camera.set_projection_perspective(45.0, aspect, 0.1, 1000.0);
        }
    }

    fn update_camera_look_at(&mut self) {
        let Some(camera) = &mut self.camera else {
            return;
        };

        let yaw = self.camera_yaw;
        let pitch = self.camera_pitch.clamp(-1.4, 1.4);
        self.camera_pitch = pitch;

        let cos_pitch = pitch.cos();
        let dir = [
            yaw.cos() * cos_pitch,
            pitch.sin(),
            yaw.sin() * cos_pitch,
        ];
        let eye = self.camera_position;
        let center = [
            eye[0] + dir[0],
            eye[1] + dir[1],
            eye[2] + dir[2],
        ];
        camera.look_at(eye, center, [0.0, 1.0, 0.0]);
    }

    fn nudge_camera(&mut self, yaw_delta: f32, pitch_delta: f32, zoom_delta: f32) {
        self.camera_yaw += yaw_delta;
        self.camera_pitch += pitch_delta;
        if zoom_delta != 0.0 {
            let (forward, _, _) = camera_basis(self.camera_yaw, self.camera_pitch);
            self.camera_position[0] += forward[0] * zoom_delta;
            self.camera_position[1] += forward[1] * zoom_delta;
            self.camera_position[2] += forward[2] * zoom_delta;
        }
        self.update_camera_look_at();
    }

    fn move_camera_horizontal(&mut self, right: f32, up: f32, forward: f32) {
        let yaw = self.camera_yaw;
        let forward_dir = [yaw.cos(), 0.0, yaw.sin()];
        let right_dir = [-yaw.sin(), 0.0, yaw.cos()];
        let up_dir = [0.0, 1.0, 0.0];

        self.camera_position[0] +=
            right_dir[0] * right + up_dir[0] * up + forward_dir[0] * forward;
        self.camera_position[1] +=
            right_dir[1] * right + up_dir[1] * up + forward_dir[1] * forward;
        self.camera_position[2] +=
            right_dir[2] * right + up_dir[2] * up + forward_dir[2] * forward;
        self.update_camera_look_at();
    }

    fn update_frame_timing(&mut self, now: Instant) {
        let dt_duration = if let Some(last) = self.last_frame_time {
            now.saturating_duration_since(last)
        } else {
            std::time::Duration::from_millis(16)
        };
        self.last_frame_time = Some(now);
        self.frame_dt = dt_duration.as_secs_f32().max(0.0);

        self.frame_count = self.frame_count.saturating_add(1);
        let elapsed = now.saturating_duration_since(self.last_fps_time);
        if elapsed.as_secs_f32() >= 0.5 {
            let fps = self.frame_count as f32 / elapsed.as_secs_f32();
            let ms = (self.frame_dt * 1000.0).max(0.0);
            if let Some(window) = &self.window {
                window.set_title(&format!(
                    "{} - {:.1} fps (cadence {:.2} ms, render {:.2} ms)",
                    self.base_title, fps, ms, self.render_ms
                ));
            }
            self.frame_count = 0;
            self.last_fps_time = now;
        }
    }

    fn update_camera_movement(&mut self) {
        let move_speed = 1.5 * self.frame_dt;
        let aim_speed = 1.8 * self.frame_dt;

        if self.aim_left {
            self.camera_yaw -= aim_speed;
        }
        if self.aim_right {
            self.camera_yaw += aim_speed;
        }
        if self.aim_up {
            self.camera_pitch += aim_speed;
        }
        if self.aim_down {
            self.camera_pitch -= aim_speed;
        }

        let mut forward = 0.0;
        let mut right = 0.0;
        let mut up = 0.0;
        if self.move_forward {
            forward += move_speed;
        }
        if self.move_backward {
            forward -= move_speed;
        }
        if self.move_left {
            right -= move_speed;
        }
        if self.move_right {
            right += move_speed;
        }
        if self.move_up {
            up += move_speed;
        }
        if self.move_down {
            up -= move_speed;
        }

        if forward != 0.0 || right != 0.0 || up != 0.0 {
            self.move_camera_horizontal(right, up, forward);
        } else if self.aim_left || self.aim_right || self.aim_up || self.aim_down {
            self.update_camera_look_at();
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

    /// Render a frame
    fn render(&mut self) {
        let frame_start = Instant::now();
        if let (Some(renderer), Some(swap_chain), Some(view)) =
            (&mut self.renderer, &mut self.swap_chain, &self.view)
        {
            if renderer.begin_frame(swap_chain) {
                renderer.render(view);
                renderer.end_frame();
            }
        }
        let render_end = Instant::now();
        self.render_ms = render_end
            .saturating_duration_since(frame_start)
            .as_secs_f32()
            * 1000.0;
        self.update_frame_timing(frame_start);
        self.update_camera_movement();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Window already created
        }

        log::info!("Creating window...");

        let window_attrs = WindowAttributes::default()
            .with_title(self.base_title.clone())
            .with_inner_size(PhysicalSize::new(1280u32, 720u32))
            .with_resizable(true);

        let window = Arc::new(
            event_loop
                .create_window(window_attrs)
                .expect("Failed to create window"),
        );

        log::info!(
            "Window created: {}x{}",
            window.inner_size().width,
            window.inner_size().height
        );

        // Initialize Filament
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
                log::info!("Close requested, shutting down...");
                self.close_requested = true;
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.physical_key == PhysicalKey::Code(KeyCode::Escape) {
                    log::info!("Escape pressed, shutting down...");
                    self.close_requested = true;
                    event_loop.exit();
                    return;
                }
                let pressed = event.state == winit::event::ElementState::Pressed;
                match event.physical_key {
                    PhysicalKey::Code(KeyCode::ArrowLeft) => self.aim_left = pressed,
                    PhysicalKey::Code(KeyCode::ArrowRight) => self.aim_right = pressed,
                    PhysicalKey::Code(KeyCode::ArrowUp) => self.aim_up = pressed,
                    PhysicalKey::Code(KeyCode::ArrowDown) => self.aim_down = pressed,
                    PhysicalKey::Code(KeyCode::KeyW) => self.move_forward = pressed,
                    PhysicalKey::Code(KeyCode::KeyS) => self.move_backward = pressed,
                    PhysicalKey::Code(KeyCode::KeyA) => self.move_left = pressed,
                    PhysicalKey::Code(KeyCode::KeyD) => self.move_right = pressed,
                    PhysicalKey::Code(KeyCode::Space) => self.move_up = pressed,
                    PhysicalKey::Code(KeyCode::ControlLeft)
                    | PhysicalKey::Code(KeyCode::ControlRight) => self.move_down = pressed,
                    PhysicalKey::Code(KeyCode::Equal) => {
                        if pressed {
                            self.nudge_camera(0.0, 0.0, -0.3);
                        }
                    }
                    PhysicalKey::Code(KeyCode::Minus) => {
                        if pressed {
                            self.nudge_camera(0.0, 0.0, 0.3);
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::Resized(new_size) => {
                log::debug!("Window resized to {}x{}", new_size.width, new_size.height);
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

fn load_gltf_bytes() -> Vec<u8> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let gltf_path = manifest_dir.join(GLTF_PATH);
    std::fs::read(&gltf_path).unwrap_or_else(|err| {
        panic!(
            "Failed to read glTF asset at {}: {}",
            gltf_path.display(),
            err
        )
    })
}

fn forward_to_yaw_pitch(forward: [f32; 3]) -> (f32, f32) {
    let len = (forward[0] * forward[0] + forward[1] * forward[1] + forward[2] * forward[2])
        .sqrt()
        .max(1e-6);
    let nx = forward[0] / len;
    let ny = forward[1] / len;
    let nz = forward[2] / len;
    let yaw = nz.atan2(nx);
    let pitch = ny.asin();
    (yaw, pitch)
}

fn camera_basis(yaw: f32, pitch: f32) -> ([f32; 3], [f32; 3], [f32; 3]) {
    let cos_pitch = pitch.cos();
    let forward = [
        yaw.cos() * cos_pitch,
        pitch.sin(),
        yaw.sin() * cos_pitch,
    ];
    let world_up = [0.0, 1.0, 0.0];
    let right = normalize(cross(forward, world_up));
    let up = cross(right, forward);
    (forward, right, up)
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 1e-6 {
        [v[0] / len, v[1] / len, v[2] / len]
    } else {
        [0.0, 0.0, 0.0]
    }
}

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();

    log::info!("🚀 Previz - Filament v1.69.0 Renderer POC");
    log::info!("   Press ESC or close window to exit");

    // Create event loop and run
    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");

    log::info!("👋 Goodbye!");
}
