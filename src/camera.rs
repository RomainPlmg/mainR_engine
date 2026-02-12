use wgpu::util::DeviceExt;
use winit::{
    event::{ElementState, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Default)]
pub struct CameraController {
    sensitivity: f32,
    mouse_delta: glam::DVec2,

    forward_pressed: bool,
    backward_pressed: bool,
    right_pressed: bool,
    left_pressed: bool,
}

#[derive(Default)]
pub struct Camera {
    pub position: glam::Vec3,

    pub fov: f32,

    front: glam::Vec3,
    right: glam::Vec3,
    up: glam::Vec3,
    world_up: glam::Vec3,

    yaw: f32,
    pitch: f32,

    inv_view_proj: glam::Mat4,
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    pub inv_view_proj: [[f32; 4]; 4],
    pub origin: [f32; 3],
    _padding0: f32,
}

pub struct CameraResource {
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl CameraController {
    pub fn new(sensitivity: f32) -> Self {
        Self {
            sensitivity,
            ..Default::default()
        }
    }

    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.mouse_delta += glam::DVec2 {
            x: mouse_dx,
            y: mouse_dy,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera, width: usize, height: usize, dt: std::time::Duration) {
        let dt = dt.as_secs_f32();

        camera.yaw += self.mouse_delta.x as f32 * self.sensitivity;
        camera.pitch += self.mouse_delta.y as f32 * self.sensitivity;

        // Lock pitch to avoid backflip
        camera.pitch = camera.pitch.clamp(-89.0, 89.0);

        // Reset the mouse delta
        self.mouse_delta = glam::DVec2::ZERO;

        camera.update_vectors();
        camera.update_matrices(width, height);
    }
}

impl Camera {
    pub fn new(position: glam::Vec3) -> Self {
        Self {
            position,
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

        let view = glam::Mat4::look_at_rh(self.position, self.position + self.front, self.up);
        let proj = glam::Mat4::perspective_rh(self.fov.to_radians(), aspect, 0.1, 1000.0);

        let view_proj = proj * view;

        self.inv_view_proj = view_proj.inverse();
    }
}

impl CameraUniform {
    pub fn new(camera: &Camera) -> Self {
        Self {
            inv_view_proj: camera.inv_view_proj.to_cols_array_2d(),
            origin: camera.position.to_array(),
            ..Default::default()
        }
    }

    pub fn update(&mut self, camera: &Camera) {
        self.inv_view_proj = camera.inv_view_proj.to_cols_array_2d();
        self.origin = camera.position.to_array();
    }
}

impl CameraResource {
    pub fn new(device: &wgpu::Device, camera: &Camera) -> Self {
        let uniform = CameraUniform::new(&camera);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Camera Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            uniform,
            buffer,
            layout,
            bind_group,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, camera: &Camera) {
        // No need to update the layout & the bind group
        self.uniform.update(camera);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
