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

#[derive(Debug, Clone, Copy)]
pub enum InputAction {
    None,
    ZoomIn,
    ZoomOut,
}

impl InputState {
    pub fn handle_key(&mut self, key: PhysicalKey, pressed: bool) -> InputAction {
        match key {
            PhysicalKey::Code(KeyCode::ArrowLeft) => self.aim_left = pressed,
            PhysicalKey::Code(KeyCode::ArrowRight) => self.aim_right = pressed,
            PhysicalKey::Code(KeyCode::ArrowUp) => self.aim_up = pressed,
            PhysicalKey::Code(KeyCode::ArrowDown) => self.aim_down = pressed,
            PhysicalKey::Code(KeyCode::KeyW) => self.move_forward = pressed,
            PhysicalKey::Code(KeyCode::KeyS) => self.move_backward = pressed,
            PhysicalKey::Code(KeyCode::KeyA) => self.move_left = pressed,
            PhysicalKey::Code(KeyCode::KeyD) => self.move_right = pressed,
            PhysicalKey::Code(KeyCode::Space) => self.move_up = pressed,
            PhysicalKey::Code(KeyCode::ControlLeft) | PhysicalKey::Code(KeyCode::ControlRight) => {
                self.move_down = pressed
            }
            PhysicalKey::Code(KeyCode::Equal) => {
                if pressed {
                    return InputAction::ZoomIn;
                }
            }
            PhysicalKey::Code(KeyCode::Minus) => {
                if pressed {
                    return InputAction::ZoomOut;
                }
            }
            _ => {}
        }
        InputAction::None
    }
}
