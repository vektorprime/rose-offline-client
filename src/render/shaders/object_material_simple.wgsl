// Simplified object material shader using Bevy's standard Material pipeline
// This shader uses standard Bevy bindings without custom zone lighting

#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions::{mesh_position_local_to_world, mesh_normal_local_to_world, get_model_matrix}

#ifdef SKINNED
#import bevy_pbr::skinning::{skin_normals, skin_model}
#endif

// Material bindings at group 2 (standard Bevy material group)
@group(2) @binding(0)
var<uniform> material: StaticMeshMaterialData;
@group(2) @binding(1)
var base_texture: texture_2d<f32>;
@group(2) @binding(2)
var base_sampler: sampler;
@group(2) @binding(3)
var lightmap_texture: texture_2d<f32>;
@group(2) @binding(4)
var lightmap_sampler: sampler;
@group(2) @binding(5)
var specular_texture: texture_2d<f32>;
@group(2) @binding(6)
var specular_sampler: sampler;

struct StaticMeshMaterialData {
    flags: u32,
    alpha_cutoff: f32,
    alpha_value: f32,
    lightmap_uv_offset_x: f32,
    lightmap_uv_offset_y: f32,
    lightmap_uv_scale: f32,
    _padding: f32,
};

// Vertex struct must match Bevy's standard mesh layout
// Location 0: position, 1: normal, 2: uv
struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef LIGHTMAP_UV
    @location(3) lightmap_uv: vec2<f32>,
#endif
#ifdef SKINNED
    @location(4) joint_indices: vec4<u32>,
    @location(5) joint_weights: vec4<f32>,
#endif
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef LIGHTMAP_UV
    @location(3) lightmap_uv: vec2<f32>,
#endif
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    
#ifdef SKINNED
    var model = skin_model(vertex.joint_indices, vertex.joint_weights);
    out.world_normal = skin_normals(model, vertex.normal);
#else
    var model = get_model_matrix(vertex.instance_index);
    out.world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif
    
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.clip_position = view.view_proj * out.world_position;
    out.uv = vertex.uv;
    
#ifdef LIGHTMAP_UV
    out.lightmap_uv = vertex.lightmap_uv;
#endif
    
    return out;
}

const OBJECT_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE: u32 = 1u;
const OBJECT_MATERIAL_FLAGS_ALPHA_MODE_MASK: u32 = 2u;
const OBJECT_MATERIAL_FLAGS_ALPHA_MODE_BLEND: u32 = 4u;
const OBJECT_MATERIAL_FLAGS_HAS_ALPHA_VALUE: u32 = 8u;
const OBJECT_MATERIAL_FLAGS_SPECULAR: u32 = 16u;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
#ifdef LIGHTMAP_UV
    @location(3) lightmap_uv: vec2<f32>,
#endif
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);

    // Apply alpha value if specified
    if ((material.flags & OBJECT_MATERIAL_FLAGS_HAS_ALPHA_VALUE) != 0u) {
        output_color.a = output_color.a * material.alpha_value;
    }

    // Alpha masking
    if ((material.flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        if (output_color.a < material.alpha_cutoff) {
            discard;
        }
        output_color.a = 1.0;
    } else if ((material.flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
        output_color.a = 1.0;
    }

    // Apply specular if enabled
    if ((material.flags & OBJECT_MATERIAL_FLAGS_SPECULAR) != 0u) {
        let specular = textureSample(specular_texture, specular_sampler, in.uv).r;
        // Simple specular highlight
        output_color = vec4<f32>(output_color.rgb + vec3<f32>(specular * 0.5), output_color.a);
    }

    // Apply lightmap if LIGHTMAP_UV is defined
#ifdef LIGHTMAP_UV
    // Sample lightmap at the offset/scaled UV coordinates
    let lightmap_uv = in.lightmap_uv * material.lightmap_uv_scale + 
                      vec2<f32>(material.lightmap_uv_offset_x, material.lightmap_uv_offset_y);
    let lightmap_color = textureSample(lightmap_texture, lightmap_sampler, lightmap_uv);
    // Simple lightmap blend - multiply base color with lightmap
    output_color = vec4<f32>(output_color.rgb * lightmap_color.rgb, output_color.a);
#else
    // Simple ambient lighting (hardcoded since we removed zone lighting)
    let ambient = vec3<f32>(0.6, 0.6, 0.6);
    output_color = vec4<f32>(output_color.rgb * ambient, output_color.a);
#endif

    return output_color;
}
