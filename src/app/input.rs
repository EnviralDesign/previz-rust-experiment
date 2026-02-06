use winit::keyboard::{KeyCode, PhysicalKey};

#[derive(Default, Debug, Clone, Copy)]
pub struct InputState {
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub move_up: bool,
    pub move_down: bool,
    pub aim_left: bool,
    pub aim_right: bool,
    pub aim_up: bool,
    pub aim_down: bool,
}

impl InputState {
    pub fn handle_key(&mut self, key: PhysicalKey, pressed: bool) {
        match key {
            PhysicalKey::Code(KeyCode::ArrowLeft) => self.aim_left = pressed,
            PhysicalKey::Code(KeyCode::ArrowRight) => self.aim_right = pressed,
            PhysicalKey::Code(KeyCode::ArrowUp) => self.aim_up = pressed,
            PhysicalKey::Code(KeyCode::ArrowDown) => self.aim_down = pressed,
            _ => {}
        }
    }
}
