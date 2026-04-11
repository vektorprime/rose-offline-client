use bevy::prelude::*;

use crate::components::{BoatState, PlayerCharacter, Position};

pub fn boat_buoyancy_system(
    time: Res<Time>,
    mut query: Query<(&mut BoatState, &mut Transform, &mut Position), With<PlayerCharacter>>,
) {
    let t = time.elapsed_secs();

    for (mut boat, mut transform, mut position) in query.iter_mut() {
        if !boat.active {
            continue;
        }

        let wave_phase = position.x * 0.0001 + t * 1.5;
        let roll = wave_phase.sin() * 0.05;
        let pitch = (wave_phase * 0.7 + 1.3).sin() * 0.03;
        let heave_m = (wave_phase * 1.2).sin() * 0.1;

        boat.wave_roll = roll;
        boat.wave_pitch = pitch;

        let heading_rot = Quat::from_axis_angle(Vec3::Y, boat.heading - std::f32::consts::PI / 2.0);
        let wave_rot = Quat::from_euler(EulerRot::XZY, pitch, 0.0, roll);
        transform.rotation = heading_rot * wave_rot;

        let base_y_m = boat.water_height_cm / 100.0;
        transform.translation.y = base_y_m + heave_m;
        position.z = (base_y_m + heave_m) * 100.0;
    }
}

