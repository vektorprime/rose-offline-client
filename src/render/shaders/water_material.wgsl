//! Water material shader for ROSE Online
//!
//! Supports animated water with:
//! - 25 water animation frames in a binding_array
//! - Time-based frame blending for smooth animation
//! - Additive blending for water transparency effect
//! - Simple lighting for water surface

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::{view, globals}

// Vertex input structure
struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
}

// Vertex output structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
}

// Water material bind group (group 2 - material bindings)
// Texture array with 25 water animation frames
@group(2) @binding(0)
var water_array_texture: binding_array<texture_2d<f32>, 25>;
@group(2) @binding(1)
var water_array_sampler: sampler;

// Animation uniforms passed from CPU
// Using view built-in time for animation
struct WaterAnimationData {
    current_index: i32,
    next_index: i32,
    blend: f32,
}

// Use view.transmittance_lut to get time value for animation
// This is a workaround - we'll compute animation in shader using view time

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
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Calculate view-space Z for fog (using view_from_world which is the inverse view matrix)
    let view_z = dot(vec4<f32>(
        view.view_from_world[0].z,
        view.view_from_world[1].z,
        view.view_from_world[2].z,
        view.view_from_world[3].z
    ), in.world_position);
    
    // Animate water at 10 FPS (same as original)
    // globals.time is in seconds, wrap it to avoid precision issues
    let time = globals.time * 10.0;
    let frame_time = fract(time / 25.0) * 25.0;
    let current_index = i32(floor(frame_time)) % 25;
    let next_index = (current_index + 1) % 25;
    let blend = fract(frame_time);
    
    // Sample current and next frame for smooth animation
    let color1 = textureSample(water_array_texture[current_index], water_array_sampler, in.uv0);
    let color2 = textureSample(water_array_texture[next_index], water_array_sampler, in.uv0);
    
    // Blend between frames
    let water_color = mix(color1, color2, blend);
    
    // Apply simple lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let normal = normalize(in.world_normal);
    let diffuse = max(dot(normal, light_dir), 0.0) * 0.5 + 0.5;
    
    // Final color with additive blending (handled by blend state in material)
    let final_color = vec4<f32>(water_color.rgb * diffuse, water_color.a);
    
    return final_color;
}
