// ROSE Effect Mesh Material Extension Shader (RoseEffectExtension)
//
// Vertex ("morph") animation for effect meshes and the zone's animated objects
// (LIST_MORPH_OBJECT). ZmoTextureAssetLoader bakes the ZMO into an Rgba32Float
// texture, x = column, y = vertex index:
//   column frame:              (position.xyz, uv.x)
//   column num_frames + frame: (normal.xyz,   uv.y)   (only with normal/uv channels)
// Animated channels replace the mesh's own values, as in the original client.
//
// The same morph runs in every pipeline the mesh is drawn with: the forward main
// pass and, with PREPASS_PIPELINE, the depth/normal prepass, the deferred G-buffer
// and the shadow maps, so depth, shadows and shading all follow the animation.
// The vertex functions mirror Bevy 0.19's mesh.wgsl and prepass.wgsl.

#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    view_transformations::position_world_to_clip,
}

#ifdef PREPASS_PIPELINE
#import bevy_pbr::prepass_io::{Vertex, VertexOutput}
#else
#import bevy_pbr::forward_io::{Vertex, VertexOutput, FragmentOutput}
#import bevy_pbr::pbr_fragment::pbr_input_from_standard_material
#import bevy_pbr::pbr_functions::{alpha_discard, apply_pbr_lighting, main_pass_post_lighting_processing}
#endif

// Matches EffectMeshAnimationUniform (written by mesh_animation_system).
struct EffectMeshAnimationState {
    // Bits 0-3 = animated channels, bits 4-31 = number of frames (0 = not animated)
    flags: u32,
    // Low 16 bits = current frame, high 16 bits = next frame
    current_next_frame: u32,
    // Blend weight of the next frame
    next_weight: f32,
    // Animated alpha (unused: effect materials are opaque or alpha-masked)
    alpha: f32,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100)
var animation_texture: texture_2d<f32>;

@group(#{MATERIAL_BIND_GROUP}) @binding(102)
var<uniform> animation_state: EffectMeshAnimationState;

const ANIMATE_POSITION: u32 = 1u;
const ANIMATE_NORMAL: u32 = 2u;
const ANIMATE_UV: u32 = 4u;

struct MorphedVertex {
    position: vec3<f32>,
    normal: vec3<f32>,
    uv: vec2<f32>,
}

// Applies the current animation frame to one vertex. Bevy packs meshes into shared
// vertex buffers (draws use base_vertex = first_vertex_index), so the vertex's
// index within its mesh is vertex_index - first_vertex_index.
fn morph_vertex(mesh_vertex: MorphedVertex, vertex_index: u32, instance_index: u32) -> MorphedVertex {
    var morphed = mesh_vertex;

    let flags = animation_state.flags;
    let num_frames = flags >> 4u;
    if (num_frames == 0u) {
        return morphed;
    }

    // Vertices without animation data (and the 1x1 fallback texture) keep the mesh data.
    let local_index = vertex_index - mesh[instance_index].first_vertex_index;
    if (local_index >= textureDimensions(animation_texture).y) {
        return morphed;
    }

    let current_frame = animation_state.current_next_frame & 0xffffu;
    let next_frame = animation_state.current_next_frame >> 16u;
    let next_weight = animation_state.next_weight;

    let current_0 = textureLoad(animation_texture, vec2<u32>(current_frame, local_index), 0);
    let next_0 = textureLoad(animation_texture, vec2<u32>(next_frame, local_index), 0);
    if ((flags & ANIMATE_POSITION) != 0u) {
        morphed.position = mix(current_0.xyz, next_0.xyz, next_weight);
    }

    if ((flags & (ANIMATE_NORMAL | ANIMATE_UV)) != 0u) {
        let current_1 = textureLoad(animation_texture, vec2<u32>(current_frame + num_frames, local_index), 0);
        let next_1 = textureLoad(animation_texture, vec2<u32>(next_frame + num_frames, local_index), 0);
        if ((flags & ANIMATE_NORMAL) != 0u) {
            morphed.normal = mix(current_1.xyz, next_1.xyz, next_weight);
        }
        if ((flags & ANIMATE_UV) != 0u) {
            morphed.uv = vec2<f32>(
                mix(current_0.w, next_0.w, next_weight),
                mix(current_1.w, next_1.w, next_weight)
            );
        }
    }

    return morphed;
}

#ifdef PREPASS_PIPELINE

// Depth/normal prepass, deferred G-buffer and shadow maps (fragment: StandardMaterial's).
@vertex
fn vertex(
    vertex_in: Vertex,
#ifndef MORPH_TARGETS
    @builtin(vertex_index) vertex_index: u32,
#endif
) -> VertexOutput {
#ifdef MORPH_TARGETS
    let vertex_index = vertex_in.index;
#endif
    var out: VertexOutput;

    var mesh_vertex: MorphedVertex;
    mesh_vertex.position = vertex_in.position;
#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
    mesh_vertex.normal = vertex_in.normal;
#endif
#endif
#ifdef VERTEX_UVS_A
    mesh_vertex.uv = vertex_in.uv;
#endif
    let morphed = morph_vertex(mesh_vertex, vertex_index, vertex_in.instance_index);

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
#ifdef SKINNED
    let world_from_local = skinning::skin_model(
        vertex_in.joint_indices,
        vertex_in.joint_weights,
        vertex_in.instance_index
    );
#else
    let world_from_local = mesh_world_from_local;
#endif

    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0); // Clamp depth to avoid clipping
#endif

#ifdef VERTEX_UVS_A
    out.uv = morphed.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex_in.uv_b;
#endif

#ifdef NORMAL_PREPASS_OR_DEFERRED_PREPASS
#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, morphed.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(morphed.normal, vertex_in.instance_index);
#endif
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex_in.tangent,
        vertex_in.instance_index
    );
