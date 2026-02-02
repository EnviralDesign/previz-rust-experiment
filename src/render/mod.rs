mod camera;

pub use camera::{CameraController, CameraMovement};

use crate::filament::{Backend, Camera, Engine, Entity, EntityManager, Renderer, Scene, SwapChain, View};
use std::ffi::c_void;
use winit::dpi::PhysicalSize;
use winit::window::Window;

#[cfg(target_os = "windows")]
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

pub struct RenderContext {
    engine: Engine,
    swap_chain: SwapChain,
    renderer: Renderer,
    view: View,
    scene: Scene,
    camera: Camera,
    directional_light: Entity,
}

impl RenderContext {
    pub fn new(window: &Window) -> Self {
        let native_handle = get_native_window_handle(window);
        let window_size = window.inner_size();

        let mut engine = Engine::create(Backend::OpenGL).expect("Failed to create Filament engine");
        let swap_chain = engine
            .create_swap_chain(native_handle)
            .expect("Failed to create swap chain");
        let mut renderer = engine.create_renderer().expect("Failed to create renderer");
        renderer.set_clear_options(0.1, 0.1, 0.2, 1.0, true, true);

        let mut scene = engine.create_scene().expect("Failed to create scene");
        let mut view = engine.create_view().expect("Failed to create view");

        let mut entity_manager = engine.entity_manager();
        let camera_entity = entity_manager.create();
        let mut camera = engine
            .create_camera(camera_entity)
            .expect("Failed to create camera");

        view.set_viewport(0, 0, window_size.width, window_size.height);
        view.set_scene(&mut scene);
        view.set_camera(&mut camera);

        let light_entity = engine.create_directional_light(
            &mut entity_manager,
            [1.0, 1.0, 1.0],
            100_000.0,
            [0.0, -1.0, -0.5],
        );
        scene.add_entity(light_entity);

        Self {
            engine,
            swap_chain,
            renderer,
            view,
            scene,
            camera,
            directional_light: light_entity,
        }
    }

    pub fn engine_scene_mut(&mut self) -> (&mut Engine, &mut Scene) {
        (&mut self.engine, &mut self.scene)
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.view.set_viewport(0, 0, new_size.width, new_size.height);
        let aspect = new_size.width as f64 / new_size.height as f64;
        self.camera
            .set_projection_perspective(45.0, aspect, 0.1, 1000.0);
    }

    pub fn set_projection_for_window(&mut self, window: &Window) {
        let size = window.inner_size();
        let aspect = size.width as f64 / size.height as f64;
        self.camera
            .set_projection_perspective(45.0, aspect, 0.1, 1000.0);
    }

    pub fn render(&mut self) -> f32 {
        let frame_start = std::time::Instant::now();
        if self.renderer.begin_frame(&mut self.swap_chain) {
            self.renderer.render(&self.view);
            self.renderer.end_frame();
        }
        let render_end = std::time::Instant::now();
        render_end
            .saturating_duration_since(frame_start)
            .as_secs_f32()
            * 1000.0
    }
}

/// Get the native window handle (HWND) on Windows
#[cfg(target_os = "windows")]
fn get_native_window_handle(window: &Window) -> *mut c_void {
    match window.window_handle().unwrap().as_raw() {
        RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut c_void,
        _ => panic!("Expected Win32 window handle"),
    }
}
