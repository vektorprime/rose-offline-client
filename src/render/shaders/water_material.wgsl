//! Water material shader for ROSE Online
//!
//! Supports animated water with:
//! - 25 water animation frames in a binding_array
//! - Time-based frame blending for smooth animation
//! - Additive blending for water transparency effect
//! - Fresnel effect for angle-dependent reflectivity
//! - Specular sun highlights
//! - Procedural wave normals for dynamic surface detail
//! - Foam effects on wave crests (Phase 3)
//! - Subsurface scattering approximation (Phase 3)
//! - Pseudo-refraction via UV distortion (Phase 3)
//!
//! Note: This shader uses its own lighting uniforms instead of zone_lighting
//! because custom materials only have access to bind groups 0-2.

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

// Lighting uniforms (passed from WaterMaterial struct)
@group(2) @binding(2)
var<uniform> light_direction: vec4<f32>;  // xyz = direction, w = unused
@group(2) @binding(3)
var<uniform> ambient_color: vec4<f32>;
@group(2) @binding(4)
var<uniform> diffuse_color: vec4<f32>;

// Fog uniforms (passed from WaterMaterial struct, synced with zone lighting)
struct FogUniforms {
    fog_color: vec4<f32>,
    fog_params: vec4<f32>,  // x = density, y = min_density, z = max_density, w = unused
}
@group(2) @binding(6)
var<uniform> fog_uniforms: FogUniforms;

// Water settings uniform (passed from WaterMaterial struct)
// Layout matches WaterSettings struct in water_settings.rs
struct WaterSettingsUniform {
    // First vec4: foam_intensity, foam_threshold, sss_intensity, refraction_strength
    foam_intensity: f32,
    foam_threshold: f32,
    sss_intensity: f32,
    refraction_strength: f32,
    // Second vec4: wave_speed, fresnel_strength, specular_intensity, padding
    wave_speed: f32,
    fresnel_strength: f32,
    specular_intensity: f32,
    _padding: f32,
}
@group(2) @binding(5)
var<uniform> water_settings: WaterSettingsUniform;

// Fresnel-Schlick approximation for angle-dependent reflectivity
// F0 is the reflectance at normal incidence (0.02 for water)
fn fresnel_schlick(cos_theta: f32, F0: f32) -> f32 {
    return F0 + (1.0 - F0) * pow(1.0 - cos_theta, 5.0);
}

// === PROCEDURAL WAVE FUNCTIONS ===
// These create dynamic wave patterns that modify the surface normal
// for more realistic lighting without requiring additional textures

// Simple wave function combining multiple sine waves at different frequencies
// Creates a natural-looking wave pattern
fn wave_height(position: vec2<f32>, time: f32) -> f32 {
    // Primary wave - large slow waves
    let wave1 = sin(position.x * 2.0 + time * 1.0) * 0.5;
    // Secondary wave - medium cross-waves
    let wave2 = sin(position.y * 3.0 + time * 1.3) * 0.3;
    // Tertiary wave - diagonal ripples
    let wave3 = sin((position.x + position.y) * 1.5 + time * 0.7) * 0.2;
    // Small detail waves
    let wave4 = sin(position.x * 8.0 + position.y * 6.0 + time * 2.5) * 0.1;
    
    return wave1 + wave2 + wave3 + wave4;
}

// Calculate the normal vector from wave height using finite differences
// This gives us the surface slope which affects lighting
fn calculate_wave_normal(position: vec2<f32>, time: f32) -> vec3<f32> {
    let eps = 0.1; // Small offset for gradient calculation
    let h = wave_height(position, time);
    let hx = wave_height(position + vec2<f32>(eps, 0.0), time);
    let hz = wave_height(position + vec2<f32>(0.0, eps), time);
    
    // Calculate gradient (slope in x and z directions)
    // The normal points perpendicular to the surface
    let dx = (hx - h) / eps;
    let dz = (hz - h) / eps;
    
    // Construct normal: (-dx, 1, -dz) normalized
    // The y component is 1 because the surface is primarily horizontal
    return normalize(vec3<f32>(-dx * 0.15, 1.0, -dz * 0.15));
}

// Blend the procedural wave normal with the base normal
// wave_strength controls how much the waves affect lighting (0.0 = flat, 1.0 = full waves)
fn blend_normals(base_normal: vec3<f32>, wave_normal: vec3<f32>, wave_strength: f32) -> vec3<f32> {
    // Linear interpolation between base and wave-influenced normal
    // We keep the base normal's general direction but add wave detail
    let blended = mix(base_normal, wave_normal, wave_strength);
    return normalize(blended);
}

