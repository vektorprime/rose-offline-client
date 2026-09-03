use bevy::{
    math::{Quat, Vec3},
    prelude::{
        Assets, Commands, Entity, GlobalTransform, MessageWriter, Query, Res, State, Time,
        Transform, With, Without,
    },
};
use bevy_rapier3d::geometry::ShapeCastOptions;
use bevy_rapier3d::plugin::context::systemparams::{RapierContext, ReadRapierContext};
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, QueryFilter};
use bevy_rapier3d::rapier::prelude::Shape;

use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{
        BoatState, ColliderParent, CollisionHeightOnly, CollisionPlayer, Command, EventObject,
        FlightState, NextCommand, Position, WarpObject, COLLISION_FILTER_COLLIDABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_CHARACTER,
        COLLISION_GROUP_ITEM_DROP, COLLISION_GROUP_NPC, COLLISION_GROUP_PHYSICS_TOY,
        COLLISION_GROUP_PLAYER, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_WARP_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
        COLLISION_GROUP_ZONE_WATER,
    },
    events::QuestTriggerEvent,
    resources::{AppState, CurrentZone, GameConnection},
    zone_content::boats::NpcBoat,
    zone_content::docks::Dock,
    zone_loader::ZoneLoaderAsset,
};

/// Sailing collision tuning.
///
/// Dock deck dimensions mirror the private consts in src/zone_content/docks.rs:
/// DECK_LENGTH_M = PLANKS_PER_ROW(8) * PLANK_LENGTH_M(6.0) = 48 m and
/// DECK_WIDTH_M = PLANK_ROWS(8) * PLANK_WIDTH_M(1.1) = 8.8 m. Docks are purely
/// visual (no rapier collider), so the footprint is checked by hand here.
const DOCK_LENGTH_M: f32 = 48.0;
const DOCK_WIDTH_M: f32 = 8.8;
/// Extra radius around the deck that blocks the boat hull (hull is ~6.7 m long).
const DOCK_MARGIN_M: f32 = 3.0;
/// Terrain must rise this far above the water surface (m) to block the boat.
const TERRAIN_BLOCK_MARGIN_M: f32 = 0.25;
/// Block radius around NPC boat centers in cm (hull is ~6.7 m long x ~3.5 m wide).
const NPC_BOAT_BLOCK_RADIUS_CM: f32 = 400.0;

