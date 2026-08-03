use bevy::prelude::*;

use crate::components::{BoatState, Position};

pub fn boat_buoyancy_system(
    time: Res<Time>,
    mut query: Query<(&mut BoatState, &mut Transform, &mut Position)>,
    mut boat_visual_root_query: Query<&mut Transform, Without<BoatState>>,
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

        // The boat hull's bow points along local -Z. Motion follows the heading
        // (world direction (sin h, 0, -cos h)), which requires a world yaw of
        // -heading. The hull root is a child of the player entity whose rotation
        // is driven by the character-facing system (yaw = facing - PI/2), so the
        // correct root-local rotation is the player rotation inverted, then the
        // boat yaw and wave pitch/roll applied in hull space.
        let wave_rot = Quat::from_euler(EulerRot::XZY, pitch, 0.0, roll);
        let boat_world_yaw = Quat::from_axis_angle(Vec3::Y, -boat.heading);
        if let Some(model_root_entity) = boat.model_root_entity {
            if let Ok(mut root_transform) =
                boat_visual_root_query.get_mut(model_root_entity)
            {
                root_transform.rotation =
                    transform.rotation.inverse() * boat_world_yaw * wave_rot;
            }
        }

        let base_y_m = boat.water_height_cm / 100.0;
        transform.translation.y = base_y_m + heave_m;
        position.z = (base_y_m + heave_m) * 100.0;
    }
}
