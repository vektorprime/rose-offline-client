// Particle shader using storage buffers for GPU-driven rendering
// CORRECTED VERSION - Fixed coordinate space transformations and alpha handling

#import bevy_render::view::View
#import bevy_pbr::mesh_bindings::mesh

@group(0) @binding(0) var<uniform> view: View;

// Storage buffers at group(2) bindings 0-3
@group(2) @binding(0)
var<storage, read> positions: array<vec4<f32>>;

@group(2) @binding(1)
var<storage, read> sizes: array<vec2<f32>>;

@group(2) @binding(2)
var<storage, read> colors: array<vec4<f32>>;

@group(2) @binding(3)
var<storage, read> textures: array<vec4<f32>>;

// Texture and sampler at bindings 4-5
@group(2) @binding(4)
var base_color_texture: texture_2d<f32>;

@group(2) @binding(5)
var base_color_sampler: sampler;

// Uniforms at bindings 6-9
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

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vertex(model: VertexInput) -> VertexOutput {
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

    // Get billboard vectors
    var camera_right: vec3<f32>;
    var camera_up: vec3<f32>;

    if (billboard_type == 1u) {
        // Y-axis billboard: rotate only around Y-axis
        // Extract camera right in XZ plane
        let view_right = view.world_from_view[0].xyz;
        camera_right = normalize(vec3<f32>(view_right.x, 0.0, view_right.z));
        camera_up = vec3<f32>(0.0, 1.0, 0.0);
    } else if (billboard_type == 2u) {
        // Full billboard: face camera completely
        camera_right = view.world_from_view[0].xyz;
        camera_up = view.world_from_view[1].xyz;
    } else {
        // No billboard (world-space aligned)
        camera_right = vec3<f32>(1.0, 0.0, 0.0);
        camera_up = vec3<f32>(0.0, 1.0, 0.0);
    }

    let particle_position = positions[particle_idx].xyz;
    let theta = positions[particle_idx].w;
    let size = sizes[particle_idx];
    
    // Apply rotation
    let sin_cos = vec2<f32>(cos(theta), sin(theta));
    let rotation = mat2x2<f32>(
        vec2<f32>(sin_cos.x, -sin_cos.y),
        vec2<f32>(sin_cos.y, sin_cos.x),
    );
    let vertex_position = rotation * vertex_positions[vert_idx];

    // Build quad in world space
    let world_position = 
        particle_position +
        (camera_right * vertex_position.x * size.x) +
        (camera_up * vertex_position.y * size.y);

    // CRITICAL FIX: Transform world -> clip correctly
    var out: VertexOutput;
    out.position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.color = colors[particle_idx];

    // Calculate UV coordinates from texture atlas info
    let texture = textures[particle_idx];
    if (vertex_positions[vert_idx].x < 0.0) {
        out.uv.x = texture.x;
    } else {
        out.uv.x = texture.z;
    }

    if (vertex_positions[vert_idx].y > 0.0) {
        out.uv.y = texture.y;
    } else {
        out.uv.y = texture.w;
    }

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(base_color_texture, base_color_sampler, in.uv);
    let result = in.color * texture_color;
    
    // Discard pixels with very low alpha to prevent black box artifacts
    // This is crucial for particles with black backgrounds in textures
    if (result.a < 0.01) {
        discard;
    }
    
    // Output premultiplied alpha for proper blending
    // RGB values are multiplied by alpha to avoid dark halos
    return vec4<f32>(result.rgb * result.a, result.a);
}
