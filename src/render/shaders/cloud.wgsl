//! Procedural Cloud Shader for Bevy 0.16
//!
//! Renders realistic clouds using:
//! - Fractional Brownian Motion (fBm) noise
//! - Multiple noise octaves for detail
//! - Time-based animation
//! - Time-of-day lighting integration

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings view

// === Uniforms ===
// NOTE: CloudMaterial uploads a single uniform buffer at material binding(0)
// packed as 7 vec4 values (28 floats, 112 bytes). Keep this mapping in sync
// with src/render/cloud_material.rs::as_bind_group().
struct CloudUniforms {
    data: array<vec4<f32>, 7>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> cloud_uniforms: CloudUniforms;

fn cloud_time() -> f32 { return cloud_uniforms.data[0].x; }
fn cloud_speed() -> f32 { return cloud_uniforms.data[0].y; }
fn cloud_wind_direction() -> vec3<f32> {
    return vec3<f32>(cloud_uniforms.data[0].z, cloud_uniforms.data[0].w, cloud_uniforms.data[1].x);
}
fn cloud_density_value() -> f32 { return cloud_uniforms.data[1].y; }
fn cloud_coverage() -> f32 { return cloud_uniforms.data[1].z; }
fn cloud_noise_scale() -> f32 { return cloud_uniforms.data[2].x; }
fn cloud_noise_octaves() -> f32 { return cloud_uniforms.data[2].y; }
fn cloud_brightness() -> f32 { return cloud_uniforms.data[3].x; }
fn cloud_opacity() -> f32 { return cloud_uniforms.data[3].y; }
fn cloud_softness() -> f32 { return cloud_uniforms.data[3].z; }
fn cloud_sun_direction() -> vec3<f32> { return cloud_uniforms.data[4].xyz; }
fn cloud_sun_color() -> vec3<f32> { return cloud_uniforms.data[5].xyz; }
fn cloud_ambient_color() -> vec3<f32> { return cloud_uniforms.data[6].xyz; }
fn cloud_tod_factor() -> f32 { return cloud_uniforms.data[6].w; }

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) view_direction: vec3<f32>,
}

// === Noise Functions ===

/// Hash function for procedural noise
fn hash(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

/// 3D hash returning vec3
fn hash3(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        hash(p),
        hash(p + vec3<f32>(31.123, 17.456, 23.789)),
        hash(p + vec3<f32>(47.321, 13.654, 29.987)),
    );
}

