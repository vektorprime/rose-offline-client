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
#import bevy_pbr::mesh_functions::get_tag
#endif

// Extension bindings from RoseObjectExtension
// Lightmap parameters: x, y = the part's cell (column, row) in the lightmap page,
// z = scale (1 / parts per row),
// w = parts per row of a shared lightmap page (0 = use x/y as the cell;
// > 0 = the cell is given by the mesh's MeshTag, see below)
// Only applied when the material has a lightmap (ROSE_OBJECT_LIGHTMAP).
@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var<uniform> lightmap_params: vec4<f32>;

// The original client blends lightmaps with MODULATE2X in gamma space
// (color * lightmap * 2, so a mid-grey lightmap leaves the color unchanged).
// Lightmaps are sampled as sRGB, i.e. linear, and (2 * lm)^2.2 = 2^2.2 * lm_linear,
// so the linear-space equivalent scales the sampled lightmap by 2^2.2.
const LIGHTMAP_MODULATE_2X_LINEAR: f32 = 4.594794;

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
    
    // Specular only for ZSC materials with the specular flag (ROSE_OBJECT_SPECULAR,
    // set when the material has a specular texture): the specular map's red channel
    // drives the PBR reflectance (0.0 = matte, 1.0 = shiny). Every other material
    // keeps the StandardMaterial reflectance; the unbound texture would be Bevy's
    // white fallback (reflectance 1.0).
    #ifdef ROSE_OBJECT_SPECULAR
    #ifdef VERTEX_UVS_A
    {
        let specular_sample = textureSample(specular_texture, specular_sampler, in.uv);
        pbr_input.material.reflectance = vec3<f32>(specular_sample.r);
    }
    #endif
    #endif
    
    // Sample lightmap texture if the material has one and UV_B is available
    // Lightmap uses the second UV channel with offset/scale transformation
    var lightmap_color = vec3<f32>(1.0);
    #ifdef ROSE_OBJECT_LIGHTMAP
    #ifdef VERTEX_UVS_B
    {
        // Lightmap cell. Zone objects share one material per lightmap page
        // (w = parts per row) and carry their cell index in MeshTag; the cell is
        // (column, row) = (index % per_row, index / per_row).
        var lightmap_cell = lightmap_params.xy;
        let lightmap_parts_per_row = u32(lightmap_params.w);
        if (lightmap_parts_per_row > 0u) {
            let lightmap_cell_index = get_tag(in.instance_index);
            lightmap_cell = vec2<f32>(
                f32(lightmap_cell_index % lightmap_parts_per_row),
                f32(lightmap_cell_index / lightmap_parts_per_row)
            );
        }

        // Each part's UV_B spans one cell: page UV = (uv_b + cell) / parts per row,
        // as in the original client's lightmap vertex shader.
        let lightmap_uv = (in.uv_b + lightmap_cell) * lightmap_params.z;
        lightmap_color = textureSample(lightmap_texture, lightmap_sampler, lightmap_uv).rgb
            * LIGHTMAP_MODULATE_2X_LINEAR;
    }
    #endif
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
