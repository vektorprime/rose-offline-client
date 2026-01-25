// Post-processing shader for screen-space effects
// Includes bloom, tone mapping, and color grading

#import bevy_render::view View

@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var input_texture: texture_2d<f32>;
@group(1) @binding(1)
var input_sampler: sampler;

// Post-processing parameters
struct PostProcessingParams {
    exposure: f32,
    gamma: f32,
    contrast: f32,
    saturation: f32,
    bloom_intensity: f32,
    bloom_threshold: f32,
    vignette_strength: f32,
    vignette_radius: f32,
};

var<push_constant> params: PostProcessingParams;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @location(0) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Fullscreen quad vertices
    var pos: vec2<f32>;
    if (input.vertex_idx == 0u) { pos = vec2<f32>(-1.0, -1.0); }
    else if (input.vertex_idx == 1u) { pos = vec2<f32>(1.0, -1.0); }
    else { pos = vec2<f32>(-1.0, 1.0); }
    
    out.position = vec4<f32>(pos, 0.0, 1.0);
    out.uv = input.uv;
    return out;
}

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Tone mapping functions
fn aces_tonemap(color: vec3<f32>) -> vec3<f32> {
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;
    
    let x = (color.r * 0.57135 + color.g * 0.299 + color.b * 0.054) * 0.5;
    let y = color.g * 0.5;
    let z = (color.r * 0.086 + color.g * 0.089 + color.b * 0.445) * 0.5;
    
    let r = x * (A * x + B) / (x * (A * x + C) + D * E);
    let g = y * (A * y + B) / (y * (A * y + C) + D * E);
    let b = z * (A * z + B) / (z * (A * z + C) + D * E);
    
    return vec3<f32>(r, g, b);
}

fn filmic_tonemap(color: vec3<f32>) -> vec3<f32> {
    let x = max(0.0, color - 0.004);
    return (x * (6.2 * x + 0.5)) / (x * (6.2 * x + 1.7) + 0.06);
}

fn reinhard_tonemap(color: vec3<f32>) -> vec3<f32> {
    return color / (1.0 + color);
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    // Sample the input texture
    var color = textureSample(input_texture, input_sampler, input.uv).rgba;
    
    // Apply exposure
    color.rgb *= params.exposure;
    
    // Simple bloom simulation (screen-space)
    let bloom = max(vec3<f32>(0.0), color.rgb - params.bloom_threshold);
    bloom *= params.bloom_intensity;
    
    // Combine base color with bloom
    color.rgb += bloom;
    
    // Apply tone mapping (ACES is high quality)
    color.rgb = aces_tonemap(color.rgb * 0.6); // Scale for ACES
    
    // Apply gamma correction
    color.rgb = pow(color.rgb, vec3<f32>(1.0 / params.gamma));
    
    // Apply contrast
    color.rgb = mix(vec3<f32>(0.5), color.rgb, params.contrast);
    
    // Apply saturation
    let luminance = dot(color.rgb, vec3<f32>(0.299, 0.587, 0.114));
    color.rgb = mix(vec3<f32>(luminance), color.rgb, params.saturation);
    
    // Apply vignette effect
    let vignette = 1.0 - smoothstep(0.0, params.vignette_radius, 
                                   length(input.uv - vec2<f32>(0.5, 0.5)));
    vignette = pow(vignette, params.vignette_strength * 4.0);
    color.rgb *= vignette;
    
    return color;
}
