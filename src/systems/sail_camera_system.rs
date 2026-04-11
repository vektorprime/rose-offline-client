use bevy::prelude::*;

use crate::components::{BoatState, PlayerCharacter};
use crate::systems::OrbitCamera;

pub fn sail_camera_system(
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
        orbit_camera.follow_distance = orbit_camera.follow_distance.clamp(10.0, 22.0);
    }
}

