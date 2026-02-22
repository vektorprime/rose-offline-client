// ROSE Object Material Extension Shader
// Applies lightmap and specular to zone objects (trees, buildings, decorations)
//
// This shader extends Bevy's StandardMaterial with:
// - Lightmap texture support
// - Specular texture support
//
// Note: Zone lighting fog has been removed because ExtendedMaterial shaders
// only have access to bind groups 0, 1, and 2. Group 3 (zone lighting) is not
// automatically available in the material extension pipeline.
// Bevy's built-in fog system is used instead.

#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::alpha_discard

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_deferred_functions::deferred_output
#else
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing}
#endif

// Extension bindings from RoseObjectExtension
// Lightmap parameters: x = offset_x, y = offset_y, z = scale, w = unused
@group(2) @binding(100)
var<uniform> lightmap_params: vec4<f32>;

// Lightmap texture and sampler
@group(2) @binding(101)
var lightmap_texture: texture_2d<f32>;

@group(2) @binding(102)
var lightmap_sampler: sampler;

// Specular texture and sampler
@group(2) @binding(103)
var specular_texture: texture_2d<f32>;

@group(2) @binding(104)
var specular_sampler: sampler;

#ifdef PREPASS_PIPELINE
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate PBR input from standard material
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // Deferred rendering - just pass through
    let out = deferred_output(in, pbr_input);
    return out;
}
#else
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var out: FragmentOutput;
    
    // Generate PBR input from standard material
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // CRITICAL: Apply alpha discard for foliage transparency
    // Without this, pixels that should be transparent are rendered as opaque squares
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    
    // Sample specular texture using primary UV
    // Specular value controls the intensity of specular highlights (0.0 = matte, 1.0 = shiny)
    // Default to 0.5 (Bevy's default reflectance) if texture sampling fails
    var specular_value = 0.5;
    #ifdef VERTEX_UVS
    {
        let specular_sample = textureSample(specular_texture, specular_sampler, in.uv);
        specular_value = specular_sample.r;
    }
    #endif
    
    // Apply specular to PBR material reflectance before lighting
    // This affects how strong the specular highlights appear
    // Note: In Bevy 0.16, reflectance changed from f32 to vec3<f32>
    pbr_input.material.reflectance = vec3<f32>(specular_value);
    
    // Sample lightmap texture if UV_B is available
    // Lightmap uses the second UV channel with offset/scale transformation
    var lightmap_color = vec3<f32>(1.0);
    #ifdef VERTEX_UVS_B
    {
        // Calculate lightmap UV: scale and offset from lightmap_params
        let lightmap_uv = vec2<f32>(
            in.uv_b.x * lightmap_params.z + lightmap_params.x,
            in.uv_b.y * lightmap_params.z + lightmap_params.y
        );
        lightmap_color = textureSample(lightmap_texture, lightmap_sampler, lightmap_uv).rgb;
    }
    #endif
    
    // Apply standard Bevy PBR lighting
    // This includes response to directional lights, ambient lights, and environment
    let color = apply_pbr_lighting(pbr_input);
    
    // Apply lightmap as ambient occlusion (multiply with lit color)
    let lit_color = vec4<f32>(color.rgb * lightmap_color, color.a);
    
    // Apply post-processing (tonemapping, Bevy's built-in fog, etc.)
    // Note: Bevy's fog is applied automatically in main_pass_post_lighting_processing
    out.color = main_pass_post_lighting_processing(pbr_input, lit_color);
    
    return out;
}
#endif
