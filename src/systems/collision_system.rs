use bevy::{
    math::{Quat, Vec3},
    prelude::{
        Assets, Camera3d, Commands, Entity, GlobalTransform, MessageWriter, Query, Res, State,
        Time, Transform, With, Without,
    },
};
use bevy_rapier3d::geometry::ShapeCastOptions;
use bevy_rapier3d::plugin::context::systemparams::{RapierContext, ReadRapierContext};
use bevy_rapier3d::prelude::{Collider, CollisionGroups, Group, QueryFilter};
use bevy_rapier3d::rapier::prelude::Shape;

use rose_game_common::messages::client::ClientMessage;

use crate::{
    components::{
        BoatState, ColliderParent, CollisionHeightOnly, CollisionPlayer, EventObject, FlightState,
        GroundHeightCache, NextCommand, Position, WarpObject, COLLISION_FILTER_COLLIDABLE,
        COLLISION_FILTER_INSPECTABLE, COLLISION_FILTER_MOVEABLE, COLLISION_GROUP_CHARACTER,
        COLLISION_GROUP_ITEM_DROP, COLLISION_GROUP_NPC, COLLISION_GROUP_PHYSICS_TOY,
        COLLISION_GROUP_PLAYER, COLLISION_GROUP_ZONE_EVENT_OBJECT,
        COLLISION_GROUP_ZONE_TERRAIN, COLLISION_GROUP_ZONE_WARP_OBJECT,
        COLLISION_GROUP_ZONE_WATER,
    },
    events::QuestTriggerEvent,
    render::{underwater_effect::UnderwaterVolumes, WaterReflectionCamera},
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

/// Distance from the camera (m) beyond which NPCs stop running the per-frame
/// Rapier ground queries and reuse their last resolved ground height.
const GROUND_QUERY_DISTANCE_M: f32 = 150.0;
/// Horizontal movement (cm) that invalidates a cached ground height.
const GROUND_CACHE_EPSILON_CM: f32 = 1.0;
/// Downward ground ray reach (m) for the height-only system: zone objects
/// never sit more than this far above the terrain, and the terrain itself is
/// covered by the heightmap.
const NPC_GROUND_RAY_DISTANCE_M: f32 = 20.0;
/// Downward ground ray reach (m) for the player (was 10000.0; the terrain hit
/// was redundant with the heightmap).
const PLAYER_GROUND_RAY_DISTANCE_M: f32 = 50.0;
/// Maximum number of stacked faces the ascending object-top scan climbs.
const OBJECT_TOP_SCAN_MAX_STEPS: usize = 10;
/// Step size (m) of the ascending object-top scan.
const OBJECT_TOP_SCAN_STEP_M: f32 = 0.1;
/// Radius (m) of the feet-sphere used to find zone objects under the entity.
const GATE_BALL_RADIUS_M: f32 = 0.35;

/// True when `world_position` (meters) is inside a water volume at or below
/// its surface. Under water the ground ray can only ever return the terrain
/// (already known from the heightmap), so both the ray and the feet-sphere
/// queries can be skipped.
fn point_is_submerged(world_position: Vec3, underwater_volumes: &UnderwaterVolumes) -> bool {
    for volume in underwater_volumes.volumes.iter() {
        let dx = (world_position.x - volume.center.x).abs();
        let dz = (world_position.z - volume.center.z).abs();
        if dx <= volume.half_extents.x
            && dz <= volume.half_extents.y
            && world_position.y <= volume.surface_y
        {
            return true;
        }
    }
    false
}

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

    // Collect the zone objects touching the feet sphere into a fixed-size
    // buffer (no per-call heap allocation; a small sphere can only touch a
    // handful of objects).
    let gate_ball = Collider::ball(GATE_BALL_RADIUS_M);
    let mut intersecting_objects = [Entity::PLACEHOLDER; 16];
    let mut intersecting_count = 0;
    rapier_context.intersect_shape(
        feet_position,
        Quat::default(),
        <&dyn Shape>::from(&gate_ball),
        QueryFilter::new().groups(object_groups),
        |hit_entity| {
            if intersecting_count < intersecting_objects.len() {
                intersecting_objects[intersecting_count] = hit_entity;
                intersecting_count += 1;
            }
            true
        },
    );

    if intersecting_count == 0 {
        return None;
    }

    // Ascending scan: probe upward in small windows and climb past each
    // surface found (excluding it so stacked faces are crossed one at a time).
    // Any surface the feet sphere can touch is within one sphere radius of the
    // feet, so the first window covers that reach and every later window is
    // one step tall; the scan terminates as soon as a window comes up empty,
    // which takes only a few iterations instead of 64 full-length raycasts.
    let first_reach = GATE_BALL_RADIUS_M - OBJECT_TOP_SCAN_STEP_M;
    let mut top_height = None;
    let mut probe_y = feet_position.y + OBJECT_TOP_SCAN_STEP_M;
    let mut reach = first_reach;
    let mut excluded_collider = None;

    for _ in 0..OBJECT_TOP_SCAN_MAX_STEPS {
        let predicate = |entity: Entity| intersecting_objects[..intersecting_count].contains(&entity);
        let mut filter = QueryFilter::new()
            .groups(object_groups)
            .predicate(&predicate);
        if let Some(excluded_collider) = excluded_collider {
            filter = filter.exclude_collider(excluded_collider);
        }

        if let Some((hit_entity, distance)) =
            rapier_context.cast_ray(
                Vec3::new(feet_position.x, probe_y, feet_position.z),
                Vec3::Y,
                reach,
                false,
                filter,
            )
        {
            let hit_height = probe_y + distance;
            top_height = Some(top_height.map_or(hit_height, |height: f32| height.max(hit_height)));
            probe_y = hit_height + OBJECT_TOP_SCAN_STEP_M;
            reach = OBJECT_TOP_SCAN_STEP_M;
            excluded_collider = Some(hit_entity);
        } else {
            break;
        }
    }

    top_height
}

