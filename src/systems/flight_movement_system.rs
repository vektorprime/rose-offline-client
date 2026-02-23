use bevy::prelude::*;

use crate::components::{FacingDirection, FlightState, PlayerCharacter, Position};
use crate::resources::FlightSettings;

/// System that handles flight movement when Space bar is held.
/// The character flies forward in the direction they are facing.
///
/// This system:
/// - Checks if Space bar is held when the player is in flight mode
/// - Accelerates the player forward in their facing direction
/// - Decelerates when not thrusting
/// - Updates the Position component to move the character
pub fn flight_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    flight_settings: Res<FlightSettings>,
    time: Res<Time>,
    mut query: Query<(&mut FlightState, &FacingDirection, &mut Position), With<PlayerCharacter>>,
) {
    for (mut flight_state, facing, mut position) in query.iter_mut() {
        // Only process if flying
        if !flight_state.is_flying {
            continue;
        }

        // Check if Space bar is held
        let is_thrusting = keyboard.pressed(KeyCode::Space);
        flight_state.is_thrusting = is_thrusting;

        if is_thrusting {
            // Accelerate
            flight_state.current_speed = (flight_state.current_speed
                + flight_settings.acceleration * time.delta_secs())
                .min(flight_settings.max_speed);

            // Move in facing direction (horizontal only - ignore Y component)
            let forward = get_horizontal_forward_direction(facing.actual);
            position.position += forward * flight_state.current_speed * time.delta_secs();
        } else {
            // Decelerate when not thrusting
            flight_state.current_speed = (flight_state.current_speed
                - flight_settings.deceleration * time.delta_secs())
                .max(0.0);

            // Continue moving while slowing down (momentum)
            if flight_state.current_speed > 0.0 {
                let forward = get_horizontal_forward_direction(facing.actual);
                position.position += forward * flight_state.current_speed * time.delta_secs();
            }
        }
    }
}

/// Converts the facing angle to a horizontal forward direction vector.
///
/// The facing angle is stored as radians where:
/// - The transform rotation is applied as: `Quat::from_axis_angle(Vec3::Y, actual - PI/2)`
/// - This means we need to compute forward from the adjusted angle
///
/// Returns a normalized Vec3 with Y = 0 (horizontal movement only)
fn get_horizontal_forward_direction(facing_angle: f32) -> Vec3 {
    // The facing angle represents rotation around Y axis
    // Forward direction in XZ plane from angle:
    // X = cos(angle - PI/2) = sin(angle)
    // Z = sin(angle - PI/2) = -cos(angle)
    Vec3::new(facing_angle.sin(), 0.0, -facing_angle.cos()).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_horizontal_forward_direction_north() {
        // Facing north (positive Z) means angle = PI (180 degrees)
        // Transform rotation would be PI - PI/2 = PI/2 around Y
        let forward = get_horizontal_forward_direction(std::f32::consts::PI);
        assert!(forward.x.abs() < 0.001);
        assert!(forward.z > 0.9); // Should point positive Z
        assert!(forward.y.abs() < 0.001);
    }

    #[test]
    fn test_get_horizontal_forward_direction_east() {
        // Facing east (positive X) means angle = PI/2 (90 degrees)
        let forward = get_horizontal_forward_direction(std::f32::consts::PI / 2.0);
        assert!(forward.x > 0.9); // Should point positive X
        assert!(forward.z.abs() < 0.001);
        assert!(forward.y.abs() < 0.001);
    }

    #[test]
    fn test_get_horizontal_forward_direction_south() {
        // Facing south (negative Z) means angle = 0
        let forward = get_horizontal_forward_direction(0.0);
        assert!(forward.x.abs() < 0.001);
        assert!(forward.z < -0.9); // Should point negative Z
        assert!(forward.y.abs() < 0.001);
    }

    #[test]
    fn test_get_horizontal_forward_direction_west() {
        // Facing west (negative X) means angle = -PI/2 or 3*PI/2
        let forward = get_horizontal_forward_direction(-std::f32::consts::PI / 2.0);
        assert!(forward.x < -0.9); // Should point negative X
        assert!(forward.z.abs() < 0.001);
        assert!(forward.y.abs() < 0.001);
    }
}
