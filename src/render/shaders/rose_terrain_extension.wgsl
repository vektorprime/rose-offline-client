#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::pbr_types
#import bevy_pbr::pbr_functions

// Custom extension bindings for terrain (Group 1, Bindings 100+)
@group(1) @binding(100) var<uniform> tile_params: vec4<f32>;
@group(1) @binding(101) var tile_0_texture: texture_2d<f32>;
@group(1) @binding(102) var tile_0_sampler: sampler;
@group(1) @binding(103) var tile_1_texture: texture_2d<f32>;
@group(1) @binding(104) var tile_1_sampler: sampler;
@group(1) @binding(105) var tile_2_texture: texture_2d<f32>;
@group(1) @binding(106) var tile_2_sampler: sampler;
@group(1) @binding(107) var tile_3_texture: texture_2d<f32>;
@group(1) @binding(108) var tile_3_sampler: sampler;
@group(1) @binding(109) var tile_4_texture: texture_2d<f32>;
@group(1) @binding(110) var tile_4_sampler: sampler;
@group(1) @binding(111) var detail_texture: texture_2d<f32>;
@group(1) @binding(112) var detail_sampler: sampler;

#import bevy_pbr::pbr_fragment

// Helper function to sample the correct tile texture based on index
fn sample_tile(tile_id: u32, uv: vec2<f32>) -> vec4<f32> {
    if (tile_id == 0u) {
        return textureSample(tile_0_texture, tile_0_sampler, uv);
    } else if (tile_id == 1u) {
        return textureSample(tile_1_texture, tile_1_sampler, uv);
    } else if (tile_id == 2u) {
        return textureSample(tile_2_texture, tile_2_sampler, uv);
    } else if (tile_id == 3u) {
        return textureSample(tile_3_texture, tile_3_sampler, uv);
    } else {
        return textureSample(tile_4_texture, tile_4_sampler, uv);
    }
}

// Helper function to apply tile rotation to UVs
fn apply_tile_rotation(uv: vec2<f32>, rotation: u32) -> vec2<f32> {
    var result = uv;
    if (rotation == 2u) {
        result.x = 1.0 - result.x;
    } else if (rotation == 3u) {
        result.y = 1.0 - result.y;
    } else if (rotation == 4u) {
        result.x = 1.0 - result.x;
        result.y = 1.0 - result.y;
    } else if (rotation == 5u) {
        let x = result.x;
        result.x = result.y;
        result.y = 1.0 - x;
    } else if (rotation == 6u) {
        let x = result.x;
        result.x = result.y;
        result.y = x;
    }
    return result;
}

@fragment
fn fragment(
    in: FragmentInput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    // 1. Let Bevy generate the standard PBR input
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // 2. Extract tile info from UV coordinates (packed into uv1)
    // tile_info is passed through uv1 as: layer1_id (8b) | layer2_id (8b) | rotation (8b) | padding (8b)
    let tile_info = u32(in.uv1.x * 255.0) | (u32(in.uv1.y * 255.0) << 8u);
    let tile_layer1_id = (tile_info) & 0xffu;
    let tile_layer2_id = (tile_info >> 8u) & 0xffu;
    let tile_rotation = (tile_info >> 16u) & 0xffu;

    // 3. Apply tile rotation to layer2 UVs
    let layer2_uv = apply_tile_rotation(in.uv1, tile_rotation);

    // 4. Sample both tile layers
    let layer1 = sample_tile(tile_layer1_id, in.uv1);
    let layer2 = sample_tile(tile_layer2_id, layer2_uv);

    // 5. Mix the two tile layers based on layer2's alpha
    let terrain_color = mix(layer1, layer2, layer2.a);

    // 6. Sample detail texture and apply
    let detail = textureSample(detail_texture, detail_sampler, in.uv * tile_params.xy);
    pbr_input.material.base_color *= terrain_color * detail;

    // 7. Let Bevy handle lighting
    var out = apply_pbr_lighting(pbr_input);

    // 8. Apply post-processing
    out = main_pass_post_lighting_processing(pbr_input, out);

    return out;
}