#[allow(clippy::too_many_arguments)]
pub fn collision_height_only_system(
    mut commands: Commands,
    mut query_collision_entity: Query<
        (
            Entity,
            &mut Position,
            &mut Transform,
            Option<&mut GroundHeightCache>,
        ),
        With<CollisionHeightOnly>,
    >,
    rapier_context: ReadRapierContext,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    underwater_volumes: Option<Res<UnderwaterVolumes>>,
    time: Res<Time>,
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<WaterReflectionCamera>)>,
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

    let camera_pos = camera_query.iter().next().map(|gt| gt.translation());
    let ground_query_dist_sq = GROUND_QUERY_DISTANCE_M * GROUND_QUERY_DISTANCE_M;

    for (entity, mut position, mut transform, mut ground_cache) in
        query_collision_entity.iter_mut()
    {
        // Update X/Z from position (only when the value actually changes, so
        // idle entities are not marked dirty every frame)
        let new_x = position.x / 100.0;
        let new_z = -position.y / 100.0;
        if transform.translation.x != new_x {
            transform.translation.x = new_x;
        }
        if transform.translation.z != new_z {
            transform.translation.z = new_z;
        }

        // The two Rapier scene queries (downward ray + feet sphere) are gated
        // on horizontal movement and on camera distance: idle or far-away NPCs
        // reuse the last resolved ground height instead. Entities that are
        // currently falling (e.g. after walking off a cliff) keep re-resolving
        // so objects below them (bridges, platforms) are still caught mid-fall.
        let world_pos = Vec3::new(new_x, transform.translation.y, new_z);
        let in_range = camera_pos.map_or(true, |camera_pos| {
            world_pos.distance_squared(camera_pos) < ground_query_dist_sq
        });
        let cache_valid = ground_cache.as_ref().map_or(false, |cache| {
            (cache.last_x - position.x).abs() <= GROUND_CACHE_EPSILON_CM
                && (cache.last_y - position.y).abs() <= GROUND_CACHE_EPSILON_CM
        });
        let fall_distance = time.delta().as_secs_f32() * 9.81;
        let falling = ground_cache.as_ref().map_or(false, |cache| {
            transform.translation.y - cache.cached_ground_y > fall_distance
        });
        let use_cached = cache_valid || (!in_range && ground_cache.is_some());
        let resolve = !use_cached || (falling && in_range);

        let target_y = if resolve {
            // Get terrain height from heightmap
            let terrain_height: f32 =
                current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

            // Under water the ray can only ever return the terrain, so it is
            // skipped together with the feet-sphere.
            let target_y = if underwater_volumes
                .as_deref()
                .map_or(false, |volumes| point_is_submerged(world_pos, volumes))
            {
                terrain_height
            } else {
                // Cast ray downward to detect collision objects (bridges, platforms,
                // castle steps, etc.). Zone objects are matched via MOVEABLE or
                // INSPECTABLE so NOT_MOVEABLE objects (steps, buildings) are included.
                // Entity-class colliders are excluded so an entity never stands on its
                // own (or another entity's) collider; the terrain trimesh is also
                // excluded because the heightmap already covers the ground.
                let ray_origin = Vec3::new(
                    position.x / 100.0,
                    transform.translation.y + 1.0,
                    -position.y / 100.0,
                );
                let ray_direction = Vec3::new(0.0, -1.0, 0.0);
                let max_fall_distance = NPC_GROUND_RAY_DISTANCE_M;

                let collision_height: Option<f32> = if let Some((_hit_entity, distance)) =
                    rapier_context.cast_ray(
                        ray_origin,
                        ray_direction,
                        max_fall_distance,
                        false,
                        QueryFilter::new().groups(CollisionGroups::new(
                            COLLISION_FILTER_MOVEABLE | COLLISION_FILTER_INSPECTABLE,
                            !COLLISION_GROUP_PHYSICS_TOY
                                & !COLLISION_GROUP_ZONE_WATER
                                & !COLLISION_GROUP_ZONE_TERRAIN
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
                find_object_top_height(&rapier_context, feet_position)
                    .map_or(target_y, |object_top| target_y.max(object_top))
            };

            // Remember the resolved height so the queries can be skipped again
            if let Some(cache) = ground_cache.as_deref_mut() {
                cache.last_x = position.x;
                cache.last_y = position.y;
                cache.cached_ground_y = target_y;
            } else {
                commands.entity(entity).insert(GroundHeightCache {
                    last_x: position.x,
                    last_y: position.y,
                    cached_ground_y: target_y,
                });
            }
            target_y
        } else {
            ground_cache
                .as_ref()
                .map_or(0.0, |cache| cache.cached_ground_y)
        };

        // Apply gravity-based falling
        let old_y = transform.translation.y;

        let new_y = if old_y - target_y > fall_distance {
            // Falling
            old_y - fall_distance
        } else {
            // On ground
            target_y
        };
        if transform.translation.y != new_y {
            transform.translation.y = new_y;
        }

        // Update position height (only when the value actually changes)
        let new_position_z = transform.translation.y * 100.0;
        if position.z != new_position_z {
            position.z = new_position_z;
        }
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
        // (only when the value actually changes, so the player is not marked dirty)
        let new_x = position.x / 100.0;
        let new_z = -position.y / 100.0;
        if transform.translation.x != new_x {
            transform.translation.x = new_x;
        }
        if transform.translation.z != new_z {
            transform.translation.z = new_z;
        }
        // NOTE: Y is NOT set here - collision_player_system handles all Y positioning
        // This allows proper gravity-based falling when moving to lower terrain
    }
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
    current_zone: Option<Res<CurrentZone>>,
    game_connection: Option<Res<GameConnection>>,
    underwater_volumes: Option<Res<UnderwaterVolumes>>,
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
            let new_x = position.x / 100.0;
            let new_y = position.z / 100.0; // Use position.z for height
            let new_z = -position.y / 100.0;
            if transform.translation.x != new_x {
                transform.translation.x = new_x;
            }
            if transform.translation.y != new_y {
                transform.translation.y = new_y;
            }
            if transform.translation.z != new_z {
                transform.translation.z = new_z;
            }
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

                    // Stop movement intent
                    commands.entity(entity).insert(NextCommand::with_stop());

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

                commands.entity(entity).insert(NextCommand::with_stop());

                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::MoveCollision {
                            position: blocked_position,
                        })
                        .ok();
                }
            }

            // Sync transform from server-authoritative position (only when the
            // value actually changes)
            let new_x = position.x / 100.0;
            let new_y = position.z / 100.0;
            let new_z = -position.y / 100.0;
            if transform.translation.x != new_x {
                transform.translation.x = new_x;
            }
            if transform.translation.y != new_y {
                transform.translation.y = new_y;
            }
            if transform.translation.z != new_z {
                transform.translation.z = new_z;
            }
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

                // Stop movement intent
                commands.entity(entity).insert(NextCommand::with_stop());

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

        // Get terrain height from heightmap
        let terrain_height = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;

        // Under water the ray can only ever return the terrain, so it is
        // skipped together with the feet-sphere.
        let submerged = underwater_volumes
            .as_deref()
            .map_or(false, |volumes| point_is_submerged(transform.translation, volumes));

        let target_y = if submerged {
            terrain_height
        } else {
            // The terrain trimesh is excluded from this ray (the heightmap
            // already covers the ground); it only needs to reach zone objects
            // (bridges, platforms), which never sit more than a short distance
            // above the terrain.
            let ray_origin = Vec3::new(
                position.x / 100.0,
                transform.translation.y + 1.35,
                -position.y / 100.0,
            );
            let ray_direction = Vec3::new(0.0, -1.0, 0.0);
            let max_fall_distance = PLAYER_GROUND_RAY_DISTANCE_M;

            let collision_height: Option<f32> = if let Some((_hit_entity, distance)) =
                rapier_context.cast_ray(
                    ray_origin,
                    ray_direction,
                    max_fall_distance,
                    false,
                    QueryFilter::new().groups(CollisionGroups::new(
                        COLLISION_FILTER_MOVEABLE,
                        !COLLISION_GROUP_PHYSICS_TOY & !COLLISION_GROUP_ZONE_TERRAIN,
                    )),
                ) {
                let hit_y = (ray_origin + ray_direction * distance).y;
                Some(hit_y)
            } else {
                None
            };

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
            find_object_top_height(&rapier_context, feet_position)
                .map_or(target_y, |object_top| target_y.max(object_top))
        };

        // Update entity translation based on server-authoritative position
        // Z (height) is updated locally for smooth visual feedback
        let old_y = transform.translation.y;
        let new_x = position.x / 100.0;
        let new_z = -position.y / 100.0;
        if transform.translation.x != new_x {
            transform.translation.x = new_x;
        }
        if transform.translation.z != new_z {
            transform.translation.z = new_z;
        }

        let new_y = if old_y - target_y > fall_distance {
            old_y - fall_distance
        } else {
            target_y
        };
        if transform.translation.y != new_y {
            transform.translation.y = new_y;
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
