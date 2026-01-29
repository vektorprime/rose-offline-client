use bevy::{
    math::Vec3,
    prelude::{Camera3d, Commands, Entity, EventReader, Query, Res, With},
};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    animation::CameraAnimation,
    components::PlayerCharacter,
    events::ZoneEvent,
    resources::GameConnection,
    systems::{FreeCamera, OrbitCamera},
};

pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    query_player: Query<Entity, With<PlayerCharacter>>,
) {
    bevy::log::info!("[CAMERA] game_state_enter_system called");
    
    // Reset camera
    let player_entity = match query_player.get_single() {
        Ok(entity) => entity,
        Err(e) => {
            bevy::log::error!("[CAMERA] Failed to get player entity: {:?}", e);
            return;
        }
    };
    
    bevy::log::info!("[CAMERA] Setting up orbit camera for player entity: {:?}", player_entity);
    
    let camera_count = query_cameras.iter().count();
    bevy::log::info!("[CAMERA] Found {} camera entities", camera_count);
    
    for entity in query_cameras.iter() {
        bevy::log::info!("[CAMERA] Configuring camera entity: {:?}", entity);
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<CameraAnimation>()
            .insert(OrbitCamera::new(
                player_entity,
                Vec3::new(0.0, 1.7, 0.0),
                15.0,
            ));
        bevy::log::info!("[CAMERA] OrbitCamera attached with offset (0.0, 1.7, 0.0), distance 15.0");
    }
    
    if camera_count == 0 {
        bevy::log::warn!("[CAMERA] NO CAMERAS FOUND - this will cause black screen!");
    }
}

#[allow(clippy::too_many_arguments)]
pub fn game_zone_change_system(
    mut zone_events: EventReader<ZoneEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    for zone_event in zone_events.read() {
        match zone_event {
            &ZoneEvent::Loaded(_) => {
                // Tell server we are ready to join the zone
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::JoinZoneRequest)
                        .ok();
                }
            }
        }
    }
}
