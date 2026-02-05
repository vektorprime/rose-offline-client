#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::alpha_discard

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_deferred_functions::deferred_output
#else
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing}
#endif

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate PBR input from standard material
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
#ifdef PREPASS_PIPELINE
    // Deferred rendering - just pass through
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    // Apply standard PBR lighting
    out.color = apply_pbr_lighting(pbr_input);
    // Apply post-processing
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
#endif
    
    return out;
}
