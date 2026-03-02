// World UI Shader for rendering UI elements in world space
#import bevy_render::view::View

// Vertex shader bindings
@group(0) @binding(0)
var<uniform> view: View;

@group(1) @binding(0)
var base_texture: texture_2d<f32>;

@group(1) @binding(1)
var base_sampler: sampler;

// Zone lighting uniforms (when ZONE_LIGHTING_GROUP_2 is defined)
// Define ZoneLightingData struct locally to avoid bind group conflict with imported module
// which declares zone_lighting at group(3)
#ifdef ZONE_LIGHTING_GROUP_2
struct ZoneLightingData {
    // Group 0: 64 bytes (4 vec4)
    map_ambient_color: vec4<f32>,
    character_ambient_color: vec4<f32>,
    character_diffuse_color: vec4<f32>,
    light_direction: vec4<f32>,
    
    // Group 1: 64 bytes (4 vec4)
    fog_color: vec4<f32>,
    day_color: vec4<f32>,
    night_color: vec4<f32>,
    // Pack 4 f32 values into vec4 for alignment
    fog_params: vec4<f32>, // x = fog_density, y = fog_min_density, z = fog_max_density, w = fog_height_density
    
    // Group 2: 48 bytes (3 vec4)
    // Pack 4 f32 values into vec4 for alignment
    fog_height_params: vec4<f32>, // x = fog_min_height, y = fog_max_height, z = time_of_day, w = unused
    // Pack 2 f32 values with padding
    fog_alpha_params: vec4<f32>, // x = fog_alpha_range_start, y = fog_alpha_range_end, zw = unused
    _padding: vec4<f32>, // Padding to ensure total size is multiple of 16
};

@group(2) @binding(0)
var<uniform> zone_lighting: ZoneLightingData;
#endif

struct VertexInput {
    @location(0) world_position: vec3<f32>,
    @location(1) screen_position: vec2<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) world_position: vec3<f32>,
};

@vertex
fn vertex(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    
    // Transform world position directly to clip space
    let clip_pos = view.clip_from_world * vec4<f32>(in.world_position, 1.0);
    
    // Convert to NDC and apply offset, then convert back to clip space
    // screen_position is in pixels, convert to NDC (-1 to 1)
    let viewport_size = view.viewport.zw;
    let ndc_offset = vec2<f32>(in.screen_position.x, -in.screen_position.y) / viewport_size * 2.0;
    let ndc_pos = clip_pos.xy / clip_pos.w + ndc_offset;
    
    // Convert back to clip space by multiplying by w
    out.clip_position = vec4<f32>(ndc_pos * clip_pos.w, clip_pos.z, clip_pos.w);
    out.uv = in.uv;
    out.color = in.color;
    out.world_position = in.world_position;
    
    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample the texture
    let tex_color = textureSample(base_texture, base_sampler, in.uv);
    
    // Apply vertex color
    var out_color = tex_color * in.color;
    
#ifdef ZONE_LIGHTING_GROUP_2
    // Apply simple fog if zone lighting is available
    // let view_pos = view.world_position - in.world_position;
    // let distance = length(view_pos);
    // let fog_far = zone_lighting.fog_params.z;
    // let fog_near = zone_lighting.fog_params.y;
    // let fog_factor = clamp((fog_far - distance) / (fog_far - fog_near), 0.0, 1.0);
    // out_color = mix(zone_lighting.fog_color, out_color, fog_factor);
#endif
    
    return out_color;
}
