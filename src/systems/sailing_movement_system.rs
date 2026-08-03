use bevy::prelude::*;

use rose_game_common::messages::client::ClientMessage;

use crate::components::{BoatState, FacingDirection, PlayerCharacter, Position};
use crate::render::underwater_effect::UnderwaterVolumes;
use crate::resources::{GameConnection, WaterSettings, WindState};
use crate::sailing::{sailing_step, SailingStepInput};

/// Interval between sail state reports to the server (seconds).
const SAIL_REPORT_INTERVAL: f32 = 0.1;

fn sample_water_surface_height_cm(
    position_cm: Vec3,
    previous_water_height_cm: f32,
    underwater_volumes: &UnderwaterVolumes,
    water_settings: &WaterSettings,
) -> f32 {
    let world_x = position_cm.x / 100.0;
    let world_z = -position_cm.y / 100.0;

    let mut closest_surface_cm: Option<(f32, f32)> = None;

    for volume in underwater_volumes.volumes.iter() {
        let dx = (world_x - volume.center.x).abs();
        let dz = (world_z - volume.center.z).abs();
        let inside_bounds = dx <= volume.half_extents.x && dz <= volume.half_extents.y;

        if !inside_bounds {
            continue;
        }

        let surface_cm = volume.surface_y * 100.0;
        let score = (surface_cm - position_cm.z).abs();

        match closest_surface_cm {
            Some((best_score, _)) if score >= best_score => {}
            _ => {
                closest_surface_cm = Some((score, surface_cm));
            }
        }
    }

    if let Some((_, surface_cm)) = closest_surface_cm {
        surface_cm
    } else if previous_water_height_cm.abs() > f32::EPSILON {
        previous_water_height_cm
    } else {
        water_settings.water_surface_y * 100.0
    }
}

pub fn sailing_movement_system(
    time: Res<Time>,
    wind: Res<WindState>,
    underwater_volumes: Res<UnderwaterVolumes>,
    water_settings: Res<WaterSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    game_connection: Option<Res<GameConnection>>,
    mut report_accumulator: Local<f32>,
    mut boat_query: Query<
        (&mut BoatState, &mut Position, &mut FacingDirection),
        With<PlayerCharacter>,
    >,
) {
    for (mut boat, mut position, mut facing) in boat_query.iter_mut() {
        if !boat.active {
            continue;
        }

        let dt = time.delta_secs();

        let steer_input = if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft)
        {
            -1.0
        } else if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            1.0
        } else {
            0.0
        };
        let throttle_input = if keyboard.pressed(KeyCode::KeyW) {
            1.0
        } else if keyboard.pressed(KeyCode::KeyS) {
            -1.0
        } else {
            0.0
        };

        let step = sailing_step(SailingStepInput {
            heading: boat.heading,
            speed: boat.speed,
            rudder: steer_input,
            throttle: throttle_input,
            max_speed: boat.max_speed,
            wind_angle: wind.angle,
            wind_speed: wind.speed,
            dt,
        });
        boat.rudder = steer_input;
        boat.heading = step.heading;
        boat.speed = step.speed;
        position.position.x += step.forward_cm.x;
        position.position.y += step.forward_cm.y;

        let water_height_cm = sample_water_surface_height_cm(
            position.position,
            boat.water_height_cm,
            &underwater_volumes,
            &water_settings,
        );
        boat.water_height_cm = water_height_cm;
        position.position.z = water_height_cm;

        facing.desired = boat.heading;

        // Send a periodic state report (inputs + full boat state + position)
        // to the server for validation and authoritative broadcasting to
        // other clients. The local player does NOT reconcile from the echoed
        // SailState; its own prediction is authoritative (server corrections
        // arrive via AdjustPosition).
        if let Some(game_connection) = game_connection.as_ref() {
            *report_accumulator += dt;
            if *report_accumulator >= SAIL_REPORT_INTERVAL {
                *report_accumulator = 0.0;
                game_connection
                    .client_message_tx
                    .send(ClientMessage::SailInput {
                        rudder: steer_input,
                        throttle: throttle_input,
                        heading: boat.heading,
                        speed: boat.speed,
                        sail_trim: boat.sail_trim,
                        x: position.position.x,
                        y: position.position.y,
                        z: position.position.z,
                    })
                    .ok();
            }
        }
    }
}
