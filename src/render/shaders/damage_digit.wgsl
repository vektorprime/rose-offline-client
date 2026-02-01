#import bevy_pbr::mesh_view_bindings::view
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::mesh_functions::{get_model_matrix, mesh_position_local_to_world}

struct DamageDigitMaterial {
    texture: texture_2d<f32>,
    sampler: sampler,
};

@group(1) @binding(0)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(1)
var base_color_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
    @builtin(instance_index) instance_index: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vertex(model: VertexInput) -> VertexOutput {
    var vertex_positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, 0.5),
        vec2<f32>(-0.5, 0.5),
        vec2<f32>(-0.5, -0.5),
        vec2<f32>(0.5, -0.5),
        vec2<f32>(0.5, 0.5),
    );

    let vert_idx = model.vertex_idx % 6u;
    
    // In a standard Material implementation, we'd usually use a Mesh.
    // For now, we'll assume the mesh is a simple quad and we use instance_index if needed,
    // but the original shader used storage buffers.
    // Since we are migrating to Material, we should ideally move to Mesh-based rendering.
    // However, to keep it simple and matching the plan's "Group 1" instruction:
    
    let camera_right = normalize(vec3<f32>(view.view_proj.x.x, view.view_proj.y.x, view.view_proj.z.x));
    let camera_up = normalize(vec3<f32>(view.view_proj.x.y, view.view_proj.y.y, view.view_proj.z.y));

    let model_matrix = get_model_matrix(model.instance_index);
    let world_pos = model_matrix[3].xyz;
    
    let vertex_position = vertex_positions[vert_idx];
    
    // Simple billboard
    let world_space = world_pos + 
                     (camera_right * vertex_position.x) + 
                     (camera_up * vertex_position.y);

    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(world_space, 1.0);
    out.uv = vertex_position + 0.5;
    out.uv.y = 1.0 - out.uv.y;

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(base_color_texture, base_color_sampler, in.uv);
}
