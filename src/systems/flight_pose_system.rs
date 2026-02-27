//! Flight Pose System
//!
//! This system applies visual-only flight poses to the character model when flying.
//! The pose includes:
//! - Forward lean (pitch) on the body - visual only, doesn't affect movement
//! - Toe-down rotation on the feet
//! - Ragdoll "hanging from wings" effect:
//!   - Body slightly lowered (simulating hanging from wings)
//!   - Arms dangling downward
//!   - Legs hanging naturally with slight knee bend
//!   - Head tilted up to look forward while hanging
//! - Pose only activates after the character is airborne (current_speed > 0.1)

use bevy::prelude::*;

use crate::components::{CharacterModel, CharacterModelPart, FacingDirection, FlightState, PlayerCharacter};

/// Forward lean angle for flight pose in radians (~17 degrees)
const FLIGHT_PITCH_ANGLE: f32 = 0.3;

/// Toe-down rotation angle for feet in radians (~30 degrees)
const TOE_DOWN_ANGLE: f32 = 0.52;

/// Speed at which the flight pose blends in/out (0.0 to 1.0 per second)
const POSE_BLEND_SPEED: f32 = 5.0;

/// Minimum speed threshold to consider character as "airborne" for pose activation
const AIRBORNE_SPEED_THRESHOLD: f32 = 0.1;

// ============================================
// Ragdoll Hanging Pose Constants
// ============================================

/// Body downward translation for hanging effect (in local units)
/// Simulates the body hanging from the wings attached at shoulder blades
const RAGDOLL_BODY_HANG_OFFSET: f32 = -0.08;

/// Arms dangling rotation angle in radians (~45 degrees down)
const RAGDOLL_ARMS_DANGLE_ANGLE: f32 = 0.785;

/// Legs hanging rotation at hips in radians (~15 degrees back)
const RAGDOLL_LEGS_HANG_ANGLE: f32 = 0.26;

/// Head tilt-up angle in radians (~10 degrees up)
/// Character looks forward while hanging from wings
const RAGDOLL_HEAD_TILT_ANGLE: f32 = 0.175;

