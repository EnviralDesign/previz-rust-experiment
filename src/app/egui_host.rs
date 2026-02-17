use egui_winit::winit::event::WindowEvent;
use winit::window::Window;

pub struct EguiFrameOutput {
    pub clipped_primitives: Vec<egui::ClippedPrimitive>,
    pub textures_delta: egui::TexturesDelta,
    pub pixels_per_point: f32,
    pub screen_size_px: [u32; 2],
    pub wants_pointer_input: bool,
    pub wants_keyboard_input: bool,
}

pub struct EguiHost {
    context: egui::Context,
    winit_state: egui_winit::State,
}

impl EguiHost {
    pub fn new(window: &Window) -> Self {
        let context = egui::Context::default();
        let viewport_id = egui::ViewportId::ROOT;
        let winit_state = egui_winit::State::new(
            context.clone(),
            viewport_id,
            window,
            None,
            None,
            None,
        );

        Self {
            context,
            winit_state,
        }
    }

    pub fn on_window_event(&mut self, window: &Window, event: &WindowEvent) -> bool {
        self.winit_state.on_window_event(window, event).consumed
    }

    pub fn run_ui<F>(&mut self, window: &Window, run_ui: F) -> EguiFrameOutput
    where
        F: FnMut(&egui::Context),
    {
        let raw_input = self.winit_state.take_egui_input(window);
        let full_output = self.context.run(raw_input, run_ui);
        self.winit_state
            .handle_platform_output(window, full_output.platform_output.clone());
        let pixels_per_point = self.context.pixels_per_point();
        let clipped_primitives = self
            .context
            .tessellate(full_output.shapes, pixels_per_point);
        let size = window.inner_size();

        EguiFrameOutput {
            clipped_primitives,
            textures_delta: full_output.textures_delta,
            pixels_per_point,
            screen_size_px: [size.width.max(1), size.height.max(1)],
            wants_pointer_input: self.context.wants_pointer_input(),
            wants_keyboard_input: self.context.wants_keyboard_input(),
        }
    }
}
