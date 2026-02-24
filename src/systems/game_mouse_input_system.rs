use bevy::{
    input::ButtonInput,
    math::Vec3,
    prelude::{
        BevyError, Camera, Camera3d, Entity, EventWriter, GlobalTransform, Local, MouseButton, Query, Res, ResMut,
        State, With,
    },
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_egui::EguiContexts;
use bevy_rapier3d::{
    plugin::context::systemparams::ReadRapierContext,
    prelude::{CollisionGroups, QueryFilter},
};

use rose_game_common::components::{ItemDrop, Team};

use crate::{
    components::{
        ColliderParent, ClientEntity, ClientEntityType, FlightState, PlayerCharacter, Position, ZoneObject,
        COLLISION_FILTER_CLICKABLE, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_PLAYER,
    },
    events::{MoveDestinationEffectEvent, PlayerCommandEvent},
    resources::{AppState, SelectedTarget, UiCursorType},
};

pub type PlayerQuery<'w> = (Entity, &'w Team, Option<&'w FlightState>);

/// Game mouse input system - handles mouse clicks for movement, attacking, and interaction
/// This system has been refactored to reduce the number of parameters to 10
pub fn game_mouse_input_system(
    app_state: Res<State<AppState>>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    rapier_context: ReadRapierContext,
    mut egui_ctx: EguiContexts,
    query_hit_entity: Query<(
        Option<&Team>,
        Option<&Position>,
        Option<&ItemDrop>,
        Option<&ZoneObject>,
        Option<&ClientEntity>,
    )>,
    query_player: Query<PlayerQuery, With<PlayerCharacter>>,
    query_collider_parent: Query<&ColliderParent>,
    mut player_command_events: EventWriter<PlayerCommandEvent>,
    mut selected_target: ResMut<SelectedTarget>,
) -> Result<(), BevyError> {
    let Ok(rapier_context) = rapier_context.single() else {
        return Ok(());
    };
    
    // Check if we're in the game state
    if *app_state.get() != AppState::Game {
        return Ok(());
    }
    selected_target.hover = None;

    let Ok(window) = query_window.get_single() else {
        return Ok(());
    };

    if !matches!(window.cursor_options.grab_mode, CursorGrabMode::None) {
        // Cursor is currently grabbed
        return Ok(());
    }

    let Some(cursor_position) = window.cursor_position() else {
        // Failed to get cursor position
        return Ok(());
    };

    if egui_ctx.ctx_mut().wants_pointer_input() {
        return Ok(());
    }

    let (_player_entity, player_team, player_flight_state) = if let Ok(result) = query_player.get_single() {
        result
    } else {
        return Ok(());
    };
    
    // Check if player is flying - if so, skip terrain click-to-move
    // but still allow interaction with UI and entities
    let is_flying = player_flight_state.map_or(false, |fs| fs.is_flying);
    
    let Ok((camera, camera_transform)) = query_camera.get_single() else {
        return Ok(());
    };

    if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
        if let Some((collider_entity, distance)) = rapier_context.cast_ray(
            ray.origin,
            *ray.direction,
            10000000.0,
            false,
            QueryFilter::new().groups(CollisionGroups::new(
                COLLISION_FILTER_CLICKABLE,
                !COLLISION_GROUP_PLAYER & !COLLISION_GROUP_PHYSICS_TOY,
            )),
        ) {
            let hit_position = ray.get_point(distance);
            let hit_entity = query_collider_parent
                .get(collider_entity)
                .map_or(collider_entity, |collider_parent| collider_parent.entity);

            if let Ok((
                hit_team,
                hit_entity_position,
                hit_item_drop,
                hit_zone_object,
                hit_client_entity,
            )) = query_hit_entity.get(hit_entity)
            {
                if let Some(hit_client_entity) = hit_client_entity {
                    match hit_client_entity.entity_type {
                        ClientEntityType::Character => {
                            selected_target.cursor_type = UiCursorType::User
                        }
                        ClientEntityType::Monster => {
                            selected_target.cursor_type = UiCursorType::Attack
                        }
                        ClientEntityType::Npc => {
                            selected_target.cursor_type = UiCursorType::Npc
                        }
                        ClientEntityType::ItemDrop => {
                            selected_target.cursor_type = UiCursorType::PickupItem
                        }
                    }
                } else {
                    selected_target.cursor_type = UiCursorType::Default;
                }

                if let Some(hit_team) = hit_team {
                    if hit_team.id != Team::DEFAULT_NPC_TEAM_ID && hit_team.id != player_team.id {
                        selected_target.cursor_type = UiCursorType::Attack;
                    }
                }

                if hit_zone_object.is_some() {
                    // Only allow terrain click-to-move when NOT flying
                    if !is_flying && mouse_button_input.just_pressed(MouseButton::Left) {
                        player_command_events.write(PlayerCommandEvent::Move(
                            Position::new(Vec3::new(
                                hit_position.x * 100.0,
                                -hit_position.z * 100.0,
                                f32::max(0.0, hit_position.y * 100.0),
                            )),
                            None,
                        ));
                    }
                } else if hit_item_drop.is_some() {
                    selected_target.hover = Some(hit_entity);

                    if mouse_button_input.just_pressed(MouseButton::Left) {
                        if let Some(hit_entity_position) = hit_entity_position {
                            // Move to target item drop, once we are close enough the command_system
                            // will send the pickup client message to perform the actual pickup
                            player_command_events.write(PlayerCommandEvent::Move(
                                hit_entity_position.clone(),
                                Some(hit_entity),
                            ));
                        }
                    }
                } else if let Some(hit_team) = hit_team {
                    selected_target.hover = Some(hit_entity);

                    if mouse_button_input.just_pressed(MouseButton::Left) {
                        if selected_target
                            .selected
                            .map_or(false, |selected_entity| selected_entity == hit_entity)
                        {
                            if hit_team.id == Team::DEFAULT_NPC_TEAM_ID
                                || hit_team.id == player_team.id
                            {
                                // Only allow click-to-move to friendly units when NOT flying
                                if !is_flying {
                                    if let Some(hit_entity_position) = hit_entity_position {
                                        player_command_events.write(PlayerCommandEvent::Move(
                                            hit_entity_position.clone(),
                                            Some(hit_entity),
                                        ));
                                    }
                                }
                            } else {
                                player_command_events.write(PlayerCommandEvent::Attack(hit_entity));
                            }
                        } else {
                            selected_target.selected = Some(hit_entity);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}
