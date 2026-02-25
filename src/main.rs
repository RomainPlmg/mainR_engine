use crate::app::App;
use winit::event_loop::EventLoop;

mod app;
mod camera;
mod chunk;
mod gpu_context;
mod player;
mod player_controller;
mod state;
mod svo;
mod voxel;
mod world;

fn main() {
    env_logger::init();
    let event_loop = EventLoop::with_user_event().build().unwrap();
    let mut app = App::new(&event_loop);
    event_loop.run_app(&mut app).unwrap();
}
