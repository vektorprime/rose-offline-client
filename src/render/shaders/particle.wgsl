#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_world}

@group(1) @binding(0)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(1)
var base_color_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vertex(model: VertexInput) -> VertexOutput {
    let model_matrix = get_model_matrix(model.instance_index);
    
    var out: VertexOutput;
    out.position = view.view_proj * mesh_position_local_to_world(model_matrix, vec4<f32>(model.position, 1.0));
    out.color = model.color;
    out.uv = model.uv;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(base_color_texture, base_color_sampler, in.uv);
}
