#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions

// Custom extension bindings (Group 1, Bindings 100+)
@group(1) @binding(100) var<uniform> lightmap_params: vec4<f32>;
@group(1) @binding(101) var lightmap_texture: texture_2d<f32>;
@group(1) @binding(102) var lightmap_sampler: sampler;
@group(1) @binding(103) var specular_texture: texture_2d<f32>;
@group(1) @binding(104) var specular_sampler: sampler;

#import bevy_pbr::pbr_fragment

@fragment
fn fragment(
    in: FragmentInput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // 1. Let Bevy generate the standard PBR input
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // 2. Apply ROSE lightmap logic
    let lightmap_uv = in.uv * lightmap_params.z + lightmap_params.xy;
    let lightmap_color = textureSample(lightmap_texture, lightmap_sampler, lightmap_uv);
    pbr_input.material.base_color *= lightmap_color;

    // 3. Apply specular map if available
    let specular = textureSample(specular_texture, specular_sampler, in.uv);
    pbr_input.material.perceptual_roughness *= (1.0 - specular.r);

    // 4. Let Bevy handle lighting
    var out = apply_pbr_lighting(pbr_input);

    // 5. Apply post-processing
    out = main_pass_post_lighting_processing(pbr_input, out);

    return out;
}
