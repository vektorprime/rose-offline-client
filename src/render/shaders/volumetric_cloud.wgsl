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
    @builtin(position) @invariant clip_position: vec4<f32>,
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
    
    // Direction from center (for noise sampling)
    let direction = local_pos / max(length(local_pos), 0.001);

    // Big, smooth billow bumps unique per cloud - push the surface outward
    // into rounded puffs (classic cumulus/cauliflower look).
    let billow = fbm(direction * (1.5 + seed * 1.1) + vec3<f32>(0.0, 0.35, 0.0), 3.0);
    let warp_amp = 0.45 + seed * 0.30;
    let puffy_local = local_pos + direction * (billow - 0.20) * warp_amp;
    let radius = length(puffy_local);

    // Discard if outside sphere (raised cutoff so puffs aren't clipped)
    if (radius > 1.45) {
        return 0.0;
    }

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
    let shape_density = smoothstep(noise_threshold - 0.22, noise_threshold + 0.40, combined_noise);
    
    // IMPORTANT:
    // We are shading a sphere surface mesh (not true raymarched volume).
    // Surface fragments are near radius ~1.0, so center-weighted radial falloff
    // would zero out density and make clouds disappear.
    // Use a shell-preserving term that keeps density high near the surface.
    let shell_outer = mix(1.30, 1.42, seed2);
    let shell_inner = mix(0.75, 0.88, seed) + fbm(direction * 2.0, 2.0) * 0.08;
    let shell_density = smoothstep(shell_outer, shell_inner, radius);

    // Combine shape and shell term
    let base_density = shape_density * shell_density;
    
    // Add internal volumetric variation
    let internal_noise = fbm(animated_pos * cloud_noise_scale() * mix(2.3, 4.4, seed2), 2.0);
    let internal_factor = (internal_noise + 1.0) * 0.5;
    
    let density = base_density * (0.7 + 0.6 * internal_factor);

    // Flatten the underside of each puff so silhouettes read as cumulus
    // (rounded top, nearly flat bottom) instead of plain circles.
    let underside = smoothstep(-0.60, -0.10, local_pos.y);

    // Gentle billowing so puffs swell and shrink slowly.
    let breathe = 0.90 + 0.10 * sin(cloud_time() * 0.13 + seed * 6.28318);
    return density * underside * cloud_density_value() * breathe;
}

fn cloud_lighting(world_pos: vec3<f32>, local_pos: vec3<f32>, view_dir: vec3<f32>, cloud_dens: f32) -> vec3<f32> {
    let sun_direction = normalize(cloud_sun_direction());
    let up = vec3<f32>(0.0, 1.0, 0.0);

    // Surface normal on cloud blob shell.
    let normal = normalize(local_pos);

    // View vector from fragment to camera (for silhouette/rim calculations).
    let to_camera = normalize(-view_dir);
    let ndotv = clamp(dot(normal, to_camera), 0.0, 1.0);
    let rim = 1.0 - ndotv;

    // Toon body shading: soft 3-band quantization (highlight / mid / shade)
    // so clouds keep a clean, cartoon look.
    let top_amount = smoothstep(-0.1, 0.85, local_pos.y);
    let sun_from_above = max(0.0, dot(up, sun_direction));
    let shade = top_amount * 0.75 + sun_from_above * 0.25;
    let band = smoothstep(0.28, 0.34, shade) * 0.06 + smoothstep(0.60, 0.68, shade) * 0.10;
    let toon_light = 0.99 + band;

    // Keep body color close to white regardless of ambient/day-night values,
    // preventing gray shadowing on cloud texture.
    let sun_tint = mix(vec3<f32>(1.0, 1.0, 1.0), cloud_sun_color(), 0.10);
    var body_color = vec3<f32>(toon_light, toon_light, toon_light) * sun_tint * cloud_brightness();

    // White on top, warm cream on the underside (classic cartoon cumulus).
    let vertical_tint = mix(
        vec3<f32>(1.0, 0.96, 0.90),
        vec3<f32>(1.0, 1.0, 1.0),
        smoothstep(0.0, 0.75, local_pos.y),
    );
    body_color *= vertical_tint;

    // Slight internal puff variation while preserving white floor.
    let puff = smoothstep(0.2, 0.9, cloud_dens);
    body_color *= mix(0.95, 1.02, puff);
    body_color = max(body_color, vec3<f32>(0.97, 0.97, 0.97));

    // Cartoon outline band: darkened rim near silhouette.
    let outline_band = smoothstep(0.55, 0.82, rim);
    let outline_strength = outline_band * smoothstep(0.15, 0.75, cloud_dens);
    let outline_color = vec3<f32>(0.50, 0.58, 0.72);

    // Small bright rim accent keeps the outline from looking too flat.
    let rim_highlight = cloud_sun_color() * pow(rim, 2.2) * 0.08;

    return mix(body_color + rim_highlight, outline_color, outline_strength * 0.88);
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
    
    // Soft, noisy silhouette: dither the discard with a static per-pixel hash
    // to avoid aliased edges while keeping a fully opaque look.
    let edge_dither = hash(in.world_position * 0.8 + vec3<f32>(1.7, 3.1, 5.3));
    if (cloud_dens < 0.045 + edge_dither * 0.055) {
        discard;
    }
    
    let cloud_color = cloud_lighting(in.world_position, in.local_position, in.view_direction, cloud_dens);
    
    let tod_mult = max(0.98, cloud_tod_factor());
    let final_color = cloud_color * tod_mult;

    // Return fully opaque cloud fragments (non-transparent look).
    return vec4<f32>(final_color, 1.0);
}
