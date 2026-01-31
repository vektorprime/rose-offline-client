// Minimal world UI shader - simplified for stability
#import bevy_render::view View

@group(0) @binding(0)
var<uniform> view: View;

struct Vertex {
    @location(0) world_position: vec3<f32>,
    @location(1) screen_offset: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    var clip_pos = view.view_proj * vec4<f32>(vertex.world_position, 1.0);
    clip_pos = clip_pos / clip_pos.w;
    clip_pos.x = clip_pos.x + (vertex.screen_offset.x * 2.0 / view.viewport.z);
    clip_pos.y = clip_pos.y + (vertex.screen_offset.y * 2.0 / view.viewport.w);

    out.clip_position = clip_pos;
    out.uv = vertex.uv;
    out.color = vertex.color;
    return out;
}

@group(1) @binding(0)
var base_texture: texture_2d<f32>;
@group(1) @binding(1)
var base_sampler: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    return textureSample(base_texture, base_sampler, in.uv) * in.color;
}
