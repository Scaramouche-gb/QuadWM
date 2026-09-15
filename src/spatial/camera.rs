use glam::{Mat4, Vec3};

pub struct Camera {
    pub eye: Vec3,
    pub yaw: f32,   // in radians
    pub pitch: f32, // in radians
    pub fov_y: f32, // in radians
    pub aspect: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(aspect: f32) -> Self {
        Self {
            eye: Vec3::new(0.0, 1.0, 3.0),
            yaw: -std::f32::consts::FRAC_PI_2,
            pitch: 0.0,
            fov_y: 60.0_f32.to_radians(),
            aspect,
            z_near: 0.1,
            z_far: 100.0,
        }
    }

    pub fn forward(&self) -> Vec3 {
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();

        Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize()
    }

    pub fn right(&self) -> Vec3 {
        self.forward().cross(Vec3::Y).normalize()
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    pub fn build_view_projection_matrix(&self) -> Mat4 {
        let view = Mat4::look_to_rh(self.eye, self.forward(), Vec3::Y);
        let proj = Mat4::perspective_rh(self.fov_y, self.aspect, self.z_near, self.z_far);
        proj * view
    }
}

pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    pub move_forward: bool,
    pub move_backward: bool,
    pub move_left: bool,
    pub move_right: bool,
    pub move_up: bool,
    pub move_down: bool,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            move_forward: false,
            move_backward: false,
            move_left: false,
            move_right: false,
            move_up: false,
            move_down: false,
        }
    }

    pub fn process_mouse(&mut self, camera: &mut Camera, dx: f32, dy: f32) {
        camera.yaw += dx * self.sensitivity;
        camera.pitch -= dy * self.sensitivity;

        let max_pitch = 89.0_f32.to_radians();
        camera.pitch = camera.pitch.clamp(-max_pitch, max_pitch);
    }

    pub fn update_camera(&self, camera: &mut Camera, dt: f32) {
        let forward = camera.forward();
        let right = camera.right();
        let up = Vec3::Y;

        let mut move_vec = Vec3::ZERO;
        if self.move_forward {
            move_vec += forward;
        }
        if self.move_backward {
            move_vec -= forward;
        }
        if self.move_right {
            move_vec += right;
        }
        if self.move_left {
            move_vec -= right;
        }
        if self.move_up {
            move_vec += up;
        }
        if self.move_down {
            move_vec -= up;
        }

        if move_vec.length_squared() > 0.0 {
            camera.eye += move_vec.normalize() * self.speed * dt;
        }
    }
}