// === PHASE 3: WATER QUALITY IMPROVEMENTS ===

// === ORGANIC FOAM NOISE FUNCTIONS ===
// These create smooth, irregular patterns without cell-like structures

// Enhanced hash function for better randomness distribution
// Creates pseudo-random values from 2D coordinates
fn hash(p: vec2<f32>) -> f32 {
    let p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    let p3_dot = p3 + dot(p3, p3.yzx + 33.33);
    return fract((p3_dot.x + p3_dot.y) * p3_dot.z);
}

// Hash returning vec2 for gradient calculations
fn hash2(p: vec2<f32>) -> vec2<f32> {
    let p3 = fract(vec3<f32>(p.xyx) * vec3<f32>(0.1031, 0.1030, 0.0973));
    let p3_dot = p3 + dot(p3, p3.yzx + 33.33);
    return fract(vec2<f32>((p3_dot.x + p3_dot.y) * p3_dot.z, (p3_dot.x + p3_dot.z) * p3_dot.y));
}

// Gradient noise (Perlin-like) - creates smooth, organic patterns
// Unlike Voronoi, this doesn't create cell edges or circular patterns
fn gradient_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Smoothstep interpolation for smooth curves
    let u = f * f * (3.0 - 2.0 * f);
    
    // Sample 4 corners and interpolate
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    
    return mix(
        mix(a, b, u.x),
        mix(c, d, u.x),
        u.y
    );
}

// Domain warping for organic distortion
// Distorts the noise coordinates to break up regular patterns
fn warp_noise(p: vec2<f32>, time: f32) -> f32 {
    // First warp layer - subtle distortion
    let warp1 = vec2<f32>(
        gradient_noise(p + vec2<f32>(time * 0.1, 0.0)),
        gradient_noise(p + vec2<f32>(0.0, time * 0.12))
    );
    
    // Apply first warp
    let warped_p = p + warp1 * 0.4;
    
    // Second warp layer for more complexity
    let warp2 = vec2<f32>(
        gradient_noise(warped_p * 1.5 + vec2<f32>(time * 0.08, time * 0.05)),
        gradient_noise(warped_p * 1.5 + vec2<f32>(time * 0.05, time * 0.08))
    );
    
    // Apply second warp
    let final_p = warped_p + warp2 * 0.2;
    
    // Multiple octaves for natural detail
    let n1 = gradient_noise(final_p * 2.0);
    let n2 = gradient_noise(final_p * 4.0) * 0.5;
    let n3 = gradient_noise(final_p * 8.0) * 0.25;
    
    return (n1 + n2 + n3) / 1.75;
}

// Organic foam noise - combines warped noise with turbulence
// Creates irregular, patchy patterns without cells or circles
fn organic_foam_noise(p: vec2<f32>, time: f32) -> f32 {
    // Base warped noise for organic shapes
    let base = warp_noise(p, time);
    
    // Add turbulence at different scales
    let turb1 = gradient_noise(p * 3.0 + time * 0.2);
    let turb2 = gradient_noise(p * 6.0 - time * 0.15);
    
    // Combine for complex, non-repeating patterns
    return base * 0.6 + turb1 * 0.25 + turb2 * 0.15;
}

// Fractal Brownian Motion (FBM) for more complex noise patterns
// Layers multiple octaves of noise at different frequencies
fn fbm(p: vec2<f32>, time: f32) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var freq = 1.0;
    var pos = p;
    
    // Accumulate 4 octaves of noise
    for (var i = 0; i < 4; i++) {
        value += amplitude * gradient_noise(pos * freq + time * 0.1);
        freq *= 2.0;
        amplitude *= 0.5;
    }
    
    return value;
}

