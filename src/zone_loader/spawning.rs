use super::*;

mod objects;
mod terrain;
mod water;

use self::objects::{spawn_animated_object, spawn_effect_object, spawn_object, spawn_sound_object};
use self::terrain::{spawn_new_terrain, spawn_terrain};
use self::water::spawn_water;

pub fn spawn_zone(
    params: &mut SpawnZoneParams,
    zone_data: &ZoneLoaderAsset,
) -> Result<(Entity, Vec<UntypedHandle>), anyhow::Error> {
    let _span = info_span!("spawn_zone", zone_id = zone_data.zone_id.get()).entered();
    log::info!("[SPAWN ZONE] ===========================================");
    log::info!(
        "[SPAWN ZONE] spawn_zone called for zone_id: {}",
        zone_data.zone_id.get()
    );
    log::info!("[SPAWN ZONE] Zone path: {:?}", zone_data.zone_path);
    log::info!("[SPAWN ZONE] Number of blocks: {}", zone_data.blocks.len());
    let new_terrain_blocks = zone_data
        .blocks
        .iter()
        .filter(|b| {
            b.as_ref()
                .map(|b| b.new_terrain_mesh.is_some())
                .unwrap_or(false)
        })
        .count();
    log::info!(
        "[SPAWN ZONE] Blocks with new_terrain_mesh: {}",
        new_terrain_blocks
    );
    log::info!("[SPAWN ZONE] Number of NPCs: {}", zone_data.npcs.len());
    log::info!("[SPAWN ZONE] ===========================================");

    // DIAGNOSTIC: Track function entry
    log::info!("[SPAWN ZONE DIAGNOSTIC] spawn_zone function ENTRY - about to spawn zone entity");

    // Memory tracking: Count blocks with data
    let blocks_with_data = zone_data.blocks.iter().filter(|b| b.is_some()).count();
    log::info!(
        "[MEMORY] Blocks with data: {}/{}",
        blocks_with_data,
        zone_data.blocks.len()
    );

    let SpawnZoneParams {
        commands,
        asset_server,
        game_data,
        vfs_resource,
        meshes,
        specular_texture,
        standard_materials,
        terrain_materials,
        water_materials,
        effect_mesh_materials,
        object_materials,
        particle_materials,
        storage_buffers,
        zone_loader_assets: _,
        render_config,
        memory_tracking,
        ref mut water_spawned_events,
        terrain_noise,
        effect_cache,
    } = params;
    log::info!(
        "[SPAWN ZONE] render_config.use_new_terrain: {}",
        render_config.use_new_terrain
    );

    let zone_list_entry = game_data
        .zone_list
        .get_zone(zone_data.zone_id)
        .ok_or(ZoneLoadError::InvalidZoneId)?;

    let mut tile_textures: Vec<Handle<Image>> =
        Vec::with_capacity(zone_data.zon.tile_textures.len());
    for path in zone_data.zon.tile_textures.iter() {
        if path == "end" {
            break;
        }

        let handle = asset_server.load(path);
        memory_tracking.log_texture_handle_created(path);
        tile_textures.push(handle);
    }
    log::info!("[SPAWN ZONE] Loaded {} tile textures", tile_textures.len());
    log::info!(
        "[MEMORY] Tile texture handles created: {}",
        tile_textures.len()
    );

    let water_material = {
        // Use custom WaterMaterial with fully procedural shading (no game texture dependencies)
        let material = water_materials.add(WaterMaterial::default());
        log::info!("[SPAWN ZONE] Procedural water material created (texture-free)");
        log::info!("[MEMORY] Water material handle created");
        material
    };

    let mut zone_loading_assets: Vec<UntypedHandle> = Vec::default();
    let zone_entity = commands
        .spawn((
            Zone {
                id: zone_data.zone_id,
            },
            Visibility::Visible,
            ViewVisibility::default(),
            InheritedVisibility::default(),
            Transform::from_xyz(5200.0, 0.0, -5200.0),
            GlobalTransform::default(),
            Aabb::from_min_max(Vec3::splat(-100000.0), Vec3::splat(100000.0)),
            RenderLayers::layer(0),
        ))
        .id();
    log::info!(
        "[ZONE LOADER DEBUG] Spawned Zone entity {:?} with Visibility::Visible and large Aabb",
        zone_entity
    );
    // info!("[ASSET LIFECYCLE] Zone entity spawned: {:?} (zone_id: {})", zone_entity, zone_data.zone_id.get());
    memory_tracking.log_entity_spawned("Zone", 0);
    log::info!("[SPAWN ZONE] Zone entity spawned: {:?}", zone_entity);
    log::info!("[MEMORY] Zone entity created: {:?}", zone_entity);

    // DIAGNOSTIC: Confirm zone entity was spawned
    log::info!(
        "[SPAWN ZONE DIAGNOSTIC] ✓ Zone entity SUCCESSFULLY SPAWNED: entity={:?}, zone_id={}",
        zone_entity,
        zone_data.zone_id.get()
    );

    // Cartoon sky removed - now using Bevy 0.16 built-in atmospheric scattering
    // The Atmosphere and AtmosphereSettings components are added to the camera instead
    // This provides physics-based Rayleigh and Mie scattering with dynamic time-of-day
    log::info!(
        "[SPAWN ZONE] Using Bevy 0.16 built-in atmospheric scattering (cartoon sky disabled)"
    );

    let mut terrain_count = 0;
    let mut water_count = 0;
    let mut event_object_count = 0;
    let mut warp_object_count = 0;
    let mut cnst_object_count = 0;
    let mut deco_object_count = 0;
    let mut animated_object_count = 0;
    let mut effect_object_count = 0;
    let mut sound_object_count = 0;

    for block_y in 0..64 {
        for block_x in 0..64 {
            if let Some(block_data) = zone_data.blocks[block_x + block_y * 64].as_ref() {
                log::info!(
                    "[SPAWN ZONE] Processing block {}_{}, new_terrain_mesh: {:?}",
                    block_x,
                    block_y,
                    block_data.new_terrain_mesh.is_some()
                );
                let terrain_entity =
                    if render_config.use_new_terrain && block_data.new_terrain_mesh.is_some() {
                        spawn_new_terrain(
                            commands,
                            asset_server,
                            meshes,
                            standard_materials,
                            zone_data,
                            block_data,
                        )
                    } else {
                        spawn_terrain(
                            commands,
                            asset_server,
                            meshes,
                            terrain_materials,
                            &tile_textures,
                            zone_data,
                            block_data,
                            terrain_noise,
                        )
                    };
                commands.entity(zone_entity).add_child(terrain_entity);
                terrain_count += 1;

                if let Some(ifo) = block_data.ifo.as_ref() {
                    let lightmap_path = zone_data
                        .zone_path
                        .join(format!("{}_{}/LIGHTMAP/", block_x, block_y));

                    for (plane_start, plane_end) in ifo.water_planes.iter() {
                        let (water_entity, water_center, water_half_extents) = spawn_water(
                            commands,
                            meshes,
                            block_x as u32,
                            block_y as u32,
                            ifo.water_size,
                            Vec3::new(plane_start.x, plane_start.y, plane_start.z),
                            Vec3::new(plane_end.x, plane_end.y, plane_end.z),
                            &water_material,
                        );
                        commands.entity(zone_entity).add_child(water_entity);
                        water_count += 1;

                        // Send event to spawn fish in this water
                        // log::info!("[FISH DEBUG] Sending WaterSpawnedEvent from zone_loader: water_entity={:?}, zone_entity={:?}, center={:?}, extents={:?}",
                        //     water_entity, zone_entity, water_center, water_half_extents);
                        water_spawned_events.write(WaterSpawnedEvent {
                            water_entity,
                            zone_entity,
                            water_center,
                            water_half_extents,
                        });
                    }

                    for (ifo_object_id, event_object) in ifo.event_objects.iter().enumerate() {
                        let event_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            object_materials.as_mut(),
                            specular_texture,
                            &game_data.zsc_event_object,
                            &lightmap_path,
                            None,
                            &event_object.object,
                            ifo_object_id,
                            event_object.object.object_id as usize,
                            ZoneObject::EventObject,
                            ZoneObject::EventObjectPart,
                            COLLISION_GROUP_ZONE_EVENT_OBJECT,
                        );

                        commands.entity(event_entity).insert(EventObject::new(
                            event_object.quest_trigger_name.clone(),
                            event_object.script_function_name.clone(),
                        ));
                        commands.entity(zone_entity).add_child(event_entity);
                        event_object_count += 1;
                    }

                    for (ifo_object_id, warp_object) in ifo.warps.iter().enumerate() {
                        let warp_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            object_materials.as_mut(),
                            specular_texture,
                            &game_data.zsc_special_object,
                            &lightmap_path,
                            None,
                            warp_object,
                            ifo_object_id,
                            1,
                            ZoneObject::WarpObject,
                            ZoneObject::WarpObjectPart,
                            COLLISION_GROUP_ZONE_WARP_OBJECT,
                        );

                        commands
                            .entity(warp_entity)
                            .insert(WarpObject::new(WarpGateId::new(warp_object.warp_id)));
                        commands.entity(zone_entity).add_child(warp_entity);
                        warp_object_count += 1;
                    }

                    for (ifo_object_id, object_instance) in ifo.cnst_objects.iter().enumerate() {
                        let lit_object = block_data.lit_cnst.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == ifo_object_id + 1)
                        });

                        let object_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            object_materials.as_mut(),
                            specular_texture,
                            &zone_data.zsc_cnst,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            ifo_object_id,
                            object_instance.object_id as usize,
                            ZoneObject::CnstObject,
                            ZoneObject::CnstObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        cnst_object_count += 1;
                    }

                    for (ifo_object_id, object_instance) in ifo.deco_objects.iter().enumerate() {
                        let lit_object = block_data.lit_deco.as_ref().and_then(|lit| {
                            lit.objects
                                .iter()
                                .find(|lit_object| lit_object.id as usize == ifo_object_id + 1)
                        });

                        let object_entity = spawn_object(
                            commands,
                            asset_server,
                            &mut zone_loading_assets,
                            vfs_resource,
                            object_materials.as_mut(),
                            specular_texture,
                            &zone_data.zsc_deco,
                            &lightmap_path,
                            lit_object,
                            object_instance,
                            ifo_object_id,
                            object_instance.object_id as usize,
                            ZoneObject::DecoObject,
                            ZoneObject::DecoObjectPart,
                            COLLISION_GROUP_ZONE_OBJECT,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        deco_object_count += 1;
                    }

                    // Animated objects and effect objects
                    for object_instance in ifo.animated_objects.iter() {
                        let object_entity = spawn_animated_object(
                            commands,
                            asset_server,
                            effect_mesh_materials.as_mut(),
                            &game_data.stb_morph_object,
                            object_instance,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        animated_object_count += 1;
                    }

                    for (ifo_object_id, effect_object) in ifo.effect_objects.iter().enumerate() {
                        let object_entity = spawn_effect_object(
                            commands,
                            asset_server,
                            vfs_resource,
                            effect_mesh_materials.as_mut(),
                            particle_materials.as_mut(),
                            meshes,
                            storage_buffers.as_mut(),
                            effect_object,
                            ifo_object_id,
                            effect_cache,
                        );
                        commands.entity(zone_entity).add_child(object_entity);
                        effect_object_count += 1;
                    }

                    for (ifo_object_id, sound_object) in ifo.sound_objects.iter().enumerate() {
                        let object_entity =
                            spawn_sound_object(commands, asset_server, sound_object, ifo_object_id);
                        commands.entity(zone_entity).add_child(object_entity);
                        sound_object_count += 1;
                    }
                }
            }
        }
    }

    log::info!("[SPAWN ZONE] ===========================================");
    log::info!("[SPAWN ZONE] Zone spawning complete");
    log::info!("[SPAWN ZONE] Terrain entities: {}", terrain_count);
    log::info!("[SPAWN ZONE] Water entities: {}", water_count);
    log::info!("[SPAWN ZONE] Event objects: {}", event_object_count);
    log::info!("[SPAWN ZONE] Warp objects: {}", warp_object_count);
    log::info!("[SPAWN ZONE] Cnst objects: {}", cnst_object_count);
    log::info!("[SPAWN ZONE] Deco objects: {}", deco_object_count);
    log::info!("[SPAWN ZONE] Animated objects: {}", animated_object_count);
    log::info!("[SPAWN ZONE] Effect objects: {}", effect_object_count);
    log::info!("[SPAWN ZONE] Sound objects: {}", sound_object_count);
    let total_entities = terrain_count
        + water_count
        + event_object_count
        + warp_object_count
        + cnst_object_count
        + deco_object_count
        + animated_object_count
        + effect_object_count
        + sound_object_count;
    log::info!("[SPAWN ZONE] Total entities spawned: {}", total_entities);

    // Enhanced zone entity count logging
    let object_count = event_object_count
        + warp_object_count
        + cnst_object_count
        + deco_object_count
        + animated_object_count
        + effect_object_count
        + sound_object_count;
    info!(
        "[ZONE] Zone '{}' spawned with {} entities",
        zone_data.zone_id.get(),
        total_entities
    );
    info!("[ZONE]   Terrain entities: {}", terrain_count);
    info!("[ZONE]   Water entities: {}", water_count);
    info!("[ZONE]   Object entities: {}", object_count);

    log::info!(
        "[MEMORY] Zone loading assets: {}",
        zone_loading_assets.len()
    );
    log::info!("[MEMORY TRACKING] Zone spawn complete - logging memory summary");
    memory_tracking.log_summary();
    log::info!("[SPAWN ZONE] ===========================================");

    // DIAGNOSTIC: About to return from spawn_zone
    log::info!(
        "[SPAWN ZONE DIAGNOSTIC] ✓ spawn_zone returning SUCCESS: entity={:?}, assets_count={}",
        zone_entity,
        zone_loading_assets.len()
    );

    Ok((zone_entity, zone_loading_assets))
}

