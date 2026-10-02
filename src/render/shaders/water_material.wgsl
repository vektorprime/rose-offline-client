//! Water material shader for ROSE Online
//!
//! Physically based water surface drawn in the main HDR pass with
//! premultiplied alpha over the opaque scene (the lake bed):
//!   out = reflection * F + (1 - F) * (in-scattered body light) + glint + ...
//!   alpha = 1 - (1 - F) * T   (T = transmittance down to the bed and back)
//! so the bed shows through by exactly (1 - F) * T.
//!
//! - Waves: 2-8 directional waves (4 octaves) with analytic slopes, noise
//!   wave-group envelopes and a noise warp on the short octaves (no regular
//!   stripes or glitter lattice); octaves smaller than a few pixels are faded
//!   out and their slope variance moves into the glint roughness, so distant
//!   water does not shimmer.
//! - Fresnel: Schlick with F0 = 0.02 (water IOR 1.33), scaled by the
//!   Fresnel Strength setting (0.5 = physical).
//! - Reflection: the mirrored camera's texture, offset by where the
//!   wave-tilted reflection ray lands (exact for distant scenery). Pixels the
//!   reflection camera left empty (alpha 0: sky, no atmosphere on that camera)
//!   and disabled reflections use an analytic sky lit by the view's
//!   directional lights, blended toward the starry sky's night color by the
//!   night factor.
//! - Sun/moon glint: GGX from the brightest directional light.
//! - Absorption along the refracted path through a procedural depth field
//!   (terrain is not in the depth prepass); objects/characters that are in
//!   the depth prepass get their exact depth, a soft contact edge and foam.
//! - Caustics on the visible bed, light scattered through wave crests,
//!   whitecaps (only with steep waves).
//! - Underside (camera underwater): Snell's window with total internal
//!   reflection.
//!
//! All lighting comes from Bevy's `lights` uniform and is scaled by
//! `view.exposure`, like the PBR materials, so the water follows the scene's
//! sun, moon, sky fill and ambient at every time of day.

