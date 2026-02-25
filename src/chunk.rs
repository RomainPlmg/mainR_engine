use noise::{NoiseFn, Perlin};

use crate::voxel::Voxel;

pub const CHUNK_SIZE: u32 = 16;
pub const VOXELS_PER_CHUNK: u32 = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

pub struct Chunk {
    voxels: Vec<Voxel>,
}

impl Chunk {
    pub fn new() -> Self {
        let voxels = vec![Voxel::default(); (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];
        Self { voxels }
    }

    pub fn generate(&mut self, perlin: &Perlin, chunk_coord: glam::IVec3) {
        for lx in 0..CHUNK_SIZE {
            for lz in 0..CHUNK_SIZE {
                let world_x = chunk_coord.x * CHUNK_SIZE as i32 + lx as i32;
                let world_z = chunk_coord.z * CHUNK_SIZE as i32 + lz as i32;

                let height = perlin.get([world_x as f64 / 64.0, world_z as f64 / 64.0]);
                let height = ((height + 1.0) / 2.0 * CHUNK_SIZE as f64) as usize;

                for ly in 0..height.min(CHUNK_SIZE as usize) {
                    let color = if ly < 2 {
                        glam::vec3(0.5, 0.4, 0.3)
                    } else if ly == height - 1 {
                        glam::vec3(0.2, 0.8, 0.3)
                    } else {
                        glam::vec3(0.6, 0.6, 0.6)
                    };

                    self.voxels[(lx + (ly as u32 * CHUNK_SIZE) + (lz * CHUNK_SIZE * CHUNK_SIZE))
                        as usize] = Voxel::new(color, true);
                }
            }
        }
    }

    pub fn iter_voxels<'a>(&'a self) -> impl Iterator<Item = (usize, &'a Voxel)> {
        self.voxels
            .iter()
            .enumerate()
            .filter(|(_, v)| v.color & (0xFF) != 0)
    }

    pub fn index_to_local_pos(index: usize) -> glam::IVec3 {
        let x = (index as u32 % CHUNK_SIZE) as i32;
        let y = ((index as u32 / CHUNK_SIZE) % CHUNK_SIZE) as i32;
        let z = (index as u32 / CHUNK_SIZE) as i32;
        glam::IVec3::new(x, y, z)
    }

    pub fn local_to_world_pos(local_pos: &glam::IVec3, chunk_pos: &glam::IVec3) -> glam::IVec3 {
        (chunk_pos * CHUNK_SIZE as i32) + local_pos
    }

    pub fn as_bytes(&self) -> &[u8] {
        return bytemuck::cast_slice(&self.voxels);
    }
}