#endif
#endif

#ifdef VERTEX_COLORS
    out.color = vertex_in.color;
#endif

#ifdef MOTION_VECTOR_PREPASS
    // The previous frame's morph state is not kept: this frame's shape with the
    // previous transform (motion vectors then only carry the object's own motion).
#ifdef SKINNED
#ifdef HAS_PREVIOUS_SKIN
    let prev_model = skinning::skin_prev_model(
        vertex_in.joint_indices,
        vertex_in.joint_weights,
        vertex_in.instance_index
    );
#else
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_in.instance_index);
#endif
#else
    let prev_model = mesh_functions::get_previous_world_from_local(vertex_in.instance_index);
#endif
    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        prev_model,
        vec4<f32>(morphed.position, 1.0)
    );
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_in.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}

#else // PREPASS_PIPELINE

// Forward main pass.
@vertex
fn vertex(
    vertex_in: Vertex,
#ifndef MORPH_TARGETS
    @builtin(vertex_index) vertex_index: u32,
#endif
) -> VertexOutput {
#ifdef MORPH_TARGETS
    let vertex_index = vertex_in.index;
#endif
    var out: VertexOutput;

    var mesh_vertex: MorphedVertex;
#ifdef VERTEX_POSITIONS
    mesh_vertex.position = vertex_in.position;
#endif
#ifdef VERTEX_NORMALS
    mesh_vertex.normal = vertex_in.normal;
#endif
#ifdef VERTEX_UVS_A
    mesh_vertex.uv = vertex_in.uv;
#endif
    let morphed = morph_vertex(mesh_vertex, vertex_index, vertex_in.instance_index);

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_in.instance_index);
#ifdef SKINNED
    let world_from_local = skinning::skin_model(
        vertex_in.joint_indices,
        vertex_in.joint_weights,
        vertex_in.instance_index
    );
#else
    let world_from_local = mesh_world_from_local;
#endif

#ifdef VERTEX_NORMALS
#ifdef SKINNED
    out.world_normal = skinning::skin_normals(world_from_local, morphed.normal);
#else
    out.world_normal = mesh_functions::mesh_normal_local_to_world(morphed.normal, vertex_in.instance_index);
#endif
#endif

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(morphed.position, 1.0));
    out.position = position_world_to_clip(out.world_position.xyz);
#endif

#ifdef VERTEX_UVS_A
    out.uv = morphed.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex_in.uv_b;
#endif

#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local,
        vertex_in.tangent,
        vertex_in.instance_index
    );
#endif

#ifdef VERTEX_COLORS
    out.color = vertex_in.color;
#endif

#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    out.instance_index = vertex_in.instance_index;
#endif

#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_in.instance_index, mesh_world_from_local[3]);
#endif

    return out;
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    // ExtendedMaterial fragment shaders must discard masked pixels themselves.
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

#endif // PREPASS_PIPELINE
