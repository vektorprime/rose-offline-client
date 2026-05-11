#define_import_path bevy_pbr::blood_overlay

#import bevy_pbr::{
    forward_io::VertexOutput,
    pbr_functions::get_surface_color,
    pbr_types::PbrInput,
}
#import bevy_render::maths::saturate

// Blood overlay texture group
#if MATERIAL_BIND_GROUP < 200
#define BLOOD_OVERLAY_BIND_GROUP 200
#else
#define BLOOD_OVERLAY_BIND_GROUP MATERIAL_BIND_GROUP
#endif

@group(BLOOD_OVERLAY_BIND_GROUP) @binding(200)
var blood_overlay_texture: texture_2d<f32>;

@group(BLOOD_OVERLAY_BIND_GROUP) @binding(201)
var blood_overlay_sampler: sampler;

@group(BLOOD_OVERLAY_BIND_GROUP) @binding(202)
var<uniform> blood_params: BloodOverlayUniform;

struct BloodOverlayUniform {
    intensity: f32,
    texture_width: f32,
    texture_height: f32,
    _padding: f32,
}

/// Apply blood overlay to the surface color.
/// This function is called from the material extension fragment shader.
fn apply_blood_overlay(pbr_input: PbrInput, surface_color: vec4<f32>) -> vec4<f32> {
    // If no blood overlay texture, return original color
    if blood_params.intensity < 0.01 {
        return surface_color;
    }

    // Sample the blood overlay texture at the UV coordinates
    let blood_color = textureSample(blood_overlay_texture, blood_overlay_sampler, pbr_input.uv);
    
    // The blood texture has blood pixels in RGB with alpha indicating coverage
    // Blend blood color with surface color using the blood alpha
    let blood_alpha = blood_color.a * blood_params.intensity;
    
    // Skip if no blood at this UV coordinate
    if blood_alpha < 0.01 {
        return surface_color;
    }

    // Blend blood with surface color
    // blood_color.rgb is already in the target color (dark red)
    // We want to replace the surface color where blood exists
    return mix(surface_color, blood_color.rgb, saturate(blood_alpha));
}
