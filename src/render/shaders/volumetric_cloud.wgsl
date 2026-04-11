//! 3D Volumetric Cloud Shader for Bevy 0.18.1
//!
//! Renders fluffy cumulus-style 3D clouds using:
//! - Volumetric sphere rendering
//! - Multi-layered fBm noise for cumulus shape
//! - Noise-based radius deformation for puffy appearance
//! - Time-based wind drift animation
//! - Time-of-day lighting integration

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings view

struct CloudUniforms {
    data: array<vec4<f32>, 6>,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> cloud_uniforms: CloudUniforms;

fn cloud_time() -> f32 { return cloud_uniforms.data[0].x; }
fn cloud_density_value() -> f32 { return cloud_uniforms.data[0].y; }
fn cloud_opacity() -> f32 { return cloud_uniforms.data[0].z; }
fn cloud_brightness() -> f32 { return cloud_uniforms.data[0].w; }
fn cloud_noise_scale() -> f32 { return cloud_uniforms.data[1].x; }
fn cloud_noise_octaves() -> f32 { return cloud_uniforms.data[1].y; }
fn cloud_sun_direction() -> vec3<f32> { return cloud_uniforms.data[2].xyz; }
fn cloud_sun_color() -> vec3<f32> { return cloud_uniforms.data[3].xyz; }
fn cloud_ambient_color() -> f32 { return cloud_uniforms.data[4].x; }
fn cloud_ambient_color_g() -> f32 { return cloud_uniforms.data[4].y; }
fn cloud_ambient_color_b() -> f32 { return cloud_uniforms.data[4].z; }
fn cloud_tod_factor() -> f32 { return cloud_uniforms.data[4].w; }
fn cloud_drift_speed() -> vec3<f32> { return cloud_uniforms.data[5].xyz; }

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) local_position: vec3<f32>,
    @location(2) view_direction: vec3<f32>,
    @location(3) cloud_origin: vec3<f32>,
}

fn hash(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

fn hash3(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        hash(p),
        hash(p + vec3<f32>(31.123, 17.456, 23.789)),
        hash(p + vec3<f32>(47.321, 13.654, 29.987)),
    );
}

fn quintic(t: f32) -> f32 {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn quintic3(t: vec3<f32>) -> vec3<f32> {
    return t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
}

fn gradient_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = quintic3(f);
    
    let g000 = normalize(hash3(i + vec3<f32>(0.0, 0.0, 0.0)) * 2.0 - 1.0);
    let g100 = normalize(hash3(i + vec3<f32>(1.0, 0.0, 0.0)) * 2.0 - 1.0);
    let g010 = normalize(hash3(i + vec3<f32>(0.0, 1.0, 0.0)) * 2.0 - 1.0);
    let g110 = normalize(hash3(i + vec3<f32>(1.0, 1.0, 0.0)) * 2.0 - 1.0);
    let g001 = normalize(hash3(i + vec3<f32>(0.0, 0.0, 1.0)) * 2.0 - 1.0);
    let g101 = normalize(hash3(i + vec3<f32>(1.0, 0.0, 1.0)) * 2.0 - 1.0);
    let g011 = normalize(hash3(i + vec3<f32>(0.0, 1.0, 1.0)) * 2.0 - 1.0);
    let g111 = normalize(hash3(i + vec3<f32>(1.0, 1.0, 1.0)) * 2.0 - 1.0);
    
    let d000 = dot(g000, f - vec3<f32>(0.0, 0.0, 0.0));
    let d100 = dot(g100, f - vec3<f32>(1.0, 0.0, 0.0));
    let d010 = dot(g010, f - vec3<f32>(0.0, 1.0, 0.0));
    let d110 = dot(g110, f - vec3<f32>(1.0, 1.0, 0.0));
    let d001 = dot(g001, f - vec3<f32>(0.0, 0.0, 1.0));
    let d101 = dot(g101, f - vec3<f32>(1.0, 0.0, 1.0));
    let d011 = dot(g011, f - vec3<f32>(0.0, 1.0, 1.0));
    let d111 = dot(g111, f - vec3<f32>(1.0, 1.0, 1.0));
    
    return mix(
        mix(mix(d000, d100, u.x), mix(d010, d110, u.x), u.y),
        mix(mix(d001, d101, u.x), mix(d011, d111, u.x), u.y),
        u.z
    );
}

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