/// Finds the top surface (in meters) of a zone object that an entity is currently
/// inside of or touching at `feet_position` (e.g. an NPC spawned underneath castle
/// steps). Mirrors the original game's `UpdateFootHeight_Other`: objects intersecting
/// the entity's foot sphere get raycast upward to find their top, so entities stuck
/// inside objects are placed on top of them instead of remaining underneath.
///
/// Returns `None` when the entity is not intersecting any zone object, or when no
/// surface can be found above it.
fn find_object_top_height(rapier_context: &RapierContext, feet_position: Vec3) -> Option<f32> {
    // Objects the entity's feet are inside of / touching. Terrain and water are
    // excluded so entities standing on flat ground are not affected. Zone objects
    // are matched via MOVEABLE or INSPECTABLE so that NOT_MOVEABLE objects (castle
    // steps, buildings) are included - they also need to be stood on.
    //
    // Entity-class colliders (player, NPCs, characters, item drops) are excluded
    // from the memberships: otherwise the entity's OWN collider matches the query
    // (its filter includes INSPECTABLE) and the upward ray climbs to the top of
    // its own collider, launching the entity upward every frame.
    let object_groups = CollisionGroups::new(
        COLLISION_FILTER_MOVEABLE | COLLISION_FILTER_INSPECTABLE,
        !COLLISION_GROUP_PHYSICS_TOY
            & !COLLISION_GROUP_ZONE_TERRAIN
            & !COLLISION_GROUP_ZONE_WATER
            & !COLLISION_GROUP_PLAYER
            & !COLLISION_GROUP_NPC
            & !COLLISION_GROUP_CHARACTER
            & !COLLISION_GROUP_ITEM_DROP,
    );

    let gate_ball = Collider::ball(0.35);
    let mut intersecting_objects = Vec::new();
    rapier_context.intersect_shape(
        feet_position,
        Quat::default(),
        <&dyn Shape>::from(&gate_ball),
        QueryFilter::new().groups(object_groups),
        |hit_entity| {
            intersecting_objects.push(hit_entity);
            true
        },
    );

    if intersecting_objects.is_empty() {
        return None;
    }

    // Cast a ray upward from just above the feet, restricted to the intersecting
    // objects, and keep the highest hit: that is the top of the object the entity
    // is stuck inside. Each iteration excludes the previous hit collider so the
    // ray keeps climbing through the object's faces.
    let up_origin = Vec3::new(feet_position.x, feet_position.y + 0.1, feet_position.z);
    let mut top_height = None;
    let mut excluded_collider = None;

    for _ in 0..64 {
        let predicate = |entity| intersecting_objects.contains(&entity);
        let mut filter = QueryFilter::new()
            .groups(object_groups)
            .predicate(&predicate);
        if let Some(excluded_collider) = excluded_collider {
            filter = filter.exclude_collider(excluded_collider);
        }

        if let Some((hit_entity, distance)) =
            rapier_context.cast_ray(up_origin, Vec3::Y, 100.0, false, filter)
        {
            let hit_height = up_origin.y + distance;
            top_height = Some(top_height.map_or(hit_height, |height: f32| height.max(hit_height)));
            excluded_collider = Some(hit_entity);
        } else {
            break;
        }
    }

    top_height
}

