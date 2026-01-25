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
    // Height-based fog parameters
    fog_min_height: f32,
    fog_max_height: f32,
    fog_height_density: f32,
    // Time of day parameters
    time_of_day: f32,
    day_color: vec4<f32>,
    night_color: vec4<f32>,
};

#ifdef ZONE_LIGHTING_GROUP_2
@group(2) @binding(0)
var<uniform> zone_lighting: ZoneLighting;
#else
@group(3) @binding(0)
var<uniform> zone_lighting: ZoneLighting;
#endif

fn apply_zone_lighting_fog(world_position: vec4<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
    // Base fog calculation
    var fog_amount: f32 = clamp(1.0 - exp2(-zone_lighting.fog_density * zone_lighting.fog_density * view_z * view_z * 1.442695), 0.0, 1.0);
    
    // Height-based fog density
    let height_fog = smoothstep(zone_lighting.fog_min_height, zone_lighting.fog_max_height, world_position.y);
    fog_amount *= mix(1.0, height_fog, zone_lighting.fog_height_density);

    // Time-of-day fog color blending
    var final_fog_color = zone_lighting.fog_color.rgb;
    if (zone_lighting.time_of_day > 0.0) {
        let day_night_blend = smoothstep(0.2, 0.8, zone_lighting.time_of_day);
        final_fog_color = mix(zone_lighting.night_color.rgb, zone_lighting.day_color.rgb, day_night_blend);
    }

#ifdef ZONE_LIGHTING_DISABLE_COLOR_FOG
    var fog_color: vec4<f32> = fragment_color;
#else
    var fog_color: vec4<f32> = vec4<f32>(mix(fragment_color.rgb, final_fog_color, clamp(fog_amount, zone_lighting.fog_min_density, zone_lighting.fog_max_density)), fragment_color.a);
#endif

    // Volumetric fog scattering effect
    let scatter = pow(fog_amount, 0.5) * 0.2;
    fog_color = vec4<f32>(fog_color.rgb + scatter * final_fog_color, fog_color.a);

    if (fog_amount >= zone_lighting.fog_alpha_range_end) {
        discard;
    } else if (fog_amount >= zone_lighting.fog_alpha_range_start) {
        fog_color.a = fog_color.a *(1.0 - (fog_amount - zone_lighting.fog_alpha_range_start) / (zone_lighting.fog_alpha_range_end - zone_lighting.fog_alpha_range_start));
    }

    return fog_color;
}

fn apply_zone_lighting(world_position: vec4<f32>, world_normal: vec3<f32>, fragment_color: vec4<f32>, view_z: f32) -> vec4<f32> {
#ifdef ZONE_LIGHTING_CHARACTER
    // Enhanced character lighting with rim lighting and time-of-day effects
    let N = normalize(world_normal);
    let L = normalize(zone_lighting.light_direction.xyz);
    let V = normalize(-world_position.xyz); // View direction
    
    // Diffuse lighting
    let diffuse = clamp(dot(N, L), 0.0, 1.0);
    
    // Rim lighting for better silhouette definition
    let rim = pow(1.0 - saturate(dot(N, V)), 2.0) * 0.2;
    
    // Time-of-day color temperature adjustment
    let time_factor = mix(0.8, 1.2, zone_lighting.time_of_day); // Warmer at night, cooler during day
    let time_color = mix(vec3<f32>(1.0, 0.8, 0.6), vec3<f32>(0.8, 0.9, 1.0), zone_lighting.time_of_day);
    
    let light = saturate(
        zone_lighting.character_ambient_color.rgb * time_color * time_factor +
        zone_lighting.character_diffuse_color.rgb * diffuse * time_color +
        zone_lighting.character_diffuse_color.rgb * rim * 0.5
    );
    let lit_color = vec4<f32>(fragment_color.rgb * light.rgb, fragment_color.a);
#else
    // Enhanced map lighting with subtle time-of-day effects
    let time_factor = mix(0.9, 1.1, zone_lighting.time_of_day);
    let lit_color = vec4<f32>(fragment_color.rgb * zone_lighting.map_ambient_color.rgb * time_factor, fragment_color.a);
#endif

    return apply_zone_lighting_fog(world_position, lit_color, view_z);
}
