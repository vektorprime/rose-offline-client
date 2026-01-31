// Simplified terrain material shader using Bevy's standard Material pipeline

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::{mesh_normal_local_to_world, mesh_position_local_to_world, mesh_position_local_to_clip, get_model_matrix}

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
    out.clip_position = mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv = vertex.uv;
    return out;
}

// Simplified: use individual texture bindings instead of arrays
@group(2) @binding(0)
var texture0: texture_2d<f32>;
@group(2) @binding(1)
var sampler0: sampler;

@group(2) @binding(2)
var texture1: texture_2d<f32>;
@group(2) @binding(3)
var sampler1: sampler;

@group(2) @binding(4)
var texture2: texture_2d<f32>;
@group(2) @binding(5)
var sampler2: sampler;

@group(2) @binding(6)
var texture3: texture_2d<f32>;
@group(2) @binding(7)
var sampler3: sampler;

@group(2) @binding(8)
var detail_texture: texture_2d<f32>;
@group(2) @binding(9)
var detail_sampler: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
};

@fragment
fn fragment(input: FragmentInput) -> @location(0) vec4<f32> {
    // Simple checkerboard pattern to blend textures based on UV coordinates
    let uv_tile = fract(input.uv * 2.0);
    let tile_x = u32(uv_tile.x > 0.5);
    let tile_y = u32(uv_tile.y > 0.5);
    let tile_index = (tile_y * 2u + tile_x) % 4u;
    
    // Sample from the appropriate texture
    var color: vec4<f32>;
    switch tile_index {
        case 0u: {
            color = textureSample(texture0, sampler0, input.uv * 4.0);
        }
        case 1u: {
            color = textureSample(texture1, sampler1, input.uv * 4.0);
        }
        case 2u: {
            color = textureSample(texture2, sampler2, input.uv * 4.0);
        }
        case 3u: {
            color = textureSample(texture3, sampler3, input.uv * 4.0);
        }
        default: {
            color = textureSample(texture0, sampler0, input.uv * 4.0);
        }
    }
    
    // Add detail texture
    let detail = textureSample(detail_texture, detail_sampler, input.uv * 8.0);
    color = vec4<f32>(color.rgb * detail.rgb, color.a);

    // Simple ambient lighting
    let ambient = vec3<f32>(0.5, 0.5, 0.5);
    color = vec4<f32>(color.rgb * ambient, color.a);

    return color;
}
