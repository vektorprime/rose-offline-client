use bevy::{
    math::Vec3Swizzles,
    prelude::{Query, Res, Time},
};

use rose_game_common::components::MoveSpeed;

use crate::components::{Command, CommandMove, FacingDirection, Position};

pub fn update_position_system(
    mut query: Query<(&Command, &MoveSpeed, &mut FacingDirection, &mut Position)>,
    time: Res<Time>,
) {
    for (command, move_speed, mut facing_direction, mut position) in query.iter_mut() {
        let Command::Move(CommandMove { destination, .. }) = *command else {
            continue;
        };

        let direction = destination.xy() - position.xy();
        let distance_squared = direction.length_squared();

        if distance_squared == 0.0 {
            // Arrived: only write back when the position actually differs, so an
            // idle mover stops marking Position changed every frame.
            if position.position != destination {
                position.position = destination;
            }
        } else {
            // Update rotation only when the desired angle actually changes
            let desired_angle = direction.y.atan2(direction.x) + std::f32::consts::PI;
            if (facing_direction.desired - desired_angle).abs() > 0.001 {
                facing_direction.desired = desired_angle;
            }

            // Move to position
            let move_vector = direction.normalize() * move_speed.speed * time.delta_secs();
            if move_vector.length_squared() >= distance_squared {
                position.position = destination;
            } else {
                position.x += move_vector.x;
                position.y += move_vector.y;
            }
        }
    }
}
