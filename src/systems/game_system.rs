use bevy::{
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, MessageReader, Query, Res, ResMut, With},
};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    animation::CameraAnimation,
    components::PlayerCharacter,
    events::ZoneEvent,
    resources::{CurrentZone, GameConnection, WaterSettings},
    systems::{FreeCamera, OrbitCamera},
};

const OCEAN_ZONE_ID: u16 = 200;

fn apply_default_zone_water_settings(water_settings: &mut WaterSettings) {
    // Keep base rendering defaults outside of special ocean zones.
    water_settings.wave_amplitude = 0.5;
    water_settings.wave_frequency = 2.0;
    water_settings.foam_intensity = 0.5;
}

fn apply_ocean_zone_water_settings(water_settings: &mut WaterSettings) {
    // Open-ocean tuning per sailing expansion plan section D.7.
    water_settings.wave_amplitude = 1.5;
    water_settings.wave_frequency = 0.8;
    water_settings.foam_intensity = 1.2;
}

pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
) {
    //bevy::log::info!("[CAMERA] game_state_enter_system called");

    // Reset camera
    let player_entity = match query_player.single() {
        Ok(entity) => entity,
        Err(e) => {
            //bevy::log::error!("[CAMERA] Failed to get player entity: {:?}", e);
            return;
        }
    };

    //bevy::log::info!("[CAMERA] Setting up orbit camera for player entity: {:?}", player_entity);

    let camera_count = query_cameras.iter().count();
    //bevy::log::info!("[CAMERA] Found {} camera entities", camera_count);

    for entity in query_cameras.iter() {
        //bevy::log::info!("[CAMERA] Configuring camera entity: {:?}", entity);
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<CameraAnimation>()
            .insert(OrbitCamera::new(
                player_entity,
                Vec3::new(0.0, 1.7, 0.0),
                15.0,
            ));
        //bevy::log::info!("[CAMERA] OrbitCamera attached with offset (0.0, 1.7, 0.0), distance 15.0");
        let _ = entity; // Suppress unused variable warning
    }

    if camera_count == 0 {
        //bevy::log::warn!("[CAMERA] NO CAMERAS FOUND - this will cause black screen!");
    }
}

#[allow(clippy::too_many_arguments)]
pub fn game_zone_change_system(
    mut zone_events: MessageReader<ZoneEvent>,
    game_connection: Option<Res<GameConnection>>,
    current_zone: Option<Res<CurrentZone>>,
    mut water_settings: ResMut<WaterSettings>,
) {
    for zone_event in zone_events.read() {
        match zone_event {
            &ZoneEvent::Loaded(zone_id) => {
                // Tell server we are ready to join the zone
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::JoinZoneRequest)
                        .ok();
                }

                let effective_zone_id = current_zone
                    .as_ref()
                    .map(|zone| zone.id.get())
                    .unwrap_or(zone_id.get());

                if effective_zone_id == OCEAN_ZONE_ID {
                    apply_ocean_zone_water_settings(&mut water_settings);
                } else {
                    apply_default_zone_water_settings(&mut water_settings);
                }
            }
        }
    }
}
