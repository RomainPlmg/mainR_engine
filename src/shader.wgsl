// ===========================
// Structures
// ===========================

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct CameraUniform {
    inv_view_proj: mat4x4<f32>,
    origin: vec3<f32>,
};

struct WorldUniforms {
    grid_size: u32,
};

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
};

struct BoundingBox {
    min: vec3<f32>,   // Down left corner of the box
    max: vec3<f32>,   // Top right corner of the box
}

struct SVONode {
    children_idx: u32,
    color: u32,
}

struct HitState {
    node_idx: u32,       // Node index in the SVO
    parent_idx: u32,     // Parent index in the SVO
    box: BoundingBox,    // Last intersected node
    depth: u32,          // Depth of the octree
}

// ===========================
// Constants
// ===========================

const EPSILON: f32 = 0.00001;
const CHUNK_SIZE: u32 = 16;

// ===========================
// Bindings
// ===========================

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> svo: array<SVONode>;
@group(1) @binding(1)
var<uniform> world_params: WorldUniforms;

// ===========================
// Utility functions
// ===========================

fn get_env_color(ray: Ray) -> vec3<f32> {
    let sky_gradient = ray.dir.y + 1;
    return mix(vec3<f32>(1.0), vec3<f32>(0.5, 0.7, 1.0), sky_gradient);
}

fn intersect_aabb(ray: Ray, box: BoundingBox) -> vec2<f32> {
    let t0 = (box.min - ray.origin) / ray.dir;
    let t1 = (box.max - ray.origin) / ray.dir;

    // If negative direction, t0 > t1
    let t_min = min(t0, t1);
    let t_max = max(t0, t1);

    // In the box if we are in all 3 axis
    let t_enter = max(t_min.x, max(t_min.y, t_min.z));
    let t_exit = min(t_max.x, min(t_max.y, t_max.z));

    return vec2<f32>(t_enter, t_exit);
}

fn child_box(parent: BoundingBox, child_idx: u32) -> BoundingBox {
    let half = (parent.max - parent.min) / 2.0;
    let center = parent.min + half;

    var b: BoundingBox;
    b.min.x = select(parent.min.x, center.x, bool(child_idx & 4u));
    b.min.y = select(parent.min.y, center.y, bool(child_idx & 2u));
    b.min.z = select(parent.min.z, center.z, bool(child_idx & 1u));
    b.max = b.min + half;
    return b;
}

fn first_child(ray: Ray, parent: BoundingBox) -> u32 {
    var best_t = 9999999.0;
    var best_idx = 0u;

    for (var i = 0u; i < 8u; i++) {
        let child = child_box(parent, i);
        let t = intersect_aabb(ray, child);

        if (t.x <= t.y && t.y > 0.0 && max(t.x, 0.0) < best_t) {
            best_t = max(t.x, 0.0);
            best_idx = i;
        }
    }
    return best_idx;
}

// ===========================
// Vertex shader
// ===========================

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    // Render a giant triangle as the canvas
    let x = f32(i32(vertex_index == 1u)) * 4.0 - 1.0;
    let y = f32(i32(vertex_index == 2u)) * 4.0 - 1.0;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x, y);
    return out;
}

// ===========================
// Fragment shader
// ===========================

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize coordinates between 0 & 1
    let ndc = in.uv;
    let target_world = camera.inv_view_proj * vec4<f32>(ndc, 1.0, 1.0);
    let target_pos = target_world.xyz / target_world.w;
    
    var ray: Ray;
    ray.origin = camera.origin;
    ray.dir = normalize(target_pos - ray.origin);

    let world_size = f32(world_params.grid_size * CHUNK_SIZE);
    let max_depth = u32(log2(world_size));

    // return vec4<f32>(f32(max_depth) / 10.0, 0.0, 0.0, 1.0);

    var current_box: BoundingBox;
    current_box.min = vec3<f32>(0.0);
    current_box.max = vec3<f32>(world_size);
    var node_idx = 0u;

    /***************** Octree navigation *****************/
    for(var depth = 0u; depth <= max_depth; depth++) {
        let node = svo[node_idx];

        if (node.children_idx == 0u) {
            return vec4<f32>(1.0); 
            return vec4<f32>(get_env_color(ray), 1.0);
        }
        if (node.children_idx == 0xFFFFFFFFu) {
            return vec4<f32>(unpack4x8unorm(node.color).rgb, 1.0);
        }

        let child_offset = first_child(ray, current_box);
        current_box = child_box(current_box, child_offset);

        node_idx = node.children_idx + child_offset;
    }

    return vec4<f32>(1.0); 
}