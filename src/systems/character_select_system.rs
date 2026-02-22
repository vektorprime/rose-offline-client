use std::time::{Duration, Instant};

use log::info;

use bevy::{
    input::ButtonInput,
    prelude::{
        AssetServer, Camera, Camera3d, Commands, Component, ViewVisibility, InheritedVisibility,
        Entity, EventReader, EventWriter, GlobalTransform, Handle, Local,
        MouseButton, NextState, Query, Res, ResMut, Resource, Vec2, Visibility, With, World,
    },
    render::mesh::skinning::SkinnedMesh,
    window::{CursorGrabMode, PrimaryWindow, Window},
};
use bevy_egui::{egui, EguiContexts};
use bevy_rapier3d::{
    plugin::context::systemparams::ReadRapierContext,
    prelude::{CollisionGroups, QueryFilter},
};

use rose_data::{CharacterMotionAction, ZoneId};
use rose_game_common::messages::{client::ClientMessage, server::CreateCharacterError};

use crate::{
    animation::{CameraAnimation, SkeletalAnimation, ZmoAsset},
    components::{
        CharacterModel, ColliderParent, COLLISION_FILTER_CLICKABLE, COLLISION_GROUP_CHARACTER,
        COLLISION_GROUP_PLAYER,
    },
    events::{CharacterSelectEvent, GameConnectionEvent, LoadZoneEvent, WorldConnectionEvent},
    resources::{
        AppState, CharacterList, CharacterSelectState, GameData, ServerConfiguration,
        WorldConnection,
    },
    systems::{FreeCamera, OrbitCamera},
};

#[derive(Component)]
pub struct CharacterSelectCharacter {
    pub index: usize,
}

#[derive(Resource)]
pub struct CharacterSelectModelList {
    models: Vec<(Option<String>, Entity)>,
    select_motion: Handle<ZmoAsset>,
}

