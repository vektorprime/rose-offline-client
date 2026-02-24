use bevy::{
    asset::Assets,
    pbr::MeshMaterial3d,
    prelude::*,
    render::mesh::Mesh3d,
};
use crate::components::{GrassBlade, PlayerCharacter, Season, SeasonMarker, SummerFlower};
use crate::resources::{CurrentZone, SeasonMaterials, SeasonSettings, SummerSettings};
use crate::zone_loader::ZoneLoaderAsset;

/// Spawns grass blades and flowers for summer season
/// Vegetation spawns on terrain near the player and sways in the wind
pub fn summer_vegetation_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    summer_settings: Res<SummerSettings>,
    season_materials: Res<SeasonMaterials>,
    player_query: Query<&GlobalTransform, With<PlayerCharacter>>,
    grass_query: Query<(), With<GrassBlade>>,
    flower_query: Query<(), With<SummerFlower>>,
    time: Res<Time>,
    mut frame_counter: Local<u32>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    *frame_counter += 1;
    
    // Debug log every 60 frames to avoid spam
    if *frame_counter % 60 == 0 {
        // info!(
        //     "[SummerSystem] enabled={}, season={:?}, has_player={}, grass_count={}, flower_count={}",
        //     settings.enabled,
        //     settings.current_season,
        //     player_query.get_single().is_ok(),
        //     grass_query.iter().len(),
        //     flower_query.iter().len()
        // );
    }
    
    if !settings.enabled {
        if *frame_counter % 60 == 0 {
            //info!("[SummerSystem] Returning early - season system disabled");
        }
        return;
    }
    
    if settings.current_season != Season::Summer {
        if *frame_counter % 60 == 0 {
            //info!("[SummerSystem] Returning early - current season is {:?}, not Summer", settings.current_season);
        }
        return;
    }

    // Get player position for player-relative spawning
    let Ok(player_transform) = player_query.get_single() else {
        if *frame_counter % 60 == 0 {
            //info!("[SummerSystem] Returning early - no player found");
        }
        return;
    };
    let player_pos = player_transform.translation();

    let dt = time.delta_secs();

    // Count current vegetation
    let current_grass_count = grass_query.iter().len();
    let current_flower_count = flower_query.iter().len();

    // Get zone data for terrain height sampling
    let zone_data = current_zone.as_ref().and_then(|cz| zone_loader_assets.get(&cz.handle));

    // Spawn grass blades if below maximum
    if current_grass_count < summer_settings.max_grass_blades {
        // Spawn a few grass blades per frame based on spawn rate
        let grass_to_spawn = ((summer_settings.max_grass_blades - current_grass_count) as f32 * dt * 0.5).min(10.0) as usize;
        
        for _ in 0..grass_to_spawn {
            spawn_grass_blade(
                &mut commands,
                &summer_settings,
                &season_materials,
                player_pos,
                zone_data,
            );
        }
    }

    // Spawn flowers if below maximum and random chance succeeds
    if current_flower_count < summer_settings.max_flowers {
        let flower_chance = summer_settings.flower_spawn_chance * dt;
        
        if rand::random::<f32>() < flower_chance {
            spawn_summer_flower(
                &mut commands,
                &summer_settings,
                &season_materials,
                player_pos,
                zone_data,
            );
        }
    }
}

/// Helper function to get terrain height at a position
/// Returns the terrain height in world units, or a fallback value if zone data is unavailable
fn get_terrain_height_at(zone_data: Option<&ZoneLoaderAsset>, world_x: f32, world_z: f32) -> f32 {
    if let Some(zone) = zone_data {
        // Convert world coordinates to zone coordinates
        // World Z is negative of zone Y (see game_connection_system.rs for coordinate transform)
        let zone_x = world_x * 100.0;
        let zone_y = -world_z * 100.0;
        zone.get_terrain_height(zone_x, zone_y) / 100.0
    } else {
        // Fallback: use player height or default
        0.0
    }
}

/// Spawns a single grass blade at a random position near the player
fn spawn_grass_blade(
    commands: &mut Commands,
    summer_settings: &SummerSettings,
    season_materials: &SeasonMaterials,
    player_pos: Vec3,
    zone_data: Option<&ZoneLoaderAsset>,
) {
    // Random position within spawn radius
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let radius = rand::random::<f32>().sqrt() * summer_settings.spawn_radius;
    let offset_x = angle.cos() * radius;
    let offset_z = angle.sin() * radius;

    // Calculate world position
    let world_x = player_pos.x + offset_x;
    let world_z = player_pos.z + offset_z;
    
    // Sample terrain height at this position
    let terrain_height = get_terrain_height_at(zone_data, world_x, world_z);
    
    let position = Vec3::new(
        world_x,
        terrain_height,
        world_z,
    );

    // Random grass height within range
    let height = summer_settings.grass_height_range.0
        + rand::random::<f32>() * (summer_settings.grass_height_range.1 - summer_settings.grass_height_range.0);

    // Random grass material
    let material_index = rand::random::<usize>() % season_materials.grass_materials.len();
    let grass_material = season_materials.grass_materials[material_index].clone();

    // Random sway parameters for variation
    let sway_offset = rand::random::<f32>() * std::f32::consts::TAU;
    let sway_speed = summer_settings.grass_sway_speed * (0.8 + rand::random::<f32>() * 0.4);
    let sway_amplitude = summer_settings.grass_sway_amplitude * (0.8 + rand::random::<f32>() * 0.4);

    commands.spawn((
        Mesh3d(season_materials.grass_mesh.clone()),
        MeshMaterial3d(grass_material),
        Transform::from_translation(position)
            .with_scale(Vec3::new(
                summer_settings.grass_width,
                height,
                1.0,
            )),
        GrassBlade {
            sway_offset,
            sway_speed,
            sway_amplitude,
            height,
        },
        SeasonMarker(Season::Summer),
    ));
}

