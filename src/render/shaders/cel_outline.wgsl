// Screen-space cel outlines (render/cel_outline.rs).
//
// A fullscreen triangle drawn after the opaque main pass and before the transparent
// pass, alpha-blending the outline color into the main HDR target. Water, particles,
// effects and name tags are drawn over the lines afterwards; volumetric fog,
// bloom, tonemapping and SMAA then process them like the rest of the frame.
//
// Edges come from the prepass depth and normal textures, sampled on two rings of eight
// neighbors around each pixel:
// - silhouette: a neighbor is clearly farther away than this pixel. The line is drawn
//   only on the nearer side, so it hugs the object in front.
// - crease: the surface normal turns sharply between two pixels at about the same depth.
// The normal texture's alpha (2 bits) carries the outline class written by the
// material prepass: 1 = regular, 1/3 = alpha-masked (foliage etc.: silhouettes only,
// against a much farther background), 0 = nothing drawn there (sky).

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput
#import bevy_render::view::View

// Matches CelOutline (cel_outline.rs).
struct CelOutlineSettings {
    color: vec4<f32>,
    thickness: f32,
    depth_threshold: f32,
    normal_threshold: f32,
    masked_depth_scale: f32,
    fade_start: f32,
    fade_end: f32,
    reference_height: f32,
    _padding: f32,
}

@group(0) @binding(0) var depth_texture: texture_depth_2d;
@group(0) @binding(1) var normal_texture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> view: View;
@group(0) @binding(3) var<uniform> settings: CelOutlineSettings;

const CLASS_NONE: u32 = 0u;
const CLASS_MASKED: u32 = 1u;

// Distance used for pixels with nothing drawn (reverse-Z depth 0 is infinitely far).
const SKY_DISTANCE: f32 = 1.0e9;

struct GBufferSample {
    // View-space distance along the view direction.
    distance: f32,
    normal: vec3<f32>,
    class: u32,
}

fn load_sample(pixel: vec2<i32>) -> GBufferSample {
    let size = vec2<i32>(textureDimensions(depth_texture));
    let p = clamp(pixel, vec2<i32>(0), size - vec2<i32>(1));

    var s: GBufferSample;
    let depth = textureLoad(depth_texture, p, 0);
    // Infinite reverse-Z perspective: depth = near / distance, near = clip_from_view[3][2].
    s.distance = select(SKY_DISTANCE, view.clip_from_view[3][2] / depth, depth > 0.0);

    let normal = textureLoad(normal_texture, p, 0);
    s.normal = normalize(normal.xyz * 2.0 - vec3<f32>(1.0));
    s.class = u32(round(normal.a * 3.0));
    if (depth <= 0.0) {
        s.class = CLASS_NONE;
    }
    return s;
}

const DIRECTIONS: array<vec2<i32>, 8> = array<vec2<i32>, 8>(
    vec2<i32>(1, 0), vec2<i32>(-1, 0), vec2<i32>(0, 1), vec2<i32>(0, -1),
    vec2<i32>(1, 1), vec2<i32>(-1, 1), vec2<i32>(1, -1), vec2<i32>(-1, -1),
);

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let pixel = vec2<i32>(floor(in.position.xy));
    let center = load_sample(pixel);
    if (center.class == CLASS_NONE) {
        discard;
    }

    let fade = 1.0 - smoothstep(settings.fade_start, settings.fade_end, center.distance);
    if (fade <= 0.0) {
        discard;
    }

    // Line width in pixels, scaled with the vertical resolution.
    let scale = view.viewport.w / settings.reference_height;
    let outer_radius = max(1, i32(floor(settings.thickness * scale + 0.5)));
    let inner_radius = max(1, (outer_radius + 1) / 2);

    // Relative depth change per pixel expected on this pixel's own plane, so surfaces
    // seen at a grazing angle (ground, walls along the view) do not count as edges.
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let near_point = view.world_from_clip * vec4<f32>(ndc, 1.0, 1.0);
    let view_ray = normalize(near_point.xyz / near_point.w - view.world_position);
    let n_dot_v = clamp(abs(dot(center.normal, view_ray)), 0.05, 1.0);
    let tan_theta = sqrt(1.0 - n_dot_v * n_dot_v) / n_dot_v;
    let pixel_angle = 2.0 / (view.clip_from_view[1][1] * view.viewport.w);
    let plane_slope = pixel_angle * tan_theta;

    var depth_threshold = settings.depth_threshold;
    if (center.class == CLASS_MASKED) {
        depth_threshold *= settings.masked_depth_scale;
    }
    let crease_threshold = 1.0 - settings.normal_threshold;

    // Function-scope copy: only variables may be indexed with a runtime index.
    var directions = DIRECTIONS;
    var edge = 0.0;
    for (var ring = 0; ring < 2; ring++) {
        let radius = select(inner_radius, outer_radius, ring == 0);
        if (ring == 1 && radius == outer_radius) {
            break;
        }
        for (var i = 0; i < 8; i++) {
            let offset = directions[i] * radius;
            let neighbor = load_sample(pixel + offset);
            let offset_length = length(vec2<f32>(offset));
            let relative = (neighbor.distance - center.distance) / center.distance;

            // Silhouette: the neighbor is farther than this pixel's plane predicts.
            if (relative > depth_threshold + plane_slope * offset_length * 1.5) {
                edge = 1.0;
                break;
            }

            // Crease between two regular surfaces at the same depth; drawn on the
            // nearer (or equal) side only.
            if (center.class != CLASS_MASKED && neighbor.class > CLASS_MASKED
                    && relative >= 0.0
                    && dot(center.normal, neighbor.normal) < crease_threshold) {
                edge = 1.0;
                break;
            }
        }
        if (edge > 0.0) {
            break;
        }
    }

    let alpha = settings.color.a * edge * fade;
    if (alpha <= 0.0) {
        discard;
    }
    return vec4<f32>(settings.color.rgb, alpha);
}
