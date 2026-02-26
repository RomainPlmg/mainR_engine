use crate::{
    chunk::{self, *},
    svo::{SVO, SVONode},
    voxel::Voxel,
};
use dashmap::DashMap;
use noise::Perlin;
use wgpu::util::DeviceExt;

pub struct World {
    chunks: DashMap<glam::IVec3, Chunk>,
    octree: SVO,
    pub params: WorldParams,
}

pub struct WorldResource {
    pub layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    svo_buffer: wgpu::Buffer,
    uniform_buffer: wgpu::Buffer,
}

pub struct WorldParams {
    pub view_distance: u32,
}

#[repr(C, align(16))]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct WorldUniform {
    pub view_distance: u32,
    _padding0: [u32; 3],
}

impl World {
    pub fn new() -> Self {
        let params = WorldParams::default();
        let chunks = DashMap::new();
        let perlin = Perlin::new(1);

        for x in 0..params.view_distance {
            for z in 0..params.view_distance {
                for y in 0..1 {
                    let chunk_coord = glam::IVec3::new(x as i32, y as i32, z as i32);
                    let mut chunk = Chunk::new();
                    chunk.generate(&perlin, chunk_coord);
                    chunks.insert(chunk_coord, chunk);
                }
            }
        }

        // Fill the octree
        let mut octree = SVO::new();
        let max_depth = (params.view_distance * CHUNK_SIZE).ilog2();

        for entry in chunks.iter() {
            let chunk_coord = entry.key();
            let chunk = entry.value();

            for (index, voxel) in chunk.iter_voxels() {
                let local_voxel_coord = Chunk::index_to_local_pos(index);
                let global_voxel_coord = Chunk::local_to_world_pos(&local_voxel_coord, chunk_coord);

                octree.insert(global_voxel_coord.as_uvec3(), voxel.color, max_depth);
            }
        }

        println!("{} kB", octree.size() / 1000);

        Self {
            chunks,
            octree,
            params,
        }
    }
}

impl WorldResource {
    pub fn new(device: &wgpu::Device, view_distance: u32) -> Self {
        let uniform = WorldUniform {
            view_distance: view_distance,
            ..Default::default()
        };

        // Sparse Voxel Octree buffer
        let svo_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Pool Storage Buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            size: 32000000 as u64, // TODO: Dynamic allocation
            mapped_at_creation: false,
        });

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("World Bind Group Layout"),
            entries: &[
                // Binding 0 -> SVO
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 1 -> World Uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("World Bind Group"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: svo_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            layout,
            bind_group,
            svo_buffer,
            uniform_buffer,
        }
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, world: &World) {
        queue.write_buffer(
            &self.svo_buffer,
            0,
            world.octree.as_bytes(),
        );
    }
}

impl Default for WorldParams {
    fn default() -> WorldParams {
        WorldParams { view_distance: 16 }
    }
}
