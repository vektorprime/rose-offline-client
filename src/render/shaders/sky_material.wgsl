// Minimal sky material shader - simplified for stability
#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_functions get_model_matrix

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    let model = get_model_matrix(vertex.instance_index);
    let pos = view.view_proj * model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = pos.xyww;
    out.uv = vertex.uv;
    return out;
}

@group(2) @binding(0)
var sky_texture_day: texture_2d<f32>;
@group(2) @binding(1)
var sky_sampler_day: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Simple texture sampling only - no blending
    return textureSample(sky_texture_day, sky_sampler_day, in.uv);
}
