//! Procedural Starry Sky Shader for Bevy 0.16
//!
//! This shader renders a dense, high-detail star field with:
//! - Multiple star layers (distant, medium, bright stars)
//! - Procedural star generation using hash functions
//! - Twinkling animation
//! - Moon rendering with phase support
//! - Night-time visibility controlled by time uniform
//!
//! IMPORTANT: Stars are ONLY visible at night (night_factor > 0.5)
//! During the day, the normal Bevy Atmosphere sky is visible instead.
//!
//! DEBUG MODE: Set DEBUG_MODE to 1-5 to enable visual debugging:
//!   1 = YELLOW: Shader is executing (should always show at night)
//!   2 = RED gradient based on night_factor (0=black, 1=red)
//!   3 = GREEN gradient based on star calculation
//!   4 = BLUE gradient based on dir.y (horizon check)
//!   5 = Normal rendering with extra brightness

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings view

// DEBUG MODE: Set to 1-5 for visual debugging, 0 for normal rendering
// DEBUG 1 = YELLOW: Verify shader is executing (should always show at night)
// DEBUG 2 = RED gradient based on night_factor
// DEBUG 3 = GREEN gradient based on star calculation
// DEBUG 4 = BLUE gradient based on dir.y (horizon check)
// DEBUG 5 = Normal rendering with extra brightness
const DEBUG_MODE: i32 = 0;

// Uniforms for time and star settings - must match AsBindGroup bindings
@group(2) @binding(0) var<uniform> time: f32;
@group(2) @binding(1) var<uniform> star_density: f32;
@group(2) @binding(2) var<uniform> star_brightness: f32;
@group(2) @binding(3) var<uniform> night_factor: f32;  // 0.0 = day, 1.0 = night
@group(2) @binding(4) var<uniform> moon_phase: f32;    // 0.0 to 1.0
@group(2) @binding(5) var<uniform> moon_direction: vec3<f32>;  // Normalized moon direction

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
}

/// Hash function for procedural generation
/// Returns a pseudo-random value between 0 and 1
fn hash3(p: vec3<f32>) -> f32 {
    var p3 = fract(p * 0.1031);
    p3 += dot(p3, p3.zyx + 31.32);
    return fract((p3.x + p3.y) * p3.z);
}

/// Hash function returning vec3
fn hash3v(p: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(
        hash3(p),
        hash3(p + vec3<f32>(31.123, 17.456, 23.789)),
        hash3(p + vec3<f32>(47.321, 13.654, 29.987)),
    );
}

/// 3D noise function for subtle variations
fn noise3(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 0.0)), hash3(i + vec3<f32>(1.0, 0.0, 0.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 0.0)), hash3(i + vec3<f32>(1.0, 1.0, 0.0)), u.x),
            u.y
        ),
        mix(
            mix(hash3(i + vec3<f32>(0.0, 0.0, 1.0)), hash3(i + vec3<f32>(1.0, 0.0, 1.0)), u.x),
            mix(hash3(i + vec3<f32>(0.0, 1.0, 1.0)), hash3(i + vec3<f32>(1.0, 1.0, 1.0)), u.x),
            u.y
        ),
        u.z
    );
}

/// Generate a single star layer
/// scale: controls star density (higher = more dense stars, smaller apparent size)
/// brightness_base: base brightness multiplier
/// twinkle_speed: how fast stars twinkle
fn star_layer(dir: vec3<f32>, scale: f32, brightness_base: f32, twinkle_speed: f32) -> f32 {
    // Grid-based star placement
    let p = dir * scale;
    let grid_id = floor(p);
    let grid_fract = fract(p);
    
    var star_brightness = 0.0;
    
    // Check neighboring cells for stars
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell_offset = vec3<f32>(f32(x), f32(y), f32(z));
                let cell_id = grid_id + cell_offset;
                
                // Random star position within cell
                let rand_vals = hash3v(cell_id);
                let star_pos = rand_vals.xyz;
                
                // Star exists based on density threshold
                let star_exists = step(rand_vals.x, star_density);
                
                // Distance from current pixel to star position
                let diff = grid_fract - cell_offset - star_pos;
                let dist = length(diff);
                
                // Star size varies based on random value
                let star_size = 0.02 + rand_vals.y * 0.03;
                
                // Star intensity with smooth falloff
                let intensity = smoothstep(star_size, 0.0, dist);
                
                // Twinkling effect
                let twinkle_phase = rand_vals.z * 6.28318;
                let twinkle = 0.7 + 0.3 * sin(time * twinkle_speed + twinkle_phase);
                
                // Star color temperature variation (slight blue/white/yellow tint)
                star_brightness += intensity * star_exists * brightness_base * twinkle;
            }
        }
    }
    
    return star_brightness;
}

