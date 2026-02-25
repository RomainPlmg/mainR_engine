#[derive(Default, Debug, Clone, Copy)]
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
        nodes.push(SVONode::default());

        Self { nodes }
    }

    pub fn allocate_children(&mut self) -> u32 {
        let new_idx = self.nodes.len() as u32;

        for _ in 0..8 {
            self.nodes.push(SVONode::default());
        }

        new_idx
    }
}
