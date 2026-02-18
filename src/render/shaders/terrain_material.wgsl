//! Terrain material shader for ROSE Online
//!
//! Supports texture splatting with:
//! - Up to 100 tile textures in a binding_array
//! - Per-vertex tile_info for texture selection and rotation
//! - Two-layer blending with alpha
//! - Lightmap support via UV0

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings view

// Vertex input structure
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,      // Lightmap UVs
    @location(3) uv1: vec2<f32>,      // Tile texture UVs
    @location(4) tile_info: u32,      // Packed tile info: layer1 | layer2 << 8 | rotation << 16
}

// Vertex output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,      // Lightmap UVs
    @location(3) uv1: vec2<f32>,      // Tile texture UVs
    @location(4) tile_info: u32,      // Packed tile info
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

// Apply rotation to UV coordinates based on tile_info rotation value
fn apply_rotation(uv: vec2<f32>, rotation: u32) -> vec2<f32> {
    var result = uv;
    if (rotation == 2u) {
        // Flip horizontal
        result.x = 1.0 - result.x;
    } else if (rotation == 3u) {
        // Flip vertical
        result.y = 1.0 - result.y;
    } else if (rotation == 4u) {
        // Flip both
        result.x = 1.0 - result.x;
        result.y = 1.0 - result.y;
    } else if (rotation == 5u) {
        // Rotate 90 degrees clockwise
        let x = result.x;
        result.x = result.y;
        result.y = 1.0 - x;
    } else if (rotation == 6u) {
        // Rotate 90 degrees counter-clockwise
        let x = result.x;
        result.x = 1.0 - result.y;
        result.y = x;
    }
    // rotation 0 or 1: no change
    return result;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Unpack tile info
    let tile_layer1_id: u32 = (in.tile_info) & 0xffu;
    let tile_layer2_id: u32 = (in.tile_info >> 8u) & 0xffu;
    let tile_rotation: u32 = (in.tile_info >> 16u) & 0xffu;

    // Apply rotation to layer2 UVs
    var layer2_uv: vec2<f32> = in.uv1;
    layer2_uv = apply_rotation(layer2_uv, tile_rotation);

    // Sample both texture layers
    let layer1 = textureSample(tile_array_texture[tile_layer1_id], tile_array_sampler, in.uv1);
    let layer2 = textureSample(tile_array_texture[tile_layer2_id], tile_array_sampler, layer2_uv);

    // Blend layers using layer2 alpha
    // layer2.a determines how much of layer2 to show (0 = all layer1, 1 = all layer2)
    let terrain_color = mix(layer1, layer2, layer2.a);

    // Apply simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let normal = normalize(in.world_normal);
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.5 + 0.5;

    let final_color = vec4<f32>(terrain_color.rgb * diffuse, 1.0);

    return final_color;
}