// === FOAM EFFECTS (ORGANIC) ===
// Foam appears on wave crests and creates organic white patterns
// Uses gradient noise with domain warping - no cells or circles
fn calculate_foam(wave_height_val: f32, time: f32, uv: vec2<f32>, world_pos_xz: vec2<f32>, foam_threshold: f32) -> f32 {
    // Calculate foam factor based on wave height using smoothstep for soft edges
    let foam_factor = smoothstep(foam_threshold, foam_threshold + 0.3, wave_height_val);
    
    // Use world position for foam patterns so they stay in place
    // Scale for appropriate pattern size
    let world_noise_scale = 0.08;
    let base_p = world_pos_xz * world_noise_scale;
    
    // Get organic foam pattern using warped noise
    let organic_pattern = organic_foam_noise(base_p, time);
    
    // Add fine detail using gradient noise at higher frequency
    let detail_scale = 0.3;
    let detail = gradient_noise(world_pos_xz * detail_scale + time * 0.2);
    
    // Combine patterns - organic base with detail overlay
    let combined = organic_pattern * 0.75 + detail * 0.25;
    
    // Create irregular foam patches using smooth threshold
    // The smoothstep creates soft, blobby transitions
    let foam_mask = smoothstep(0.35, 0.65, combined);
    
    // Add variation based on wave height for dynamic foam
    let height_variation = smoothstep(0.0, 0.4, wave_height_val);
    
    // Combine foam factor with organic mask
    let foam = foam_factor * foam_mask * (0.6 + 0.4 * height_variation);
    
    // Subtle animation for living foam effect
    let animated_foam = foam * (0.9 + 0.1 * sin(time * 0.8 + combined * 3.0));
    
    return clamp(animated_foam, 0.0, 1.0);
}

// === WATER EDGE SPLASH EFFECT (ORGANIC) ===
// Creates foam/splash effect where water meets terrain
// Uses organic noise patterns instead of cells
fn calculate_edge_splash(world_pos: vec3<f32>, time: f32, uv: vec2<f32>) -> f32 {
    // Use world position XZ for detecting shore proximity
    let shore_noise = gradient_noise(world_pos.xz * 0.3);
    
    // Effective shore height varies based on terrain
    let base_shore_height = 0.5;
    let shore_height = base_shore_height + shore_noise * 1.5;
    
    // Calculate distance from water surface to effective shore height
    let dist_to_shore = abs(world_pos.y - shore_height);
    
    // Create splash effect that decreases with distance from shore
    let splash_range = 3.0;
    let edge_factor = smoothstep(splash_range, 0.0, dist_to_shore);
    
    // Use organic foam noise for irregular splash patterns
    let splash_pattern = organic_foam_noise(world_pos.xz * 0.1, time);
    
    // Add animated wave surge effect
    let wave_surge = sin(time * 2.0 + world_pos.x * 0.5) * 0.5 + 0.5;
    
    // Create breaking wave effect
    let breaking_wave = smoothstep(0.3, 0.7, wave_surge) * edge_factor;
    
    // Animated splash intensity
    let splash_animation = 0.5 + 0.5 * sin(time * 2.5 + splash_pattern * 4.0);
    
    // Combine edge factor with organic foam pattern
    let splash = (edge_factor * splash_pattern * 0.6 + breaking_wave * 0.4) * splash_animation;
    
    return clamp(splash, 0.0, 1.0);
}

// === SUBSURFACE SCATTERING (SSS) APPROXIMATION ===
// Simulates light penetrating and scattering through water
// This gives water a glowing appearance when viewed at certain angles
fn calculate_sss(view_dir: vec3<f32>, light_dir: vec3<f32>, normal: vec3<f32>, sss_intensity: f32) -> vec3<f32> {
    // SSS is strongest when looking through water toward the light
    // VdotN determines the viewing angle relative to the surface
    let VdotN = dot(view_dir, normal);
    
    // LdotN determines how much light hits the surface
    let LdotN = dot(light_dir, normal);
    
    // SSS factor: strongest when view and light are on opposite sides
    // This simulates light passing through the water volume
    let sss_factor = pow(max(0.0, -VdotN * LdotN), 2.0);
    
    // SSS color (cyan/turquoise tint typical of water light scattering)
    // This color mimics how water absorbs red light and scatters blue/green
    let sss_color = vec3<f32>(0.0, 0.8, 0.7);
    
    // Return SSS contribution with intensity scaling
    return sss_color * sss_factor * sss_intensity;
}

// === PSEUDO-REFRACTION EFFECT ===
// Since we can't access the opaque render texture directly, create a pseudo-refraction
// effect by distorting UV coordinates based on wave normals
fn apply_refraction(uv: vec2<f32>, normal: vec3<f32>, time: f32, refraction_strength: f32) -> vec2<f32> {
    // Distort UVs based on wave normal XZ components
    // The normal's X and Z components indicate surface tilt
    // Scale by refraction_strength (default 0.05 gives subtle distortion)
    let base_distortion = refraction_strength * 0.6; // Scale factor for visible effect
    let distortion = normal.xz * base_distortion;
    
    // Add time-based animation for flowing water effect
    let animated_distortion = distortion + vec2<f32>(
        sin(time * 0.5 + uv.y * 3.0) * refraction_strength * 0.3,
        cos(time * 0.3 + uv.x * 3.0) * refraction_strength * 0.3
    );
    
    return uv + animated_distortion;
}

