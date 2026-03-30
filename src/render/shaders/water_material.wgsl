//! Water material shader for ROSE Online
//!
//! Supports animated water with:
//! - 25 water animation frames in a binding_array
//! - Time-based frame blending for smooth animation
//! - Additive blending for water transparency effect
//! - Fresnel effect for angle-dependent reflectivity
//! - Specular sun highlights
//! - Procedural wave normals for dynamic surface detail
//! - Foam effects on wave crests
//! - Subsurface scattering approximation
//! - Pseudo-refraction via UV distortion
//! - Depth-based color gradient (shallow to deep water)
//! - Bottom visibility in shallow water
//! - Caustics effects
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
    @builtin(position) @invariant clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
}

// Water material bind group (group 2 - material bindings)
// Texture array with 25 water animation frames
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var water_array_texture: binding_array<texture_2d<f32>, 25>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var water_array_sampler: sampler;

// wgpu 27 forbids combining binding arrays with uniform buffers in the same bind group.
// Pack all per-material values into a read-only storage buffer:
// [0] light_direction (vec4)
// [1] ambient_color (vec4)
// [2] diffuse_color (vec4)
// [3] settings_1: foam_intensity, foam_threshold, sss_intensity, refraction_strength
// [4] settings_2: wave_speed, fresnel_strength, specular_intensity, wave_amplitude
// [5] fog_color (vec4)
// [6] fog_params: density, min_density, max_density, wave_frequency
// [7] depth_1: min_depth, max_depth, shallow_threshold, bottom_visibility
// [8] deep_color (vec4)
// [9] shallow_color (vec4)
// [10] depth_scale: x, y, wave_layers (as float), caustics_intensity
// [11] caustics: scale, speed, water_surface_y, padding
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var<storage, read> water_material_data: array<vec4<f32>, 12>;

fn light_direction_value() -> vec3<f32> {
    return water_material_data[0].xyz;
}

fn ambient_color_value() -> vec3<f32> {
    return water_material_data[1].rgb;
}

fn diffuse_color_value() -> vec3<f32> {
    return water_material_data[2].rgb;
}

fn foam_intensity_value() -> f32 {
    return water_material_data[3].x;
}

fn foam_threshold_value() -> f32 {
    return water_material_data[3].y;
}

fn sss_intensity_value() -> f32 {
    return water_material_data[3].z;
}

fn refraction_strength_value() -> f32 {
    return water_material_data[3].w;
}

fn wave_speed_value() -> f32 {
    return water_material_data[4].x;
}

fn fresnel_strength_value() -> f32 {
    return water_material_data[4].y;
}

fn specular_intensity_value() -> f32 {
    return water_material_data[4].z;
}

fn wave_amplitude_value() -> f32 {
    return water_material_data[4].w;
}

fn fog_color_value() -> vec3<f32> {
    return water_material_data[5].rgb;
}

fn fog_density_value() -> f32 {
    return water_material_data[6].x;
}

fn fog_min_density_value() -> f32 {
    return water_material_data[6].y;
}

fn fog_max_density_value() -> f32 {
    return water_material_data[6].z;
}

fn wave_frequency_value() -> f32 {
    return water_material_data[6].w;
}

// === NEW DEPTH-RELATED ACCESSORS ===

fn min_depth_value() -> f32 {
    return water_material_data[7].x;
}

fn max_depth_value() -> f32 {
    return water_material_data[7].y;
}

fn shallow_threshold_value() -> f32 {
    return water_material_data[7].z;
}

fn bottom_visibility_value() -> f32 {
    return water_material_data[7].w;
}

fn deep_color_value() -> vec4<f32> {
    return water_material_data[8];
}

fn shallow_color_value() -> vec4<f32> {
    return water_material_data[9];
}

fn depth_scale_x_value() -> f32 {
    return water_material_data[10].x;
}

fn depth_scale_y_value() -> f32 {
    return water_material_data[10].y;
}

fn wave_layers_value() -> f32 {
    return water_material_data[10].z;
}

fn caustics_intensity_value() -> f32 {
    return water_material_data[10].w;
}

fn caustics_scale_value() -> f32 {
    return water_material_data[11].x;
}

fn caustics_speed_value() -> f32 {
    return water_material_data[11].y;
}

fn water_surface_y_value() -> f32 {
    return water_material_data[11].z;
}

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

// === DEPTH-BASED WATER COLOR FUNCTIONS ===
// These functions generate procedural water color based on depth
// allowing for realistic shallow-to-deep water transitions