// REMOVED: CartoonSky - using Bevy 0.16 Atmosphere instead
// const SKY_DOME_RADIUS: f32 = 500.0;

// /// Spawns a cartoon procedural sky entity and returns the entity along with asset handles.
// /// Uses CartoonSkyMaterial for procedural sky rendering with day/night cycle support.
// fn spawn_cartoon_sky(
//     commands: &mut Commands,
//     meshes: &mut Assets<Mesh>,
//     cartoon_sky_materials: &mut Assets<CartoonSkyMaterial>,
// ) -> (Entity, Vec<UntypedHandle>) {
//     log::info!("[SPAWN CARTOON SKY] Creating procedural cartoon sky dome");
//
//     // Create a UV sphere mesh for the sky dome
//     // The sphere is inverted (rendered from inside) for sky rendering
//     let sky_dome_mesh = Mesh::from(bevy::math::primitives::Sphere {
//         radius: SKY_DOME_RADIUS,
//     });
//
//     let mesh_handle = meshes.add(sky_dome_mesh);
//     log::info!("[SPAWN CARTOON SKY] Created sky dome mesh with radius {}", SKY_DOME_RADIUS);
//
//     // Create the cartoon sky material with default settings
//     // The material will be updated by cartoon_sky_material_system based on ZoneTime
//     let material = cartoon_sky_materials.add(CartoonSkyMaterial::default());
//
//     log::info!("[SPAWN CARTOON SKY] Created cartoon sky material");
//
//     // Spawn the sky dome entity
//     // Note: No asset loading needed - everything is procedural
//     // CRITICAL: Include NotShadowCaster - sky should never cast shadows
//     let entity = commands
//         .spawn((
//             Mesh3d(mesh_handle),
//             MeshMaterial3d(material),
//             Transform::from_xyz(0.0, 0.0, 0.0),
//             GlobalTransform::default(),
//             ViewVisibility::default(),
//             Visibility::Visible,
//             InheritedVisibility::default(),
//             NoFrustumCulling,
//             Aabb::from_min_max(Vec3::splat(-SKY_DOME_RADIUS * 1.1), Vec3::splat(SKY_DOME_RADIUS * 1.1)),
//             RenderLayers::layer(0),
//             NotShadowCaster,  // Sky should never cast shadows
//         ))
//         .id();
//
//     log::info!("[SPAWN CARTOON SKY] Cartoon sky entity spawned: {:?}", entity);
//
//     // No external assets to load - everything is procedural
//     (entity, Vec::new())
// }
