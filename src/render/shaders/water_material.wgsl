#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_functions::{mesh_position_local_to_world, mesh_normal_local_to_world, get_model_matrix}
#import rose_client::zone_lighting::apply_zone_lighting

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let model = get_model_matrix(vertex.instance_index);
    out.world_position = mesh_position_local_to_world(model, vec4<f32>(vertex.position, 1.0));
    out.world_normal = mesh_normal_local_to_world(vertex.normal, vertex.instance_index);
    out.uv0 = vertex.uv0;

    out.clip_position = view.view_proj * out.world_position;
    return out;
}

@group(2) @binding(0)
var water_array_texture: binding_array<texture_2d<f32>>;
@group(2) @binding(1)
var water_array_sampler: sampler;

struct WaterTextureIndex {
    current_index: i32,
    next_index: i32,
    next_weight: f32,
};
var<push_constant> water_texture_index: WaterTextureIndex;

struct FragmentInput {
    @builtin(position) frag_coord: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv0: vec2<f32>,
};

@fragment
fn fragment(in: FragmentInput) -> @location(0) vec4<f32> {
    let view_z = dot(vec4<f32>(
        view.inverse_view[0].z,
        view.inverse_view[1].z,
        view.inverse_view[2].z,
        view.inverse_view[3].z
    ), in.world_position);

    // Sample water textures with animation
    let color1 = textureSample(water_array_texture[water_texture_index.current_index], water_array_sampler, in.uv0);
    let color2 = textureSample(water_array_texture[water_texture_index.next_index], water_array_sampler, in.uv0);
    var water_color = mix(color1, color2, water_texture_index.next_weight);
    
    // Enhanced water effects
    let N = normalize(in.world_normal);
    let V = normalize(view.world_position.xyz - in.world_position.xyz);
    
    // Fresnel effect for water edges
    let fresnel = pow(1.0 - saturate(dot(N, V)), 3.0) * 0.5;
    
    // Specular highlights
    let L = normalize(vec3<f32>(0.5, 1.0, 0.3)); // Light direction
    let H = normalize(L + V);
    let specular = pow(max(dot(N, H), 0.0), 64.0) * 0.8;
    
    // Add water surface effects
    water_color = vec4<f32>(water_color.rgb + fresnel * vec3<f32>(0.2, 0.3, 0.4), water_color.a); // Blue-ish edge glow
    water_color = vec4<f32>(water_color.rgb + specular * vec3<f32>(1.0, 0.9, 0.8), water_color.a); // White highlights
    
    // Depth-based color variation
    let depth_factor = saturate(1.0 - view_z * 0.0001); // Adjust multiplier as needed
    water_color = vec4<f32>(mix(water_color.rgb, vec3<f32>(0.1, 0.3, 0.6), depth_factor * 0.3), water_color.a);
    
    // Apply zone lighting
    return apply_zone_lighting(in.world_position, in.world_normal, water_color, view_z);
}
