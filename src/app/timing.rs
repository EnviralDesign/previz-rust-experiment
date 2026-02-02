use std::time::Instant;
use winit::window::Window;

pub struct FrameTiming {
    last_frame_time: Option<Instant>,
    last_fps_time: Instant,
    frame_count: u32,
    pub frame_dt: f32,
    render_ms: f32,
    base_title: String,
}

impl FrameTiming {
    pub fn new(base_title: String) -> Self {
        Self {
            last_frame_time: None,
            last_fps_time: Instant::now(),
            frame_count: 0,
            frame_dt: 1.0 / 60.0,
            render_ms: 0.0,
            base_title,
        }
    }

    pub fn set_render_ms(&mut self, render_ms: f32) {
        self.render_ms = render_ms;
    }

    pub fn update(&mut self, window: Option<&Window>, now: Instant) {
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
            if let Some(window) = window {
                window.set_title(&format!(
                    "{} - {:.1} fps (cadence {:.2} ms, render {:.2} ms)",
                    self.base_title, fps, ms, self.render_ms
                ));
            }
            self.frame_count = 0;
            self.last_fps_time = now;
        }
    }
}
