// Damage Digit shader using storage buffers for GPU-driven rendering
// Reverted from mesh-based to storage buffer approach (Bevy 0.11 style with 0.14 syntax)

#import bevy_render::view::View

@group(0) @binding(0)
var<uniform> view: View;

struct PositionBuffer { data: array<vec4<f32>>, };
struct SizeBuffer { data: array<vec2<f32>>, };
struct UvBuffer { data: array<vec4<f32>>, };

@group(3) @binding(0)
var<storage, read> positions: PositionBuffer;
@group(3) @binding(1)
var<storage, read> sizes: SizeBuffer;
@group(3) @binding(2)
var<storage, read> uvs: UvBuffer;
@group(3) @binding(3)
var base_color_texture: texture_2d<f32>;
@group(3) @binding(4)
var base_color_sampler: sampler;

struct VertexInput {
  @builtin(vertex_index) vertex_idx: u32,
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
  let digit_idx = model.vertex_idx / 6u;

  let camera_right = view.view_from_world[0].xyz;
  let camera_up = view.view_from_world[1].xyz;

  let particle_position = positions.data[digit_idx].xyz;
  let x_offset = positions.data[digit_idx].w;
  let size = sizes.data[digit_idx];
  var vertex_position: vec2<f32> = vertex_positions[vert_idx].xy;
  vertex_position.x = vertex_positions[vert_idx].x + x_offset;

  var world_space: vec3<f32> =
    particle_position +
    (camera_right * vertex_position.x * size.x) +
    (camera_up * vertex_position.y * size.y);

  var out: VertexOutput;
  out.position = view.clip_from_view * vec4<f32>(world_space, 1.0);

  let texture = uvs.data[digit_idx];
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
  return textureSample(base_color_texture, base_color_sampler, in.uv);
}
