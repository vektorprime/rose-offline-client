use bevy::{
    math::{Quat, Vec3},
    prelude::{
        Assets, Commands, Entity, EventWriter, Query, Res, State, Time, Transform, With,
    },
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, QueryFilter, ReadDefaultRapierContext};
use bevy_rapier3d::geometry::ShapeCastOptions;

use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{
        ColliderParent, CollisionHeightOnly, CollisionPlayer, EventObject, NextCommand, Position,
        WarpObject, COLLISION_FILTER_COLLIDABLE, COLLISION_FILTER_MOVEABLE,
        COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_TERRAIN, COLLISION_GROUP_ZONE_WARP_OBJECT,
    },
    events::QuestTriggerEvent,
    resources::{AppState, CurrentZone, GameConnection},
    zone_loader::ZoneLoaderAsset,
};

#[allow(clippy::too_many_arguments)]
pub fn collision_height_only_system(
    mut query_collision_entity: Query<
        (Entity, &mut Position, &mut Transform),
        With<CollisionHeightOnly>,
    >,
    rapier_context: ReadDefaultRapierContext,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    // DIAGNOSTIC: Log when collision_height_only_system runs
    //log::info!("[DIAG_COLLISION_HEIGHT_ONLY] collision_height_only_system started");

    let current_zone = if let Some(current_zone) = current_zone {
        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY] CurrentZone resource is available");
        current_zone
    } else {
        log::warn!("[DIAG_COLLISION_HEIGHT_ONLY] CurrentZone resource is NOT available - early return");
        return;
    };
    let current_zone_data = zone_loader_assets.get(&current_zone.handle);

    // DIAGNOSTIC: Check if zone data is loaded
    if current_zone_data.is_none() {
        log::warn!("[DIAG_COLLISION_HEIGHT_ONLY] CurrentZone data is NOT available - using fallback terrain height of 0.0");
    }

    // DIAGNOSTIC: Log when collision_height_only_system runs
    let mut iteration_count = 0;
    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        iteration_count += 1;
        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY] Processing entity #{}: {:?}", iteration_count, entity);
        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Position before: x={:.2}, y={:.2}, z={:.2}", position.x, position.y, position.z);
        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Transform before: x={:.2}, y={:.2}, z={:.2}", transform.translation.x, transform.translation.y, transform.translation.z);
        
        let ray_origin = Vec3::new(position.x / 100.0, 100000.0, -position.y / 100.0);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);

        // Cast ray down to see if we are standing on any objects
        let collision_height = if let Some((_, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            100000000.0,
            false,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            Some((ray_origin + ray_direction * distance).y)
        } else {
            None
        };

        // We can never be below the heightmap, but use default height if zone data is not available
        let terrain_height = if let Some(zone_data) = current_zone_data {
            zone_data.get_terrain_height(position.x, position.y) / 100.0
        } else {
            // Use default height of 0.0 when zone data is not yet loaded
            0.0
        };

        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Raycast from Y=100000.0, collision_height={:?}, terrain_height={:.2}", collision_height, terrain_height);

        // Update entity translation and position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        let new_y = if let Some(collision_height) = collision_height {
            //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Using collision_height.max(terrain_height)");
            collision_height.max(terrain_height)
        } else {
            //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Using terrain_height only (no collision detected)");
            terrain_height
        };
        transform.translation.y = new_y;
        position.z = transform.translation.y * 100.0;

        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Transform after: x={:.2}, y={:.2}, z={:.2}", transform.translation.x, transform.translation.y, transform.translation.z);
        //log::info!("[DIAG_COLLISION_HEIGHT_ONLY]   Position after: x={:.2}, y={:.2}, z={:.2}", position.x, position.y, position.z);
    }

    //log::info!("[DIAG_COLLISION_HEIGHT_ONLY] Processed {} entities with CollisionHeightOnly", iteration_count);
}

