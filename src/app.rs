use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

use crate::state::State;

pub struct App {
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>, // State as option 'cause window can't be created before the Resumed state
    last_frame: std::time::Instant,
}

impl App {
    pub fn new(event_loop: &EventLoop<State>) -> Self {
        let proxy = Some(event_loop.create_proxy());
        Self {
            proxy,
            state: None,
            last_frame: std::time::Instant::now(),
        }
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

        let state = pollster::block_on(State::new(window)).unwrap();
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => {
                self.state.take(); // Destroy the state
                event_loop.exit()
            }
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let dt = std::time::Instant::now().elapsed();
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
