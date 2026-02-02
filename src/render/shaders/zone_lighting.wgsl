// Zone lighting shader module
// Updated for Bevy 0.14.2 naga 0.14.2 - fixed 16-byte alignment

#define_import_path rose_client::zone_lighting

struct ZoneLighting {
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

// Zone lighting is at group 3 (after view 0, mesh 1, material 2)
@group(3) @binding(0)
var<uniform> zone_lighting: ZoneLighting;

fn apply_zone_lighting_fog(world_position: vec4<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    let fog_density = zone_lighting.fog_params.x;
    let fog_min_density = zone_lighting.fog_params.y;
    let fog_max_density = zone_lighting.fog_params.z;
    let fog_alpha_range_start = zone_lighting.fog_alpha_params.x;
    let fog_alpha_range_end = zone_lighting.fog_alpha_params.y;
    
    var fog_amount: f32 = clamp(1.0 - exp2(-fog_density * fog_density * view_z * view_z * 1.442695), 0.0, 1.0);

    var fog_color: vec4<f32> = vec4<f32>(mix(fragment_color.rgb, zone_lighting.fog_color.rgb, clamp(fog_amount, fog_min_density, fog_max_density)), fragment_color.a);

    if (fog_amount >= fog_alpha_range_end) {
        discard;
    } else if (fog_amount >= fog_alpha_range_start) {
        fog_color.a = fog_color.a *(1.0 - (fog_amount - fog_alpha_range_start) / (fog_alpha_range_end - fog_alpha_range_start));
    }

    return fog_color;
}

fn apply_zone_lighting(world_position: vec4<f32>, world_normal: vec3<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    let light = saturate(zone_lighting.character_ambient_color.rgb + zone_lighting.character_diffuse_color.rgb * clamp(dot(world_normal, zone_lighting.light_direction.xyz), 0.0, 1.0));
    let lit_color = vec4<f32>(fragment_color.rgb * light.rgb, fragment_color.a);

    return apply_zone_lighting_fog(world_position, lit_color, view_z);
}
