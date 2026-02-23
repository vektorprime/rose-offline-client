// Angelic Wing Material Shader
// Provides ethereal glow and shimmer effects for angelic wings
//
// Features:
// - Base white/silver color with golden tips based on UV position
// - Soft blue/white glow emanating from within
// - Semi-transparent edges for ethereal look
// - Animated shimmer/pulse effect using time

#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::get_world_from_local
#import bevy_pbr::view_transformations::position_world_to_clip

// Vertex input structure
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
}

// Vertex output / Fragment input structure
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) world_normal: vec3<f32>,
}

// Material uniforms - packed according to AsBindGroup layout
// All uniforms are in binding 0, packed together
struct WingUniforms {
    base_color: vec4<f32>,      // 16 bytes
    glow_color: vec4<f32>,      // 16 bytes
    glow_intensity: f32,        // 4 bytes
    time: f32,                  // 4 bytes
    shimmer_speed: f32,         // 4 bytes
    alpha: f32,                 // 4 bytes
}

@group(2) @binding(0)
var<uniform> uniforms: WingUniforms;

/// Vertex shader - transforms wing mesh to screen space
@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform position to world space
    let world_from_local = get_world_from_local(mesh);
    let world_position = world_from_local * vec4<f32>(vertex.position, 1.0);
    out.world_position = world_position.xyz;
    
    // Transform to clip space
    out.clip_position = position_world_to_clip(world_position.xyz, view);
    
    // Pass through UV coordinates
    out.uv = vertex.uv;
    
    // Transform normal to world space
    let normal_matrix = mat3x3<f32>(
        mesh.world_from_local[0].xyz,
        mesh.world_from_local[1].xyz,
        mesh.world_from_local[2].xyz,
    );
    out.world_normal = normalize(normal_matrix * vertex.normal);
    
    return out;
}

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

/// Fragment shader - creates the ethereal wing appearance
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Normalize the UV coordinates for consistent effects across wing size
    let uv = in.uv;
    
    // Calculate distance from wing tip (top of UV) for golden tip effect
    // UV.y = 0 is wing base (near body), UV.y = 1 is wing tip
    let tip_factor = smoothstep(0.6, 1.0, uv.y);
    
    // Golden color for wing tips
    let golden_color = vec3<f32>(1.0, 0.85, 0.5);
    
    // Mix base color with golden tips
    var base_rgb = uniforms.base_color.rgb;
    base_rgb = mix(base_rgb, golden_color, tip_factor * 0.6);
    
    // Calculate feather-like pattern using UV
    // Create vertical feather lines
    let feather_pattern = sin(uv.x * 30.0 + uv.y * 5.0) * 0.5 + 0.5;
    let feather_mask = smoothstep(0.3, 0.7, feather_pattern);
    
    // Edge fade for ethereal look - stronger at edges
    let edge_x = 1.0 - abs(uv.x - 0.5) * 2.0;
    let edge_y = 1.0 - abs(uv.y - 0.5) * 2.0;
    let edge_factor = edge_x * edge_y;
    let edge_fade = smoothstep(0.0, 0.3, edge_factor);
    
    // Animated shimmer effect using time and noise
    let shimmer_time = uniforms.time * uniforms.shimmer_speed;
    let shimmer_uv = uv * 8.0 + vec2<f32>(shimmer_time * 0.5, shimmer_time * 0.3);
    let shimmer_noise = fbm(shimmer_uv);
    
    // Pulsing glow effect
    let pulse = sin(shimmer_time * 2.0) * 0.5 + 0.5;
    let pulse_intensity = mix(0.7, 1.0, pulse);
    
    // Calculate glow based on view angle (fresnel-like effect)
    let view_dir = normalize(view.world_position.xyz - in.world_position);
    let normal = normalize(in.world_normal);
    let fresnel = pow(1.0 - max(dot(view_dir, normal), 0.0), 2.0);
    
    // Combine glow effects
    let glow_amount = uniforms.glow_intensity * (
        fresnel * 0.5 +                    // Edge glow
        shimmer_noise * 0.3 +              // Shimmer variation
        pulse_intensity * 0.2              // Pulse effect
    );
    
    // Add glow color to base
    var final_rgb = base_rgb;
    final_rgb = mix(final_rgb, uniforms.glow_color.rgb, glow_amount);
    
    // Add bright highlights for sparkle effect
    let sparkle_threshold = 0.85;
    let sparkle = smoothstep(sparkle_threshold, 1.0, shimmer_noise);
    final_rgb += vec3<f32>(1.0) * sparkle * 0.3;
    
    // Calculate final alpha with edge fade
    var final_alpha = uniforms.alpha;
    final_alpha *= edge_fade;
    final_alpha *= mix(0.8, 1.0, feather_mask);  // Slight variation from feather pattern
    
    // Reduce alpha at very edges for soft look
    final_alpha *= smoothstep(0.0, 0.15, edge_factor);
    
    return vec4<f32>(final_rgb, final_alpha);
}
