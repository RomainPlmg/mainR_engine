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
    size: f32,
}

struct SVONode {
    children_idx: u32,
    color: u32,
}

struct StackNode {
    node_idx: u32,
    box: BoundingBox,
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
    let t1 = (box.min + box.size - ray.origin) / ray.dir;

    // If negative direction, t0 > t1
    let t_min = min(t0, t1);
    let t_max = max(t0, t1);

    // In the box if we are in all 3 axis
    let t_enter = max(t_min.x, max(t_min.y, t_min.z));
    let t_exit = min(t_max.x, min(t_max.y, t_max.z));

    return vec2<f32>(t_enter, t_exit);
}

fn child_box(parent: BoundingBox, child_idx: u32) -> BoundingBox {
    let half = parent.size / 2.0;
    let center = parent.min + half;

    var b: BoundingBox;
    b.min.x = select(parent.min.x, center.x, bool(child_idx & 4u));
    b.min.y = select(parent.min.y, center.y, bool(child_idx & 2u));
    b.min.z = select(parent.min.z, center.z, bool(child_idx & 1u));
    b.size = half;
    return b;
}

// Return an array of children indices, sorted by ray intersection
fn sort_children(ray: Ray) -> array<u32, 8> {
    var mask = 0u;
    if (ray.dir.x < 0.0) { mask |= 4u; } // 0b100
    if (ray.dir.y < 0.0) { mask |= 2u; } // 0b010
    if (ray.dir.z < 0.0) { mask |= 1u; } // 0b001

    return array<u32, 8>(
        0u ^ mask,
        1u ^ mask,
        2u ^ mask,
        3u ^ mask,
        4u ^ mask,
        5u ^ mask,
        6u ^ mask,
        7u ^ mask,
    );
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

    var world_box: BoundingBox;
    world_box.min = vec3<f32>(0.0);
    world_box.size = world_size;

    // Check if intersect the world
    let hit = intersect_aabb(ray, world_box);
    if (hit.x > hit.y || hit.y < 0.0) { // No intersection
        return vec4<f32>(get_env_color(ray), 1.0);
    }
    // ray.origin = ray.origin + ray.dir * (hit.x + EPSILON);

    var stack: array<StackNode, 16>; // TODO: Calculate optimal size, depending on the view distance
    var stack_ptr = 0u;

    // Push world box into the stack
    stack[stack_ptr].node_idx = 0u;
    stack[stack_ptr].box = world_box;
    stack_ptr++;

    var iteration = 0u;

    /***************** Octree navigation *****************/
    while (stack_ptr > 0u && iteration < 256) {
        iteration++;

        // Pop the parent
        stack_ptr--;
        let stack_entry = stack[stack_ptr];
        let current_node = svo[stack_entry.node_idx];

        if (current_node.children_idx == 0xFFFFFFFFu) { // Leaf
            return vec4<f32>(vec3<f32>(1.0 / f32(iteration + 1)), 1.0);
            return vec4<f32>(unpack4x8unorm(current_node.color).rgb, 1.0);
        }

        if (current_node.children_idx != 0u) { // Non-empty children
            // return vec4<f32>(1.0, 0.0, 0.0, 1.0);
            let sorted_children = sort_children(ray);
            // Push all intersected children in the stack
            for (var i = 7; i >= 0; i--) {
                let ci = sorted_children[u32(i)];
                let curr_box = child_box(stack_entry.box, ci);

                let child_node = svo[current_node.children_idx + ci];
                if (child_node.children_idx == 0u) { // If child is empty
                    continue;
                }

                let hit = intersect_aabb(ray, curr_box);
                if (hit.x <= hit.y + EPSILON && hit.y > 0.0) {
                    stack[stack_ptr].node_idx = current_node.children_idx + ci;
                    stack[stack_ptr].box = curr_box;
                    stack_ptr++;
                }
            }
        }

        if (stack_ptr >= 16) { return vec4<f32>(0.0, 1.0, 1.0, 1.0); } // Safe limit
    }

    return vec4<f32>(get_env_color(ray), 1.0); 
}