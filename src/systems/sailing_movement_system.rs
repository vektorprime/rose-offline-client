use bevy::prelude::*;

use crate::components::{BoatState, FacingDirection, PlayerCharacter, Position};
use crate::render::underwater_effect::UnderwaterVolumes;
use crate::resources::WaterSettings;
use crate::resources::WindState;

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

fn sail_speed_factor(angle_to_wind: f32) -> f32 {
    let angle = angle_to_wind.abs();
    if angle < 0.78 {
        (angle / 0.78).powf(2.0) * 0.3
    } else if angle < 1.57 {
        let t = (angle - 0.78) / (1.57 - 0.78);
        0.3 + t * 0.7
    } else if angle < 2.36 {
        let t = (angle - 1.57) / (2.36 - 1.57);
        1.0 - t * 0.2
    } else {
        let t = (angle - 2.36) / (std::f32::consts::PI - 2.36);
        0.8 - t * 0.3
    }
}

pub fn sailing_movement_system(
    time: Res<Time>,
    wind: Res<WindState>,
    underwater_volumes: Res<UnderwaterVolumes>,
    water_settings: Res<WaterSettings>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut boat_query: Query<(&mut BoatState, &mut Position, &mut FacingDirection), With<PlayerCharacter>>,
) {
    for (mut boat, mut position, mut facing) in boat_query.iter_mut() {
        if !boat.active {
            continue;
        }

        let dt = time.delta_secs();

        let steer_input = if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            -1.0
        } else if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            1.0
        } else {
            0.0
        };
        boat.rudder = steer_input;

        let turn_rate = 1.0 * (boat.speed / boat.max_speed).clamp(0.1, 1.0);
        boat.heading += steer_input * turn_rate * dt;
        boat.heading = boat.heading.rem_euclid(std::f32::consts::TAU);

        if keyboard.pressed(KeyCode::KeyW) {
            boat.sail_trim = (boat.sail_trim - 0.5 * dt).max(0.0);
        }
        if keyboard.pressed(KeyCode::KeyS) {
            boat.sail_trim = (boat.sail_trim + 0.5 * dt).min(std::f32::consts::PI);
        }

        let angle_to_wind = (boat.heading - wind.angle).rem_euclid(std::f32::consts::TAU);
        let angle_to_wind_abs = if angle_to_wind > std::f32::consts::PI {
            std::f32::consts::TAU - angle_to_wind
        } else {
            angle_to_wind
        };

        let speed_factor = sail_speed_factor(angle_to_wind_abs);
        let target_speed_base = boat.max_speed * speed_factor * (wind.speed / 5.0).clamp(0.0, 2.0);

        let optimal_trim = angle_to_wind_abs * 0.5;
        let trim_efficiency = 1.0 - ((boat.sail_trim - optimal_trim).abs() / std::f32::consts::PI);
        let target_speed = target_speed_base * trim_efficiency.clamp(0.1, 1.0);

        let accel = if target_speed > boat.speed { 2.0 } else { 1.5 };
        boat.speed += (target_speed - boat.speed) * accel * dt;
        boat.speed = boat.speed.clamp(0.0, boat.max_speed);

        let forward = Vec3::new(boat.heading.sin(), boat.heading.cos(), 0.0);
        let movement = forward * boat.speed * dt * 100.0;
        position.position.x += movement.x;
        position.position.y += movement.y;

        let water_height_cm = sample_water_surface_height_cm(
            position.position,
            boat.water_height_cm,
            &underwater_volumes,
            &water_settings,
        );
        boat.water_height_cm = water_height_cm;
        position.position.z = water_height_cm;

        facing.desired = boat.heading;
    }
}

