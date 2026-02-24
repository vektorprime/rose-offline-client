//! Underwater post-processing effect shader
//!
//! Implements:
//! - Volumetric fog using Beer-Lambert law
//! - Depth-based color absorption (red absorbed fastest, blue penetrates)
//! - Procedural caustics effect

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

// =============================================================================
// Bindings
// =============================================================================

/// Source texture (the rendered scene)
@group(0) @binding(0) var source_texture: texture_2d<f32>;

/// Sampler for source texture
@group(0) @binding(1) var source_sampler: sampler;

/// Uniform buffer with underwater settings
@group(0) @binding(2) var<uniform> underwater: UnderwaterSettings;

// =============================================================================
// Structs
// =============================================================================

struct UnderwaterSettings {
    /// Whether camera is underwater (0.0 or 1.0)
    is_underwater: f32,
    /// Water surface Y coordinate
    water_surface_y: f32,
    /// Fog density
    fog_density: f32,
    /// Maximum visibility distance
    max_visibility: f32,
    /// Fog color (RGBA)
    fog_color: vec4<f32>,
    /// Light absorption coefficients (RGB)
    absorption: vec3<f32>,
    /// Caustics intensity
    caustics_intensity: f32,
    /// Caustics scale
    caustics_scale: f32,
    /// Caustics speed
    caustics_speed: f32,
    /// Time for animation
    time: f32,
    /// Padding
    _padding: vec3<f32>,
}

// =============================================================================
// Noise Functions for Caustics
// =============================================================================

/// Simple hash function for procedural noise
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(p.xyx * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

/// Value noise for caustic patterns
fn value_noise(uv: vec2<f32>) -> f32 {
    let i = floor(uv);
    let f = fract(uv);
    
    // Smooth interpolation
    let u = f * f * (3.0 - 2.0 * f);
    
    // Sample four corners
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    
    // Bilinear interpolation
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

/// Gradient noise (Perlin-style) for smoother caustics
fn gradient_noise(uv: vec2<f32>) -> f32 {
    let i = floor(uv);
    let f = fract(uv);
    
    // Smooth interpolation curve
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    
    // Hash to get gradient directions
    let ga = hash(i);
    let gb = hash(i + vec2<f32>(1.0, 0.0));
    let gc = hash(i + vec2<f32>(0.0, 1.0));
    let gd = hash(i + vec2<f32>(1.0, 1.0));
    
    // Compute gradient contributions
    let ca = cos(ga * 6.28318);
    let cb = cos(gb * 6.28318);
    let cc = cos(gc * 6.28318);
    let cd = cos(gd * 6.28318);
    
    // Blend contributions
    return mix(
        mix(ca, cb, u.x),
        mix(cc, cd, u.x),
        u.y
    ) * 0.5 + 0.5;
}

/// Fractal Brownian Motion for more detailed caustics
fn fbm(uv: vec2<f32>, octaves: u32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var uv_mut = uv;
    
    for (var i = 0u; i < octaves; i++) {
        value += amplitude * gradient_noise(uv_mut * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    return value;
}

/// Animated caustics pattern
fn calculate_caustics(
    world_pos: vec2<f32>,
    time: f32,
    scale: f32,
    speed: f32
) -> f32 {
    let uv = world_pos * scale;
    let t = time * speed;
    
    // Multiple layers of animated noise for realistic caustics
    let caustic1 = fbm(uv + t * 0.1, 4u);
    let caustic2 = fbm(uv * 1.5 - t * 0.15, 3u);
    let caustic3 = fbm(uv * 2.0 + t * 0.08, 2u);
    
    // Combine layers with different weights
    let combined = caustic1 * 0.5 + caustic2 * 0.3 + caustic3 * 0.2;
    
    // Create sharp caustic patterns using threshold
    // This simulates the focused light beams underwater
    let sharp_caustic = smoothstep(0.4, 0.7, combined);
    
    return sharp_caustic;
}

// =============================================================================
// Underwater Effects
// =============================================================================

/// Beer-Lambert law for light absorption
/// Transmittance = e^(-absorption * depth)
fn calculate_transmittance(
    depth: f32,
    absorption: vec3<f32>
) -> vec3<f32> {
    return exp(-absorption * depth);
}

/// Exponential fog based on depth
fn calculate_underwater_fog(
    depth: f32,
    density: f32,
    fog_color: vec3<f32>
) -> vec3<f32> {
    // Exponential fog: fog_factor = 1 - e^(-density * depth)
    let fog_factor = 1.0 - exp(-density * depth);
    return mix(vec3<f32>(1.0), fog_color, saturate(fog_factor));
}

/// Apply depth-based color absorption
/// Red is absorbed fastest, blue penetrates deepest
fn apply_light_absorption(
    color: vec3<f32>,
    depth: f32,
    absorption: vec3<f32>
) -> vec3<f32> {
    let transmittance = calculate_transmittance(depth, absorption);
    return color * transmittance;
}

// =============================================================================
// Main Entry Point
// =============================================================================

@fragment
fn fragment_main(
    in: FullscreenVertexOutput,
) -> @location(0) vec4<f32> {
    // Early out if not underwater - just pass through the source
    if (underwater.is_underwater < 0.5) {
        return textureSample(source_texture, source_sampler, in.uv);
    }
    
    // Sample the source texture
    let source_color = textureSample(source_texture, source_sampler, in.uv);
    
    // Calculate depth effect (simulated since we don't have depth buffer access)
    // In a full implementation, we would use the depth prepass texture
    // For now, we use a uniform depth based on camera depth below surface
    let depth = underwater.water_surface_y * 0.1; // Simplified depth estimate
    
    // Apply light absorption first (affects the scene color)
    var color = apply_light_absorption(
        source_color.rgb,
        depth,
        underwater.absorption
    );
    
    // Calculate underwater fog
    let fog_color = calculate_underwater_fog(
        depth,
        underwater.fog_density,
        underwater.fog_color.rgb
    );
    
    // Blend fog with the absorbed color
    color = mix(color, fog_color, saturate(underwater.fog_density * depth));
    
    // Calculate caustics overlay
    // Use screen-space UV for caustics (would be better with world position)
    let caustics_uv = in.uv * 100.0; // Scale for visible caustic pattern
    let caustics = calculate_caustics(
        caustics_uv,
        underwater.time,
        underwater.caustics_scale,
        underwater.caustics_speed
    );
    
    // Apply caustics as additive light
    let caustics_color = vec3<f32>(caustics) * underwater.caustics_intensity;
    color += caustics_color * (1.0 - underwater.fog_density * depth);
    
    // Preserve the original alpha
    return vec4<f32>(color, source_color.a);
}
