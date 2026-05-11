use bevy::prelude::*;

use crate::components::{FacingDirection, FlightState, NextCommand, PlayerCharacter, Position};
use crate::resources::{CurrentZone, FlightSettings, GameConnection};
use crate::systems::OrbitCamera;
use crate::zone_loader::ZoneLoaderAsset;
use rose_game_common::messages::client::ClientMessage;

/// Server-authoritative flight movement system.
/// 
/// The character flies forward in the direction the camera is facing,
/// including vertical movement (up/down based on camera pitch).
///
/// Coordinate systems:
/// - Position (game units, centimeters): x=right, y=forward, z=up
/// - Transform/World (meters): x=right, y=up, z=back
///
/// This system:
/// - Checks if Space bar is held when the player is in flight mode
/// - Gets the camera's view direction to determine flight direction
/// - Calculates flight movement locally for smooth visual feedback
/// - Sends movement intent to server via NextCommand and MoveCollision
/// - Does NOT mutate Position directly (server-authoritative)
/// - Updates FacingDirection component locally for responsive rotation
pub fn flight_movement_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    flight_settings: Res<FlightSettings>,
    time: Res<Time>,
    camera_query: Query<&Transform, With<OrbitCamera>>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    mut commands: Commands,
    game_connection: Option<Res<GameConnection>>,
    mut query: Query<(Entity, &mut FlightState, &mut FacingDirection, &Position), With<PlayerCharacter>>,
) {
    // Minimum height above terrain (in cm)
    let min_height_above_terrain = 100.0; // 1 meter above terrain
    
    // Get terrain height function
    let get_terrain_height = |x: f32, y: f32| -> f32 {
        if let Some(current_zone) = current_zone.as_ref() {
            if let Some(zone_data) = zone_loader_assets.get(&current_zone.handle) {
                return zone_data.get_terrain_height(x, y);
            }
        }
        0.0
    };
    
    for (entity, mut flight_state, mut facing, position) in query.iter_mut() {
        // Only process if flying
        if !flight_state.is_flying {
            continue;
        }

        // Ensure player stays above terrain (local check for visual smoothness)
        let terrain_height = get_terrain_height(position.x, position.y);
        let min_z = terrain_height + min_height_above_terrain;
        
        // Check if Space bar is held
        let is_thrusting = keyboard.pressed(KeyCode::Space);
        flight_state.is_thrusting = is_thrusting;

        if is_thrusting {
            // Accelerate
            flight_state.current_speed = (flight_state.current_speed
                + flight_settings.acceleration * time.delta_secs())
                .min(flight_settings.max_speed);

            // Get camera's view direction from its transform
            let (forward, horizontal_forward) = if let Ok(camera_transform) = camera_query.single() {
                let camera_forward = camera_transform.forward();
                
                // Convert from world space to position space
                let position_x = camera_forward.x;
                let position_y = -camera_forward.z;
                let position_z = camera_forward.y;
                
                // Add 15% upward bias to make flight angle point more upward
                let upward_bias = 0.15;
                let adjusted_z = position_z + upward_bias;
                
                let forward = Vec3::new(position_x, position_y, adjusted_z).normalize();
                
                // Calculate horizontal direction for facing (without vertical component)
                let horizontal = Vec3::new(position_x, position_y, 0.0).normalize_or_zero();
                
                (forward, horizontal)
            } else {
                let forward = get_horizontal_forward_direction(facing.actual);
                (forward, forward)
            };

            // Store the flight direction for momentum when stopping
            flight_state.last_flight_direction = forward;

            // Calculate potential new position
            let movement = forward * flight_state.current_speed * time.delta_secs() * 100.0;
            let new_x = position.x + movement.x;
            let new_y = position.y + movement.y;
            let new_z = (position.z + movement.z).max(min_z);
            
            // Update facing direction to match horizontal component of movement
            if horizontal_forward.x.abs() > 0.01 || horizontal_forward.y.abs() > 0.01 {
                facing.desired = horizontal_forward.y.atan2(horizontal_forward.x) + std::f32::consts::PI;
            }

            // Send movement intent to server
            // The server will validate and update Position accordingly
            let intended_position = Vec3::new(new_x, new_y, new_z);
            
            commands.entity(entity).insert(NextCommand::with_move(intended_position, None, None));
            
            if let Some(game_connection) = game_connection.as_ref() {
                game_connection
                    .client_message_tx
                    .send(ClientMessage::MoveCollision {
                        position: intended_position,
                    })
                    .ok();
            }

        } else {
            // Not thrusting - decelerate and hover in place (no descent)
            flight_state.current_speed = (flight_state.current_speed
                - flight_settings.deceleration * time.delta_secs())
                .max(0.0);

            // Continue moving in the last flight direction while slowing down (momentum)
            if flight_state.current_speed > 0.0 {
                let forward = flight_state.last_flight_direction;
                let movement = forward * flight_state.current_speed * time.delta_secs() * 100.0;
                
                // Calculate potential new position
                let new_x = position.x + movement.x;
                let new_y = position.y + movement.y;
                let new_z = (position.z + movement.z).max(min_z);
                
                // Send movement intent to server
                let intended_position = Vec3::new(new_x, new_y, new_z);
                
                commands.entity(entity).insert(NextCommand::with_move(intended_position, None, None));
                
                if let Some(game_connection) = game_connection.as_ref() {
                    game_connection
                        .client_message_tx
                        .send(ClientMessage::MoveCollision {
                            position: intended_position,
                        })
                        .ok();
                }
            }
            
            // No descent - player hovers in place when not thrusting
        }
    }
}

/// Converts the facing angle to a horizontal forward direction vector in position space.
///
/// The facing angle is stored as radians in the XY plane (position space).
/// Returns a normalized Vec3 with Z = 0 (horizontal movement only)
fn get_horizontal_forward_direction(faced_angle: f32) -> Vec3 {
    // In position space: x=right, y=forward
    // Forward direction from angle:
    // x = sin(angle)
    // y = cos(angle)
    Vec3::new(faced_angle.sin(), faced_angle.cos(), 0.0).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_horizontal_forward_direction_north() {
        // Facing north (positive Y in position space) means angle = 0
        let forward = get_horizontal_forward_direction(0.0);
        assert!(forward.x.abs() < 0.001);
        assert!(forward.y > 0.9); // Should point positive Y (forward)
        assert!(forward.z.abs() < 0.001);
    }

    #[test]
    fn test_get_horizontal_forward_direction_east() {
        // Facing east (positive X) means angle = PI/2 (90 degrees)
        let forward = get_horizontal_forward_direction(std::f32::consts::PI / 2.0);
        assert!(forward.x > 0.9); // Should point positive X
        assert!(forward.y.abs() < 0.001);
        assert!(forward.z.abs() < 0.001);
    }

    #[test]
    fn test_get_horizontal_forward_direction_south() {
        // Facing south (negative Y) means angle = PI (180 degrees)
        let forward = get_horizontal_forward_direction(std::f32::consts::PI);
        assert!(forward.x.abs() < 0.001);
        assert!(forward.y < -0.9); // Should point negative Y
        assert!(forward.z.abs() < 0.001);
    }

    #[test]
    fn test_get_horizontal_forward_direction_west() {
        // Facing west (negative X) means angle = -PI/2 or 3*PI/2
        let forward = get_horizontal_forward_direction(-std::f32::consts::PI / 2.0);
        assert!(forward.x < -0.9); // Should point negative X
        assert!(forward.y.abs() < 0.001);
        assert!(forward.z.abs() < 0.001);
    }
}