// === ZONE FOG APPLICATION ===
// Apply exponential fog from zone lighting to integrate water with the scene
fn apply_zone_fog(fragment_color: vec3<f32>, world_position: vec4<f32>) -> vec3<f32> {
    // Calculate view-space Z distance for fog
    // view.position is camera position, so we compute distance from camera
    let camera_to_fragment = world_position.xyz - view.world_position.xyz;
    let view_z = length(camera_to_fragment);
    
    // Get fog parameters from uniforms (synced with zone lighting)
    let fog_density = fog_uniforms.fog_params.x;
    let fog_min_density = fog_uniforms.fog_params.y;
    let fog_max_density = fog_uniforms.fog_params.z;
    let fog_color = fog_uniforms.fog_color.rgb;
    
    // Calculate exponential fog amount
    // Using the same formula as zone_lighting.wgsl for consistency
    var fog_amount: f32 = clamp(1.0 - exp2(-fog_density * fog_density * view_z * view_z * 1.442695), 0.0, 1.0);
    
    // Clamp fog amount between min and max density
    fog_amount = clamp(fog_amount, fog_min_density, fog_max_density);
    
    // Blend fragment color with fog color
    return mix(fragment_color, fog_color, fog_amount);
}

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
fn fragment(in: VertexOutput, @builtin(front_facing) is_front_facing: bool) -> @location(0) vec4<f32> {
    // === PROCEDURAL WAVE NORMALS (calculate early for refraction) ===
    // Calculate wave normal based on world position for consistent wave patterns
    // Use wave_speed from settings to control animation speed
    let wave_time = globals.time * 2.0 * water_settings.wave_speed;
    
    // Use world position XZ for wave calculation (water is on XZ plane)
    let wave_pos = in.world_position.xz * 0.5; // Scale down for larger waves
    
    // Calculate procedural wave normal
    let wave_normal = calculate_wave_normal(wave_pos, wave_time);
    
    // Get base normal from mesh and normalize
    var base_normal = normalize(in.world_normal);
    
    // Flip normal when viewing from below (back-facing fragments)
    // This ensures correct lighting when camera is underwater
    if (!is_front_facing) {
        base_normal = -base_normal;
    }
    
    // Blend wave normal with base normal
    // wave_strength of 0.3 gives visible waves without being too extreme
    let N = blend_normals(base_normal, wave_normal, 0.3);
    
    // === PHASE 3: PSEUDO-REFRACTION ===
    // Apply UV distortion based on wave normal for a refraction-like effect
    let refracted_uv = apply_refraction(in.uv0, wave_normal, wave_time, water_settings.refraction_strength);
    
    // Animate water at 10 FPS (same as original)
    // globals.time is in seconds, wrap it to avoid precision issues
    let anim_time = globals.time * 10.0;
    let frame_time = fract(anim_time / 25.0) * 25.0;
    let current_index = i32(floor(frame_time)) % 25;
    let next_index = (current_index + 1) % 25;
    let blend = fract(frame_time);
    
    // Sample current and next frame using refracted UVs for distorted appearance
    let color1 = textureSample(water_array_texture[current_index], water_array_sampler, refracted_uv);
    let color2 = textureSample(water_array_texture[next_index], water_array_sampler, refracted_uv);
    
    // Blend between frames
    let water_color = mix(color1, color2, blend);
    
    // === LIGHTING (using uniforms instead of zone_lighting) ===
    let light_dir = normalize(light_direction.xyz);
    
    // Apply ambient and diffuse lighting to water
    let diffuse = max(dot(N, light_dir), 0.0);
    let ambient_light = ambient_color.rgb;
    let diffuse_light = diffuse_color.rgb * diffuse;
    let lighting = saturate(ambient_light + diffuse_light);
    
    // Apply lighting to water color
    let lit_water_color = water_color.rgb * lighting;
    
    // === FRESNEL EFFECT (IMPROVED - MORE REFLECTION) ===
    // Calculate view direction (from fragment to camera)
    // view.world_position is camera position in world space
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let VdotN = max(dot(view_dir, N), 0.0);
    
    // Fresnel effect: more reflective at grazing angles
    // F0 = 0.2 for significantly increased base reflection (was 0.02, then 0.1)
    // This makes water much more reflective at all angles
    let base_fresnel = fresnel_schlick(VdotN, 0.2);
    
    // Apply fresnel_strength from settings with a boost multiplier
    // reflection_boost increased to 2.0 for more prominent reflections
    let reflection_boost = 2.0;
    let fresnel = base_fresnel * water_settings.fresnel_strength * reflection_boost;
    
    // === SPECULAR SUN HIGHLIGHTS ===
    // Calculate specular highlight using Blinn-Phong half vector
    // The wave normal creates dynamic specular highlights
    let half_vec = normalize(light_dir + view_dir);
    let spec = pow(max(dot(N, half_vec), 0.0), 256.0);
    
    // Add specular highlight to color (bright sun reflection)
    // Use specular_intensity from settings with boost for more visible highlights
    let specular_boost = 1.3;
    let specular_color = vec3<f32>(spec * water_settings.specular_intensity * specular_boost);
    
    // === PHASE 3: SUBSURFACE SCATTERING ===
    // Calculate SSS for light passing through water
    let sss_contribution = calculate_sss(view_dir, light_dir, N, water_settings.sss_intensity);
    
    // === PHASE 3: FOAM EFFECTS (IMPROVED) ===
    // Calculate wave height for foam (reuse wave position)
    let current_wave_height = wave_height(wave_pos, wave_time);
    
    // Calculate foam factor using improved Voronoi-based noise
    // Pass world position XZ for stable, natural foam patterns
    let foam_factor = calculate_foam(current_wave_height, wave_time, in.uv0, in.world_position.xz, water_settings.foam_threshold);
    
    // === WATER EDGE SPLASH EFFECT (IMPROVED) ===
    // Calculate splash/foam where water meets terrain using full world position
    let edge_splash = calculate_edge_splash(in.world_position.xyz, wave_time, in.uv0);
    
    // Combine foam from waves and edge splash
    let total_foam = foam_factor + edge_splash * 0.5;
    
    // Foam color (white with slight blue tint for more natural look)
    let foam_color = vec3<f32>(0.95, 0.97, 1.0);
    
    // === SKY REFLECTION COLOR (IMPROVED) ===
    // Enhanced sky color approximation for more vivid reflections
    // Uses fresnel to blend sky color into water at grazing angles
    let sky_color = vec3<f32>(0.35, 0.55, 0.95); // More saturated blue sky
    let horizon_color = vec3<f32>(0.65, 0.75, 0.92); // Lighter at horizon
    let sun_reflection_color = vec3<f32>(1.0, 0.95, 0.85); // Warm sun tint
    let sky_reflection = mix(horizon_color, sky_color, VdotN);
    
    // === COMBINE ALL EFFECTS ===
    // Start with lit water color
    var final_color = lit_water_color;
    
    // Add sky reflection based on fresnel (increased from 0.4 to 0.6 for more reflection)
    final_color = mix(final_color, sky_reflection, fresnel * 0.6);
    
    // Add specular highlights
    final_color = final_color + specular_color;
    
    // Add subsurface scattering contribution
    final_color = final_color + sss_contribution;
    
    // Blend in foam on wave crests and edges (total_foam combines both)
    // Use foam_intensity from settings
    final_color = mix(final_color, foam_color, total_foam * water_settings.foam_intensity);
    
    // === APPLY ZONE FOG ===
    // Apply fog from zone lighting to integrate water with the scene
    final_color = apply_zone_fog(final_color, in.world_position);
    
    // Base alpha from water texture
    let base_alpha = water_color.a;
    
    // Ensure minimum opacity of 0.7 to make water less see-through
    let min_alpha = 0.7;
    let clamped_base_alpha = max(base_alpha, min_alpha);
    
    // Increase alpha at grazing angles (Fresnel makes water more opaque at low angles)
    // Also increase alpha where there's foam
    let final_alpha = mix(clamped_base_alpha, 1.0, fresnel * 0.5) + total_foam * 0.2;
    
    // Final color with additive blending (handled by blend state in material)
    // Fog is now applied via fog_uniforms (binding 6)
    return vec4<f32>(final_color, saturate(final_alpha));
}
