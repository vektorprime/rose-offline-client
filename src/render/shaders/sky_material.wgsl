// Sky material shader with day/night blending
// Restored from Bevy 0.11 with updated 0.14 syntax

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::get_world_from_local

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    // RESTORED: Proper sky rendering without translation (sky stays at camera position)
    let untranslated_inv_view =  mat4x4<f32>(view.view_from_world[0].xyzw,
                                             view.view_from_world[1].xyzw,
                                             view.view_from_world[2].xyzw,
                                             vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let untranslated_proj = view.clip_from_view * untranslated_inv_view;
    let untranslated_model = mat4x4<f32>(mesh.world_from_local[0].xyzw,
                                         mesh.world_from_local[1].xyzw,
                                         mesh.world_from_local[2].xyzw,
                                         vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let pos = untranslated_proj * untranslated_model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = pos.xyww;
    out.uv = vertex.uv;
    return out;
}

// RESTORED: Day and night texture bindings
@group(2) @binding(0)
var sky_texture_day: texture_2d<f32>;
@group(2) @binding(1)
var sky_sampler_day: sampler;
@group(2) @binding(2)
var sky_texture_night: texture_2d<f32>;
@group(2) @binding(3)
var sky_sampler_night: sampler;
@group(2) @binding(4)
var<uniform> day_weight: f32;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // RESTORED: Day/night texture blending
    var color_day: vec4<f32> = textureSample(sky_texture_day, sky_sampler_day, in.uv);
    var color_night: vec4<f32> = textureSample(sky_texture_night, sky_sampler_night, in.uv);
    return vec4<f32>(mix(color_night.xyz, color_day.xyz, day_weight), 1.0);
}
