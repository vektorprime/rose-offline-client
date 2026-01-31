// Simplified effect mesh material shader using Bevy's standard Material pipeline

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::{mesh_normal_local_to_world, mesh_position_local_to_world, get_model_matrix}

struct EffectMeshMaterialData {
    flags: u32,
    alpha_cutoff: f32,
    _padding: vec2<f32>,
};

const EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32 = 0x1u;
const EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32 = 0x2u;

@group(2) @binding(0)
var<uniform> material: EffectMeshMaterialData;
@group(2) @binding(1)
var base_texture: texture_2d<f32>;
@group(2) @binding(2)
var base_sampler: sampler;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let model = get_model_matrix(vertex.instance_index);

    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    out.clip_position = view.view_proj * out.world_position;
    return out;
}

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);

    // Simple ambient lighting (hardcoded since we removed zone lighting)
    let ambient = vec3<f32>(0.6, 0.6, 0.6);
    output_color = vec4<f32>(output_color.rgb * ambient, output_color.a);

    // Alpha masking
    if ((material.flags & EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        if (output_color.a < material.alpha_cutoff) {
            discard;
        }
        output_color.a = 1.0;
    } else if ((material.flags & EFFECT_MESH_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
        output_color.a = 1.0;
    }

    return output_color;
}