/// System that applies a flight pose to the player character model when flying.
///
/// When flying and airborne:
/// - Applies a forward pitch rotation (lean forward) to visual model parts only
/// - Applies toe-down rotation to feet
/// - Applies ragdoll "hanging from wings" pose:
///   - Body lowered slightly (hanging effect)
///   - Arms dangling downward
///   - Legs hanging naturally
///   - Head tilted up to look forward
/// - Smoothly interpolates to the flight pose
///
/// When flight ends or character is not yet airborne:
/// - Smoothly returns to the original rotation and position
///
/// This system applies rotations to CharacterModel parts (Body, Feet, etc.) rather than
/// the player root transform. This ensures the flight pose is visual-only and doesn't
/// affect movement direction (which comes from FacingDirection and camera).
pub fn flight_pose_system(
    time: Res<Time>,
    player_query: Query<(&FlightState, &FacingDirection, &CharacterModel), With<PlayerCharacter>>,
    mut body_transforms: Query<&mut Transform, (With<CharacterModel>, Without<PlayerCharacter>)>,
) {
    let delta_time = time.delta_secs();

    for (flight_state, _facing_direction, character_model) in player_query.iter() {
        // Check if character is airborne (actually moving in flight)
        // Pose should only apply after lift-off, not just when flight mode is toggled
        let is_airborne = flight_state.is_flying && flight_state.current_speed > AIRBORNE_SPEED_THRESHOLD;

        if is_airborne {
            // Flying and airborne - blend towards full flight pose
            // Get current pose blend (we'll update it on FlightState in a separate query)
            let pose_blend = flight_state.pose_blend;
            let blend_factor = POSE_BLEND_SPEED * delta_time;
            
            // Calculate target pose blend
            let _target_blend = (pose_blend + POSE_BLEND_SPEED * delta_time).min(1.0);
            
            // ============================================
            // BODY: Forward lean + hanging effect
            // ============================================
            if let Some(body_entities) = get_model_part_entities(character_model, CharacterModelPart::Body) {
                for &body_entity in body_entities {
                    if let Ok(mut transform) = body_transforms.get_mut(body_entity) {
                        // Calculate the flight pitch rotation (forward lean)
                        let flight_pitch = Quat::from_rotation_x(-FLIGHT_PITCH_ANGLE);
                        
                        // Interpolate rotation
                        transform.rotation = transform.rotation.slerp(flight_pitch, blend_factor);
                        
                        // Apply hanging effect - lower body slightly
                        // This simulates the character hanging from their wings
                        let target_translation = Vec3::new(0.0, RAGDOLL_BODY_HANG_OFFSET, 0.0);
                        transform.translation = transform.translation.lerp(target_translation, blend_factor * 0.5);
                    }
                }
            }
            
            // ============================================
            // HANDS/ARMS: Dangling downward (ragdoll style)
            // ============================================
            if let Some(hands_entities) = get_model_part_entities(character_model, CharacterModelPart::Hands) {
                for &hands_entity in hands_entities {
                    if let Ok(mut transform) = body_transforms.get_mut(hands_entity) {
                        // Arms dangle down - rotate around X-axis to point downward
                        // Combined with forward lean for natural hanging pose
                        let arm_dangle = Quat::from_rotation_x(RAGDOLL_ARMS_DANGLE_ANGLE);
                        let flight_pitch = Quat::from_rotation_x(-FLIGHT_PITCH_ANGLE * 0.5);
                        let combined_rotation = flight_pitch * arm_dangle;
                        
                        transform.rotation = transform.rotation.slerp(combined_rotation, blend_factor);
                    }
                }
            }
            
            // ============================================
            // FEET/LEGS: Hanging naturally with toe-down
            // ============================================
            if let Some(feet_entities) = get_model_part_entities(character_model, CharacterModelPart::Feet) {
                for &feet_entity in feet_entities {
                    if let Ok(mut transform) = body_transforms.get_mut(feet_entity) {
                        // Legs hang with slight back angle (like dangling from a bar)
                        // Combined with toe-down rotation
                        let leg_hang = Quat::from_rotation_x(-RAGDOLL_LEGS_HANG_ANGLE);
                        let toe_down = Quat::from_rotation_x(TOE_DOWN_ANGLE);
                        let combined_rotation = leg_hang * toe_down;
                        
                        transform.rotation = transform.rotation.slerp(combined_rotation, blend_factor);
                    }
                }
            }
            
            // ============================================
            // HEAD: Tilt up to look forward while hanging
            // ============================================
            if let Some(head_entities) = get_model_part_entities(character_model, CharacterModelPart::Head) {
                for &head_entity in head_entities {
                    if let Ok(mut transform) = body_transforms.get_mut(head_entity) {
                        // Head tilts up to look forward while body hangs
                        let head_tilt = Quat::from_rotation_x(RAGDOLL_HEAD_TILT_ANGLE);
                        transform.rotation = transform.rotation.slerp(head_tilt, blend_factor);
                    }
                }
            }
        } else {
            // Not airborne - blend back to normal pose
            let pose_blend = flight_state.pose_blend;
            
            if pose_blend > 0.01 {
                let blend_factor = POSE_BLEND_SPEED * delta_time;
                
                // Blend back to identity rotation and zero translation for body parts
                let identity = Quat::IDENTITY;
                
                // Reset Body rotation and translation
                if let Some(body_entities) = get_model_part_entities(character_model, CharacterModelPart::Body) {
                    for &body_entity in body_entities {
                        if let Ok(mut transform) = body_transforms.get_mut(body_entity) {
                            transform.rotation = transform.rotation.slerp(identity, blend_factor);
                            // Reset translation to zero (remove hanging offset)
                            transform.translation = transform.translation.lerp(Vec3::ZERO, blend_factor * 0.5);
                        }
                    }
                }
                
                // Reset Hands rotation
                if let Some(hands_entities) = get_model_part_entities(character_model, CharacterModelPart::Hands) {
                    for &hands_entity in hands_entities {
                        if let Ok(mut transform) = body_transforms.get_mut(hands_entity) {
                            transform.rotation = transform.rotation.slerp(identity, blend_factor);
                        }
                    }
                }
                
                // Reset Feet rotation
                if let Some(feet_entities) = get_model_part_entities(character_model, CharacterModelPart::Feet) {
                    for &feet_entity in feet_entities {
                        if let Ok(mut transform) = body_transforms.get_mut(feet_entity) {
                            transform.rotation = transform.rotation.slerp(identity, blend_factor);
                        }
                    }
                }
                
                // Reset Head rotation
                if let Some(head_entities) = get_model_part_entities(character_model, CharacterModelPart::Head) {
                    for &head_entity in head_entities {
                        if let Ok(mut transform) = body_transforms.get_mut(head_entity) {
                            transform.rotation = transform.rotation.slerp(identity, blend_factor);
                        }
                    }
                }
            }
        }
    }
}