/// Generate nebula-like background for depth
fn nebula_background(dir: vec3<f32>) -> vec3<f32> {
    // Multiple octaves of noise for nebula effect
    let n1 = noise3(dir * 3.0) * 0.5;
    let n2 = noise3(dir * 6.0 + vec3<f32>(100.0, 0.0, 0.0)) * 0.25;
    let n3 = noise3(dir * 12.0 + vec3<f32>(0.0, 100.0, 0.0)) * 0.125;
    
    let nebula = n1 + n2 + n3;
    
    // Subtle blue/purple nebula colors
    let nebula_color = mix(
        vec3<f32>(0.02, 0.02, 0.05),  // Dark blue
        vec3<f32>(0.04, 0.02, 0.06),  // Purple tint
        nebula
    );
    
    return nebula_color * 0.3;
}

/// Render the moon as a bright disc with phase
fn render_moon(dir: vec3<f32>) -> vec3<f32> {
    // Moon angular size (approximately 0.5 degrees in radians, scaled for visibility)
    let moon_angular_size = 0.05;
    
    // Distance from moon center
    let moon_dist = length(dir - moon_direction);
    
    // Moon disc with soft edge
    let moon_disc = smoothstep(moon_angular_size, moon_angular_size * 0.8, moon_dist);
    
    if (moon_disc < 0.01) {
        return vec3<f32>(0.0);
    }
    
    // Moon surface detail using noise
    let moon_noise = noise3(dir * 50.0) * 0.1 + 0.9;
    
    // Moon phase (simplified)
    // 0.0 = new moon, 0.5 = full moon, 1.0 = new moon
    let phase_angle = moon_phase * 6.28318;
    let phase_factor = cos(phase_angle) * 0.5 + 0.5;
    
    // Moon color (slightly warm white)
    let moon_color = vec3<f32>(1.0, 0.98, 0.95);
    
    // Moon glow
    let glow_size = moon_angular_size * 3.0;
    let glow = smoothstep(glow_size, moon_angular_size, moon_dist) * 0.3;
    
    return moon_color * (moon_disc * moon_noise * phase_factor + glow);
}

