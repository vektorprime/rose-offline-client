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
#ifdef ZONE_LIGHTING_GROUP_2
#import rose_client::zone_lighting::ZoneLighting
@group(2) @binding(0)
var<uniform> zone_lighting: ZoneLighting;
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
    
    // Transform world position to clip space
    let clip_pos = view.clip_from_view * vec4<f32>(in.world_position, 1.0);
    
    // Convert to screen space and apply offset
    let screen_pos = (clip_pos.xy / clip_pos.w) + (in.screen_position / view.viewport.zw) * 2.0;
    
    out.clip_position = vec4<f32>(screen_pos, clip_pos.z, clip_pos.w);
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
    let view_pos = view.world_position - in.world_position;
    let distance = length(view_pos);
    let fog_far = zone_lighting.fog_params.z;
    let fog_near = zone_lighting.fog_params.y;
    let fog_factor = clamp((fog_far - distance) / (fog_far - fog_near), 0.0, 1.0);
    out_color = mix(zone_lighting.fog_color, out_color, fog_factor);
#endif
    
    return out_color;
}
