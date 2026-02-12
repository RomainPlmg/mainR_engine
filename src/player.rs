use crate::camera::{Camera};

#[derive(Default)]
pub struct Player {
    pub position: glam::Vec3,
    pub camera: Camera,
}

impl Player {
    pub fn new(position: glam::Vec3) -> Self {
        let camera = Camera::new(position);

        Self { position, camera }
    }
}