#[allow(clippy::too_many_arguments)]
pub fn collision_player_system_join_zone(
    mut query_collision_entity: Query<
        (&mut Position, &mut Transform),
        With<CollisionPlayer>,
    >,
    rapier_context: ReadDefaultRapierContext,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    app_state_current: Res<State<AppState>>,
) {
    // DIAGNOSTIC: Log at the very start of the function (before any queries)
   //  log::info!("[DIAG_COLLISION_PLAYER_JOIN_ZONE] collision_player_system_join_zoin called");

    // DIAGNOSTIC: Log the current AppState
   //  log::info!("[DIAG_COLLISION_PLAYER_JOIN_ZONE] Current AppState: {:?}", app_state_current.get());

    let current_zone: Option<&CurrentZone> = current_zone.as_deref();
    
    // DIAGNOSTIC: Log if current_zone is None (was line 118 early return)
    if current_zone.is_none() {
        log::warn!("[DIAG_COLLISION_PLAYER_JOIN_ZONE] Early return at line 118: current_zone is None - zone not yet loaded");
    }
    
    let current_zone_data = if let Some(current_zone) = current_zone {
        zone_loader_assets.get(&current_zone.handle)
    } else {
        None
    };
    
    // DIAGNOSTIC: Log if current_zone_data is None (was line 124 early return)
    if current_zone_data.is_none() {
        log::warn!("[DIAG_COLLISION_PLAYER_JOIN_ZONE] Early return at line 124: current_zone_data is None - zone data not yet loaded");
    }

    // DIAGNOSTIC: Log whether the query for CollisionPlayer is empty
    let is_empty = query_collision_entity.is_empty();
   //  log::info!("[DIAG_COLLISION_PLAYER_JOIN_ZONE] CollisionPlayer query is_empty: {}", is_empty);

    // DIAGNOSTIC: Log when collision_player_system_join_zoin runs
    // log::info!("[COLLISION_PLAYER_JOIN_ZONE] Running collision_player_system_join_zoin");
    let mut iteration_count = 0;
    for (mut position, mut transform) in query_collision_entity.iter_mut() {
        iteration_count += 1;
        // DIAGNOSTIC: Log each entity found in the query
       //  log::info!("[DIAG_COLLISION_PLAYER_JOIN_ZONE] Processing entity #{}", iteration_count);
        // log::info!("[COLLISION_PLAYER_JOIN_ZONE] Processing entity iteration #{}", iteration_count);
        // log::info!("[COLLISION_PLAYER_JOIN_ZONE]   Position before: x={:.2}, y={:.2}, z={:.2}", position.x, position.y, position.z);
        // log::info!("[COLLISION_PLAYER_JOIN_ZONE]   Transform before: x={:.2}, y={:.2}, z={:.2}", transform.translation.x, transform.translation.y, transform.translation.z);
        
        let ray_origin = Vec3::new(position.x / 100.0, 100000.0, -position.y / 100.0);
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);

        // Cast ray down to see if we are standing on any objects
        let collision_height = if let Some((_, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            100000000.0,
            false,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            Some((ray_origin + ray_direction * distance).y)
        } else {
            None
        };

        // We can never be below the heightmap, but use default height if zone data is not available
        let terrain_height = if let Some(zone_data) = current_zone_data {
            zone_data.get_terrain_height(position.x, position.y) / 100.0
        } else {
            // Use default height of 0.0 when zone data is not yet loaded
            0.0
        };

        // log::info!("[COLLISION_PLAYER_JOIN_ZONE]   Raycast from Y=100000.0, collision_height={:?}, terrain_height={:.2}", collision_height, terrain_height);

        // Update entity translation and position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        transform.translation.y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };
        position.z = transform.translation.y * 100.0;

        // log::info!("[COLLISION_PLAYER_JOIN_ZONE]   Transform after: x={:.2}, y={:.2}, z={:.2}", transform.translation.x, transform.translation.y, transform.translation.z);
        // log::info!("[COLLISION_PLAYER_JOIN_ZONE]   Position after: x={:.2}, y={:.2}, z={:.2}", position.x, position.y, position.z);
    }
    
    if iteration_count == 0 {
        log::warn!("[COLLISION_PLAYER_JOIN_ZONE] No entities processed! Query filter might not be matching.");
    } else {
        // log::info!("[COLLISION_PLAYER_JOIN_ZONE] Processed {} entities with CollisionPlayer", iteration_count);
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collision_player_system(
    mut commands: Commands,
    mut query_collision_entity: Query<
        (Entity, &mut Position, &mut Transform),
        With<CollisionPlayer>,
    >,
    mut query_event_object: Query<&mut EventObject>,
    mut quest_trigger_events: EventWriter<QuestTriggerEvent>,
    mut query_warp_object: Query<&mut WarpObject>,
    query_collider_parent: Query<&ColliderParent>,
    current_zone: Option<Res<CurrentZone>>,
    game_connection: Option<Res<GameConnection>>,
    rapier_context: ReadDefaultRapierContext,
    time: Res<Time>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    let current_zone = if let Some(current_zone) = current_zone {
        current_zone
    } else {
        return;
    };
    let current_zone_data =
        if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
            current_zone_data
        } else {
            return;
        };

    // DIAGNOSTIC: Log when collision_player_system runs
    // log::info!("[COLLISION_PLAYER] Running collision_player_system");
    
    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        // log::info!("[COLLISION_PLAYER] Processing entity: {:?}", entity);
        // log::info!("[COLLISION_PLAYER]   Position before: x={:.2}, y={:.2}, z={:.2}", position.x, position.y, position.z);
        // log::info!("[COLLISION_PLAYER]   Transform before: x={:.2}, y={:.2}, z={:.2}", transform.translation.x, transform.translation.y, transform.translation.z);
        
        // Cast ray forward to collide with walls
        let new_translation = Vec3::new(
            position.x / 100.0,
            transform.translation.y,
            -position.y / 100.0,
        );
        let collider_radius = 0.4;
        let translation_delta = new_translation - transform.translation;
        if translation_delta.length() > 0.00001 {
            let cast_origin = transform.translation + Vec3::new(0.0, 1.2, 0.0);
            let cast_direction = translation_delta.normalize();

            if let Some((_, distance)) = rapier_context.cast_shape(
                cast_origin + cast_direction * collider_radius,
                Quat::default(),
                cast_direction,
                &Collider::ball(collider_radius),
                ShapeCastOptions {
                    max_time_of_impact: translation_delta.length(),
                    target_distance: 0.0,
                    compute_impact_geometry_on_penetration: false,
                    stop_at_penetration: false,
                },
                QueryFilter::new().groups(CollisionGroups::new(
                    COLLISION_FILTER_COLLIDABLE,
                    !COLLISION_GROUP_ZONE_TERRAIN & !COLLISION_GROUP_PHYSICS_TOY,
                )),
            ) {
                let collision_translation =
                    cast_origin + translation_delta * (distance.time_of_impact - 0.1).max(0.0);
                position.x = collision_translation.x * 100.0;
                position.y = -(collision_translation.z * 100.0);
                position.z = collision_translation.y * 100.0;

                commands.entity(entity).insert(NextCommand::with_stop());

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::MoveCollision {
                            position: position.position,
                        })
                        .ok();
                }
            }
        }

        // Cast ray down to see if we are standing on any objects
        let fall_distance = time.delta().as_secs_f32() * 9.81;
        let ray_origin = Vec3::new(
            position.x / 100.0,
            position.z / 100.0 + 1.35,
            -position.y / 100.0,
        );
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let collision_height = if let Some((_, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            1.35 + fall_distance,
            false,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            Some((ray_origin + ray_direction * distance).y)
        } else {
            None
        };

        // We can never be below the heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        let target_y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };

        // Update entity translation and position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;

        if transform.translation.y - target_y > fall_distance {
            transform.translation.y -= fall_distance;
        } else {
            transform.translation.y = target_y;
        }

        position.z = transform.translation.y * 100.0;

        // Check if we are now colliding with any warp / event object
        rapier_context.intersections_with_shape(
            Vec3::new(
                position.x / 100.0,
                position.z / 100.0 + 1.0,
                -position.y / 100.0,
            ),
            Quat::default(),
            &Collider::ball(1.0),
            QueryFilter::new().groups(CollisionGroups::new(
                Group::all(),
                COLLISION_GROUP_ZONE_EVENT_OBJECT | COLLISION_GROUP_ZONE_WARP_OBJECT,
            )),
            |hit_entity| {
                let hit_entity = query_collider_parent
                    .get(hit_entity)
                    .map_or(hit_entity, |collider_parent| collider_parent.entity);

                if let Ok(mut hit_event_object) = query_event_object.get_mut(hit_entity) {
                    if time.elapsed().as_secs_f64() - hit_event_object.last_collision > 5.0 {
                        if !hit_event_object.quest_trigger_name.is_empty() {
                            quest_trigger_events.send(QuestTriggerEvent::DoTrigger(
                                hit_event_object.quest_trigger_name.as_str().into(),
                            ));
                        }

                        hit_event_object.last_collision = time.elapsed().as_secs_f64();
                    }
                } else if let Ok(mut hit_warp_object) = query_warp_object.get_mut(hit_entity) {
                    if time.elapsed().as_secs_f64() - hit_warp_object.last_collision > 5.0 {
                        if let Some(game_connection) = game_connection.as_ref() {
                            game_connection
                                .client_message_tx
                                .send(ClientMessage::WarpGateRequest {
                                    warp_gate_id: hit_warp_object.warp_id,
                                })
                                .ok();
                        }

                        hit_warp_object.last_collision = time.elapsed().as_secs_f64();
                    }
                }
                true
            },
        );
    }
}