/// Smooth interpolation (quintic) for f32
fn quintic(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

/// Smooth interpolation (quintic) for vec3 - component-wise
fn quintic3(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

/// Perlin-like gradient noise
fn gradient_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = quintic3(f);
    
    // Sample gradients at cube corners
    let g000 = normalize(hash3(i + vec3<f32>(0.0, 0.0, 0.0)) * 2.0 - 1.0);
    let g100 = normalize(hash3(i + vec3<f32>(1.0, 0.0, 0.0)) * 2.0 - 1.0);
    let g010 = normalize(hash3(i + vec3<f32>(0.0, 1.0, 0.0)) * 2.0 - 1.0);
    let g110 = normalize(hash3(i + vec3<f32>(1.0, 1.0, 0.0)) * 2.0 - 1.0);
    let g001 = normalize(hash3(i + vec3<f32>(0.0, 0.0, 1.0)) * 2.0 - 1.0);
    let g101 = normalize(hash3(i + vec3<f32>(1.0, 0.0, 1.0)) * 2.0 - 1.0);
    let g011 = normalize(hash3(i + vec3<f32>(0.0, 1.0, 1.0)) * 2.0 - 1.0);
    let g111 = normalize(hash3(i + vec3<f32>(1.0, 1.0, 1.0)) * 2.0 - 1.0);
    
    // Calculate dot products
    let d000 = dot(g000, f - vec3<f32>(0.0, 0.0, 0.0));
    let d100 = dot(g100, f - vec3<f32>(1.0, 0.0, 0.0));
    let d010 = dot(g010, f - vec3<f32>(0.0, 1.0, 0.0));
    let d110 = dot(g110, f - vec3<f32>(1.0, 1.0, 0.0));
    let d001 = dot(g001, f - vec3<f32>(0.0, 0.0, 1.0));
    let d101 = dot(g101, f - vec3<f32>(1.0, 0.0, 1.0));
    let d011 = dot(g011, f - vec3<f32>(0.0, 1.0, 1.0));
    let d111 = dot(g111, f - vec3<f32>(1.0, 1.0, 1.0));
    
    // Trilinear interpolation
    return mix(
        mix(mix(d000, d100, u.x), mix(d010, d110, u.x), u.y),
        mix(mix(d001, d101, u.x), mix(d011, d111, u.x), u.y),
        u.z
    );
}

/// Fractional Brownian Motion (fBm) for cloud detail
/// Combines multiple octaves of noise at different frequencies
fn fbm(p: vec3<f32>, octaves: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var max_value = 0.0;
    
    let octave_count = i32(octaves + 0.5);
    
    for (var i = 0; i < octave_count; i++) {
        value += amplitude * gradient_noise(p * frequency);
        max_value += amplitude;
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    return value / max_value;
}

/// Cloud density function
/// Returns density value (0-1) at a given position
fn cloud_density(p: vec3<f32>) -> f32 {
    // Animate position with wind
    let animated_p = p + cloud_wind_direction() * cloud_time() * cloud_speed();
    
    // Scale for cloud features
    let scaled_p = animated_p * cloud_noise_scale() * 0.001;
    
    // Base cloud shape with fBm (returns values roughly in [-1, 1] range)
    let base = fbm(scaled_p, cloud_noise_octaves());
    
    // Add larger-scale coverage variation
    let coverage_noise = fbm(scaled_p * 0.3, 2.0);
    
    // Combine for final density - fBm returns [-1,1], remap to [0,1]
    // This fixes the bug where noise centered at 0 was compared against positive threshold
    let raw_density = (base * 0.6 + coverage_noise * 0.4) * 0.5 + 0.5;
    
    // Apply coverage threshold
    // coverage = 0.0 means clear sky (high threshold), 1.0 means overcast (low threshold)
    // Invert coverage so higher coverage = more clouds
    let threshold = 1.0 - cloud_coverage();
    let cloud_value = smoothstep(threshold - 0.15, threshold + 0.35, raw_density);
    
    // Apply density multiplier
    return cloud_value * cloud_density_value();
}

/// Calculate cloud lighting
/// Returns (lit_color, shadow_factor)
fn cloud_lighting(
    world_pos: vec3<f32>,
    view_dir: vec3<f32>,
    cloud_dens: f32,
) -> vec3<f32> {
    // Sun lighting with directional component
    let sun_direction = cloud_sun_direction();
    let sun_dot = max(0.0, dot(vec3<f32>(0.0, 1.0, 0.0), sun_direction));
    
    // Direct sun lighting (brighter on sun-facing side)
    let direct_light = cloud_sun_color() * sun_dot * 1.5;
    
    // Ambient lighting (sky color)
    let ambient_light = cloud_ambient_color() * 0.5;
    
    // Edge lighting effect (clouds glow at edges when backlit)
    let edge_factor = 1.0 - cloud_dens;
    let rim_light = cloud_sun_color() * pow(edge_factor, 2.0) * max(0.0, -dot(view_dir, sun_direction)) * 0.5;
    
    // Combine lighting
    let total_light = direct_light + ambient_light + rim_light;
    
    // Apply brightness multiplier
    return total_light * cloud_brightness();
}

// === Vertex Shader ===
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let world_from_local = get_world_from_local(vertex.instance_index);
    let world_position = mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    
    out.clip_position = mesh_position_local_to_clip(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_position = world_position.xyz;
    
    // View direction from camera
    let camera_pos = view.world_from_view[3].xyz;
    out.view_direction = normalize(world_position.xyz - camera_pos);
    
    return out;
}

// === Fragment Shader ===
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Get cloud density at this position
    let cloud_dens = cloud_density(in.world_position);
    
    // Discard if no cloud - use very low threshold for debugging
    if (cloud_dens < 0.001) {
        discard;
    }
    
    // Apply softness (feathering at edges)
    let soft_dens = cloud_dens * (1.0 - cloud_softness() * 0.5);
    
    // Calculate lighting
    let cloud_color = cloud_lighting(in.world_position, in.view_direction, cloud_dens);
    
    // Apply time-of-day factor
    let final_color = cloud_color * cloud_tod_factor();
    
    // Final alpha with opacity multiplier - boost for visibility
    let alpha = soft_dens * cloud_opacity() * cloud_tod_factor() * 2.0;
    
    return vec4<f32>(final_color, alpha);
}
