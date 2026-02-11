#[derive(Default, Debug)]
pub struct CameraSettings {
    pub position: glam::Vec3,
    pub fov: f32,
    pub sensitivity: f32,
}

#[derive(Default)]
pub struct Camera {
    settings: CameraSettings,

    front: glam::Vec3,
    right: glam::Vec3,
    up: glam::Vec3,
    world_up: glam::Vec3,

    yaw: f32,
    pitch: f32,

    inv_view_proj: glam::Mat4,
}

impl Camera {
    pub fn new(settings: CameraSettings) -> Self {
        Self {
            settings,
            world_up: glam::Vec3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            ..Default::default()
        }
    }

    fn update_vectors(&mut self) {
        let mut front = glam::Vec3::ZERO;
        front.x = self.yaw.to_radians().cos() * self.pitch.to_radians().cos();
        front.y = self.pitch.to_radians().sin();
        front.z = self.yaw.to_radians().sin() * self.pitch.to_radians().cos();

        self.front = front.normalize();
        self.right = self.front.cross(self.world_up).normalize();
        self.up = self.right.cross(self.front).normalize();
    }

    fn update_matrices(&mut self, width: usize, height: usize) {
        let aspect = width as f32 / height as f32;

        let view = glam::Mat4::look_at_rh(
            self.settings.position,
            self.settings.position + self.front,
            self.up,
        );
        let proj = glam::Mat4::perspective_rh(self.settings.fov.to_radians(), aspect, 0.1, 1000.0);

        let view_proj = proj * view;

        self.inv_view_proj = view_proj.inverse();
    }
}
