use std::u32;

#[repr(C)]
#[derive(Default, Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SVONode {
    children_idx: u32, // 0 if empty
    color: u32,        // Packed color, for LOD
}

#[derive(Default)]
pub struct SVO {
    pub nodes: Vec<SVONode>,
}

impl SVO {
    pub fn new() -> Self {
        let mut nodes = Vec::with_capacity(1024); // Reserved capacity -> 1kB
        nodes.push(SVONode::default()); // The root node

        Self { nodes }
    }

    pub fn allocate_children(&mut self) -> u32 {
        let new_idx = self.nodes.len() as u32;

        for _ in 0..8 {
            self.nodes.push(SVONode::default());
        }

        new_idx
    }

    pub fn insert(&mut self, coord: glam::UVec3, color: u32, max_depth: u32) {
        let mut current_node_idx = 0; // Start in the root
        let mut size = 2_u32.pow(max_depth); // World size in block

        // Work coordinates (begin to world coordinates)
        let mut cur_coord = coord;

        for _ in 0..max_depth {
            size /= 2; // Subdivide world

            // Determine the child from the cube position (0-7)
            let child_x = if cur_coord.x >= size { 1 } else { 0 };
            let child_y = if cur_coord.y >= size { 1 } else { 0 };
            let child_z = if cur_coord.z >= size { 1 } else { 0 };

            let child_offset = (child_x << 2) | (child_y << 1) | child_z;

            // Allocate childs in the actual node if required
            if self.nodes[current_node_idx].children_idx == 0 {
                let new_node_idx = self.allocate_children();
                self.nodes[current_node_idx].children_idx = new_node_idx;
            }

            // Go down to the next level
            current_node_idx = (self.nodes[current_node_idx].children_idx + child_offset) as usize;

            // Convert coordinates to local coord for the next level
            if child_x == 1 {
                cur_coord.x -= size;
            }
            if child_y == 1 {
                cur_coord.y -= size;
            }
            if child_z == 1 {
                cur_coord.z -= size;
            }
        }

        // In the leaf, set the color
        self.nodes[current_node_idx].children_idx = u32::MAX;
        self.nodes[current_node_idx].color = color;
    }

    pub fn size(&self) -> usize {
        self.nodes.len() * size_of::<SVONode>()
    }

    pub fn as_bytes(&self) -> &[u8] {
        return bytemuck::cast_slice(&self.nodes);
    }
}
