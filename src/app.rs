use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::PhysicalKey,
    window::{Window, WindowId},
};

use crate::state::State;

pub struct App {
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>, // State as option 'cause window can't be created before the Resumed state
    last_render_time: Instant,
    frame_count: u32,
    accum_time: Duration,
}

impl App {
    pub fn new(event_loop: &EventLoop<State>) -> Self {
        let proxy = Some(event_loop.create_proxy());
        Self {
            proxy,
            state: None,
            last_render_time: Instant::now(),
            frame_count: 0,
            accum_time: Duration::ZERO,
        }
    }

    fn update_fps(&mut self) -> (Duration, Option<String>) {
        let now = Instant::now();
        let delta_time = now - self.last_render_time;
        self.last_render_time = now;

        self.accum_time += delta_time;
        self.frame_count += 1;

        let mut new_title = None;
        if self.accum_time >= Duration::from_millis(200) {
            // fps = frame_count / 0.2s (donc count * 5)
            let fps = self.frame_count * 5;
            new_title = Some(format!(
                "MainR Engine - FPS: {} | {:.2}ms",
                fps,
                delta_time.as_secs_f32() * 1000.0
            ));

            self.accum_time = Duration::ZERO;
            self.frame_count = 0;
        }

        (delta_time, new_title)
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes().with_inner_size(LogicalSize::new(1280.0, 720.0)),
                )
                .unwrap(),
        );

        let mut state = pollster::block_on(State::new(window)).unwrap();
        state.display.set_cursor_locked(true);
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                self.state.take(); // Destroy the state
                event_loop.exit();
                return;
            }
            WindowEvent::Resized(size) => {
                if let Some(state) = &mut self.state {
                    state.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let (dt, title) = self.update_fps();
                if let Some(state) = &mut self.state {
                    if let Some(t) = title {
                        state.display.window.set_title(&t);
                    }
                    state.update(dt);

                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => {
                            state.resize(state.display.config.width, state.display.config.height)
                        }
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(code),
                        ..
                    },
                ..
            } => {
                if let Some(game_state) = &mut self.state {
                    game_state.player_controller.process_keyboard(code, state);
                }
            }
            _ => (),
        }
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event {
            if let Some(state) = &mut self.state {
                state.camera_controller.process_mouse(delta.0, delta.1);
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(state) = &self.state {
            state.display.window.request_redraw();
        }
    }
}
