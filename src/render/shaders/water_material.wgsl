// Water material shader with single texture binding
// Updated for Bevy 0.14 (replaced binding array with single texture)

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::{mesh_position_local_to_world, mesh_normal_local_to_world, get_world_from_local}

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex, @builtin(instance_index) instance_index: u32) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = get_world_from_local(instance_index);
    out.world_position = mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal, instance_index);
    out.uv0 = vertex.uv0;

    out.clip_position = view.clip_from_world * out.world_position;
    return out;
}

// Single texture binding for water
@group(2) @binding(0) var water_texture: texture_2d<f32>;
@group(2) @binding(1) var water_sampler: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    // Sample water texture
    let water_color = textureSample(water_texture, water_sampler, in.uv0);
    
    // FIXED: Simple ambient lighting instead of zone_lighting (not available in standard Material)
    let ambient = vec3<f32>(0.7, 0.7, 0.7);
    return vec4<f32>(water_color.rgb * ambient, water_color.a);
}