/// Helper function to get entities for a specific model part
fn get_model_part_entities(character_model: &CharacterModel, part: CharacterModelPart) -> Option<&Vec<Entity>> {
    let (_, entities) = &character_model.model_parts[part];
    if entities.is_empty() {
        None
    } else {
        Some(entities)
    }
}

/// System that updates the FlightState pose_blend value.
/// This runs separately to track the blend state on the FlightState component.
pub fn flight_pose_blend_update_system(
    time: Res<Time>,
    mut query: Query<&mut FlightState, With<PlayerCharacter>>,
) {
    let delta_time = time.delta_secs();
    
    for mut flight_state in query.iter_mut() {
        let is_airborne = flight_state.is_flying && flight_state.current_speed > AIRBORNE_SPEED_THRESHOLD;
        
        if is_airborne {
            // Increase pose blend towards 1.0
            flight_state.pose_blend = (flight_state.pose_blend + POSE_BLEND_SPEED * delta_time).min(1.0);
        } else {
            // Decrease pose blend towards 0.0
            flight_state.pose_blend = (flight_state.pose_blend - POSE_BLEND_SPEED * delta_time).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flight_pitch_angle() {
        // Verify the pitch angle is approximately 17 degrees
        let angle_degrees = FLIGHT_PITCH_ANGLE.to_degrees();
        assert!(angle_degrees > 15.0 && angle_degrees < 20.0);
    }

    #[test]
    fn test_toe_down_angle() {
        // Verify the toe-down angle is approximately 30 degrees
        let angle_degrees = TOE_DOWN_ANGLE.to_degrees();
        assert!(angle_degrees > 25.0 && angle_degrees < 35.0);
    }

    #[test]
    fn test_pose_blend_speed() {
        // Verify blend speed allows full transition in reasonable time
        // At 5.0 per second, full transition takes 0.2 seconds
        let full_transition_time = 1.0 / POSE_BLEND_SPEED;
        assert!(full_transition_time < 0.5); // Should be faster than 0.5 seconds
    }

    #[test]
    fn test_airborne_threshold() {
        // Verify airborne threshold is reasonable
        assert!(AIRBORNE_SPEED_THRESHOLD > 0.0);
        assert!(AIRBORNE_SPEED_THRESHOLD < 1.0);
    }
    
    // ============================================
    // Ragdoll Hanging Pose Tests
    // ============================================
    
    #[test]
    fn test_ragdoll_body_hang_offset() {
        // Body should hang down slightly (negative Y)
        assert!(RAGDOLL_BODY_HANG_OFFSET < 0.0);
        // But not too extreme
        assert!(RAGDOLL_BODY_HANG_OFFSET > -0.2);
    }
    
    #[test]
    fn test_ragdoll_arms_dangle_angle() {
        // Arms should dangle approximately 45 degrees down
        let angle_degrees = RAGDOLL_ARMS_DANGLE_ANGLE.to_degrees();
        assert!(angle_degrees > 40.0 && angle_degrees < 50.0);
    }
    
    #[test]
    fn test_ragdoll_legs_hang_angle() {
        // Legs should hang back approximately 15 degrees
        let angle_degrees = RAGDOLL_LEGS_HANG_ANGLE.to_degrees();
        assert!(angle_degrees > 10.0 && angle_degrees < 20.0);
    }
    
    #[test]
    fn test_ragdoll_head_tilt_angle() {
        // Head should tilt up approximately 10 degrees
        let angle_degrees = RAGDOLL_HEAD_TILT_ANGLE.to_degrees();
        assert!(angle_degrees > 5.0 && angle_degrees < 15.0);
    }
}