pub fn character_select_enter_system(
    mut commands: Commands,
    mut query_window: Query<&mut Window, With<PrimaryWindow>>,
    query_cameras: Query<Entity, With<Camera3d>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
) {
    log::info!("[CHAR_SELECT] Enter system called - setting up character select screen");
    if let Ok(mut window) = query_window.get_single_mut() {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }

    // Reset camera
    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<OrbitCamera>()
            .insert(CameraAnimation::once(
                asset_server.load("3DDATA/TITLE/CAMERA01_INSELECT01.ZMO"),
            ));
    }

    // Reset state
    commands.insert_resource(CharacterSelectState::Entering);

    // Spawn entities to use for character list models
    let mut models = Vec::with_capacity(game_data.character_select_positions.len());
    for (index, transform) in game_data.character_select_positions.iter().enumerate() {
        let entity = commands
            .spawn((
                CharacterSelectCharacter { index },
                *transform,
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id();
        models.push((None, entity));
    }
    commands.insert_resource(CharacterSelectModelList {
        models,
        select_motion: asset_server.load("3DDATA/MOTION/AVATAR/EVENT_SELECT_M1.ZMO"),
    });
}

pub fn character_select_exit_system(
    mut commands: Commands,
    model_list: Res<CharacterSelectModelList>,
) {
    // Despawn character models
    for (_, entity) in model_list.models.iter() {
        commands.entity(*entity).despawn();
    }

    commands.remove_resource::<CharacterList>();
    commands.remove_resource::<CharacterSelectState>();
    commands.remove_resource::<CharacterSelectModelList>();
}

pub fn character_select_models_system(
    mut commands: Commands,
    mut model_list: ResMut<CharacterSelectModelList>,
    character_list: Option<Res<CharacterList>>,
    character_select_state: Res<CharacterSelectState>,
    query_characters: Query<(Option<&SkeletalAnimation>, &CharacterModel), With<SkinnedMesh>>,
) {
    // Ensure all character list models are up to date
    if let Some(character_list) = character_list.as_ref() {
        for (index, character) in character_list.characters.iter().enumerate() {
            let entity = model_list.models[index].1;

            // If the character list has changed, recreate model
            if model_list.models[index].0.as_ref() != Some(&character.info.name) {
                commands
                    .entity(model_list.models[index].1)
                    .insert((character.info.clone(), character.equipment.clone()));
                model_list.models[index].0 = Some(character.info.name.clone());
            }

            if let Ok((skeletal_animation, character_model)) = query_characters.get(entity) {
                let deleting = character.delete_time.is_some();
                let selected = if let CharacterSelectState::CharacterSelect(Some(selected_index)) =
                    *character_select_state
                {
                    selected_index == index
                } else {
                    false
                };

                let desired_motion = if deleting {
                    &character_model.action_motions[CharacterMotionAction::Sit]
                } else if selected {
                    &model_list.select_motion
                } else {
                    &character_model.action_motions[CharacterMotionAction::Stop1]
                };

                if skeletal_animation.map_or(true, |x| x.motion().id() != desired_motion.id()) {
                    commands
                        .entity(entity)
                        .insert(SkeletalAnimation::repeat(desired_motion.clone(), None));
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn character_select_system(
    mut commands: Commands,
    mut app_state: ResMut<NextState<AppState>>,
    mut character_select_state: ResMut<CharacterSelectState>,
    mut egui_context: EguiContexts,
    mut game_connection_events: EventReader<GameConnectionEvent>,
    mut world_connection_events: EventReader<WorldConnectionEvent>,
    mut load_zone_events: EventWriter<LoadZoneEvent>,
    mut join_zone_id: Local<Option<ZoneId>>,
    query_camera: Query<
        (Entity, &Camera, &GlobalTransform, Option<&CameraAnimation>),
        With<Camera3d>,
    >,
    world_connection: Option<Res<WorldConnection>>,
    mut character_list: Option<ResMut<CharacterList>>,
    server_configuration: Res<ServerConfiguration>,
    asset_server: Res<AssetServer>,
) {
    let character_select_state = &mut *character_select_state;
    let world_connection = if let Some(world_connection) = world_connection {
        world_connection
    } else {
        // Disconnected, return to login
        app_state.set(AppState::GameLogin);
        return;
    };

    for event in world_connection_events.read() {
        match event {
            WorldConnectionEvent::CreateCharacterSuccess { character_slot: _ } => {
                if let Ok((camera_entity, _, _, _)) = query_camera.get_single() {
                    commands.entity(camera_entity).insert(CameraAnimation::once(
                        asset_server.load("3DDATA/TITLE/CAMERA01_OUTCREATE01.ZMO"),
                    ));
                }
                *character_select_state = CharacterSelectState::CharacterSelect(None);

                world_connection
                    .client_message_tx
                    .send(ClientMessage::GetCharacterList)
                    .ok();
            }
            WorldConnectionEvent::CreateCharacterError { error } => match error {
                CreateCharacterError::Failed => {
                    // TODO: Show modal error dialog with error message
                    // character_select_state.create_character_error_message =
                    //    "Unknown error creating character".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
                CreateCharacterError::AlreadyExists => {
                    // TODO: Show modal error dialog with error message
                    // character_select_state.create_character_error_message =
                    //    "Character name already exists".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
                CreateCharacterError::NoMoreSlots => {
                    // TODO: Show modal error dialog with error message
                    //character_select_state.create_character_error_message =
                    //    "Cannot create more characters".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
                CreateCharacterError::InvalidValue => {
                    // TODO: Show modal error dialog with error message
                    // character_select_state.create_character_error_message = "Invalid value".into();
                    *character_select_state = CharacterSelectState::CharacterCreate;
                }
            },
            WorldConnectionEvent::DeleteCharacterStart { name, delete_time } => {
                if let Some(character_list) = character_list.as_mut() {
                    for character in character_list.characters.iter_mut() {
                        if character.info.name == *name {
                            character.delete_time = Some(*delete_time);
                            break;
                        }
                    }
                } else {
                    world_connection
                        .client_message_tx
                        .send(ClientMessage::GetCharacterList)
                        .ok();
                }
            }
            WorldConnectionEvent::DeleteCharacterCancel { name } => {
                if let Some(character_list) = character_list.as_mut() {
                    for character in character_list.characters.iter_mut() {
                        if character.info.name == *name {
                            character.delete_time = None;
                            break;
                        }
                    }
                } else {
                    world_connection
                        .client_message_tx
                        .send(ClientMessage::GetCharacterList)
                        .ok();
                }
            }
            WorldConnectionEvent::DeleteCharacterError { name: _ } => {
                // TODO: Show delete character error message
            }
        }
    }

    match character_select_state {
        CharacterSelectState::Entering => {
            let camera_result = query_camera.get_single();
            let camera_motion = camera_result.ok().and_then(|(_, _, _, m)| m);
            let camera_completed = camera_motion.map_or(true, |animation| animation.completed());
            log::info!("[CHAR_SELECT] Entering state - camera_completed: {}, auto_login: {}",
                camera_completed, server_configuration.auto_login);
            if camera_completed
                || server_configuration.auto_login
            {
                log::info!("[CHAR_SELECT] Transitioning to CharacterSelect(None)");
                *character_select_state = CharacterSelectState::CharacterSelect(None);
            }
        }
        CharacterSelectState::CharacterSelect(_) => {}
        CharacterSelectState::CharacterCreate => {}
        CharacterSelectState::CharacterCreating => {
            egui::Window::new("Creating character...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Creating character");
                });
        }
        CharacterSelectState::ConnectingGameServer => {
            egui::Window::new("Connecting...")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .collapsible(false)
                .show(egui_context.ctx_mut(), |ui| {
                    ui.label("Connecting to game");
                });

            for event in game_connection_events.read() {
                let &GameConnectionEvent::Connected(zone_id) = event;

                // Start camera animation
                if let Ok((camera_entity, _, _, _)) = query_camera.get_single() {
                    commands.entity(camera_entity).insert(CameraAnimation::once(
                        asset_server.load("3DDATA/TITLE/CAMERA01_INGAME01.ZMO"),
                    ));
                }

                *character_select_state = CharacterSelectState::Leaving;
                *join_zone_id = Some(zone_id);
            }
        }
        CharacterSelectState::Leaving => {
            let camera_motion = query_camera.get_single().ok().and_then(|(_, _, _, m)| m);
            if camera_motion.map_or(true, |animation| animation.completed())
                || server_configuration.auto_login
            {
                // Wait until camera motion complete, then load the zone!
                *character_select_state = CharacterSelectState::Loading;
                load_zone_events.write(LoadZoneEvent::new(join_zone_id.take().unwrap()));
            }
        }
        CharacterSelectState::Loading => {}
    }
}

pub fn character_select_event_system(
    mut commands: Commands,
    mut character_select_state: ResMut<CharacterSelectState>,
    mut character_select_events: EventReader<CharacterSelectEvent>,
    character_list: Option<Res<CharacterList>>,
    world_connection: Option<Res<WorldConnection>>,
) {
    for event in character_select_events.read() {
        match event {
            CharacterSelectEvent::SelectCharacter(index) => {
                if matches!(
                    *character_select_state,
                    CharacterSelectState::CharacterSelect(_)
                ) {
                    *character_select_state = CharacterSelectState::CharacterSelect(Some(*index));
                }
            }
            CharacterSelectEvent::PlaySelected => {
                if let CharacterSelectState::CharacterSelect(Some(selected_character_index)) =
                    *character_select_state
                {
                    if let Some(character_list) = character_list.as_ref() {
                        if let Some(selected_character) =
                            character_list.characters.get(selected_character_index)
                        {
                            if selected_character.delete_time.is_none() {
                                if let Some(world_connection) = world_connection.as_ref() {
                                    world_connection
                                        .client_message_tx
                                        .send(ClientMessage::SelectCharacter {
                                            slot: selected_character_index as u8,
                                            name: selected_character.info.name.clone(),
                                        })
                                        .ok();
                                }

                                *character_select_state =
                                    CharacterSelectState::ConnectingGameServer;
                            }
                        }
                    }
                }
            }
            CharacterSelectEvent::DeleteSelected => {
                if let CharacterSelectState::CharacterSelect(Some(selected_character_index)) =
                    *character_select_state
                {
                    if let Some(character_list) = character_list.as_ref() {
                        if let Some(selected_character) =
                            character_list.characters.get(selected_character_index)
                        {
                            if let Some(world_connection) = world_connection.as_ref() {
                                world_connection
                                    .client_message_tx
                                    .send(ClientMessage::DeleteCharacter {
                                        slot: selected_character_index as u8,
                                        name: selected_character.info.name.clone(),
                                        is_delete: selected_character.delete_time.is_none(),
                                    })
                                    .ok();
                            }
                        }
                    }
                }
            }
            CharacterSelectEvent::Disconnect => {
                commands.remove_resource::<WorldConnection>();
            }
        }
    }
}

/// Resource to track character select input state (cursor position and last click time)
#[derive(Resource, Default)]
pub struct CharacterSelectInputState {
    pub cursor_position: Option<Vec2>,
    pub last_click_time: Option<Instant>,
    pub selected_character_index: Option<usize>,
}

/// Combined system for character selection input handling
#[allow(clippy::too_many_arguments)]
pub fn character_select_input_system(
    _character_select_state: Res<CharacterSelectState>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    rapier_context: ReadRapierContext,
    mut input_state: ResMut<CharacterSelectInputState>,
    mut egui_ctx: EguiContexts,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_entities: Query<(Option<&ColliderParent>, Option<&CharacterSelectCharacter>)>,
    mut character_select_events: EventWriter<CharacterSelectEvent>,
) {
    // Get the single rapier context
    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    // Only process input when egui is not using the mouse
    let ctx = egui_ctx.ctx_mut();
    if ctx.is_pointer_over_area() || ctx.is_using_pointer() {
        return;
    }

    // Get cursor position from window
    let Ok(window) = query_window.get_single() else {
        return;
    };

    let Some(cursor_position) = window.cursor_position() else {
        input_state.cursor_position = None;
        return;
    };
    input_state.cursor_position = Some(cursor_position);

    // Check for left mouse button click
    if !mouse_button_input.just_pressed(MouseButton::Left) {
        return;
    }

    // Debounce: Only allow one click per 200ms
    let now = Instant::now();
    if let Some(last_click_time) = input_state.last_click_time {
        if now.duration_since(last_click_time) < Duration::from_millis(200) {
            return;
        }
    }
    input_state.last_click_time = Some(now);

    // Get camera for raycasting
    let Ok((camera, camera_transform)) = query_camera.get_single() else {
        return;
    };

    // Generate ray from cursor position
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return;
    };

    log::info!("[CHAR_SELECT_INPUT] Casting ray from origin {:?} direction {:?}", ray.origin, ray.direction);
    
    // Cast ray and find the closest hit
    let query_filter = QueryFilter::new().groups(CollisionGroups::new(
        COLLISION_FILTER_CLICKABLE,
        COLLISION_GROUP_CHARACTER | COLLISION_GROUP_PLAYER,
    ));

    if let Some((collider_entity, _distance)) = rapier_context.cast_ray(
        ray.origin,
        *ray.direction,
        f32::MAX,
        true,
        query_filter,
    ) {
        log::info!("[CHAR_SELECT_INPUT] Ray hit entity {:?}", collider_entity);
        // The hit entity might be the collider, so we need to find the parent
        if let Ok((Some(collider_parent), _)) = query_entities.get(collider_entity) {
            if let Ok((_, Some(character_select))) = query_entities.get(collider_parent.entity) {
                let selected_index = character_select.index;
                input_state.selected_character_index = Some(selected_index);

                // Send character select event
                character_select_events.write(CharacterSelectEvent::SelectCharacter(selected_index));
            }
        } else if let Ok((_, Some(character_select))) = query_entities.get(collider_entity) {
            let selected_index = character_select.index;
            input_state.selected_character_index = Some(selected_index);

            // Send character select event
            character_select_events.write(CharacterSelectEvent::SelectCharacter(selected_index));
        }
    } else {
        log::info!("[CHAR_SELECT_INPUT] No ray hit found");
    }
}
