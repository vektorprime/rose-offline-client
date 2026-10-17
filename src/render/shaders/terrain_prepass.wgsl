// Depth / normal prepass for the terrain (TerrainMaterial).
//
// The terrain mesh uses TerrainMaterial's own vertex layout (position 0, normal 1,
// lightmap UV 2, tile UV 3, tile info 4; TerrainMaterial::specialize also runs for the
// prepass pipelines), which Bevy's default prepass shader cannot read. Being in the
// prepass puts the terrain into the prepass depth and normal textures that the cel
// outline pass (cel_outline.wgsl) and SSAO read; without it they see objects hidden
// behind hills and no terrain normals.
// Mirrors Bevy 0.19's prepass.wgsl for a static, unskinned mesh.

#import bevy_pbr::{
    mesh_functions,
    mesh_view_bindings::view,
    view_transformations::position_world_to_clip,
}

#ifdef PREPASS_FRAGMENT
#import bevy_pbr::prepass_io
#endif

#ifdef MOTION_VECTOR_PREPASS
#import bevy_pbr::prepass_bindings::previous_view_uniforms
#endif

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) world_position: vec4<f32>,
#ifdef MOTION_VECTOR_PREPASS
    @location(2) previous_world_position: vec4<f32>,
#endif
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    @location(3) unclipped_depth: f32,
#endif
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    let world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local,
        vec4<f32>(vertex.position, 1.0),
    );
    // Same expression as the terrain main pass (terrain_material.wgsl).
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.unclipped_depth = out.position.z;
    out.position.z = min(out.position.z, 1.0);
#endif

    out.world_normal = mesh_functions::mesh_normal_local_to_world(
        vertex.normal,
        vertex.instance_index,
    );

#ifdef MOTION_VECTOR_PREPASS
    out.previous_world_position = mesh_functions::mesh_position_local_to_world(
        mesh_functions::get_previous_world_from_local(vertex.instance_index),
        vec4<f32>(vertex.position, 1.0),
    );
#endif

    return out;
}

#ifdef PREPASS_FRAGMENT
@fragment
fn fragment(in: VertexOutput) -> prepass_io::FragmentOutput {
    var out: prepass_io::FragmentOutput;

#ifdef NORMAL_PREPASS
    // Alpha 1 = regular cel outline class (see rose_object_extension.wgsl).
    out.normal = vec4(normalize(in.world_normal) * 0.5 + vec3(0.5), 1.0);
#endif

#ifdef UNCLIPPED_DEPTH_ORTHO_EMULATION
    out.frag_depth = in.unclipped_depth;
#endif

#ifdef MOTION_VECTOR_PREPASS
    let clip_position_t = view.unjittered_clip_from_world * in.world_position;
    let clip_position = clip_position_t.xy / clip_position_t.w;
    let previous_clip_position_t =
        previous_view_uniforms.clip_from_world * in.previous_world_position;
    let previous_clip_position = previous_clip_position_t.xy / previous_clip_position_t.w;
    out.motion_vector = (clip_position - previous_clip_position) * vec2(0.5, -0.5);
#endif

    return out;
}
#endif