/// Check if direction is facing away from sun (night side)
/// Returns higher values for directions away from sun
fn get_night_visibility(dir: vec3<f32>) -> f32 {
    // The night_factor uniform controls overall visibility
    // We also fade stars near the horizon during day
    let horizon_factor = 1.0 - smoothstep(0.0, 0.2, abs(dir.y));
    
    return night_factor * (1.0 - horizon_factor * (1.0 - night_factor));
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // Get the world-from-local transform matrix using Bevy 0.16.1 API
    let world_from_local = get_world_from_local(vertex.instance_index);

    // Transform vertex position to world space
    // The sphere is centered at origin, so world_position = vertex position transformed by mesh transform
    let world_position = mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

    // Output clip position using Bevy 0.16.1 API
    out.clip_position = mesh_position_local_to_clip(world_from_local, vec4<f32>(vertex.position, 1.0));
    out.world_position = world_position.xyz;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize direction for star field rendering
    let dir = normalize(in.world_position);
    
    // ============================================================
    // DEBUG MODE 1: YELLOW - Verify shader is executing at all
    // If you see yellow at night, the shader pipeline works
    // ============================================================
    if (DEBUG_MODE == 1) {
        if (night_factor > 0.01) {
            return vec4<f32>(1.0, 1.0, 0.0, 1.0);  // Bright yellow
        }
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);  // Transparent during day
    }
    
    // ============================================================
    // DEBUG MODE 2: RED gradient - Visualize night_factor value
    // Brighter red = higher night_factor (should be 1.0 at night)
    // ============================================================
    if (DEBUG_MODE == 2) {
        return vec4<f32>(night_factor, 0.0, 0.0, night_factor);
    }
    
    // ============================================================
    // DEBUG MODE 3: GREEN - Visualize star calculation result
    // Green intensity shows total star brightness
    // ============================================================
    if (DEBUG_MODE == 3) {
        if (night_factor < 0.01) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        let distant_stars = star_layer(dir, 80.0, 0.4 * star_brightness, 2.0);
        let medium_stars = star_layer(dir, 40.0, 0.7 * star_brightness, 3.0);
        let bright_stars = star_layer(dir, 20.0, 1.2 * star_brightness, 4.0);
        let rare_stars = star_layer(dir, 10.0, 2.0 * star_brightness, 5.0);
        let total_stars = distant_stars + medium_stars + bright_stars + rare_stars;
        // Clamp to visible range and return as green
        let green_val = clamp(total_stars, 0.0, 1.0);
        return vec4<f32>(0.0, green_val, 0.0, 1.0);
    }
    
    // ============================================================
    // DEBUG MODE 4: BLUE - Visualize horizon check (dir.y)
    // Shows if the horizon culling is working correctly
    // ============================================================
    if (DEBUG_MODE == 4) {
        if (night_factor < 0.01) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        // Blue = above horizon, black = below horizon
        if (dir.y < -0.05) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);  // Below horizon
        }
        // Gradient from horizon (0) to zenith (1)
        let height = (dir.y + 0.05) / 1.05;
        return vec4<f32>(0.0, 0.0, height, 1.0);
    }
    
    // ============================================================
    // DEBUG MODE 5: Normal rendering with BRIGHTNESS BOOST
    // Use this to see if stars are too dim
    // ============================================================
    if (DEBUG_MODE == 5) {
        if (night_factor < 0.01) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        if (dir.y < -0.05) {
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        }
        
        // Generate stars with 5x brightness boost
        let distant_stars = star_layer(dir, 80.0, 2.0 * star_brightness, 2.0);
        let medium_stars = star_layer(dir, 40.0, 3.5 * star_brightness, 3.0);
        let bright_stars = star_layer(dir, 20.0, 6.0 * star_brightness, 4.0);
        let rare_stars = star_layer(dir, 10.0, 10.0 * star_brightness, 5.0);
        let total_stars = distant_stars + medium_stars + bright_stars + rare_stars;
        
        let star_color = vec3<f32>(0.9, 0.92, 1.0);
        var final_color = star_color * total_stars;
        
        return vec4<f32>(final_color, 1.0);
    }
    
    // ============================================================
    // NORMAL RENDERING MODE (DEBUG_MODE = 0)
    // ============================================================
    
    // CRITICAL: Only render stars at night!
    // night_factor is 0.0 during day, 1.0 at night
    // During day, return transparent so Bevy Atmosphere sky is visible
    if (night_factor < 0.01) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);  // Fully transparent
    }
    
    // Only render stars on the upper hemisphere (above horizon)
    if (dir.y < -0.05) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);  // Transparent below horizon
    }
    
    // Get night visibility factor
    let night_vis = get_night_visibility(dir);
    
    // Skip rendering if visibility is too low
    if (night_vis < 0.01) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    
    // Generate multiple star layers for depth and density
    // Layer 1: Distant, numerous small stars
    let distant_stars = star_layer(dir, 80.0, 0.4 * star_brightness, 2.0);
    
    // Layer 2: Medium distance stars
    let medium_stars = star_layer(dir, 40.0, 0.7 * star_brightness, 3.0);
    
    // Layer 3: Close, bright stars
    let bright_stars = star_layer(dir, 20.0, 1.2 * star_brightness, 4.0);
    
    // Layer 4: Very bright, rare stars
    let rare_stars = star_layer(dir, 10.0, 2.0 * star_brightness, 5.0);
    
    // Combine all star layers
    let total_stars = distant_stars + medium_stars + bright_stars + rare_stars;
    
    // Add nebula background for depth
    let nebula = nebula_background(dir);
    
    // Render moon
    let moon = render_moon(dir);
    
    // Star color with slight blue-white tint
    let star_color = vec3<f32>(0.9, 0.92, 1.0);
    
    // Combine all elements
    var final_color = nebula + star_color * total_stars + moon;
    
    // Apply night visibility factor
    final_color *= night_vis;
    
    // Add subtle horizon glow during transition
    let horizon_glow = smoothstep(-0.1, 0.2, dir.y) * smoothstep(0.3, 0.1, dir.y);
    final_color += vec3<f32>(0.02, 0.015, 0.01) * horizon_glow * night_vis;
    
    // Return with alpha based on night factor for smooth transitions
    return vec4<f32>(final_color, night_factor);
}
