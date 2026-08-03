use bevy::prelude::*;

use crate::components::{BoatState, PlayerCharacter};
use crate::systems::OrbitCamera;

pub fn sail_camera_system(
    time: Res<Time>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    boat_query: Query<&BoatState, With<PlayerCharacter>>,
    mut camera_query: Query<&mut OrbitCamera>,
) {
    let Ok(boat) = boat_query.single() else {
        return;
    };

    if !boat.active {
        return;
    }

    for mut orbit_camera in camera_query.iter_mut() {
        let target_distance =
            (13.5 + (boat.speed / boat.max_speed.max(0.1)).clamp(0.0, 1.0) * 6.5).clamp(12.0, 24.0);
        orbit_camera.follow_distance = orbit_camera.follow_distance.clamp(12.0, 24.0);

        if !mouse_buttons.pressed(MouseButton::Right) {
            orbit_camera.follow_distance +=
                (target_distance - orbit_camera.follow_distance) * 2.0 * time.delta_secs();

            let yaw_pitch = orbit_camera.rig.driver_mut::<dolly::prelude::YawPitch>();
            // The camera must sit behind the boat: its offset direction is
            // (sin(yaw), cos(yaw)) in world XZ (dolly Arm offset (0,0,d) rotated
            // by the YawPitch), while the boat travels along (sin(h), -cos(h)).
            // Offsetting the boat's motion gives (-sin(h), cos(h)) => yaw = -h.
            let target_yaw_degrees = -boat.heading.to_degrees();
            let mut diff = (target_yaw_degrees - yaw_pitch.yaw_degrees).rem_euclid(360.0);
            if diff > 180.0 {
                diff -= 360.0;
            }
            yaw_pitch.yaw_degrees += diff * 3.0 * time.delta_secs();

            let target_pitch_degrees = -20.0;
            yaw_pitch.pitch_degrees +=
                (target_pitch_degrees - yaw_pitch.pitch_degrees) * 1.6 * time.delta_secs();
        }

        orbit_camera.follow_offset.y = 1.75;
    }
}
