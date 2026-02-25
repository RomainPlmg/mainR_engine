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
    pub pool_capacity: u32,
    chunk_pool_buffer: wgpu::Buffer,
    indirection_buffer: wgpu::Buffer,
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

        println!("{} kB", octree.nodes.len() * size_of::<SVONode>() / 1000);

        Self {
            chunks,
            octree,
            params,
        }
    }
}

impl WorldResource {
    pub fn new(device: &wgpu::Device, view_distance: u32) -> Self {
        let indirection_count = view_distance * view_distance * view_distance;
        let pool_capacity = view_distance * view_distance * view_distance;

        let uniform = WorldUniform {
            view_distance: view_distance,
            ..Default::default()
        };

        // Chunk pool buffer store all visible chunks
        let chunk_pool_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Chunk Pool Storage Buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            size: ((pool_capacity + 1) * VOXELS_PER_CHUNK * 4) as u64, // +1 because slot 0 reserved for empty chunk
            mapped_at_creation: false,
        });

        // Indirection buffer indicates for a given index if there is a chunk
        // (eg: index 786 <=> chunk(x: 2, y: 1, z: 16) -> contains the index in the chunk pool buffer or 0 if empty)
        let indirection_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Indirection Storage Buffer"),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            size: (indirection_count * 4) as u64,
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
                // Binding 0 -> Indirection
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
                // Binding 1 -> Chunk Pool
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Binding 2 -> World Uniform
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
                    resource: indirection_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: chunk_pool_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            layout,
            bind_group,
            pool_capacity,
            chunk_pool_buffer,
            indirection_buffer,
            uniform_buffer,
        }
    }

    pub fn upload(&mut self, queue: &wgpu::Queue, world: &World) {
        let mut gpu_pool_index: u32 = 1; // Begin at 1 because 0 is for empty chunk
        let view_distance = world.params.view_distance;
        let mut indirection_data =
            vec![0u32; (view_distance * view_distance * view_distance) as usize];

        for entry in world.chunks.iter() {
            let pos = entry.key();
            let chunk = entry.value();
            queue.write_buffer(
                &self.chunk_pool_buffer,
                (gpu_pool_index * VOXELS_PER_CHUNK * size_of::<Voxel>() as u32) as u64,
                chunk.as_bytes(),
            );

            let indirection_idx: u32 = pos.x as u32
                + pos.y as u32 * view_distance
                + pos.z as u32 * view_distance * view_distance;

            indirection_data[indirection_idx as usize] = gpu_pool_index;

            gpu_pool_index += 1;
        }

        queue.write_buffer(
            &self.indirection_buffer,
            0,
            bytemuck::cast_slice(&indirection_data),
        );
    }
}

impl Default for WorldParams {
    fn default() -> WorldParams {
        WorldParams { view_distance: 16 }
    }
}