#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings::{view, globals, lights}
#ifdef DEPTH_PREPASS
#import bevy_pbr::prepass_utils::prepass_depth
#endif

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
// All per-material values are packed into a read-only storage buffer
// (see WaterMaterial::unprepared_bind_group):
// [0] waves: wave_amplitude, wave_frequency, wave_speed, wave_layers
// [1] surface: fresnel_strength, specular_intensity, sss_intensity, refraction_strength
// [2] foam/caustics: foam_intensity, foam_threshold, caustics_intensity, caustics_scale
// [3] depth: min_depth, max_depth, shallow_threshold, bottom_visibility
// [4] deep_color
// [5] shallow_color
// [6] depth_gradient_scale.xy, caustics_speed, water_surface_y
// [7] reflection: enabled, debug_show_reflection, status, sky night factor
@group(#{MATERIAL_BIND_GROUP}) @binding(0)
var<storage, read> water_material_data: array<vec4<f32>, 8>;

// Planar reflection render target (rendered by the mirrored reflection camera;
// linear HDR, exposure-scaled like the main view, alpha 0 where nothing rendered)
@group(#{MATERIAL_BIND_GROUP}) @binding(1)
var reflection_texture: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2)
var reflection_sampler: sampler;

const PI: f32 = 3.141592653589793;
const WATER_IOR: f32 = 1.333;
// Reflectance at normal incidence: ((1.333 - 1) / (1.333 + 1))^2
const WATER_F0: f32 = 0.02;
// The deep/shallow settings colors are artist colors; the diffuse reflectance
// of a real water body is far lower (a few percent).
const BODY_ALBEDO_SCALE: f32 = 0.2;
const FOAM_ALBEDO: f32 = 0.8;
// Reflectance assumed for the lake bed lit by caustics.
const BED_ALBEDO: f32 = 0.3;
// RMS slope of the capillary ripples below the smallest modelled wave.
const MICRO_SLOPE_RMS: f32 = 0.045;
// Caps HDR glints so bloom and auto exposure stay sane.
const GLINT_MAX: f32 = 40.0;
// Vertical optical depth of a clear atmosphere (Rayleigh per RGB, Mie).
const SKY_RAYLEIGH: vec3<f32> = vec3<f32>(0.047, 0.108, 0.265);
const SKY_MIE: f32 = 0.03;
const SKY_MIE_G: f32 = 0.76;
// Brings the single-scattering sky up to the brightness of Bevy's
// multiple-scattering atmosphere in the main view.
const SKY_MULTISCATTER: f32 = 2.5;
// Average of the starry sky sphere's night background (starry_sky.wgsl
// nebula, not exposure-scaled), which covers the atmosphere at night.
const NIGHT_SKY: vec3<f32> = vec3<f32>(0.018, 0.012, 0.033);

// === MATERIAL DATA ACCESSORS ===

fn wave_amplitude_value() -> f32 {
    return water_material_data[0].x;
}

fn wave_frequency_value() -> f32 {
    return water_material_data[0].y;
}

fn wave_speed_value() -> f32 {
    return water_material_data[0].z;
}

fn wave_layers_value() -> f32 {
    return water_material_data[0].w;
}

fn fresnel_strength_value() -> f32 {
    return water_material_data[1].x;
}

fn specular_intensity_value() -> f32 {
    return water_material_data[1].y;
}

fn sss_intensity_value() -> f32 {
    return water_material_data[1].z;
}

// refraction_strength drives the reflection distortion; 0.2 (the slider
// maximum) is physically correct for distant scenery, the default 0.05 keeps
// a quarter of that so nearby reflections stay readable.
fn reflection_distortion_value() -> f32 {
    return water_material_data[1].w * 5.0;
}

fn foam_intensity_value() -> f32 {
    return water_material_data[2].x;
}

fn foam_threshold_value() -> f32 {
    return water_material_data[2].y;
}

fn caustics_intensity_value() -> f32 {
    return water_material_data[2].z;
}

fn caustics_scale_value() -> f32 {
    return water_material_data[2].w;
}

fn min_depth_value() -> f32 {
    return water_material_data[3].x;
}

fn max_depth_value() -> f32 {
    return water_material_data[3].y;
}

fn shallow_threshold_value() -> f32 {
    return water_material_data[3].z;
}

fn bottom_visibility_value() -> f32 {
    return water_material_data[3].w;
}

fn deep_color_value() -> vec3<f32> {
    return water_material_data[4].rgb;
}

fn shallow_color_value() -> vec3<f32> {
    return water_material_data[5].rgb;
}

fn depth_gradient_scale_value() -> vec2<f32> {
    return water_material_data[6].xy;
}

fn caustics_speed_value() -> f32 {
    return water_material_data[6].z;
}

fn reflection_enabled_value() -> f32 {
    return water_material_data[7].x;
}

fn debug_show_reflection_value() -> f32 {
    return water_material_data[7].y;
}

fn reflection_status_value() -> f32 {
    return water_material_data[7].z;
}

// StarrySkySettings::night_factor: how much the starry sky sphere covers the
// atmosphere (0 = day, 1 = night).
fn sky_night_factor_value() -> f32 {
    return water_material_data[7].w;
}

// === NOISE ===

fn hash12(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

// Smooth value noise in [0, 1].
fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash12(i);
    let b = hash12(i + vec2<f32>(1.0, 0.0));
    let c = hash12(i + vec2<f32>(0.0, 1.0));
    let d = hash12(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// === OPTICS ===

// Schlick Fresnel for the air/water interface. `cos_theta` is measured on the
// air side (for light leaving the water, pass the transmitted angle).
fn fresnel_schlick(cos_theta: f32) -> f32 {
    let m = 1.0 - saturate(cos_theta);
    let m2 = m * m;
    return WATER_F0 + (1.0 - WATER_F0) * m2 * m2 * m;
}

// === WAVES ===

struct WaveSample {
    // d(height)/d(x, z)
    slope: vec2<f32>,
    // Height-weighted crest value, normalized to roughly -1..1 at the end
    crest: f32,
    crest_weight: f32,
    // Slope variance of octaves faded out by the pixel footprint
    lost_slope_var: f32,
}

// One directional deep-water wave. `steepness` is its mean peak slope (A * k),
// so every wavelength contributes the same slope and none dominates the
// normal; `envelope` is the local wave-group strength (mean ~1).
fn add_wave(
    acc: ptr<function, WaveSample>,
    p: vec2<f32>,
    time: f32,
    footprint: f32,
    dir: vec2<f32>,
    wavelength: f32,
    phase: f32,
    steepness: f32,
    envelope: f32,
) {
    let k = 6.2831853 / wavelength;
    // Deep-water dispersion: longer waves travel faster.
    let omega = sqrt(9.81 * k);
    let theta = k * dot(dir, p) - omega * time + phase;
    // Fade octaves with fewer than ~2-6 pixels per wavelength: they would
    // alias into shimmer. Their slopes become glint roughness instead.
    let fade = smoothstep(2.0, 6.0, wavelength / footprint);
    let local_steepness = steepness * envelope;
    (*acc).slope += dir * (local_steepness * fade * cos(theta));
    let height = steepness / k;
    (*acc).crest += height * envelope * fade * sin(theta);
    // Normalized by the mean height, so crests grow inside wave groups.
    (*acc).crest_weight += height;
    (*acc).lost_slope_var += (1.0 - fade * fade) * local_steepness * local_steepness * 0.5;
}

fn sample_waves(position: vec2<f32>, time: f32, footprint: f32) -> WaveSample {
    var acc = WaveSample(vec2<f32>(0.0), 0.0, 0.0, 0.0);
    // wave_frequency 2.0 = the wavelengths below; lower = longer swell.
    let length_scale = 2.0 / max(wave_frequency_value(), 0.05);
    // wave_amplitude 0.5 = slope 0.06 per wave (light breeze, RMS ~0.12 for 8).
    let steepness = min(0.12 * max(wave_amplitude_value(), 0.0), 0.16);
    let layers = wave_layers_value();
    // Slow, large domain warp bends the long crests.
    let p = position + 0.6 * vec2<f32>(
        sin(position.y * 0.071 + time * 0.13),
        sin(position.x * 0.083 - time * 0.11),
    );
    // Short octaves get a noise warp of about a meter that bends their crests,
    // so sun glitter does not line up in a lattice.
    let drift = vec2<f32>(time * 0.05, time * 0.03);
    let short_warp = vec2<f32>(
        value_noise(p * 0.31 + drift),
        value_noise(p * 0.31 + vec2<f32>(7.7, 3.1) - drift),
    ) - 0.5;
    let p_short = p + short_warp * 1.6;
    // Wave groups: each octave's steepness varies (mean ~1) over patches about
    // seven wavelengths wide, so crests do not form endless parallel stripes.
    let inv_scale = 1.0 / length_scale;
    let group0 = 0.4 + 1.2 * value_noise(p * (inv_scale / 37.0) + vec2<f32>(3.7, 9.2) + drift * 0.2);
    // Directions are spread around a fixed wind direction (0.8, 0.6). The
    // longest octaves are a bit gentler (as in real wave spectra), which keeps
    // distant water from turning into regular stripes of reflected sky.
    add_wave(&acc, p, time, footprint, vec2<f32>(0.800, 0.600), 6.3 * length_scale, 0.0, steepness * 0.7, group0);
    add_wave(&acc, p, time, footprint, vec2<f32>(0.311, 0.950), 4.4 * length_scale, 1.7, steepness * 0.7, group0);
    if (layers > 1.5) {
        let group1 = 0.4 + 1.2 * value_noise(p * (inv_scale / 13.7) + vec2<f32>(11.3, 2.9) - drift * 0.4);
        add_wave(&acc, p_short, time, footprint, vec2<f32>(0.979, 0.206), 2.3 * length_scale, 4.1, steepness * 0.85, group1);
        add_wave(&acc, p_short, time, footprint, vec2<f32>(-0.290, 0.957), 1.6 * length_scale, 2.9, steepness * 0.85, group1);
    }
    if (layers > 2.5) {
        let group2 = 0.4 + 1.2 * value_noise(p * (inv_scale / 5.0) + vec2<f32>(5.1, 17.6) + drift);
        add_wave(&acc, p_short, time, footprint, vec2<f32>(0.617, 0.787), 0.83 * length_scale, 0.6, steepness, group2);
        add_wave(&acc, p_short, time, footprint, vec2<f32>(0.950, -0.311), 0.59 * length_scale, 5.2, steepness, group2);
    }
    if (layers > 3.5) {
        let group3 = 0.4 + 1.2 * value_noise(p * (inv_scale / 1.9) + vec2<f32>(23.9, 6.4) - drift);
        add_wave(&acc, p_short, time, footprint, vec2<f32>(-0.730, 0.684), 0.31 * length_scale, 3.3, steepness, group3);
        add_wave(&acc, p_short, time, footprint, vec2<f32>(0.998, -0.055), 0.22 * length_scale, 1.1, steepness, group3);
    }
    acc.crest = acc.crest / max(acc.crest_weight, 1.0e-4);
    return acc;
}

// === LIGHTS ===

struct WaterLighting {
    // Irradiance entering the water (lux): every directional light (sun, moon,
    // sky fill) through the flat surface, plus ambient.
    e_down: vec3<f32>,
    // Brightest directional light (the sun by day, the moon by night): color
    // premultiplied by illuminance, and the direction toward it.
    key_color: vec3<f32>,
    key_dir: vec3<f32>,
}

fn gather_lights() -> WaterLighting {
    var out: WaterLighting;
    // Bevy shades ambient as albedo * ambient_color, i.e. irradiance pi * ambient.
    out.e_down = lights.ambient_color.rgb * PI;
    out.key_color = vec3<f32>(0.0);
    out.key_dir = vec3<f32>(0.0, 1.0, 0.0);
    var key_luminance = 0.0;
    for (var i = 0u; i < lights.n_directional_lights; i = i + 1u) {
        let color = lights.directional_lights[i].color.rgb;
        let to_light = lights.directional_lights[i].direction_to_light;
        let cos_l = max(to_light.y, 0.0);
        out.e_down += color * cos_l * (1.0 - fresnel_schlick(cos_l));
        let luminance = dot(color, vec3<f32>(0.2126, 0.7152, 0.0722));
        if (luminance > key_luminance) {
            key_luminance = luminance;
            out.key_color = color;
            out.key_dir = to_light;
        }
    }
    return out;
}

// === SKY ===

// Relative air mass along a ray with the given zenith cosine (~1 at the
// zenith, ~40 at the horizon).
fn air_mass(cos_zenith: f32) -> f32 {
    let c = max(cos_zenith, 0.0);
    return 1.0 / (c + 0.025 * exp(-11.0 * c));
}

// Cheap single-scattering clear sky lit by the view's directional lights, in
// the same exposure-scaled units as the scene. Bevy's atmosphere (main view)
// is lit by the same lights, so this tracks its brightness and color through
// the day, at sunset and under the moon.
fn sky_radiance(dir: vec3<f32>) -> vec3<f32> {
    let extinction = SKY_RAYLEIGH + vec3<f32>(SKY_MIE);
    let in_scatter = 1.0 - exp(-extinction * air_mass(dir.y));
    var radiance = vec3<f32>(0.0);
    for (var i = 0u; i < lights.n_directional_lights; i = i + 1u) {
        let to_light = lights.directional_lights[i].direction_to_light;
        // Light reaching the scattering layer: reddens and fades as it sets.
        let light_transmittance = exp(-extinction * air_mass(to_light.y) * 0.5)
            * smoothstep(-0.1, 0.05, to_light.y);
        let mu = dot(dir, to_light);
        let rayleigh_phase = 0.0596831 * (1.0 + mu * mu);
        let g2 = SKY_MIE_G * SKY_MIE_G;
        let mie_base = max(1.0 + g2 - 2.0 * SKY_MIE_G * mu, 1.0e-4);
        let mie_phase = 0.0795775 * (1.0 - g2) / (mie_base * sqrt(mie_base));
        let phase = (SKY_RAYLEIGH * rayleigh_phase + vec3<f32>(SKY_MIE * mie_phase)) / extinction;
        radiance += lights.directional_lights[i].color.rgb * light_transmittance * phase;
    }
    return radiance * in_scatter * (SKY_MULTISCATTER * view.exposure);
}

// === PLANAR REFLECTION ===

// The reflection camera is the main camera mirrored across the water plane,
// so a fragment's own screen position shows what a flat mirror reflects
// there. A wave-tilted reflection ray R lands where the main camera would see
// the mirrored direction (R.x, -R.y, R.z); the offset between the two
// projections is exact for distant scenery and slightly overstated for near
// objects (scaled by the distortion setting).
// `atmosphere` fills pixels the reflection camera left empty (the starry sky
// sphere, when visible, is in the texture itself); `visible_sky` (atmosphere
// blended with the night sky) is used where the offset leaves the texture.
fn planar_reflection(
    frag_coord: vec2<f32>,
    V: vec3<f32>,
    R: vec3<f32>,
    atmosphere: vec3<f32>,
    visible_sky: vec3<f32>,
) -> vec3<f32> {
    let uv_frag = (frag_coord - view.viewport.xy) / view.viewport.zw;
    let flat_clip = view.clip_from_world * vec4<f32>(-V, 0.0);
    let wave_clip = view.clip_from_world * vec4<f32>(R.x, -R.y, R.z, 0.0);
    var offset = vec2<f32>(0.0);
    if (flat_clip.w > 1.0e-4 && wave_clip.w > 1.0e-4) {
        let ndc_offset = wave_clip.xy / wave_clip.w - flat_clip.xy / flat_clip.w;
        offset = clamp(
            vec2<f32>(ndc_offset.x, -ndc_offset.y) * (0.5 * reflection_distortion_value()),
            vec2<f32>(-0.08),
            vec2<f32>(0.08),
        );
    }
    let uv = uv_frag + offset;
    // Fade to the analytic sky where the distortion leaves the texture.
    let outside = max(max(-uv.x, uv.x - 1.0), max(-uv.y, uv.y - 1.0));
    let inside_weight = 1.0 - saturate(outside / 0.02);
    let texel = textureSampleLevel(
        reflection_texture,
        reflection_sampler,
        clamp(uv, vec2<f32>(0.0005), vec2<f32>(0.9995)),
        0.0,
    );
    // Alpha 0 = nothing rendered (sky); clouds, the starry sky and other
    // blended geometry composite over the atmosphere by their coverage.
    let scene = texel.rgb + (1.0 - saturate(texel.a)) * atmosphere;
    return mix(visible_sky, scene, inside_weight);
}

// === DEPTH ===

// Smooth procedural water depth (m) between min_depth and max_depth, biased to
// the shallow end: most ROSE water is rivers and lakes, and the real bed depth
// is unknown here (terrain does not write the depth prepass).
fn procedural_depth(xz: vec2<f32>) -> f32 {
    let p = xz * depth_gradient_scale_value();
    let n = value_noise(p) * 0.65 + value_noise(p * 2.07 + vec2<f32>(17.3, 41.9)) * 0.35;
    let shaped = smoothstep(0.2, 0.8, n);
    return mix(min_depth_value(), max_depth_value(), shaped * shaped * shaped);
}

// Distance along the view ray from the water surface to the depth-prepass
// geometry behind it (objects, characters, boats). Huge when nothing in the
// prepass is there (sky, terrain).
fn prepass_thickness(frag_coord: vec4<f32>, world_pos: vec3<f32>) -> f32 {
#ifdef DEPTH_PREPASS
    let depth = prepass_depth(frag_coord, 0u);
    if (depth <= 0.0) {
        return 1.0e6;
    }
    // View-space z depends only on the depth for a perspective projection.
    let scene_view = view.view_from_clip * vec4<f32>(0.0, 0.0, depth, 1.0);
    let scene_z = scene_view.z / scene_view.w;
    let water_z = (view.view_from_world * vec4<f32>(world_pos, 1.0)).z;
    let ray_per_z = length(world_pos - view.world_position) / max(-water_z, 1.0e-4);
    return max(water_z - scene_z, 0.0) * ray_per_z;
#else
    return 1.0e6;
#endif
}

// === EFFECTS ===

// Two drifting layers of ridged value noise; their product leaves thin bright
// filaments like the focused light on a shallow bed.
fn caustics_pattern(p: vec2<f32>, t: f32) -> f32 {
    let a = 1.0 - abs(value_noise(p + vec2<f32>(t * 0.31, t * 0.17)) * 2.0 - 1.0);
    // Second layer rotated ~37 degrees so the ridges do not follow the noise grid.
    let q = vec2<f32>(p.x * 0.799 - p.y * 0.602, p.x * 0.602 + p.y * 0.799) * 1.37;
    let b = 1.0 - abs(value_noise(q + vec2<f32>(5.3 - t * 0.23, 1.9 + t * 0.29)) * 2.0 - 1.0);
    return pow(a * b, 6.0) * 2.0;
}

// Camera below the surface looking up: Snell's window. Inside the window the
// above-water scene (already in the target) shows through; beyond the
// critical angle (~48.6 degrees) the surface is a mirror of the dim water body.
fn underside_color(N: vec3<f32>, V: vec3<f32>, lighting: WaterLighting) -> vec4<f32> {
    let cos_i = saturate(dot(V, -N));
    let sin_t2 = WATER_IOR * WATER_IOR * (1.0 - cos_i * cos_i);
    var reflectance = 1.0;
    if (sin_t2 < 1.0) {
        reflectance = fresnel_schlick(sqrt(1.0 - sin_t2));
    }
    let inside = deep_color_value() * BODY_ALBEDO_SCALE * lighting.e_down / PI * view.exposure;
    return vec4<f32>(inside * reflectance, reflectance);
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
    let world_pos = in.world_position.xyz;
    // World meters per pixel, for wave filtering. Derivatives need uniform
    // control flow, so this comes first.
    let footprint = max(length(fwidth(world_pos.xz)), 1.0e-4);

    let time = globals.time * wave_speed_value();
    let waves = sample_waves(world_pos.xz, time, footprint);
    // Water planes are horizontal: the wave normal is built around +Y.
    var N = normalize(vec3<f32>(-waves.slope.x, 1.0, -waves.slope.y));
    let V = normalize(view.world_position - world_pos);
    let lighting = gather_lights();
    let exposure = view.exposure;

    // Camera below the plane (the material is double-sided).
    if (V.y < 0.0) {
        return underside_color(N, V, lighting);
    }

    // Keep the normal facing the viewer at grazing angles (a back-facing
    // micro-normal would produce black or sparkling pixels).
    N = normalize(N + V * max(0.05 - dot(N, V), 0.0));
    let n_dot_v = max(dot(N, V), 1.0e-3);

    // === REFLECTION ===
    let fresnel = saturate(fresnel_schlick(n_dot_v) * fresnel_strength_value() * 2.0);
    // Reflected ray, kept above the horizon (below it, it would hit the water again).
    var R = reflect(-V, N);
    R.y = max(R.y, 0.01);
    R = normalize(R);
    let atmosphere = sky_radiance(R);
    // At night the starry sky sphere covers the atmosphere in the main view.
    let visible_sky = mix(atmosphere, NIGHT_SKY, saturate(sky_night_factor_value()));
    var reflected = visible_sky;
    // Status 0 = reflection camera disabled this frame: its texture is stale.
    if (reflection_enabled_value() > 0.5 && reflection_status_value() > 0.5) {
        reflected = planar_reflection(in.clip_position.xy, V, R, atmosphere, visible_sky);
    }

    // DEBUG: raw reflection texture, or the reflection camera status as a color:
    // red = disabled, orange = no entities visible, magenta = suspiciously few.
    if (debug_show_reflection_value() > 0.5) {
        let uv_frag = (in.clip_position.xy - view.viewport.xy) / view.viewport.zw;
        var debug_color = textureSampleLevel(reflection_texture, reflection_sampler, uv_frag, 0.0).rgb;
        let status = reflection_status_value();
        if (status < 0.5) {
            debug_color = vec3<f32>(1.0, 0.0, 0.0);
        } else if (status < 1.5) {
            debug_color = vec3<f32>(1.0, 0.5, 0.0);
        } else if (status < 2.5) {
            debug_color = vec3<f32>(1.0, 0.0, 1.0);
        }
        return vec4<f32>(debug_color, 1.0);
    }

    // === TRANSMISSION AND ABSORPTION ===
    // Refracted view ray into the water (never total internal reflection from air).
    let refracted = refract(-V, N, 1.0 / WATER_IOR);
    let cos_t = max(-refracted.y, 0.05);
    // Exact depth of a prepass object behind the surface if it is shallower
    // than the procedural field (rocks, pillars, boat hulls, wading legs).
    let thickness = prepass_thickness(in.clip_position, world_pos);
    let object_depth = thickness * V.y;
    let depth = min(procedural_depth(world_pos.xz), object_depth);
    let path = depth / cos_t;
    // Bottom visibility is the transmittance at shallow_threshold depth
    // (straight down), which fixes the absorption coefficient.
    let sigma = -log(clamp(bottom_visibility_value(), 0.01, 0.99))
        / max(shallow_threshold_value(), 0.1);
    // Red is absorbed fastest, blue slightly slower than green.
    let extinction = sigma * vec3<f32>(2.2, 1.0, 0.85);
    let transmittance = exp(-extinction * path);
    let bed_transmittance = transmittance.g;

    // Light scattered back up by the water body: more of the body color the
    // longer the path, lit by everything that enters the surface.
    let body_albedo = mix(shallow_color_value(), deep_color_value(), 1.0 - bed_transmittance)
        * BODY_ALBEDO_SCALE;
    let body = (vec3<f32>(1.0) - transmittance) * body_albedo * lighting.e_down / PI * exposure;

    // Share of the bed (the opaque scene behind the surface) that stays visible.
    let bed_weight = (1.0 - fresnel) * bed_transmittance;

    // === CAUSTICS on the visible bed ===
    let key_dir = lighting.key_dir;
    var caustic_light = vec3<f32>(0.0);
    if (bed_weight > 0.02 && caustics_intensity_value() > 0.0 && key_dir.y > 0.0) {
        // Where the refracted ray meets the bed: caustics stay put on the bed
        // and show parallax against the surface.
        let bed_xz = world_pos.xz + refracted.xz / cos_t * depth;
        let pattern = caustics_pattern(
            bed_xz * (caustics_scale_value() * 10.0),
            globals.time * caustics_speed_value(),
        );
        let light_to_bed = exp(-extinction * depth / max(key_dir.y, 0.1));
        caustic_light = lighting.key_color * light_to_bed
            * (key_dir.y * BED_ALBEDO / PI * pattern * caustics_intensity_value() * 2.0 * exposure * bed_weight);
    }

    // === SUN / MOON GLINT (GGX) ===
    var glint = vec3<f32>(0.0);
    let n_dot_l = dot(N, key_dir);
    if (n_dot_l > 0.0 && key_dir.y > -0.05) {
        let H = normalize(key_dir + V);
        let n_dot_h = saturate(dot(N, H));
        let v_dot_h = saturate(dot(V, H));
        // Micro ripples plus the octaves filtered out at this distance.
        let alpha2 = clamp(
            2.0 * (MICRO_SLOPE_RMS * MICRO_SLOPE_RMS + waves.lost_slope_var),
            1.0e-4,
            0.5,
        );
        let d_base = n_dot_h * n_dot_h * (alpha2 - 1.0) + 1.0;
        let distribution = alpha2 / (PI * d_base * d_base);
        // Height-correlated Smith visibility.
        let vis_v = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - alpha2) + alpha2);
        let vis_l = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - alpha2) + alpha2);
        let visibility = 0.5 / max(vis_v + vis_l, 1.0e-4);
        let specular = distribution * visibility * fresnel_schlick(v_dot_h) * n_dot_l;
        glint = min(
            lighting.key_color * (specular * specular_intensity_value() * 2.0 * exposure),
            vec3<f32>(GLINT_MAX),
        );
    }

    // === LIGHT THROUGH WAVE CRESTS ===
    // Looking toward a low sun/moon, light passes through the thin crests and
    // tints them with the shallow water color.
    var crest_scatter = vec3<f32>(0.0);
    let view_h = -V.xz;
    let light_h = key_dir.xz;
    let h_lengths = length(view_h) * length(light_h);
    if (sss_intensity_value() > 0.0 && h_lengths > 1.0e-4) {
        let toward_light = saturate(dot(view_h, light_h) / h_lengths);
        let crest = saturate(waves.crest * 0.5 + 0.5);
        let amount = pow(toward_light, 4.0) * crest * crest * (1.0 - n_dot_v)
            * smoothstep(-0.05, 0.15, key_dir.y);
        crest_scatter = shallow_color_value() * lighting.key_color
            * (amount * sss_intensity_value() * 0.5 / PI * exposure * (1.0 - fresnel));
    }

    // === FOAM ===
    // Whitecaps need wind: they only appear with steep waves (ocean settings).
    let crest01 = saturate(waves.crest * 0.5 + 0.5);
    let whitecap = smoothstep(foam_threshold_value(), foam_threshold_value() + 0.15, crest01)
        * smoothstep(0.4, 1.2, wave_amplitude_value());
    // Where a prepass object breaks the surface.
    let contact = 1.0 - smoothstep(0.0, 0.35, object_depth);
    var foam_cover = 0.0;
    if (whitecap + contact > 0.0 && foam_intensity_value() > 0.0) {
        let breakup = value_noise(world_pos.xz * 1.7 + vec2<f32>(time * 0.21, -time * 0.17)) * 0.6
            + value_noise(world_pos.xz * 4.3 - vec2<f32>(time * 0.13, time * 0.29)) * 0.4;
        foam_cover = saturate(
            (whitecap * smoothstep(0.45, 0.75, breakup)
                + contact * smoothstep(0.3, 0.6, breakup + contact * 0.3))
                * foam_intensity_value() * 1.5,
        );
    }
    let foam = FOAM_ALBEDO * lighting.e_down / PI * exposure;

    // === COMBINE (premultiplied alpha) ===
    var color = fresnel * reflected + (1.0 - fresnel) * body + caustic_light + crest_scatter + glint;
    var alpha = 1.0 - bed_weight;
    color = mix(color, foam, foam_cover);
    alpha = mix(alpha, 1.0, foam_cover);

    // Soft edge where a prepass object meets the surface (no hard cut line).
    let contact_edge = smoothstep(0.0, 0.06, thickness);
    return vec4<f32>(color * contact_edge, saturate(alpha) * contact_edge);
}
