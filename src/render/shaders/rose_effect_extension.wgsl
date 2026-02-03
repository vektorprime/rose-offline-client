#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions

// Custom extension bindings (Group 1, Bindings 100+)
@group(1) @binding(100) var animation_texture: texture_2d<f32>;
@group(1) @binding(101) var animation_sampler: sampler;

#import bevy_pbr::pbr_fragment

@fragment
fn fragment(
    in: FragmentInput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // 1. Let Bevy generate the standard PBR input
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // 2. Apply ROSE effect mesh logic
    // Sample the base color texture from StandardMaterial
    // The animation texture can be sampled here if needed for frame-based animation
    // For now, we just use the standard PBR lighting

    // 3. Let Bevy handle lighting
    var out = apply_pbr_lighting(pbr_input);

    // 4. Apply post-processing
    out = main_pass_post_lighting_processing(pbr_input, out);

    return out;
}
