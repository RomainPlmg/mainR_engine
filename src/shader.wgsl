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

    return vec4<f32>(ray.dir * 0.5 + 0.5, 1.0);
}