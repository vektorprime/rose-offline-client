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
@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> lightmap_params: vec4<f32>;

// Lightmap texture and sampler
@group(#{MATERIAL_BIND_GROUP}) @binding(101)
var lightmap_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var lightmap_sampler: sampler;

// Specular texture and sampler
@group(#{MATERIAL_BIND_GROUP}) @binding(103)
var specular_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(104)
var specular_sampler: sampler;

// Blood overlay texture and sampler (UV-space combat painting)
@group(#{MATERIAL_BIND_GROUP}) @binding(106)
var blood_overlay_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(107)
var blood_overlay_sampler: sampler;

// x = intensity [0..1], y = enabled flag (0/1), z/w reserved
@group(#{MATERIAL_BIND_GROUP}) @binding(108)
var<uniform> blood_params: vec4<f32>;

#ifdef PREPASS_PIPELINE
@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate PBR input from standard material
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // CRITICAL: Apply alpha discard for foliage transparency in deferred prepass
    // Without this, pixels that should be transparent are rendered as opaque squares in the G-buffer
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

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
    #ifdef VERTEX_UVS_A
    {
        let specular_sample = textureSample(specular_texture, specular_sampler, in.uv);
        specular_value = specular_sample.r;
    }
    #endif
    
    // Apply specular to PBR material reflectance before lighting
    // This affects how strong the specular highlights appear
    // Note: In Bevy 0.16, reflectance changed from f32 to vec3<f32>
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

// Apply lightmap as ambient occlusion (multiply with lit color) BEFORE blood overlay
// so that dark lightmap values don't make blood invisible in shadowed areas.
let lit_color = vec4<f32>(color.rgb * lightmap_color, color.a);

// Apply UV-space blood overlay on top of the lit+lightmapped color.
// Blood is blended after lightmap so it remains visible even in dark/shadowed areas.
var blood_blended_rgb = lit_color.rgb;
#ifdef VERTEX_UVS_A
{
    if blood_params.y > 0.5 && blood_params.x > 0.001 {
        // DIAGNOSTIC: DEBUG_BLOOD_NEON forces bright green to verify pipeline execution
        // If bright green appears on character models, the pipeline is working but blood texture generation or sampling has an issue.
        #ifdef DEBUG_BLOOD_NEON
        let blood_sample = vec4<f32>(0.0, 1.0, 0.0, 1.0); // Bright neon green for debugging
        #else
        let blood_sample = textureSample(blood_overlay_texture, blood_overlay_sampler, in.uv);
        #endif
        let blood_alpha = clamp(blood_sample.a * blood_params.x, 0.0, 1.0);
        blood_blended_rgb = mix(lit_color.rgb, blood_sample.rgb, blood_alpha);
    }
}
#endif
    
    // Apply post-processing (tonemapping, Bevy's built-in fog, etc.)
// Note: Bevy's fog is applied automatically in main_pass_post_lighting_processing
let final_color = vec4<f32>(blood_blended_rgb, lit_color.a);
out.color = main_pass_post_lighting_processing(pbr_input, final_color);
    
    return out;
}
#endif
