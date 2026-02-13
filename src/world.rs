use std::sync::Arc;

pub const GRID_SIZE: usize = 32;

pub struct World {
    voxels: Vec<u32>,
}

pub struct WorldResource {
    pub buffer: wgpu::Buffer,
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
}

impl World {
    pub fn new() -> Self {
        let mut voxels = vec![0u32; GRID_SIZE * GRID_SIZE * GRID_SIZE];

        for x in 0..GRID_SIZE {
            for z in 0..GRID_SIZE {
                for y in 0..GRID_SIZE {
                    voxels[x + (y * GRID_SIZE) + (z * GRID_SIZE * GRID_SIZE)] = 1;
                }
            }
        }

        Self { voxels }
    }
}

impl WorldResource {
    pub fn new(device: &wgpu::Device, world: &World) -> Self {
        use wgpu::util::DeviceExt;

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("World Storage Buffer"),
            contents: bytemuck::cast_slice(&world.voxels),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("World Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0, // On le mettra au binding 0 de son propre groupe
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World Bind Group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            buffer,
            layout,
            bind_group,
        }
    }
}