/// Create cumulus cloud density with puffy, irregular shape
fn cloud_density(local_pos: vec3<f32>, world_pos: vec3<f32>, cloud_origin: vec3<f32>) -> f32 {
    let drift_offset = cloud_drift_speed() * cloud_time();
    let animated_pos = local_pos + drift_offset;

    // Stable per-cloud seed from cloud world origin.
    let seed = hash(floor(cloud_origin * 0.03125));
    let seed2 = hash(floor(cloud_origin.zyx * 0.0625 + vec3<f32>(7.0, 13.0, 19.0)));
    
    // Get position in unit sphere coordinates
    let radius = length(local_pos);
    
    // Discard if outside sphere (with small margin for noise)
    if (radius > 1.05) {
        return 0.0;
    }
    
    // Direction from center (for noise sampling)
    let direction = local_pos / max(radius, 0.001);

    // Warp sampling direction per cloud to break up repeated spherical patterns.
    let warp_vec = vec3<f32>(seed - 0.5, (seed2 - 0.5) * 0.5, fract(seed * 17.31) - 0.5);
    let warped_direction = normalize(direction + warp_vec * (0.22 + seed * 0.28));
    
    // Multiple noise layers for cumulus shape
    // Low frequency noise for overall cloud shape (large puffy blobs)
    let low_freq_noise = fbm(warped_direction * (1.2 + seed * 0.9) + vec3<f32>(0.0, 0.2, 0.0), 2.0);
    
    // Medium frequency for cloud detail
    let mid_freq_noise = fbm(warped_direction * (2.4 + seed2 * 1.6) + vec3<f32>(0.3, -0.1, 0.2), 3.0);
    
    // High frequency for fine detail
    let high_freq_noise = fbm(warped_direction * (4.8 + seed * 2.8) + vec3<f32>(-0.2, 0.3, -0.1), 2.0);
    
    // Combine noise layers - normalized to [0, 1]
    let low_w = mix(0.40, 0.62, seed);
    let mid_w = mix(0.23, 0.43, seed2);
    let high_w = max(0.08, 1.0 - low_w - mid_w);
    let combined_noise = (low_freq_noise * low_w + mid_freq_noise * mid_w + high_freq_noise * high_w + 1.0) * 0.5;
    
    // Create density threshold based on noise and radius
    // Higher noise = more cloud density at that point
    let noise_threshold = mix(0.34, 0.56, seed);
    let shape_density = smoothstep(noise_threshold - 0.15, noise_threshold + 0.25, combined_noise);
    
    // IMPORTANT:
    // We are shading a sphere surface mesh (not true raymarched volume).
    // Surface fragments are near radius ~1.0, so center-weighted radial falloff
    // would zero out density and make clouds disappear.
    // Use a shell-preserving term that keeps density high near the surface.
    let shell_outer = mix(1.02, 1.08, seed2);
    let shell_inner = mix(0.90, 0.98, seed);
    let shell_density = smoothstep(shell_outer, shell_inner, radius);

    // Combine shape and shell term
    let base_density = shape_density * shell_density;
    
    // Add internal volumetric variation
    let internal_noise = fbm(animated_pos * cloud_noise_scale() * mix(2.3, 4.4, seed2), 2.0);
    let internal_factor = (internal_noise + 1.0) * 0.5;
    
    let density = base_density * (0.7 + 0.6 * internal_factor);
    
    return density * cloud_density_value();
}

fn cloud_lighting(world_pos: vec3<f32>, local_pos: vec3<f32>, view_dir: vec3<f32>, cloud_dens: f32) -> vec3<f32> {
    let sun_direction = cloud_sun_direction();
    let up = vec3<f32>(0.0, 1.0, 0.0);
    
    // Top of cloud gets most direct light
    let sun_dot = max(0.0, dot(up, sun_direction));
    
    // Surface normals
    let normal = normalize(local_pos);
    let sun_dot_surface = max(0.0, dot(normal, sun_direction));
    
    // Cumulus clouds are brightest on top
    let top_brightness = pow(smoothstep(0.0, 1.0, local_pos.y), 0.5);
    
    // Direct sun lighting with emphasis on top
    let direct_light = cloud_sun_color() * (sun_dot * 0.7 + sun_dot_surface * 0.3) * (0.7 + 0.5 * top_brightness);
    
    // Ambient lighting (reduced influence to avoid gray cast)
    let ambient_light = vec3<f32>(cloud_ambient_color(), cloud_ambient_color_g(), cloud_ambient_color_b()) * 0.20;
    
    // Soft shadows on bottom
    let shadow_factor = 1.0 - pow(smoothstep(-1.0, 0.3, local_pos.y), 1.5) * 0.3;
    
    // Rim lighting
    let rim_factor = max(0.0, 1.0 - dot(view_dir, normal));
    let rim_light = cloud_sun_color() * pow(rim_factor, 2.0) * 0.25 * shadow_factor;
    
    let total_light = (direct_light + ambient_light + rim_light) * shadow_factor;

    // Push clouds much whiter while preserving some directional form.
    let lit = total_light * cloud_brightness();
    let white_base = vec3<f32>(1.12, 1.12, 1.10);
    let lit_floor = max(lit, vec3<f32>(1.00, 1.00, 1.00));
    return mix(white_base, lit_floor, 0.25);
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    let world_from_local = get_world_from_local(vertex.instance_index);
    let world_position = mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    
    out.clip_position = mesh_position_local_to_clip(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_position = world_position.xyz;
    out.local_position = vertex.position;
    out.cloud_origin = world_from_local[3].xyz;
    
    let camera_pos = view.world_from_view[3].xyz;
    out.view_direction = normalize(world_position.xyz - camera_pos);
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let cloud_dens = cloud_density(in.local_position, in.world_position, in.cloud_origin);
    
    // Keep enough fill to produce large fluffy silhouettes.
    if (cloud_dens < 0.03) {
        discard;
    }
    
    let cloud_color = cloud_lighting(in.world_position, in.local_position, in.view_direction, cloud_dens);
    
    let tod_mult = max(0.98, cloud_tod_factor());
    let final_color = cloud_color * tod_mult;

    // Return fully opaque cloud fragments (non-transparent look).
    return vec4<f32>(final_color, 1.0);
}
