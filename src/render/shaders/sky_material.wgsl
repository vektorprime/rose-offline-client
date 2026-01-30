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
    let untranslated_inv_view =  mat4x4<f32>(view.inverse_view.x.xyzw,
                                             view.inverse_view.y.xyzw,
                                             view.inverse_view.z.xyzw,
                                             vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let untranslated_proj = view.projection * untranslated_inv_view;
    let model = get_model_matrix(vertex.instance_index);
    let untranslated_model = mat4x4<f32>(model.x.xyzw,
                                         model.y.xyzw,
                                         model.z.xyzw,
                                         vec4<f32>(0.0, 0.0, 0.0, 1.0));
    let pos = untranslated_proj * untranslated_model * vec4<f32>(vertex.position, 1.0);

    var out: VertexOutput;
    out.clip_position = pos.xyww;
    out.uv = vertex.uv;
    return out;
}

@group(2) @binding(0)
var sky_texture_day: texture_2d<f32>;
@group(2) @binding(1)
var sky_sampler_day: sampler;
@group(2) @binding(2)
var sky_texture_night: texture_2d<f32>;
@group(2) @binding(3)
var sky_sampler_night: sampler;

struct ZoneTimePushConstant {
    day_weight: f32,
};
var<push_constant> zone_time: ZoneTimePushConstant;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var color_day: vec4<f32> = textureSample(sky_texture_day, sky_sampler_day, in.uv);
    var color_night: vec4<f32> = textureSample(sky_texture_night, sky_sampler_night, in.uv);
    return vec4<f32>(mix(color_night.xyz, color_day.xyz, zone_time.day_weight), 1.0);
}
