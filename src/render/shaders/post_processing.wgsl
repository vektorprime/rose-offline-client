// Minimal post-processing shader - simplified passthrough

#import bevy_render::view::View

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var input_texture: texture_2d<f32>;
@group(1) @binding(1)
var input_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Fullscreen triangle
    var pos: vec2<f32>;
    if (input.vertex_idx == 0u) { pos = vec2<f32>(-1.0, -1.0); }
    else if (input.vertex_idx == 1u) { pos = vec2<f32>(3.0, -1.0); }
    else { pos = vec2<f32>(-1.0, 3.0); }
    
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = pos * 0.5 + vec2<f32>(0.5, 0.5);
    return out;
}

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // Simple passthrough
    return textureSample(input_texture, input_sampler, input.uv);
}
