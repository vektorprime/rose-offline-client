use bevy::{
    math::Vec3,
    prelude::{
        Camera3d, Commands, Entity, MessageReader, Projection, Query, Res, ResMut, With, Without,
    },
};
use rose_game_common::messages::client::ClientMessage;

use crate::{
    animation::CameraAnimation,
    components::PlayerCharacter,
    events::ZoneEvent,
    graphics::GraphicsSettings,
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
    query_cameras: Query<
        Entity,
        (With<Camera3d>, Without<crate::render::WaterReflectionCamera>),
    >,
    query_player: Query<Entity, With<PlayerCharacter>>,
    graphics_settings: Option<Res<GraphicsSettings>>,
    mut query_projection: Query<
        &mut Projection,
        (With<Camera3d>, Without<crate::render::WaterReflectionCamera>),
    >,
) {
    // Reset camera
    let player_entity = match query_player.single() {
        Ok(entity) => entity,
        Err(_) => return,
    };

    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<CameraAnimation>()
            .insert(OrbitCamera::new(
                player_entity,
                Vec3::new(0.0, 1.7, 0.0),
                15.0,
            ));
    }

    // Login/character-select cinematics (CameraAnimation::once) overwrite the
    // projection (fov/near/far, e.g. far=1300) and never restore it. Without
    // this reset the game camera keeps the cinematic frustum (clipped sky,
    // wrong depth precision) until something re-applies graphics settings.
    // Same mapping as apply_view_distance_system; spawn defaults otherwise.
    let view_distance = graphics_settings
        .as_ref()
        .map(|g| g.view_distance)
        .unwrap_or(500.0);
    let far = (view_distance * 16.0).clamp(6000.0, 12000.0);
    for mut projection in query_projection.iter_mut() {
        if let Projection::Perspective(ref mut perspective) = *projection {
            perspective.fov = std::f32::consts::PI / 4.0;
            perspective.near = 0.1;
            perspective.far = far;
        }
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
