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
    time: Res<Time>,
) {
    let current_zone = if let Some(current_zone) = current_zone {
        current_zone
    } else {
        log::warn!("[NPC_TERRAIN_DIAG] collision_height_only_system: No CurrentZone resource!");
        return;
    };
    
    let current_zone_data =
        if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
            current_zone_data
        } else {
            log::warn!("[NPC_TERRAIN_DIAG] collision_height_only_system: Zone data not loaded yet!");
            return;
        };

    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        // Get terrain height from heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;
        
        // Cast ray downward to detect collision objects (bridges, platforms, etc.)
        let ray_origin = Vec3::new(
            position.x / 100.0,
            transform.translation.y + 1.0,
            -position.y / 100.0,
        );
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let max_fall_distance = 100.0; // Reduced from 10000.0 since entities now spawn at terrain height
        
        let collision_height = if let Some((_hit_entity, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            max_fall_distance,
            false,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            let hit_y = (ray_origin + ray_direction * distance).y;
            Some(hit_y)
        } else {
            None
        };

        // Target height is the maximum of terrain height and collision height
        let target_y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };

        // Apply gravity-based falling
        let fall_distance = time.delta().as_secs_f32() * 9.81;
        let old_y = transform.translation.y;
        
        // Update X/Z from position
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        
        if old_y - target_y > fall_distance {
            // Falling
            transform.translation.y = old_y - fall_distance;
        } else {
            // On ground
            transform.translation.y = target_y;
        }
        
        // Update position height
        position.z = transform.translation.y * 100.0;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn collision_player_system_join_zone(
    mut query_collision_entity: Query<
        (&mut Position, &mut Transform),
        With<CollisionPlayer>,
    >,
    _rapier_context: ReadDefaultRapierContext,
    _current_zone: Option<Res<CurrentZone>>,
    _zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    _app_state_current: Res<State<AppState>>,
) {
    // This system only syncs X/Z translation from Position component.
    // All Y positioning (including terrain following and gravity) is handled by collision_player_system.
    // This separation ensures proper terrain adherence when moving both up AND down slopes.
    
    for (mut position, mut transform) in query_collision_entity.iter_mut() {
        // Only update X/Z translation - Y is handled by collision_player_system
        // This system just ensures the horizontal position is synced from Position component
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        // NOTE: Y is NOT set here - collision_player_system handles all Y positioning
        // This allows proper gravity-based falling when moving to lower terrain
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
        log::warn!("[TERRAIN_DIAG] collision_player_system: No CurrentZone resource!");
        return;
    };
    let current_zone_data =
        if let Some(current_zone_data) = zone_loader_assets.get(&current_zone.handle) {
            current_zone_data
        } else {
            log::warn!("[TERRAIN_DIAG] collision_player_system: Zone data not loaded yet!");
            return;
        };

    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
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

        // === GROUND DETECTION RAYCAST ===
        let fall_distance = time.delta().as_secs_f32() * 9.81;
        
        let ray_origin = Vec3::new(
            position.x / 100.0,
            transform.translation.y + 1.35,
            -position.y / 100.0,
        );
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let max_fall_distance = 10000.0;
        
        let collision_height = if let Some((_hit_entity, distance)) = rapier_context.cast_ray(
            ray_origin,
            ray_direction,
            max_fall_distance,
            false,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_MOVEABLE,
                !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            let hit_y = (ray_origin + ray_direction * distance).y;
            Some(hit_y)
        } else {
            None
        };

        // Get terrain height from heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        let target_y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };

        // Update entity translation and position
        let old_y = transform.translation.y;
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;
        
        if old_y - target_y > fall_distance {
            let new_y = old_y - fall_distance;
            transform.translation.y = new_y;
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
