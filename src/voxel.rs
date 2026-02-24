#[repr(C)]
#[derive(Default, Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Voxel {
    pub color: u32,
}

impl Voxel {
    pub fn new(color: glam::Vec3, is_solid: bool) -> Self {
        if !is_solid {
            return Self { color: 0 };
        }

        let r = (color.x * 255.0) as u32;
        let g = (color.y * 255.0) as u32;
        let b = (color.z * 255.0) as u32;
        let a = 255u32;

        let color = (a << 24) | (b << 16) | (g << 8) | r;
        Self { color }
    }
}
