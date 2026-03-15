#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::alpha_discard
#import bevy_pbr::mesh_vertex_shader_inputs::{VertexInput, MeshFlags}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_deferred_functions::deferred_output
#else
#import bevy_pbr::forward_io::{VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing}
#endif

// Animation state structure matching EffectMeshAnimationUniform
struct AnimationState {
    flags: u32,           // bits 0-3 = animation flags, bits 4-31 = num_frames
    current_next_frame: u32,  // lower 16 bits = current frame, upper 16 bits = next frame
    next_weight: f32,     // interpolation weight (0.0 - 1.0)
    alpha: f32,           // animated alpha value
}

// Animation texture and sampler for effect mesh morphing
@group(0) @binding(100)
var anim_texture: texture_2d<f32>;
@group(0) @binding(101)
var anim_sampler: sampler;

// Animation uniform
@group(0) @binding(102)
var<uniform> animation_state: AnimationState;

// Animation flag constants
const ANIM_FLAG_POSITION: u32 = 0x1;
const ANIM_FLAG_NORMAL: u32 = 0x2;
const ANIM_FLAG_UV: u32 = 0x4;
const ANIM_FLAG_ALPHA: u32 = 0x8;

// Helper function to unpack animation flags
fn unpack_animation_flags(flags: u32) -> vec4<bool> {
    return vec4<bool>(
        bitSelect<bool>(flags, 0),  // position
        bitSelect<bool>(flags, 1),  // normal
        bitSelect<bool>(flags, 2),  // uv
        bitSelect<bool>(flags, 3)   // alpha
    );
}

// Helper function to get number of frames from flags
fn get_num_frames(flags: u32) -> u32 {
    return (flags >> 4) & 0x7FFFFF;  // bits 4-31
}

// Helper function to unpack frame indices
fn unpack_frame_indices(current_next_frame: u32) -> vec2<u32> {
    return vec2<u32>(
        current_next_frame & 0xFFFF,           // current frame (lower 16 bits)
        (current_next_frame >> 16) & 0xFFFF    // next frame (upper 16 bits)
    );
}

// Sample animation texture for a vertex
// Texture format: X = frame index, Y = vertex index
// RGBA = position offset (RGB) + additional data (A)
fn sample_animation_data(vertex_index: u32, frame: u32, texture_width: u32, texture_height: u32) -> vec4<f32> {
    // Convert vertex index to texture coordinates
    // UV coordinate system: U = frame/width, V = 1.0 - vertex/height (flip Y)
    let u = f32(frame) / f32(texture_width);
    let v = 1.0 - (f32(vertex_index) / f32(texture_height));
    
    // Sample with point filtering for accurate frame data
    return textureSample(anim_texture, anim_sampler, vec2f(u, v));
}

// Standard PBR vertex shader for effect meshes
@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    // Get vertex index for animation sampling
    var builtins: BuiltInVertex;
    let vertex_index = builtins.vertex_index;
    
    // Initialize with base mesh data
    var vertex_position_standard = modelSpacePosition(&in, false);
    var vertex_normal = in.normal;
    var vertex_uv = #ifdef USE_UV in.uv #else vec2f(0.) #endif;
    
#ifdef HAS_ANIMATION_TEXTURE
    // Unpack animation flags
    let anim_flags = unpack_animation_flags(animation_state.flags);
    let num_frames = get_num_frames(animation_state.flags);
    
    // Only apply animation if we have frames
    if num_frames > 0u {
        // Unpack current and next frame indices
        let frame_indices = unpack_frame_indices(animation_state.current_next_frame);
        let current_frame = frame_indices.x;
        let next_frame = frame_indices.y;
        
        // Get animation texture dimensions
        let texture_size = textureDimensions(anim_texture);
        let texture_width = texture_size.x;
        let texture_height = texture_size.y;
        
        // Sample animation data for current and next frames
        let current_data = sample_animation_data(vertex_index, current_frame, texture_width, texture_height);
        let next_data = sample_animation_data(vertex_index, next_frame, texture_width, texture_height);
        
        // Interpolate between current and next frame
        let morph_data = mix(current_data, next_data, animation_state.next_weight);
        
        // Apply position morphing
        if anim_flags.x {
            // RGB channels contain position offset
            vertex_position_standard.xyz += morph_data.rgb;
        }
        
        // Apply normal morphing
        if anim_flags.y {
            // Sample additional data for normal (use alpha channel and re-sample if needed)
            // For now, use the same texture but interpret differently
            // Normal data is stored as normalized RGB values
            vertex_normal = normalize(morph_data.rgb * 2.0 - 1.0);  // Convert from [0,1] to [-1,1]
        }
        
        // Apply UV morphing
        if anim_flags.z {
            // UV offset stored in RG channels
            vertex_uv += morph_data.rg;
        }
    }
