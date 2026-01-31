// Minimal zone lighting shader - simplified for stability
#define_import_path rose_client::zone_lighting

struct ZoneLighting {
    map_ambient_color: vec4<f32>,
    character_ambient_color: vec4<f32>,
    character_diffuse_color: vec4<f32>,
    light_direction: vec4<f32>,
    fog_color: vec4<f32>,
    fog_density: f32,
    fog_min_density: f32,
    fog_max_density: f32,
    fog_alpha_range_start: f32,
    fog_alpha_range_end: f32,
    fog_min_height: f32,
    fog_max_height: f32,
    fog_height_density: f32,
    time_of_day: f32,
    day_color: vec4<f32>,
    night_color: vec4<f32>,
};

// Zone lighting is at group 3 (after view 0, mesh 1, material 2)
@group(3) @binding(0)
var<uniform> zone_lighting: ZoneLighting;

// Simplified fog function
fn apply_zone_lighting_fog(world_position: vec4<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    // Simple distance fog
    let fog_amount = clamp(view_z * zone_lighting.fog_density * 0.001, 0.0, 1.0);
    let final_color = mix(fragment_color.rgb, zone_lighting.fog_color.rgb, fog_amount);
    return vec4<f32>(final_color, fragment_color.a);
}

// Simplified lighting function
fn apply_zone_lighting(world_position: vec4<f32>, world_normal: vec3<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    // Simple ambient lighting only
    let ambient = zone_lighting.map_ambient_color.rgb;
    let lit_color = vec4<f32>(fragment_color.rgb * ambient, fragment_color.a);
    
    // Apply fog
    return apply_zone_lighting_fog(world_position, lit_color, view_z);
}
