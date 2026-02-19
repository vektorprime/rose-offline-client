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

// === FOAM EFFECTS ===
// Foam appears on wave crests (high wave height) and creates white foam patterns
// This is purely procedural and doesn't require additional textures
fn calculate_foam(wave_height_val: f32, time: f32, uv: vec2<f32>) -> f32 {
    // Foam threshold - foam appears when wave height exceeds this value
    let foam_threshold = 0.3;
    
    // Calculate foam factor based on wave height using smoothstep for soft edges
    let foam_factor = smoothstep(foam_threshold, foam_threshold + 0.2, wave_height_val);
    
    // Add noise variation using additional sine waves
    // This creates more natural-looking foam patterns
    let noise = sin(uv.x * 15.0 + time * 1.5) * 0.5 +
                cos(uv.y * 12.0 + time * 1.2) * 0.5 +
                sin((uv.x + uv.y) * 20.0 + time * 2.0) * 0.3;
    let noise_normalized = noise * 0.33 + 0.5; // Normalize to 0-1 range
    
    // Combine foam factor with noise for varied foam appearance
    let foam = foam_factor * noise_normalized;
    
    // Animate foam with pulsing effect
    let animated_foam = foam * (0.7 + 0.3 * sin(time * 2.0));
    
    return clamp(animated_foam, 0.0, 1.0);
}

// === SUBSURFACE SCATTERING (SSS) APPROXIMATION ===
// Simulates light penetrating and scattering through water
// This gives water a glowing appearance when viewed at certain angles
fn calculate_sss(view_dir: vec3<f32>, light_dir: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
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
    return sss_color * sss_factor * 0.4;
}

// === PSEUDO-REFRACTION EFFECT ===
// Since we can't access the opaque render texture directly, create a pseudo-refraction
// effect by distorting UV coordinates based on wave normals
fn apply_refraction(uv: vec2<f32>, normal: vec3<f32>, time: f32) -> vec2<f32> {
    // Distort UVs based on wave normal XZ components
    // The normal's X and Z components indicate surface tilt
    let distortion = normal.xz * 0.03;
    
    // Add time-based animation for flowing water effect
    let animated_distortion = distortion + vec2<f32>(
        sin(time * 0.5 + uv.y * 3.0) * 0.015,
        cos(time * 0.3 + uv.x * 3.0) * 0.015
    );
    
    return uv + animated_distortion;
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
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // === PROCEDURAL WAVE NORMALS (calculate early for refraction) ===
    // Calculate wave normal based on world position for consistent wave patterns
    // Use a separate time for waves (slower than texture animation)
    let wave_time = globals.time * 2.0;
    
    // Use world position XZ for wave calculation (water is on XZ plane)
    let wave_pos = in.world_position.xz * 0.5; // Scale down for larger waves
    
    // Calculate procedural wave normal
    let wave_normal = calculate_wave_normal(wave_pos, wave_time);
    
    // Get base normal from mesh and normalize
    let base_normal = normalize(in.world_normal);
    
    // Blend wave normal with base normal
    // wave_strength of 0.3 gives visible waves without being too extreme
    let N = blend_normals(base_normal, wave_normal, 0.3);
    
    // === PHASE 3: PSEUDO-REFRACTION ===
    // Apply UV distortion based on wave normal for a refraction-like effect
    let refracted_uv = apply_refraction(in.uv0, wave_normal, wave_time);
    
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
    
    // === FRESNEL EFFECT ===
    // Calculate view direction (from fragment to camera)
    // view.world_position is camera position in world space
    let view_dir = normalize(view.world_position.xyz - in.world_position.xyz);
    let VdotN = max(dot(view_dir, N), 0.0);
    
    // Fresnel effect: more reflective at grazing angles
    // F0 = 0.02 is typical for water/air interface
    let fresnel = fresnel_schlick(VdotN, 0.02);
    
    // === SPECULAR SUN HIGHLIGHTS ===
    // Calculate specular highlight using Blinn-Phong half vector
    // The wave normal creates dynamic specular highlights
    let half_vec = normalize(light_dir + view_dir);
    let spec = pow(max(dot(N, half_vec), 0.0), 256.0);
    
    // Add specular highlight to color (bright sun reflection)
    let specular_color = vec3<f32>(spec * 0.5);
    
    // === PHASE 3: SUBSURFACE SCATTERING ===
    // Calculate SSS for light passing through water
    let sss_contribution = calculate_sss(view_dir, light_dir, N);
    
    // === PHASE 3: FOAM EFFECTS ===
    // Calculate wave height for foam (reuse wave position)
    let current_wave_height = wave_height(wave_pos, wave_time);
    
    // Calculate foam factor based on wave height
    let foam_factor = calculate_foam(current_wave_height, wave_time, in.uv0);
    
    // Foam color (white with slight blue tint for more natural look)
    let foam_color = vec3<f32>(0.95, 0.97, 1.0);
    
    // === COMBINE ALL EFFECTS ===
    // Start with lit water color
    var final_color = lit_water_color;
    
    // Add specular highlights
    final_color = final_color + specular_color;
    
    // Add subsurface scattering contribution
    final_color = final_color + sss_contribution;
    
    // Blend in foam on wave crests (foam_factor controls intensity)
    final_color = mix(final_color, foam_color, foam_factor * 0.4);
    
    // Base alpha from water texture
    let base_alpha = water_color.a;
    
    // Increase alpha at grazing angles (Fresnel makes water more opaque at low angles)
    // Also increase alpha where there's foam
    let final_alpha = mix(base_alpha, 1.0, fresnel * 0.5) + foam_factor * 0.2;
    
    // Final color with additive blending (handled by blend state in material)
    // Note: Zone fog is not available since we can't access bind group 3
    return vec4<f32>(final_color, saturate(final_alpha));
}