// Calculate procedural depth at a given world position
// Uses noise-based variation to create natural depth patterns
fn calculate_procedural_depth(world_pos_xz: vec2<f32>, time: f32) -> f32 {
    // Get depth scale from settings
    let depth_scale = vec2<f32>(depth_scale_x_value(), depth_scale_y_value());
    
    // Base depth variation using layered sine waves
    var depth_variation = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    
    // Number of layers from settings
    let layers = wave_layers_value();
    
    for (var i = 0; i < 4; i++) {
        if (f32(i) >= layers) {
            break;
        }
        
        // Each layer adds detail at different scales
        // Use dot() to combine vec2 sine result into a scalar
        let wave_vec = sin(world_pos_xz * frequency + time * 0.2 + f32(i) * 1.57);
        let wave = dot(wave_vec, vec2<f32>(1.0, 1.0)) * 0.5;
        depth_variation = depth_variation + wave * amplitude;
        
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
    }
    
    // Normalize variation to 0-1 range and map to depth range
    let normalized_variation = (depth_variation + 1.0) * 0.5;
    let min_depth = min_depth_value();
    let max_depth = max_depth_value();
    
    return min_depth + normalized_variation * (max_depth - min_depth);
}

// Generate procedural water color based on depth
// Blends between shallow and deep colors with procedural variation
fn generate_procedural_water_color(depth: f32, world_pos_xz: vec2<f32>, time: f32) -> vec4<f32> {
    // Get shallow and deep colors from settings
    let shallow_color = shallow_color_value();
    let deep_color = deep_color_value();
    
    // Calculate depth factor (0.0 = shallow, 1.0 = deep)
    let min_depth = min_depth_value();
    let max_depth = max_depth_value();
    let depth_factor = saturate((depth - min_depth) / (max_depth - min_depth + 0.001));
    
    // Base color interpolation between shallow and deep
    var base_color = mix(shallow_color, deep_color, depth_factor);
    
    // Add procedural variation based on position and time
    // This creates natural color variation across the water surface
    let variation_scale = 0.5;
    let noise1 = sin(world_pos_xz.x * variation_scale + time * 0.1);
    let noise2 = sin(world_pos_xz.y * variation_scale + time * 0.12);
    let noise3 = sin((world_pos_xz.x + world_pos_xz.y) * variation_scale * 0.5 + time * 0.08);
    
    // Combine noise for subtle color variation
    let color_variation = (noise1 + noise2 + noise3) * 0.33;
    
    // Apply variation primarily to RGB, keep alpha based on depth
    base_color = vec4<f32>(base_color.rgb + color_variation * 0.1, base_color.a);
    
    // Adjust alpha based on depth (shallower = more transparent)
    let alpha_factor = mix(0.3, 0.95, depth_factor);
    base_color.a = shallow_color.a * alpha_factor;
    
    return base_color;
}