/// Spawns a single summer flower at a random position near the player
fn spawn_summer_flower(
    commands: &mut Commands,
    summer_settings: &SummerSettings,
    season_materials: &SeasonMaterials,
    player_pos: Vec3,
    zone_data: Option<&ZoneLoaderAsset>,
) {
    // Random position within spawn radius
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let radius = rand::random::<f32>().sqrt() * summer_settings.spawn_radius;
    let offset_x = angle.cos() * radius;
    let offset_z = angle.sin() * radius;

    // Calculate world position
    let world_x = player_pos.x + offset_x;
    let world_z = player_pos.z + offset_z;
    
    // Sample terrain height at this position
    let terrain_height = get_terrain_height_at(zone_data, world_x, world_z);
    
    // Position slightly above terrain
    let position = Vec3::new(
        world_x,
        terrain_height + 0.05,
        world_z,
    );

    // Random flower color
    let color_index = rand::random::<usize>() % season_materials.summer_flower_materials.len();
    let flower_material = season_materials.summer_flower_materials[color_index].clone();

    // Random stem height within range
    let stem_height = summer_settings.flower_stem_height_range.0
        + rand::random::<f32>() * (summer_settings.flower_stem_height_range.1 - summer_settings.flower_stem_height_range.0);

    // Random sway parameters
    let sway_offset = rand::random::<f32>() * std::f32::consts::TAU;
    let sway_speed = 1.2 * (0.8 + rand::random::<f32>() * 0.4);

    commands.spawn((
        Mesh3d(season_materials.summer_flower_mesh.clone()),
        MeshMaterial3d(flower_material),
        Transform::from_translation(position)
            .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)) // Face upward
            .with_scale(Vec3::splat(summer_settings.flower_head_size)),
        SummerFlower {
            sway_offset,
            sway_speed,
            color_index,
            stem_height,
        },
        SeasonMarker(Season::Summer),
    ));
}

/// Animates vegetation swaying in the wind
/// Vegetation uses billboard behavior to always face the camera
pub fn vegetation_sway_system(
    settings: Res<SeasonSettings>,
    summer_settings: Res<SummerSettings>,
    mut grass_query: Query<(&mut Transform, &GrassBlade), (With<GrassBlade>, Without<SummerFlower>)>,
    mut flower_query: Query<(&mut Transform, &SummerFlower), (With<SummerFlower>, Without<GrassBlade>)>,
    camera_query: Query<&GlobalTransform, With<Camera3d>>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season != Season::Summer {
        return;
    }

    let current_time = time.elapsed_secs();
    let wind_intensity = summer_settings.wind_intensity;
    let wind_dir = settings.wind_direction;

    // Get camera transform for billboard behavior
    let Ok(camera_transform) = camera_query.get_single() else {
        return;
    };
    let camera_pos = camera_transform.translation();

    // Animate grass blades
    for (mut transform, grass) in grass_query.iter_mut() {
        // Calculate sway based on time and grass parameters
        let sway_phase = current_time * grass.sway_speed + grass.sway_offset;
        
        // Primary sway in wind direction
        let sway_x = (sway_phase.sin() * grass.sway_amplitude * wind_intensity) * wind_dir.x;
        let sway_z = (sway_phase.sin() * grass.sway_amplitude * wind_intensity) * wind_dir.y;
        
        // Add some perpendicular wobble for more natural movement
        let wobble = (sway_phase * 1.7).sin() * grass.sway_amplitude * 0.3 * wind_intensity;
        
        // Billboard: Make grass blade face the camera
        let to_camera = camera_pos - transform.translation;
        if to_camera.length_squared() > 0.001 {
            let forward = to_camera.normalize();
            // Create a rotation that faces the camera (billboard look-at)
            let up = Vec3::Y;
            let right = up.cross(forward).normalize();
            let corrected_up = forward.cross(right).normalize();
            let look_rotation = Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward));
            
            // Apply sway rotation on top of billboard
            let sway_rotation = Quat::from_euler(
                EulerRot::XYZ,
                sway_z + wobble * 0.5, // X rotation (forward/back)
                0.0,
                sway_x + wobble,       // Z rotation (left/right)
            );
            
            transform.rotation = look_rotation * sway_rotation;
        }
    }

    // Animate flowers
    for (mut transform, flower) in flower_query.iter_mut() {
        // Flowers sway more gently than grass
        let sway_phase = current_time * flower.sway_speed + flower.sway_offset;
        
        // Gentle sway in wind direction
        let sway_amount = 0.05 * wind_intensity;
        let sway_x = (sway_phase.sin() * sway_amount) * wind_dir.x;
        let sway_z = (sway_phase.sin() * sway_amount) * wind_dir.y;
        
        // Billboard: Make flower face the camera
        let to_camera = camera_pos - transform.translation;
        if to_camera.length_squared() > 0.001 {
            let forward = to_camera.normalize();
            // Create a rotation that faces the camera (billboard look-at)
            let up = Vec3::Y;
            let right = up.cross(forward).normalize();
            let corrected_up = forward.cross(right).normalize();
            let look_rotation = Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward));
            
            // Apply gentle sway rotation on top of billboard
            let sway_rotation = Quat::from_euler(
                EulerRot::XYZ,
                sway_z,
                0.0,
                sway_x,
            );
            
            transform.rotation = look_rotation * sway_rotation;
        }
    }
}
