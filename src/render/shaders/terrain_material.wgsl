#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::{mesh_position_local_to_world, mesh_normal_local_to_world, mesh_position_local_to_clip, get_model_matrix}
#import bevy_pbr::shadows::fetch_directional_shadow
#import rose_client::zone_lighting::apply_zone_lighting

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
    @builtin(instance_index) instance_index: u32,
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
    let model = get_model_matrix(vertex.instance_index);
    out.clip_position = mesh_position_local_to_clip(model, vec4<f32>(vertex.position, 1.0));
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv0 = vertex.uv0;
    out.uv1 = vertex.uv1;
    out.tile_info = vertex.tile_info;
    return out;
}

@group(2) @binding(0)
var tile_array_texture: binding_array<texture_2d<f32>>;
@group(2) @binding(1)
var tile_array_sampler: sampler;

// Detail texture for enhanced surface detail
@group(2) @binding(2)
var detail_texture: texture_2d<f32>;
@group(2) @binding(3)
var detail_sampler: sampler;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @location(3) uv1: vec2<f32>,
    @location(4) tile_info: u32,
};

struct FragmentOutput {
    @location(0) color: vec4<f32>,
};

// Gamma correction helpers
fn gamma_correct(color: vec3<f32>) -> vec3<f32> {
    // Simple gamma correction (assuming input is in sRGB space)
    // Use a simple approximation: color ^ 2.2 for encoding (to linear)
    return pow(color, vec3<f32>(2.2, 2.2, 2.2));
}

fn gamma_encode(color: vec3<f32>) -> vec3<f32> {
    // Convert from linear back to sRGB
    return pow(color, vec3<f32>(1.0/2.2, 1.0/2.2, 1.0/2.2));
}

// Extract tile index from tile_info
fn get_tile_index(tile_info: u32) -> u32 {
    return tile_info & 0xFFu;
}

// Check if this tile uses alpha blending
fn is_alpha_tile(tile_info: u32) -> bool {
    return (tile_info & 0x80000000u) != 0u;
}

// Get brush intensity from tile_info (for editor visualization)
fn get_brush_intensity(tile_info: u32) -> f32 {
    return f32((tile_info >> 8u) & 0xFFu) / 255.0;
}

// Improved normal calculation from height
fn calculate_normal_from_height(uv: vec2<f32>, texture_idx: u32, scale: f32) -> vec3<f32> {
    let texel_size = 1.0 / 256.0; // Adjust based on texture resolution
    let hL = textureSample(tile_array_texture[texture_idx], tile_array_sampler, uv + vec2<f32>(-texel_size, 0.0)).r;
    let hR = textureSample(tile_array_texture[texture_idx], tile_array_sampler, uv + vec2<f32>(texel_size, 0.0)).r;
    let hD = textureSample(tile_array_texture[texture_idx], tile_array_sampler, uv + vec2<f32>(0.0, -texel_size)).r;
    let hU = textureSample(tile_array_texture[texture_idx], tile_array_sampler, uv + vec2<f32>(0.0, texel_size)).r;
    
    // Calculate partial derivatives
    let dhdx = (hR - hL) * scale;
    let dhdy = (hU - hD) * scale;
    
    // Construct normal (pointing up in Z)
    return normalize(vec3<f32>(-dhdx, -dhdy, 2.0));
}

// Optimized tile sampling with mipmapping
fn sample_tile_texture(tile_idx: u32, uv: vec2<f32>) -> vec4<f32> {
    // Clamp UV coordinates to avoid edge artifacts
    let clamped_uv = clamp(uv, vec2<f32>(0.001, 0.001), vec2<f32>(0.999, 0.999));
    return textureSample(tile_array_texture[tile_idx], tile_array_sampler, clamped_uv);
}

// Fast approximate normal mapping without external normal map
fn calculate_fast_normal(uv: vec2<f32>, tile_idx: u32) -> vec3<f32> {
    let height = textureSample(tile_array_texture[tile_idx], tile_array_sampler, uv).r;
    
    // Calculate normal from height differences
    let offset = 0.01;
    let h1 = textureSample(tile_array_texture[tile_idx], tile_array_sampler, uv + vec2<f32>(offset, 0.0)).r;
    let h2 = textureSample(tile_array_texture[tile_idx], tile_array_sampler, uv + vec2<f32>(0.0, offset)).r;
    
    let dx = (h1 - height) * 2.0;
    let dy = (h2 - height) * 2.0;
    
    return normalize(vec3<f32>(-dx, -dy, 1.0));
}

@fragment
fn fragment(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;
    
    let tile_idx = get_tile_index(input.tile_info);
    
    // Sample base color from tile texture
    let base_color = sample_tile_texture(tile_idx, input.uv0);
    
    // Sample detail texture
    let detail = textureSample(detail_texture, detail_sampler, input.uv1);
    
    // Calculate normal from texture (for micro-detail)
    let fast_normal = calculate_fast_normal(input.uv0, tile_idx);
    
    // Simple directional lighting
    let light_dir = normalize(vec3<f32>(0.5, 1.0, 0.3));
    let normal = normalize(input.world_normal);
    
    // Blend normals (world normal + fast normal detail)
    let blended_normal = normalize(normal + fast_normal * 0.3);
    
    // Simple diffuse lighting
    let NdotL = max(dot(blended_normal, light_dir), 0.0);
    let ambient = 0.3;
    let lit = ambient + (1.0 - ambient) * NdotL;
    
    // Apply lighting to base color
    var final_color = base_color.rgb * lit;
    
    // Add detail contribution (subtle)
    final_color = final_color * (0.9 + 0.2 * detail.r);
    
    // Calculate view_z for zone lighting fog
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), input.world_position);
    
    // Apply zone lighting (world_position, world_normal, fragment_color, view_z)
    out.color = apply_zone_lighting(input.world_position, input.world_normal, vec4<f32>(final_color, base_color.a), view_z);
    
    return out;
}
