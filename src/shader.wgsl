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

struct DDAState {
    grid_pos: vec3<f32>,  // Current voxel position in the grid
    step_dir: vec3<f32>,  // -1 or 1 for each axis
    t_max: vec3<f32>,     // Distance along the ray until next boundary for each axis
    t_dist: vec3<f32>,    // Distance along the ray between two boundaries for each axis
}

// ===========================
// Constants
// ===========================

const EPSILON: f32 = 0.00001;
const CHUNK_SIZE: i32 = 16;
const SUN_DIR: vec3<f32> = vec3<f32>(0.4, 1.0, 0.6);
const AMBIENT_LIGHT: f32 = 0.2;

// ===========================
// Bindings
// ===========================

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> indirection: array<u32>;
@group(1) @binding(1)
var<storage, read> chunk_pool: array<u32>;
@group(1) @binding(2)
var<uniform> world_params: WorldUniforms;

// ===========================
// Utility functions
// ===========================
fn world_to_chunk(world_pos: vec3<i32>) -> vec3<i32> {
    return world_pos >> vec3<u32>(4u); // Divided by 16 (CHUNK_SIZE)
}

fn chunk_to_indir_idx(chunk_coords: vec3<i32>, grid_size: i32) -> u32 {
    let c = vec3<u32>(chunk_coords) % vec3<u32>(u32(grid_size));
    return c.x + c.y * u32(grid_size) + c.z * u32(grid_size) * u32(grid_size);
}

fn local_voxel_idx(world_pos: vec3<i32>) -> u32 {
    let local = vec3<u32>(world_pos & vec3<i32>(CHUNK_SIZE - 1));
    return local.x + local.y * u32(CHUNK_SIZE) + local.z * 256u;
}

fn is_out_of_bounds(world_pos: vec3<i32>, world_size: i32) -> bool {
    return any(world_pos < vec3<i32>(0)) || any(world_pos >= vec3<i32>(world_size));
}

fn get_env_color(ray: Ray) -> vec3<f32> {
    let sky_gradient = ray.dir.y + 1;
    return mix(vec3<f32>(1.0), vec3<f32>(0.5, 0.7, 1.0), sky_gradient);
}

fn get_voxel_color(world_pos: vec3<i32>) -> vec4<f32> {
    let gs = i32(world_params.grid_size);
    let world_size = gs * CHUNK_SIZE;

    if (is_out_of_bounds(world_pos, world_size)) {
        return vec4<f32>(0.0);
    }

    let chunk_coords = world_to_chunk(world_pos);
    let indir_idx = chunk_to_indir_idx(chunk_coords, gs);
    let pool_id = indirection[indir_idx];

    if (pool_id == 0u) { return vec4<f32>(0.0); } // Empty chunk

    let voxel_data = chunk_pool[pool_id * 4096u + local_voxel_idx(world_pos)];
    return unpack4x8unorm(voxel_data);
}

fn inside_empty_chunk(world_pos: vec3<i32>) -> bool {
    let gs = i32(world_params.grid_size);
    let world_size = gs * CHUNK_SIZE;

    if (is_out_of_bounds(world_pos, world_size)) { return true; }

    let chunk_coords = world_to_chunk(world_pos);
    let indir_idx = chunk_to_indir_idx(chunk_coords, gs);

    return indirection[indir_idx] == 0u;
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

    // DDA algorithm
    var state: DDAState;
    state.grid_pos = floor(ray.origin); // Ex: pos = 2.6 -> We are in box 2
    state.t_dist = abs(1.0/(ray.dir + EPSILON )); // Distance t between two voxel boundaries for each axis

    state.step_dir = sign(ray.dir); // Step is -1 (dir <= 0) or 1 (dir > 0) for each axis
    var next_boundary = state.grid_pos + max(state.step_dir, vec3<f32>(0.0));
    state.t_max = abs((next_boundary - ray.origin) / ray.dir); // Distance along the ray until the next boundary for each axis

    let max_steps = i32(mix(128.0, 512.0, 1.0 - abs(ray.dir.y))); // Increase max step if we are looking horizontaly

    for (var i = 0; i < max_steps; i++) {
        let step_mask = vec3<f32>(
            f32(state.t_max.x <= min(state.t_max.y, state.t_max.z)),
            f32(state.t_max.y < min(state.t_max.x, state.t_max.z)),
            f32(state.t_max.z < min(state.t_max.x, state.t_max.y))
        );

        state.t_max += step_mask * state.t_dist;
        state.grid_pos += vec3<f32>(step_mask) * state.step_dir;

        let gs = i32(world_params.grid_size);
        let world_size = gs * CHUNK_SIZE;

        if (is_out_of_bounds(vec3<i32>(state.grid_pos), world_size)) {
            break;
        }

        if (inside_empty_chunk(vec3<i32>(state.grid_pos))) {
            let chunk_origin = floor(state.grid_pos / f32(CHUNK_SIZE)) * f32(CHUNK_SIZE);
            let chunk_exit = chunk_origin + max(state.step_dir, vec3(0.0)) * f32(CHUNK_SIZE);
            let dist_to_exit = (chunk_exit - ray.origin) / ray.dir;
            let t_skip = min(min(dist_to_exit.x, dist_to_exit.y), dist_to_exit.z);

            state.grid_pos = floor(ray.origin + ray.dir * (t_skip + EPSILON));
            next_boundary = state.grid_pos + max(state.step_dir, vec3(0.0));
            state.t_max = abs((next_boundary - ray.origin) / ray.dir);
        } else {
            let voxel_color = get_voxel_color(vec3<i32>(state.grid_pos));

            if (voxel_color.a != 0.0) {
                // Diffuse light
                let normal = -step_mask * state.step_dir;
                let l = max(dot(SUN_DIR, normal), 0.0); // max to avoid negative light
                let final_color = (l + AMBIENT_LIGHT) * voxel_color.rgb;
                return vec4<f32>(final_color, 1.0);
            }
        }
    }

    return vec4<f32>(get_env_color(ray), 1.0);
}