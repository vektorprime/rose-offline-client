use bevy::prelude::*;
use rose_game_common::components::MoveMode;

use crate::{
    components::{
        BoatState, CharacterModel, ClientEntity, FacingDirection, PlayerCharacter, Position,
        RemoteBoatState,
    },
    graphics::GraphicsSettings,
    systems::{set_character_model_visibility, spawn_boat_visual},
};

pub fn remote_boat_sync_system(
    mut commands: Commands,
    time: Res<Time>,
    graphics_settings: Res<GraphicsSettings>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut query: Query<
        (
            Entity,
            &MoveMode,
            &Position,
            &FacingDirection,
            Option<&CharacterModel>,
            Option<&mut BoatState>,
            Option<&mut RemoteBoatState>,
        ),
        (With<ClientEntity>, Without<PlayerCharacter>),
    >,
) {
    let dt = time.delta_secs().max(0.001);

    for (entity, move_mode, position, facing, character_model, boat_state, remote_state) in
        query.iter_mut()
    {
        let is_sailing = matches!(move_mode, MoveMode::Sail);

        if !is_sailing {
            if let Some(mut boat_state) = boat_state {
                if let Some(model_root_entity) = boat_state.model_root_entity.take() {
                    commands.entity(model_root_entity).despawn();
                }
                boat_state.active = false;
                boat_state.speed = 0.0;
                set_character_model_visibility(&mut commands, character_model, false);
            }
            if remote_state.is_some() {
                commands.entity(entity).remove::<RemoteBoatState>();
            }
            continue;
        }

        let mut previous_position_cm = position.position;
        let mut remote_speed = 0.0;
        let mut remote_heading = facing.actual;
        let mut remote_sail_trim = None;

        if let Some(mut remote_state) = remote_state {
            let delta = position.position - remote_state.target_position_cm;
            let horizontal_delta = Vec2::new(delta.x, delta.y);
            remote_speed = horizontal_delta.length() / 100.0 / dt;
            if horizontal_delta.length_squared() > 1.0 {
                remote_heading = horizontal_delta.x.atan2(horizontal_delta.y);
            }

            previous_position_cm = remote_state.target_position_cm;
            remote_state.previous_position_cm = previous_position_cm;
            remote_state.target_position_cm = position.position;
            remote_state.previous_heading = remote_state.target_heading;
            remote_state.target_heading = remote_heading;
            remote_state.target_speed = remote_speed;
            remote_state.update_age = 0.0;
            remote_state.update_interval = dt;
            remote_sail_trim = Some(remote_state.sail_trim);
        }

        if let Some(mut boat_state) = boat_state {
            boat_state.active = true;
            boat_state.rider_entity = Some(entity);
            boat_state.heading = remote_heading;
            boat_state.speed = remote_speed.clamp(0.0, boat_state.max_speed);
            boat_state.sail_trim = remote_sail_trim.unwrap_or(boat_state.sail_trim);
            boat_state.rudder = 0.0;
            boat_state.water_height_cm = position.z;

            if boat_state.model_root_entity.is_none() {
                let model_root = spawn_boat_visual(
                    &mut commands,
                    &mut meshes,
                    &mut materials,
                    position,
                    graphics_settings.sailing.sail_deformation_quality,
                );
                boat_state.model_root_entity = Some(model_root);
                commands.entity(entity).add_child(model_root);
                set_character_model_visibility(&mut commands, character_model, true);
            }
        } else {
            let model_root = spawn_boat_visual(
                &mut commands,
                &mut meshes,
                &mut materials,
                position,
                graphics_settings.sailing.sail_deformation_quality,
            );
            commands.entity(entity).add_child(model_root);
            commands.entity(entity).insert((
                BoatState {
                    active: true,
                    rider_entity: Some(entity),
                    heading: remote_heading,
                    speed: remote_speed,
                    model_root_entity: Some(model_root),
                    water_height_cm: position.z,
                    ..default()
                },
                RemoteBoatState::from_position(previous_position_cm, remote_heading),
            ));
            set_character_model_visibility(&mut commands, character_model, true);
        }
    }
}
