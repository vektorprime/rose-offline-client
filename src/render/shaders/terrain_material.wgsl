// Terrain material shader with individual texture bindings
// Updated for Bevy 0.14 (replaced binding arrays with individual textures)
// FIXED: Removed zone_lighting dependency - not compatible with standard Material trait

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::{mesh_normal_local_to_world, mesh_position_local_to_world, mesh_position_local_to_clip, get_world_from_local}
#import bevy_pbr::shadows::fetch_directional_shadow

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,      // RESTORED: uv1 for tile mapping
    @location(4) tile_info: u32,      // RESTORED: tile_info for layer selection
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
};

@vertex
fn vertex(vertex: Vertex, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // FIXED: Use proper Bevy 0.14 mesh functions with instance_index
    let world_from_local = get_world_from_local(instance_index);
    out.world_position = mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.clip_position = view.clip_from_world * out.world_position;
    out.world_normal = mesh_normal_local_to_world(vertex.normal, instance_index);
    out.uv0 = vertex.uv0;
    out.uv1 = vertex.uv1;
    out.tile_info = vertex.tile_info;
    return out;
}

// Individual texture bindings for tile textures (5 tiles)
// FIXED: Changed from @group(1) to @group(2) for Bevy 0.14 Material trait compatibility
@group(2) @binding(0) var tile_0_texture: texture_2d<f32>;
@group(2) @binding(1) var tile_0_sampler: sampler;
@group(2) @binding(2) var tile_1_texture: texture_2d<f32>;
@group(2) @binding(3) var tile_1_sampler: sampler;
@group(2) @binding(4) var tile_2_texture: texture_2d<f32>;
@group(2) @binding(5) var tile_2_sampler: sampler;
@group(2) @binding(6) var tile_3_texture: texture_2d<f32>;
@group(2) @binding(7) var tile_3_sampler: sampler;
@group(2) @binding(8) var tile_4_texture: texture_2d<f32>;
@group(2) @binding(9) var tile_4_sampler: sampler;
@group(2) @binding(10) var<uniform> _padding: vec4<f32>;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
};

// Helper function to sample the correct tile texture based on index
fn sample_tile(tile_id: u32, uv: vec2<f32>) -> vec4<f32> {
    if (tile_id == 0u) {
        return textureSample(tile_0_texture, tile_0_sampler, uv);
    } else if (tile_id == 1u) {
        return textureSample(tile_1_texture, tile_1_sampler, uv);
    } else if (tile_id == 2u) {
        return textureSample(tile_2_texture, tile_2_sampler, uv);
    } else if (tile_id == 3u) {
        return textureSample(tile_3_texture, tile_3_sampler, uv);
    } else {
        return textureSample(tile_4_texture, tile_4_sampler, uv);
    }
}

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let view_z = dot(vec4<f32>(
        view.world_from_view[0].z,
        view.world_from_view[1].z,
        view.world_from_view[2].z,
        view.world_from_view[3].z
    ), in.world_position);

    var tile_layer1_id: u32 = (in.tile_info) & 0xffu;
    var tile_layer2_id: u32 = (in.tile_info >> 8u) & 0xffu;
    var tile_rotation: u32 = (in.tile_info >> 16u) & 0xffu;
    var layer2_uv: vec2<f32> = in.uv1;
    if (tile_rotation == 2u) {
        layer2_uv.x = 1.0 - layer2_uv.x;
    } else if (tile_rotation == 3u) {
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 4u) {
        layer2_uv.x = 1.0 - layer2_uv.x;
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 5u) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = 1.0 - x;
    } else if (tile_rotation == 6u) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = x;
    }

    let layer1 = sample_tile(tile_layer1_id, in.uv1);
    let layer2 = sample_tile(tile_layer2_id, layer2_uv);
    var lightmap = sample_tile(0u, in.uv0);
    let shadow = fetch_directional_shadow(0u, in.world_position, in.world_normal, view_z);
    lightmap = vec4<f32>(lightmap.xyz * (shadow * 0.2 + 0.8), lightmap.w);

    let terrain_color = mix(layer1, layer2, layer2.a) * lightmap * 2.0;

    // FIXED: Simple ambient lighting instead of zone_lighting (not available in standard Material)
    let ambient = vec3<f32>(0.7, 0.7, 0.7);
    return vec4<f32>(terrain_color.rgb * ambient, 1.0);
}
