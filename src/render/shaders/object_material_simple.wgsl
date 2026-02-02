// Simplified object material shader using Bevy's standard Material pipeline
// This shader uses standard Bevy bindings without custom zone lighting

#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions::{mesh_position_local_to_world, mesh_normal_local_to_world, get_world_from_local}

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
    // Pack first 3 fields into vec4 for alignment
    material_params: vec4<f32>, // x = flags (as f32), y = alpha_cutoff, z = alpha_value, w = unused
    // Pack lightmap params into vec4 for alignment
    lightmap_params: vec4<f32>, // x = offset_x, y = offset_y, z = scale, w = unused
};

// Constants for accessing material_params components
const MATERIAL_PARAM_FLAGS: u32 = 0u;
const MATERIAL_PARAM_ALPHA_CUTOFF: u32 = 1u;
const MATERIAL_PARAM_ALPHA_VALUE: u32 = 2u;

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
    var world_from_local = skin_model(vertex.joint_indices, vertex.joint_weights);
    out.world_normal = skin_normals(world_from_local, vertex.normal);
#else
    var world_from_local = get_world_from_local(vertex.instance_index);
    out.world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
#endif
    
    out.world_position = mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));
    // FIXED: Changed view.view_proj to view.clip_from_world for Bevy 0.14 compatibility
    out.clip_position = view.clip_from_world * out.world_position;
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
    // Extract material values from vec4 params
    let flags = u32(material.material_params.x);
    let alpha_cutoff = material.material_params.y;
    let alpha_value = material.material_params.z;
    let lightmap_offset_x = material.lightmap_params.x;
    let lightmap_offset_y = material.lightmap_params.y;
    let lightmap_scale = material.lightmap_params.z;

    // FIX: Add fallback color if texture sampling fails or returns black
    // This ensures objects are visible even if textures don't load properly
    var output_color: vec4<f32> = textureSample(base_texture, base_sampler, in.uv);
    
    // Fallback: if color is completely black, use a bright magenta debug color
    // This helps diagnose texture loading issues
    if (output_color.r == 0.0 && output_color.g == 0.0 && output_color.b == 0.0) {
        output_color = vec4<f32>(0.8, 0.2, 0.8, 1.0); // Magenta fallback
    }

    // Apply alpha value if specified
    if ((flags & OBJECT_MATERIAL_FLAGS_HAS_ALPHA_VALUE) != 0u) {
        output_color.a = output_color.a * alpha_value;
    }

    // Alpha masking
    if ((flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_MASK) != 0u) {
        if (output_color.a < alpha_cutoff) {
            discard;
        }
        output_color.a = 1.0;
    } else if ((flags & OBJECT_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE) != 0u) {
        output_color.a = 1.0;
    }

    // Apply specular if enabled
    if ((flags & OBJECT_MATERIAL_FLAGS_SPECULAR) != 0u) {
        let specular = textureSample(specular_texture, specular_sampler, in.uv).r;
        // Simple specular highlight
        output_color = vec4<f32>(output_color.rgb + vec3<f32>(specular * 0.5), output_color.a);
    }

    // Apply lightmap if LIGHTMAP_UV is defined
#ifdef LIGHTMAP_UV
    // Sample lightmap at the offset/scaled UV coordinates
    let lightmap_uv = in.lightmap_uv * lightmap_scale +
                      vec2<f32>(lightmap_offset_x, lightmap_offset_y);
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
