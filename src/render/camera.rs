use crate::filament::Camera;

#[derive(Debug, Clone, Copy)]
pub struct CameraMovement {
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
pub struct CameraController {
    pub position: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
}

impl CameraController {
    pub fn new(position: [f32; 3], yaw: f32, pitch: f32) -> Self {
        Self {
            position,
            yaw,
            pitch,
        }
    }

    pub fn from_bounds(center: [f32; 3], extent: [f32; 3]) -> Self {
        let radius = extent[0].max(extent[1]).max(extent[2]);
        let distance = if radius > 0.0 { radius * 3.0 } else { 3.0 };
        let position = [
            center[0] + distance,
            center[1] + distance * 0.4,
            center[2] + distance,
        ];
        let forward = [
            center[0] - position[0],
            center[1] - position[1],
            center[2] - position[2],
        ];
        let (yaw, pitch) = forward_to_yaw_pitch(forward);
        Self::new(position, yaw, pitch)
    }

    pub fn frame_bounds_preserve_orientation(&mut self, center: [f32; 3], extent: [f32; 3]) {
        let radius = extent[0].max(extent[1]).max(extent[2]);
        let distance = if radius > 0.0 { radius * 3.0 } else { 3.0 };
        let cos_pitch = self.pitch.cos();
        let forward = [
            self.yaw.cos() * cos_pitch,
            self.pitch.sin(),
            self.yaw.sin() * cos_pitch,
        ];
        self.position = [
            center[0] - forward[0] * distance,
            center[1] - forward[1] * distance,
            center[2] - forward[2] * distance,
        ];
    }

    pub fn apply(&mut self, camera: &mut Camera) {
        let (dir, _right, up) = self.basis();
        let eye = self.position;
        let center = [eye[0] + dir[0], eye[1] + dir[1], eye[2] + dir[2]];
        camera.look_at(eye, center, up);
    }

    pub fn nudge(&mut self, yaw_delta: f32, pitch_delta: f32, zoom_delta: f32) {
        self.yaw += yaw_delta;
        self.pitch += pitch_delta;
        wrap_angles(&mut self.yaw, &mut self.pitch);
        if zoom_delta != 0.0 {
            let (forward, _, _) = self.basis();
            self.position[0] += forward[0] * zoom_delta;
            self.position[1] += forward[1] * zoom_delta;
            self.position[2] += forward[2] * zoom_delta;
        }
    }

    pub fn orbit_around(&mut self, pivot: [f32; 3], yaw_delta: f32, pitch_delta: f32) {
        self.yaw += yaw_delta;
        self.pitch += pitch_delta;
        wrap_angles(&mut self.yaw, &mut self.pitch);

        let dx = self.position[0] - pivot[0];
        let dy = self.position[1] - pivot[1];
        let dz = self.position[2] - pivot[2];
        let distance = (dx * dx + dy * dy + dz * dz).sqrt().max(0.05);
        let (dir, _, _) = self.basis();
        self.position[0] = pivot[0] - dir[0] * distance;
        self.position[1] = pivot[1] - dir[1] * distance;
        self.position[2] = pivot[2] - dir[2] * distance;
    }

    pub fn basis(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        camera_basis(self.yaw, self.pitch)
    }

    pub fn move_horizontal(&mut self, right: f32, up: f32, forward: f32) {
        let yaw = self.yaw;
        let forward_dir = [yaw.cos(), 0.0, yaw.sin()];
        let right_dir = [-yaw.sin(), 0.0, yaw.cos()];
        let up_dir = [0.0, 1.0, 0.0];

        self.position[0] += right_dir[0] * right + up_dir[0] * up + forward_dir[0] * forward;
        self.position[1] += right_dir[1] * right + up_dir[1] * up + forward_dir[1] * forward;
        self.position[2] += right_dir[2] * right + up_dir[2] * up + forward_dir[2] * forward;
    }

    pub fn update_movement(&mut self, input: &CameraMovement, frame_dt: f32) -> bool {
        let move_speed = 1.5 * frame_dt;
        let aim_speed = 1.8 * frame_dt;
        let mut changed = false;

        if input.aim_left {
            self.yaw -= aim_speed;
            changed = true;
        }
        if input.aim_right {
            self.yaw += aim_speed;
            changed = true;
        }
        if input.aim_up {
            self.pitch += aim_speed;
            changed = true;
        }
        if input.aim_down {
            self.pitch -= aim_speed;
            changed = true;
        }

        let mut forward = 0.0;
        let mut right = 0.0;
        let mut up = 0.0;
        if input.move_forward {
            forward += move_speed;
        }
        if input.move_backward {
            forward -= move_speed;
        }
        if input.move_left {
            right -= move_speed;
        }
        if input.move_right {
            right += move_speed;
        }
        if input.move_up {
            up += move_speed;
        }
        if input.move_down {
            up -= move_speed;
        }

        if forward != 0.0 || right != 0.0 || up != 0.0 {
            self.move_horizontal(right, up, forward);
            changed = true;
        }

        changed
    }
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
    let forward = [yaw.cos() * cos_pitch, pitch.sin(), yaw.sin() * cos_pitch];
    let right = [-yaw.sin(), 0.0, yaw.cos()];
    let up = normalize(cross(right, forward));
    (forward, right, up)
}

fn wrap_angles(yaw: &mut f32, pitch: &mut f32) {
    const TWO_PI: f32 = std::f32::consts::PI * 2.0;
    if yaw.is_finite() {
        *yaw = (*yaw + std::f32::consts::PI).rem_euclid(TWO_PI) - std::f32::consts::PI;
    }
    if pitch.is_finite() {
        *pitch = (*pitch + std::f32::consts::PI).rem_euclid(TWO_PI) - std::f32::consts::PI;
    }
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

#[cfg(test)]
mod tests {
    use super::{CameraController, CameraMovement};

    #[test]
    fn from_bounds_produces_finite_state() {
        let camera = CameraController::from_bounds([1.0, 2.0, 3.0], [4.0, 5.0, 6.0]);
        assert!(camera.position.iter().all(|value| value.is_finite()));
        assert!(camera.yaw.is_finite());
        assert!(camera.pitch.is_finite());
    }

    #[test]
    fn movement_update_keeps_finite_values() {
        let mut camera = CameraController::new([0.0, 0.0, 5.0], 0.0, 0.0);
        let movement = CameraMovement {
            move_forward: true,
            move_backward: false,
            move_left: false,
            move_right: true,
            move_up: true,
            move_down: false,
            aim_left: false,
            aim_right: true,
            aim_up: true,
            aim_down: false,
        };
        let changed = camera.update_movement(&movement, 1.0 / 60.0);
        assert!(changed);
        assert!(camera.position.iter().all(|value| value.is_finite()));
        assert!(camera.yaw.is_finite());
        assert!(camera.pitch.is_finite());
    }

    #[test]
    fn frame_bounds_preserves_orientation() {
        let mut camera = CameraController::new([5.0, 6.0, 7.0], 1.1, -0.3);
        camera.frame_bounds_preserve_orientation([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        assert!((camera.yaw - 1.1).abs() < 1e-6);
        assert!((camera.pitch + 0.3).abs() < 1e-6);
        assert!(camera.position.iter().all(|value| value.is_finite()));
    }
}
