// Angelic Wing Material Extension Shader
// Extends StandardMaterial with ethereal glow and shimmer effects
//
// Features:
// - Base white/silver color with golden tips based on UV position
// - Soft blue/white glow emanating from within
// - Semi-transparent edges for ethereal look
// - Animated shimmer/pulse effect using time

#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
}
#endif

// Material extension uniforms - at binding 100 to avoid conflict with base material
struct WingExtension {
    base_color: vec4<f32>,
    glow_color: vec4<f32>,
    glow_intensity: f32,
    time: f32,
    shimmer_speed: f32,
    alpha: f32,
}

@group(2) @binding(100)
var<uniform> wing_extension: WingExtension;

/// Simple noise function for organic shimmer variation
fn hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.xyx) * 0.1031);
    p3 += dot(p3, p3.yzx + 33.33);
    return fract((p3.x + p3.y) * p3.z);
}

/// Smooth noise for more natural shimmer
fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    
    return mix(
        mix(hash(i), hash(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash(i + vec2<f32>(0.0, 1.0)), hash(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y,
    );
}

/// Fractal brownian motion for more complex patterns
fn fbm(p: vec2<f32>) -> f32 {
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    var pos = p;
    
    for (var i = 0; i < 4; i++) {
        value += amplitude * noise(pos * frequency);
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    return value;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate PbrInput from StandardMaterial
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // Get UV coordinates
    let uv = in.uv;
    
    // Calculate distance from wing tip for golden tip effect
    let tip_factor = smoothstep(0.6, 1.0, uv.y);
    
    // Golden color for wing tips
    let golden_color = vec3<f32>(1.0, 0.85, 0.5);
    
    // Mix base color with golden tips
    var base_rgb = wing_extension.base_color.rgb;
    base_rgb = mix(base_rgb, golden_color, tip_factor * 0.6);
    
    // Calculate feather-like pattern
    let feather_pattern = sin(uv.x * 30.0 + uv.y * 5.0) * 0.5 + 0.5;
    let feather_mask = smoothstep(0.3, 0.7, feather_pattern);
    
    // Edge fade for ethereal look
    let edge_x = 1.0 - abs(uv.x - 0.5) * 2.0;
    let edge_y = 1.0 - abs(uv.y - 0.5) * 2.0;
    let edge_factor = edge_x * edge_y;
    let edge_fade = smoothstep(0.0, 0.3, edge_factor);
    
    // Animated shimmer effect
    let shimmer_time = wing_extension.time * wing_extension.shimmer_speed;
    let shimmer_uv = uv * 8.0 + vec2<f32>(shimmer_time * 0.5, shimmer_time * 0.3);
    let shimmer_noise = fbm(shimmer_uv);
    
    // Pulsing glow effect
    let pulse = sin(shimmer_time * 2.0) * 0.5 + 0.5;
    let pulse_intensity = mix(0.7, 1.0, pulse);
    
    // Fresnel-like glow based on view angle
    let view_dir = normalize(in.world_position.xyz - in.world_position.xyz);
    let fresnel = pow(1.0 - max(dot(view_dir, pbr_input.material.normal), 0.0), 2.0);
    
    // Combine glow effects
    let glow_amount = wing_extension.glow_intensity * (
        fresnel * 0.5 +
        shimmer_noise * 0.3 +
        pulse_intensity * 0.2
    );
    
    // Apply glow to base color
    var final_rgb = base_rgb;
    final_rgb = mix(final_rgb, wing_extension.glow_color.rgb, glow_amount);
    
    // Add sparkle highlights
    let sparkle = smoothstep(0.85, 1.0, shimmer_noise);
    final_rgb += vec3<f32>(1.0) * sparkle * 0.3;
    
    // Calculate alpha with edge fade
    var final_alpha = wing_extension.alpha;
    final_alpha *= edge_fade;
    final_alpha *= mix(0.8, 1.0, feather_mask);
    final_alpha *= smoothstep(0.0, 0.15, edge_factor);
    
    // Override the base color with our custom color
    pbr_input.material.base_color = vec4<f32>(final_rgb, final_alpha);
    
    // Apply alpha discard
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // Apply lighting
    out.color = apply_pbr_lighting(pbr_input);
    // Apply post processing
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif

    return out;
}
