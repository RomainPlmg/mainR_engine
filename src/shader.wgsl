// Vertex shader
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

struct CameraUniform {
    inv_view_proj: mat4x4<f32>,
    origin: vec3<f32>,
};

struct Ray {
    origin: vec3<f32>,
    dir: vec3<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<storage, read> voxels: array<u32>;

fn get_env_color(ray: Ray) -> vec3<f32> {
    let sky_gradient = ray.dir.y + 1;
    return mix(vec3<f32>(1.0), vec3<f32>(0.5, 0.7, 1.0), sky_gradient);
}

const GRID_SIZE: u32 = 32u;

fn get_voxel(p: vec3<i32>) -> u32 {
    if (p.x < 0 || p.x >= i32(GRID_SIZE) || 
        p.y < 0 || p.y >= i32(GRID_SIZE) || 
        p.z < 0 || p.z >= i32(GRID_SIZE)) {
        return 0u;
    }
    
    // Index 1D : x + y*L + z*L*L
    let index = u32(p.x) + u32(p.y) * GRID_SIZE + u32(p.z) * GRID_SIZE * GRID_SIZE;
    return voxels[index];
}

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

// Fragment shader
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
    var grid_pos = floor(ray.origin); // Ex: pos = 2.6 -> We are in box 2
    var t_dist = abs(1.0/(ray.dir + 0.00001 )); /* If t_dist.x = 0.1, we travel 10cm on x component for each block,
                                                                * so need 10 blocks to intersect the X axis (1/0.1 = 10) 
                                                                * 0.00001 -> Avoid division by 0
                                                                */
    var step = sign(ray.dir); // Step is -1 (dir < 0) or 1 (dir > 0)
    var next_boundary = grid_pos + step * 0.5 + 0.5;
    var t_max = abs((next_boundary - ray.origin) / ray.dir);

    let cube = vec3<f32>(0.0, 0.0, 2.0);

    for (var i = 0; i < 128; i++) {
        let step_mask = vec3<f32>(
            f32(t_max.x <= min(t_max.y, t_max.z)),
            f32(t_max.y < min(t_max.x, t_max.z)),
            f32(t_max.z < min(t_max.x, t_max.y))
        );

        t_max += step_mask * t_dist;
        grid_pos += vec3<f32>(step_mask) * step;

        let voxel = get_voxel(vec3<i32>(grid_pos));

        if (voxel != 0u) {
            return vec4<f32>(ray.dir * 0.5 + 0.5, 1.0);
        }
    }

    return vec4<f32>(get_env_color(ray), 1.0);
}