#[allow(clippy::too_many_arguments)]
pub fn collision_height_only_system(
    mut query_collision_entity: Query<
        (Entity, &mut Position, &mut Transform),
        With<CollisionHeightOnly>,
    >,
    rapier_context: ReadRapierContext,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    time: Res<Time>,
) {
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };
    let current_zone = if let Some(current_zone) = current_zone {
        current_zone
    } else {
        log::warn!("[NPC_TERRAIN_DIAG] collision_height_only_system: No CurrentZone resource!");
        return;
    };

    let current_zone_data = if let Some(current_zone_data) =
        zone_loader_assets.get(&current_zone.handle)
    {
        current_zone_data
    } else {
        log::warn!("[NPC_TERRAIN_DIAG] collision_height_only_system: Zone data not loaded yet!");
        return;
    };

    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        // Get terrain height from heightmap
        let terrain_height: f32 =
            current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        // Cast ray downward to detect collision objects (bridges, platforms,
        // castle steps, etc.). Zone objects are matched via MOVEABLE or
        // INSPECTABLE so NOT_MOVEABLE objects (steps, buildings) are included.
        // Entity-class colliders are excluded so an entity never stands on its
        // own (or another entity's) collider.
        let ray_origin = Vec3::new(
            position.x / 100.0,
            transform.translation.y + 1.0,
            -position.y / 100.0,
        );
        let ray_direction = Vec3::new(0.0, -1.0, 0.0);
        let max_fall_distance = 100.0; // Reduced from 10000.0 since entities now spawn at terrain height

        let collision_height: Option<f32> = if let Some((_hit_entity, distance)) = rapier_context
            .cast_ray(
                ray_origin,
                ray_direction,
                max_fall_distance,
                false,
                QueryFilter::new().groups(CollisionGroups::new(
                    COLLISION_FILTER_MOVEABLE | COLLISION_FILTER_INSPECTABLE,
                    !COLLISION_GROUP_PHYSICS_TOY
                        & !COLLISION_GROUP_ZONE_WATER
                        & !COLLISION_GROUP_PLAYER
                        & !COLLISION_GROUP_NPC
                        & !COLLISION_GROUP_CHARACTER
                        & !COLLISION_GROUP_ITEM_DROP,
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

        // If the entity is inside a zone object (e.g. spawned underneath castle
        // steps), place it on top of that object instead of leaving it stuck below.
        let feet_position = Vec3::new(
            position.x / 100.0,
            transform.translation.y + 0.1,
            -position.y / 100.0,
        );
        let target_y = find_object_top_height(&rapier_context, feet_position)
            .map_or(target_y, |object_top| target_y.max(object_top));

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
    mut query_collision_entity: Query<(&mut Position, &mut Transform), With<CollisionPlayer>>,
    _rapier_context: ReadRapierContext,
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

/// Returns true when the entity is mid-attack or chasing an attack target.
/// Wall-collision must not overwrite that intent with Stop: the attack chase
/// recomputes its destination every frame in command_system, while a forced
/// Stop would decay the chase and leave the player standing next to the
/// monster without ever attacking.
fn has_attack_intent(
    query_attack_intent: &Query<(&Command, &NextCommand)>,
    entity: Entity,
) -> bool {
    query_attack_intent.get(entity).map_or(false, |(command, next)| {
        next.is_attack() || matches!(command, Command::Attack(_))
    })
}

/// Server-authoritative player collision system.
///
/// This system handles client-side collision detection for smooth local gameplay,
/// but does NOT mutate the Position component. Position is server-authoritative.
///
/// Key principles:
/// - Position component is READ-ONLY (server authoritative)
/// - Transform is updated from Position for rendering
/// - Collision detection runs locally for responsive feedback
/// - MoveCollision messages are sent to server when collision occurs
/// - Server validates and sends AdjustPosition if correction needed
#[allow(clippy::too_many_arguments)]
pub fn collision_player_system(
    mut commands: Commands,
    mut query_collision_entity: Query<
        (
            Entity,
            &mut Position,
            &mut Transform,
            Option<&FlightState>,
            Option<&mut BoatState>,
        ),
        With<CollisionPlayer>,
    >,
    mut query_event_object: Query<&mut EventObject>,
    mut quest_trigger_events: MessageWriter<QuestTriggerEvent>,
    mut query_warp_object: Query<&mut WarpObject>,
    query_collider_parent: Query<&ColliderParent>,
    query_docks: Query<&GlobalTransform, With<Dock>>,
    query_npc_boats: Query<&Position, (With<NpcBoat>, Without<CollisionPlayer>)>,
    query_attack_intent: Query<(&Command, &NextCommand)>,
    current_zone: Option<Res<CurrentZone>>,
    game_connection: Option<Res<GameConnection>>,
    rapier_context: ReadRapierContext,
    time: Res<Time>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
) {
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };
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

    let mut entity_count = 0;
    for (entity, mut position, mut transform, flight_state, mut boat_state) in
        query_collision_entity.iter_mut()
    {
        entity_count += 1;
        // Check if player is flying - if so, skip ground collision and use position directly
        let is_flying = flight_state.map_or(false, |fs| fs.is_flying);
        let is_sailing = boat_state.as_ref().map_or(false, |bs| bs.active);

        // Position is server-authoritative - we read from it but don't write
        // Position is in centimeters: x=right, y=forward, z=up
        // Transform is in meters: x=right, y=up, z=back

        if is_flying {
            // When flying, sync transform directly from position (including Y/height)
            transform.translation.x = position.x / 100.0;
            transform.translation.y = position.z / 100.0; // Use position.z for height
            transform.translation.z = -position.y / 100.0;
            continue; // Skip ground collision when flying
        }

        if is_sailing {
            // Sailing still needs wall/object collision, but should remain on water surface
            // (no gravity/terrain-following Y adjustment).
            let new_translation =
                Vec3::new(position.x / 100.0, position.z / 100.0, -position.y / 100.0);
            let collider_radius = 0.4;
            let translation_delta = new_translation - transform.translation;

            // Last accepted position in cm, derived from the transform synced at the
            // end of the previous frame (read BEFORE the sync block below overwrites
            // the transform). Blocked sailing steps revert to this point.
            // NOTE: collision_player_system (lib.rs:1394) is registered before
            // sailing_movement_system (lib.rs:1445); both mutate Position/BoatState
            // so they run serially in insertion order. `position` therefore already
            // holds the step sailing took last frame, and reverting to this point
            // is where the boat visibly stops.
            let prev_position = Vec3::new(
                transform.translation.x * 100.0,
                -(transform.translation.z * 100.0),
                position.z,
            );
            let water_height = position.z / 100.0;

            if translation_delta.length() > 0.00001 {
                let cast_origin = transform.translation + Vec3::new(0.0, 1.2, 0.0);
                let cast_direction = translation_delta.normalize();
                let ball_collider = Collider::ball(collider_radius);

                if let Some((_, distance)) = rapier_context.cast_shape(
                    cast_origin + cast_direction * collider_radius,
                    Quat::default(),
                    cast_direction,
                    <&dyn Shape>::from(&ball_collider),
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
                    // Calculate collision position but don't mutate Position
                    // Instead, send MoveCollision to server with the adjusted position
                    let collision_translation =
                        cast_origin + translation_delta * (distance.time_of_impact - 0.1).max(0.0);
                    let collision_position = Vec3::new(
                        collision_translation.x * 100.0,
                        -(collision_translation.z * 100.0),
                        position.z, // Preserve Z for sailing
                    );

                    // Stop movement intent, unless chasing/attacking: clobbering
                    // NextCommand::Attack here strands the player next to the
                    // monster with no attack. Keep the attack intent and let the
                    // chase recompute around the obstacle next frame.
                    if !has_attack_intent(&query_attack_intent, entity) {
                        commands.entity(entity).insert(NextCommand::with_stop());
                    }

                    // Send collision position to server for validation
                    if let Some(game_connection) = game_connection.as_ref() {
                        game_connection
                            .client_message_tx
                            .send(ClientMessage::MoveCollision {
                                position: collision_position,
                            })
                            .ok();
                    }
                }
            }

            // Blocked sailing step (terrain/dock/boat): revert the position to the
            // last accepted point, stop the boat, and inform the server so it does
            // not resend the blocked position. Wall-cast hits above only send
            // messages; these cases also revert so the boat actually stops.
            let mut blocked_position: Option<Vec3> = None;

            // (a) Islands/terrain: block if the terrain at the new position rises
            // above the water surface by more than a small margin (keeps the hull
            // off the shore; terrain height is in cm from the heightmap).
            let terrain_height =
                current_zone_data.get_terrain_height(position.x, position.y) / 100.0;
            if terrain_height > water_height + TERRAIN_BLOCK_MARGIN_M {
                blocked_position = Some(prev_position);
            }

            // Existing shallow-water backstop: also block when the terrain is
            // already at the water level (previously sent messages only; now the
            // position is reverted as well).
            if blocked_position.is_none() && terrain_height > water_height - 0.05 {
                blocked_position = Some(prev_position);
            }

            // (b) Docks: block when the boat's new position is inside the deck
            // footprint. The deck runs outward from the shore along the dock's
            // local +X (yaw-rotated; see zone_content/docks.rs), 48 x 8.8 m, and
            // the dock's GlobalTransform is in the same world space as the boat
            // transform (zone entity sits at world (5200, 0, -5200)).
            if blocked_position.is_none() {
                for dock_transform in query_docks.iter() {
                    let local = dock_transform.rotation().inverse()
                        * (new_translation - dock_transform.translation());
                    if local.x.abs() <= DOCK_LENGTH_M / 2.0 + DOCK_MARGIN_M
                        && local.z.abs() <= DOCK_WIDTH_M / 2.0 + DOCK_MARGIN_M
                    {
                        blocked_position = Some(prev_position);
                        break;
                    }
                }
            }

            // (c) NPC boats: block when within a fixed radius of another boat's
            // center (hull is ~6.7 m long, so a ~4 m radius keeps the hulls apart).
            if blocked_position.is_none() {
                for npc_position in query_npc_boats.iter() {
                    let dx = npc_position.position.x - position.x;
                    let dy = npc_position.position.y - position.y;
                    if dx * dx + dy * dy < NPC_BOAT_BLOCK_RADIUS_CM * NPC_BOAT_BLOCK_RADIUS_CM {
                        blocked_position = Some(prev_position);
                        break;
                    }
                }
            }

            if let Some(blocked_position) = blocked_position {
                // Revert to the blocked point so the boat stops; zero the speed so
                // sailing_movement_system only creeps forward while the player
                // holds throttle (and gets reverted again next frame).
                position.position.x = blocked_position.x;
                position.position.y = blocked_position.y;
                if let Some(boat) = boat_state.as_deref_mut() {
                    boat.speed = 0.0;
                }

                if !has_attack_intent(&query_attack_intent, entity) {
                    commands.entity(entity).insert(NextCommand::with_stop());
                }

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::MoveCollision {
                            position: blocked_position,
                        })
                        .ok();
                }
            }

            // Sync transform from server-authoritative position
            transform.translation.x = position.x / 100.0;
            transform.translation.y = position.z / 100.0;
            transform.translation.z = -position.y / 100.0;
            continue;
        }

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
            let ball_collider = Collider::ball(collider_radius);

            if let Some((_, distance)) = rapier_context.cast_shape(
                cast_origin + cast_direction * collider_radius,
                Quat::default(),
                cast_direction,
                <&dyn Shape>::from(&ball_collider),
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
                // Calculate collision position but don't mutate Position
                // Send MoveCollision to server with the adjusted position
                let collision_translation =
                    cast_origin + translation_delta * (distance.time_of_impact - 0.1).max(0.0);
                let collision_position = Vec3::new(
                    collision_translation.x * 100.0,
                    -(collision_translation.z * 100.0),
                    collision_translation.y * 100.0,
                );

                // Stop movement intent, unless chasing/attacking (see above).
                if !has_attack_intent(&query_attack_intent, entity) {
                    commands.entity(entity).insert(NextCommand::with_stop());
                }

                // Send collision position to server for validation
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::MoveCollision {
                            position: collision_position,
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

        let collision_height: Option<f32> = if let Some((_hit_entity, distance)) = rapier_context
            .cast_ray(
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

        // If the player is inside a zone object (e.g. teleported underneath castle
        // steps), place them on top of that object instead of leaving them stuck below.
        let feet_position = Vec3::new(
            position.x / 100.0,
            transform.translation.y + 0.1,
            -position.y / 100.0,
        );
        let target_y = find_object_top_height(&rapier_context, feet_position)
            .map_or(target_y, |object_top| target_y.max(object_top));

        // Update entity translation based on server-authoritative position
        // Z (height) is updated locally for smooth visual feedback
        let old_y = transform.translation.y;
        transform.translation.x = position.x / 100.0;
        transform.translation.z = -position.y / 100.0;

        if old_y - target_y > fall_distance {
            let new_y = old_y - fall_distance;
            transform.translation.y = new_y;
        } else {
            transform.translation.y = target_y;
        }

        // Note: We do NOT update position.z here - Position is server-authoritative
        // The transform Y is updated for visual smoothness, but Position remains unchanged

        // Check if we are now colliding with any warp / event object
        let ball_collider = Collider::ball(1.0);
        rapier_context.intersect_shape(
            Vec3::new(
                position.x / 100.0,
                position.z / 100.0 + 1.0,
                -position.y / 100.0,
            ),
            Quat::default(),
            <&dyn Shape>::from(&ball_collider),
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
                            quest_trigger_events.write(QuestTriggerEvent::DoTrigger(
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
