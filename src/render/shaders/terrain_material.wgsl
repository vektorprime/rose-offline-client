//! Terrain material shader for ROSE Online
//!
//! Simplified version for debugging - renders with basic lighting

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings view

// Vertex input structure
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,      // Lightmap UVs
    @location(3) uv1: vec2<f32>,      // Tile texture UVs
    @location(4) tile_info: u32,      // Packed tile info
}

// Vertex output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
}

// Terrain material bind group (group 2 - material bindings)
@group(2) @binding(0)
var tile_array_texture: binding_array<texture_2d<f32>, 100>;
@group(2) @binding(1)
var tile_array_sampler: sampler;

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let world_from_local = get_world_from_local(vertex.instance_index);
    
    out.clip_position = mesh_position_local_to_clip(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.world_position = mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    out.world_normal = mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.normal, 0.0),
    ).xyz;
    out.uv0 = vertex.uv0;
    out.uv1 = vertex.uv1;
    out.tile_info = vertex.tile_info;
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Unpack tile info
    let tile_layer1_id: u32 = (in.tile_info) & 0xffu;
    let tile_layer2_id: u32 = (in.tile_info >> 8u) & 0xffu;
    let tile_rotation: u32 = (in.tile_info >> 16u) & 0xffu;

    // Sample first layer texture
    let layer1 = textureSample(tile_array_texture[tile_layer1_id], tile_array_sampler, in.uv1);
    
    // Apply simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let normal = normalize(in.world_normal);
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.5 + 0.5;

    let final_color = vec4<f32>(layer1.rgb * diffuse, 1.0);

    return final_color;
}
