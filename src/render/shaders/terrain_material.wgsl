#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::mesh_view_bindings view
#import bevy_pbr::mesh_functions mesh_position_local_to_world, mesh_normal_local_to_world, mesh_position_local_to_clip
#import bevy_pbr::shadows fetch_directional_shadow
#import rose_client::zone_lighting apply_zone_lighting

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mesh_position_local_to_clip(mesh.model, vec4<f32>(vertex.position, 1.0));
    out.world_position = mesh_position_local_to_world(mesh.model, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal);
    out.uv0 = vertex.uv0;
    out.uv1 = vertex.uv1;
    out.tile_info = vertex.tile_info;
    return out;
}

@group(1) @binding(0)
var tile_array_texture: binding_array<texture_2d<f32>>;
@group(1) @binding(1)
var tile_array_sampler: sampler;

// Detail texture for enhanced surface detail
@group(1) @binding(2)
var detail_texture: texture_2d<f32>;
@group(1) @binding(3)
var detail_sampler: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), in.world_position);

    var tile_layer1_id: u32 = (in.tile_info) & 0xffu;
    var tile_layer2_id: u32 = (in.tile_info >> 8u) & 0xffu;
    var tile_rotation: u32 = (in.tile_info >> 16u) & 0xffu;
    var layer2_uv: vec2<f32> = in.uv1;
    if (tile_rotation == 2u) {
        layer2_uv.x = 1.0 - layer2_uv.x;
    } else if (tile_rotation == 3u) {
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 4u) {
        layer2_uv.x = 1.0 - layer2_uv.x;
        layer2_uv.y = 1.0 - layer2_uv.y;
    } else if (tile_rotation == 5u) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = 1.0 - x;
    } else if (tile_rotation == 6u) {
        var x: f32 = layer2_uv.x;
        layer2_uv.x = layer2_uv.y;
        layer2_uv.y = x;
    }

    let layer1 = textureSample(tile_array_texture[tile_layer1_id], tile_array_sampler, in.uv1);
    let layer2 = textureSample(tile_array_texture[tile_layer2_id], tile_array_sampler, layer2_uv);
    var lightmap = textureSample(tile_array_texture[0], tile_array_sampler, in.uv0);
    let shadow = fetch_directional_shadow(0u, in.world_position, in.world_normal, view_z);
    
    // Enhanced shadow integration - softer shadows
    let shadow_factor = shadow * 0.5 + 0.5;
    lightmap = vec4<f32>(lightmap.xyz * shadow_factor, lightmap.w);

    // Enhanced diffuse lighting with normal-based shading (balanced)
    let N = normalize(in.world_normal);
    let diffuse = max(dot(N, vec3<f32>(0.0, 1.0, 0.0)), 0.0) * 0.4 + 0.6; // Balanced contrast
    let ambient = 0.3; // Moderate ambient
    
    // Improved layer blending with height-based mixing
    let height_blend = smoothstep(0.3, 0.7, in.uv0.y); // Use Y coordinate as height
    let base_color = mix(layer1, layer2, mix(layer2.a, height_blend, 0.5)) * lightmap * 1.7; // Balanced multiplier
    
    // Apply enhanced lighting (balanced)
    var terrain_color = base_color * (diffuse + ambient) * 0.95; // Slight overall reduction
    
    // Add detail texture for surface detail (if available)
    let detail = textureSample(detail_texture, detail_sampler, in.uv0 * 10.0); // 10x tiling
    terrain_color = vec4<f32>(mix(terrain_color.rgb, terrain_color.rgb * detail.rgb, 0.3), terrain_color.a); // Subtle detail enhancement
    
    // Add specular highlights for wet/smooth surfaces
    let V = normalize(view.world_position.xyz - in.world_position.xyz);
    let R = reflect(-V, N);
    let specular = pow(max(dot(R, vec3<f32>(0.0, 1.0, 0.0)), 0.0), 16.0) * 0.05; // Reduced specular intensity
    terrain_color = vec4<f32>(terrain_color.rgb + specular, terrain_color.a);

    return apply_zone_lighting(in.world_position, in.world_normal, vec4<f32>(terrain_color.rgb, 1.0), view_z);
}