#endif
    
    var output: VertexOutput;
    output.clip_space_position = standardClipSpacePosition(vertex_position_standard);
    output.flags = in.mesh_flags;
    output.world_pos = worldSpacePositionStandard(vertex_position_standard);
    output.normal = normalize(normalTransformMatrix * vec3f(vertex_normal));
    
#ifdef USE_MORPH_TARGETS
    // Morph target support - also handled by animation texture
#endif
    
    output.tangent = normalize(tangentTransformMatrix * vec3f(in.tangent));
    if bitSelect<bool>(in.mesh_flags, MeshFlags::TANGENT_BIT) {
        if bitSelect<bool>(output.flags, MeshFlags::NEGATIVE_TANGENT_BIT) {
            output.tangent.w = -1.0;
        }
        output.normal = normalize(cross(output.tangent, output.bitang)).xyz;
    } else {
#ifdef USE_BITANGENTS
        let bitang = normalize(bitangentTransformMatrix * vec3f(in.bitangent));
        if bitSelect<bool>(output.flags, MeshFlags::NEGATIVE_BITANGENT_BIT) {
            output.tangent.w = -1.0;
        }
        output.normal = normalize(cross(output.tangent, bitang)).xyz;
    }
#endif
    } else {
        output.tangent.w = 1.0;
    }
    
#ifdef DOUBLE_SIDED
    if !in.front_facing && bitSelect<bool>(output.flags, MeshFlags::NEGATIVE_TANGENT_BIT) {
        output.tangent.w *= -1.0;
    }
#endif
    
#ifdef USE_COLOR
    let v_col = in.color * vec4f(1.);
#else
    let v_col = vec4f(1.);
#endif
#ifdef USE_UV
    let uv = vertex_uv;  // Use morphed UV if animation was applied
#else
    let uv = vec2f(0.);
#endif
#ifdef USE_IRIDESCENCE
    let iridescence = in.iridescence * 1.0;
#else
    let iridescence = vec4f(0.);
#endif
#ifdef CLEARCOAT_ROUGHNESS
    let clearcoat_roughness = in.clearcoat_roughness * 1.0;
#else
    let clearcoat_roughness = vec4f(0.);
#endif
    
    output.v_color = v_col;
    output.uv = uv;
    output.iridescence = iridescence;
    output.clearcoat_roughness = clearcoat_roughness;
    output.front_facing = in.front_facing;
#ifdef USE_SKY_NORMAL_MAP
    output.world_pos_sky_normal_map = worldSpacePositionStandard(vertex_position_standard);
#endif

#if VERTEX_OUTPUT_ADDITIONAL_ATTRIBUTES == 1
    output.v_col_a = vec4f(0.0, 0.0, 0.5, 1.0) * v_col;
#else
    output.v_col_a = v_col;
#endif

#if VERTEX_OUTPUT_ADDITIONAL_ATTRIBUTES >= 2
    let a0 = in.additional_attribute_0;
    if a0.a == 0.0 {
        output.normal_map_transform_mat0 = vec4f(1.0, 0.0, 0.0, 0.0);
        output.normal_map_transform_mat1 = vec4f(0.0, 1.0, 0.0, 0.0);
    } else {
        output.normal_map_transform_mat0 = a0.xyzw;
        output.normal_map_transform_mat1 = in.additional_attribute_1.xyzw;
    }
#endif
#if VERTEX_OUTPUT_ADDITIONAL_ATTRIBUTES >= 3
    output.vertex_color_lighting_tint = in.additional_attribute_2 * vec4f(1.);
#else
    output.vertex_color_lighting_tint = vec4f(1.);
#endif
#ifdef USE_TBN
    output.tbn_matrix = mat3x3f(output.tangent, cross(output.bitang), output.normal);
#endif
#ifdef USE_VERTEX_NORMALS
    output.world_space_vertex_normal = normalize(worldSpaceNormal(in.normal));
#endif
#ifdef DEBUG_RENDER_VERTICES
    output.flags |= u32(DEBUG_VERTICES);
#endif

    return output;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    // Generate PBR input from standard material
    var pbr_input = pbr_input_from_standard_material(in, is_front);
    
    // CRITICAL: Apply alpha discard for transparency
    // Without this, pixels that should be transparent are rendered as opaque squares
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
    
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
