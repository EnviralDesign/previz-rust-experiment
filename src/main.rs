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

use filament::{
    Backend, ElementType, Engine, Entity, IndexType, PrimitiveType, VertexAttribute,
};
use std::ffi::c_void;
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

// Baked color material compiled during build with Filament's matc
const MATERIAL_BYTES: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/bakedColor.filamat"));

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
    triangle_entity: Option<Entity>,
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
            triangle_entity: None,
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

        // Setup orthographic camera (2D projection for simple triangle)
        let aspect = window_size.width as f64 / window_size.height as f64;
        let zoom = 1.5;
        camera.set_projection_ortho(-aspect * zoom, aspect * zoom, -zoom, zoom, 0.0, 10.0);

        // Configure viewport
        view.set_viewport(0, 0, window_size.width, window_size.height);
        view.set_post_processing_enabled(false);
        view.set_scene(&mut scene);
        view.set_camera(&mut camera);

        // Create triangle
        let triangle_entity = create_triangle(&mut engine, &mut entity_manager);
        scene.add_entity(triangle_entity);
        log::info!("Triangle entity created");

        // Store everything
        self.engine = Some(engine);
        self.swap_chain = Some(swap_chain);
        self.renderer = Some(renderer);
        self.view = Some(view);
        self.scene = Some(scene);
        self.camera = Some(camera);
        self.triangle_entity = Some(triangle_entity);

        log::info!("Filament initialization complete!");
    }

    /// Handle window resize
    fn handle_resize(&mut self, new_size: PhysicalSize<u32>) {
        if let (Some(view), Some(camera)) = (&mut self.view, &mut self.camera) {
            view.set_viewport(0, 0, new_size.width, new_size.height);

            let aspect = new_size.width as f64 / new_size.height as f64;
            let zoom = 1.5;
            camera.set_projection_ortho(-aspect * zoom, aspect * zoom, -zoom, zoom, 0.0, 10.0);
        }
    }

    /// Render a frame
    fn render(&mut self) {
        if let (Some(renderer), Some(swap_chain), Some(view)) =
            (&mut self.renderer, &mut self.swap_chain, &self.view)
        {
            if renderer.begin_frame(swap_chain) {
                renderer.render(view);
                renderer.end_frame();
            }
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return; // Window already created
        }

        log::info!("Creating window...");

        let window_attrs = WindowAttributes::default()
            .with_title("Previz - Filament v1.69.0 Hello World")
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
                }
            }
            WindowEvent::Resized(new_size) => {
                log::debug!("Window resized to {}x{}", new_size.width, new_size.height);
                self.handle_resize(new_size);
            }
            WindowEvent::RedrawRequested => {
                self.render();
                // Request another frame
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Request continuous redraws
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

/// Create a simple colored triangle with vertex colors
fn create_triangle(
    engine: &mut Engine,
    entity_manager: &mut filament::EntityManager,
) -> Entity {
    // Triangle vertices (x, y) positions - equilateral triangle centered at origin
    let triangle_positions: [f32; 6] = [
        0.0, 0.866,  // Top vertex
        -1.0, -0.5,  // Bottom left
        1.0, -0.5,   // Bottom right
    ];

    // Vertex colors (RGBA packed as u32 in ABGR format)
    let triangle_colors: [u32; 3] = [
        0xFF0000FF, // Red (ABGR)
        0xFF00FF00, // Green (ABGR)
        0xFFFF0000, // Blue (ABGR)
    ];

    // Create vertex buffer with 2 attributes: position and color
    let mut vertex_buffer = engine
        .vertex_buffer_builder()
        .vertex_count(3)
        .buffer_count(2) // Separate buffers for position and color
        .attribute(
            VertexAttribute::Position,
            0, // buffer index
            ElementType::Float2,
            0, // byte offset
            8, // byte stride (2 floats * 4 bytes)
        )
        .attribute(
            VertexAttribute::Color,
            1, // buffer index
            ElementType::UByte4,
            0, // byte offset
            4, // byte stride (4 bytes)
        )
        .normalized(VertexAttribute::Color, true)
        .build()
        .expect("Failed to create vertex buffer");

    // Upload vertex data
    vertex_buffer.set_buffer_at(0, &triangle_positions, 0);
    vertex_buffer.set_buffer_at(1, &triangle_colors, 0);

    // Create index buffer
    let mut index_buffer = engine
        .index_buffer_builder()
        .index_count(3)
        .buffer_type(IndexType::UShort)
        .build()
        .expect("Failed to create index buffer");

    // Upload index data
    let indices: [u16; 3] = [0, 1, 2];
    index_buffer.set_buffer(&indices, 0);

    // Create material from embedded bytes
    let mut material = engine
        .create_material(MATERIAL_BYTES)
        .expect("Failed to create material");
    let mut material_instance = material
        .default_instance()
        .expect("Failed to get material instance");

    // Create renderable entity
    let triangle_entity = entity_manager.create();

    engine
        .renderable_builder(1)
        .bounding_box([0.0, 0.0, 0.0], [2.0, 2.0, 1.0])
        .material(0, &mut material_instance)
        .geometry(0, PrimitiveType::Triangles, &mut vertex_buffer, &mut index_buffer)
        .culling(false)
        .build(triangle_entity);

    triangle_entity
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
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = App::new();
    event_loop.run_app(&mut app).expect("Event loop error");

    log::info!("👋 Goodbye!");
}
