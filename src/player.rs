use crate::{camera::Camera, player_controller::PlayerController};

pub struct Player {
    position: glam::Vec3,
    pub camera: Camera,
}

impl Player {
    pub fn new(position: glam::Vec3) -> Self {
        let camera = Camera::new(position);

        Self { position, camera }
    }

    pub fn move_player(
        &mut self,
        controller: &PlayerController,
        dt: std::time::Duration,
        speed: f32,
    ) {
        let forward =
            glam::Vec3::new(self.camera.front.x, 0.0, self.camera.front.z).normalize_or_zero();
        let right =
            glam::Vec3::new(self.camera.right.x, 0.0, self.camera.right.z).normalize_or_zero();

        let mut direction = glam::Vec3::ZERO;

        if controller.forward {
            direction += forward;
        }
        if controller.backward {
            direction -= forward;
        }
        if controller.left {
            direction -= right;
        }
        if controller.right {
            direction += right;
        }
        if controller.up {
            direction += self.camera.world_up;
        }
        if controller.down {
            direction -= self.camera.world_up;
        }

        if direction.length_squared() > 0.0 {
            self.position += direction.normalize() * speed * dt.as_secs_f32();
        }

        self.camera.position = self.position;
    }
}
