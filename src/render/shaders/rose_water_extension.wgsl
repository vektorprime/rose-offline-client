#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions

// Custom extension bindings for water (Group 1, Bindings 100+)
@group(1) @binding(100) var<uniform> uv_animation_params: vec4<f32>;
@group(1) @binding(101) var water_texture: texture_2d<f32>;
@group(1) @binding(102) var water_sampler: sampler;

#import bevy_pbr::pbr_fragment

@fragment
fn fragment(
    in: FragmentInput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // 1. Let Bevy generate the standard PBR input
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // 2. Apply UV animation for wave movement
    // uv_animation_params: x = time, y = speed_x, z = speed_y, w = scale
    let animated_uv = in.uv * uv_animation_params.w +
                      vec2<f32>(
                          uv_animation_params.x * uv_animation_params.y,
                          uv_animation_params.x * uv_animation_params.z
                      );

    // 3. Sample water texture with animated UVs
    let water_color = textureSample(water_texture, water_sampler, animated_uv);

    // 4. Apply water color to the material
    pbr_input.material.base_color *= water_color;

    // 5. Let Bevy handle lighting
    var out = apply_pbr_lighting(pbr_input);

    // 6. Apply post-processing
    out = main_pass_post_lighting_processing(pbr_input, out);

    return out;
}
