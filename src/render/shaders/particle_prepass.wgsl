// Particle prepass shader
// Provides required @location(0) world_position output for depth prepass

#import bevy_render::view::View
#import bevy_pbr::mesh_bindings::mesh

@group(0) @binding(0) var<uniform> view: View;

// Same storage buffers as main shader
@group(2) @binding(0)
var<storage, read> positions: array<vec4<f32>>;

@group(2) @binding(1)
var<storage, read> sizes: array<vec2<f32>>;

@group(2) @binding(2)
var<storage, read> colors: array<vec4<f32>>;

@group(2) @binding(3)
var<storage, read> textures: array<vec4<f32>>;

// Texture and sampler (even if not used in prepass, must match bindings)
@group(2) @binding(4)
var base_color_texture: texture_2d<f32>;

@group(2) @binding(5)
var base_color_sampler: sampler;

@group(2) @binding(6)
var<uniform> blend_op: u32;

@group(2) @binding(7)
var<uniform> src_blend_factor: u32;

@group(2) @binding(8)
var<uniform> dst_blend_factor: u32;

@group(2) @binding(9)
var<uniform> billboard_type: u32;

struct VertexInput {
    @builtin(vertex_index) vertex_idx: u32,
}

// Prepass output - MUST have @location(0) world_position
struct PrepassOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    #ifdef NORMAL_PREPASS
    @location(1) world_normal: vec3<f32>,
    #endif
}

@vertex
fn vertex(model: VertexInput) -> PrepassOutput {
    var vertex_positions: array<vec2<f32>, 6> = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, 1.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(1.0, -1.0),
        vec2<f32>(1.0, 1.0),
    );

    let vert_idx = model.vertex_idx % 6u;
    let particle_idx = model.vertex_idx / 6u;

    // Billboard calculation (same as main shader)
    var camera_right: vec3<f32>;
    var camera_up: vec3<f32>;

    if (billboard_type == 1u) {
        let view_right = view.world_from_view[0].xyz;
        camera_right = normalize(vec3<f32>(view_right.x, 0.0, view_right.z));
        camera_up = vec3<f32>(0.0, 1.0, 0.0);
    } else if (billboard_type == 2u) {
        camera_right = view.world_from_view[0].xyz;
        camera_up = view.world_from_view[1].xyz;
    } else {
        camera_right = vec3<f32>(1.0, 0.0, 0.0);
        camera_up = vec3<f32>(0.0, 1.0, 0.0);
    }

    let particle_position = positions[particle_idx].xyz;
    let theta = positions[particle_idx].w;
    let size = sizes[particle_idx];
    
    let sin_cos = vec2<f32>(cos(theta), sin(theta));
    let rotation = mat2x2<f32>(
        vec2<f32>(sin_cos.x, -sin_cos.y),
        vec2<f32>(sin_cos.y, sin_cos.x),
    );
    let vertex_position = rotation * vertex_positions[vert_idx];

    // World position
    let world_position = 
        particle_position +
        (camera_right * vertex_position.x * size.x) +
        (camera_up * vertex_position.y * size.y);

    var out: PrepassOutput;
    out.clip_position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.world_position = world_position;
    
    #ifdef NORMAL_PREPASS
    // Billboards always face camera
    out.world_normal = normalize(view.world_position - world_position);
    #endif

    return out;
}

@fragment
fn fragment(in: PrepassOutput) {
    // Prepass fragment doesn't need to output anything for depth-only
    // Normal prepass outputs are handled automatically
}