// Calculate bottom visibility based on depth
// Returns 1.0 for fully visible bottom (shallow), 0.0 for no visibility (deep)
fn calculate_bottom_visibility(depth: f32) -> f32 {
    let shallow_threshold = shallow_threshold_value();
    let visibility = bottom_visibility_value();
    
    // Bottom is only visible in shallow water
    let visibility_factor = saturate(1.0 - (depth / shallow_threshold));
    
    return visibility_factor * visibility;
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
    
    // Get fog parameters from storage buffer (synced with zone lighting)
    let fog_density = fog_density_value();
    let fog_min_density = fog_min_density_value();
    let fog_max_density = fog_max_density_value();
    let fog_color = fog_color_value();
    
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
    let wave_time = globals.time * 2.0 * wave_speed_value();
    
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
    let refracted_uv = apply_refraction(in.uv0, wave_normal, wave_time, refraction_strength_value());
    
    // === DEPTH-BASED PROCEDURAL WATER COLOR ===
    // Calculate procedural depth at this position
    let depth = calculate_procedural_depth(in.world_position.xz, wave_time);
    
    // Generate procedural water color based on depth
    let procedural_water_color = generate_procedural_water_color(depth, in.world_position.xz, wave_time);
    
    // Calculate bottom visibility for shallow water
    let bottom_vis = calculate_bottom_visibility(depth);
    
    // === PROCEDURAL WAVE TEXTURE GENERATION ===
    // Generate procedural wave pattern using layered sine functions
    // This creates dynamic surface detail without relying on textures
    var procedural_wave = vec3<f32>(0.5); // Base gray
    
    // Layer 1: Large slow waves
    let wave1 = sin(in.world_position.xz * 0.5 + wave_time * 0.5);
    procedural_wave = procedural_wave + vec3<f32>(wave1, 0.0) * 0.3;
    
    // Layer 2: Medium cross-waves
    let wave2 = sin(in.world_position.xz * vec2<f32>(1.0, 0.7) + wave_time * 0.7);
    procedural_wave = procedural_wave + vec3<f32>(wave2, 0.0) * 0.2;
    
    // Layer 3: Diagonal ripples
    let wave3 = sin((in.world_position.x + in.world_position.z) * 0.6 + wave_time * 0.4);
    procedural_wave = procedural_wave + vec3<f32>(wave3) * 0.15;
    
    // Layer 4: Fine detail waves
    let wave4 = sin(in.world_position.xz * 3.0 + wave_time * 1.5);
    procedural_wave = procedural_wave + vec3<f32>(wave4, 0.0) * 0.1;
    
    // Apply depth-based color tint to procedural wave
    // Deep water is darker blue, shallow water is lighter turquoise
    let depth_tint = mix(shallow_color_value().rgb, deep_color_value().rgb, saturate((depth - min_depth_value()) / (max_depth_value() - min_depth_value() + 0.001)));
    procedural_wave = procedural_wave * depth_tint * 1.5;
    
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
    let texture_water_color = mix(color1, color2, blend);
    
    // Combine procedural wave with texture
    // Use procedural wave as primary for dynamic appearance, texture for additional detail
    let water_color = vec4<f32>(mix(procedural_wave, texture_water_color.rgb, 0.3), procedural_water_color.a);
    
    // === LIGHTING (using material storage buffer instead of zone_lighting) ===
    let light_dir = normalize(light_direction_value());
    
    // Apply ambient and diffuse lighting to water
    let diffuse = max(dot(N, light_dir), 0.0);
    let ambient_light = ambient_color_value();
    let diffuse_light = diffuse_color_value() * diffuse;
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
    let fresnel = base_fresnel * fresnel_strength_value() * reflection_boost;
    
    // === SPECULAR SUN HIGHLIGHTS ===
    // Calculate specular highlight using Blinn-Phong half vector
    // The wave normal creates dynamic specular highlights
    let half_vec = normalize(light_dir + view_dir);
    let spec = pow(max(dot(N, half_vec), 0.0), 256.0);
    
    // Add specular highlight to color (bright sun reflection)
    // Use specular_intensity from settings with boost for more visible highlights
    let specular_boost = 1.3;
    let specular_color = vec3<f32>(spec * specular_intensity_value() * specular_boost);
    
    // === PHASE 3: SUBSURFACE SCATTERING ===
    // Calculate SSS for light passing through water
    let sss_contribution = calculate_sss(view_dir, light_dir, N, sss_intensity_value());
    
    // === PHASE 3: FOAM EFFECTS (IMPROVED) ===
    // Calculate wave height for foam (reuse wave position)
    let current_wave_height = wave_height(wave_pos, wave_time);
    
    // Calculate foam factor using improved Voronoi-based noise
    // Pass world position XZ for stable, natural foam patterns
    let foam_factor = calculate_foam(current_wave_height, wave_time, in.uv0, in.world_position.xz, foam_threshold_value());
    
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
    final_color = mix(final_color, foam_color, total_foam * foam_intensity_value());
    
    // === APPLY ZONE FOG ===
    // Apply fog from zone lighting to integrate water with the scene
    final_color = apply_zone_fog(final_color, in.world_position);
    
    // === DEPTH-BASED ALPHA ===
    // Use procedural water color's alpha which varies with depth
    // Shallower water is more transparent, deeper water is more opaque
    let base_alpha = procedural_water_color.a;
    
    // Ensure minimum opacity based on depth (deeper = more opaque)
    let depth_factor = saturate((depth - min_depth_value()) / (max_depth_value() - min_depth_value() + 0.001));
    let min_alpha = mix(0.3, 0.85, depth_factor); // Shallow: 0.3, Deep: 0.85
    let clamped_base_alpha = max(base_alpha, min_alpha);
    
    // Increase alpha at grazing angles (Fresnel makes water more opaque at low angles)
    // Also increase alpha where there's foam
    let final_alpha = mix(clamped_base_alpha, 1.0, fresnel * 0.5) + total_foam * 0.2;
    
    // Final color with additive blending (handled by blend state in material)
    // Fog is provided via material storage buffer
    return vec4<f32>(final_color, saturate(final_alpha));
}
