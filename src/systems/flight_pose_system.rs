//! Flight Pose System
//!
//! This system applies visual-only flight poses to the character model when flying.
//! The pose includes:
//! - Forward lean (pitch) on the body - visual only, doesn't affect movement
//! - Toe-down rotation on the feet
//! - Ragdoll "hanging from wings" effect:
//!   - Body slightly lowered (simulating hanging from wings)
//!   - Arms dangling straight down instead of swinging back with the lean
//!   - Legs hanging slightly back
//!   - Head tilted up to look forward while hanging
//! - Pose only activates after the character is airborne (current_speed > 0.1)
//!
//! The pose is applied to the skeleton joints (`SkinnedMesh::joints`): the body,
//! arms and feet model parts are GPU-skinned, so their own `Transform` is ignored
//! when rendering and only the joints move them. The joints are written by the
//! skeletal animation every frame, so the pose is re-applied on top of the freshly
//! animated skeleton each frame and taken off again before the next animation pass:
//! - `flight_pose_blend_update_system` (Update) restores the un-posed joints and
//!   advances `FlightState::pose_blend`.
//! - `flight_pose_system` (PostUpdate, after `RoseAnimationSystem`, before
//!   `TransformSystems::Propagate`) applies the blended pose.

use bevy::{mesh::skinning::SkinnedMesh, prelude::*};

use crate::components::{FlightState, PlayerCharacter};

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

/// Body downward translation for hanging effect (in meters)
/// Simulates the body hanging from the wings attached at shoulder blades
const RAGDOLL_BODY_HANG_OFFSET: f32 = -0.08;

/// Arms pitch in radians. 0 = the arms keep their animated orientation, so they
/// dangle straight down instead of swinging back with the body's forward lean.
const RAGDOLL_ARMS_DANGLE_ANGLE: f32 = 0.0;

/// Legs hanging back at the hips in radians (~15 degrees back)
const RAGDOLL_LEGS_HANG_ANGLE: f32 = 0.26;

/// Head tilt-up angle in radians (~10 degrees up)
/// Character looks forward while hanging from wings
const RAGDOLL_HEAD_TILT_ANGLE: f32 = 0.175;

// ============================================
// Skeleton
// ============================================

/// Joint indices, identical in the male and female skeletons
/// (3DDATA/AVATAR/MALE.ZMD, FEMALE.ZMD). Pelvis and head match the original
/// client's BONE_IDX_PELVIS / BONE_IDX_HEAD.
const BONE_PELVIS: usize = 0;
const BONE_HEAD: usize = 4;
const BONE_LEFT_UPPER_ARM: usize = 6;
const BONE_RIGHT_UPPER_ARM: usize = 10;
const BONE_LEFT_THIGH: usize = 13;
const BONE_LEFT_FOOT: usize = 15;
const BONE_RIGHT_THIGH: usize = 17;
const BONE_RIGHT_FOOT: usize = 19;

/// Parent of each skeleton bone (dummy bones excluded). The pelvis is the root,
/// parented to the character entity.
const SKELETON_PARENTS: [usize; 21] = [
    0, 0, 1, 2, 3, 3, 5, 6, 7, 3, 9, 10, 11, 0, 13, 14, 15, 0, 17, 18, 19,
];
const SKELETON_BONE_COUNT: usize = SKELETON_PARENTS.len();

/// Posed bones and their pitch, ancestors first. Each pitch is a rotation about
/// the character's lateral axis (character space: +Y up, +Z forward, +X lateral)
/// relative to the bone's animated orientation. Positive pitches forward: the top
/// of the part moves forward and down, a downward limb swings back, toes go down.
const POSED_BONES: [(usize, f32); 8] = [
    (BONE_PELVIS, FLIGHT_PITCH_ANGLE),
    (BONE_HEAD, -RAGDOLL_HEAD_TILT_ANGLE),
    (BONE_LEFT_UPPER_ARM, RAGDOLL_ARMS_DANGLE_ANGLE),
    (BONE_RIGHT_UPPER_ARM, RAGDOLL_ARMS_DANGLE_ANGLE),
    (BONE_LEFT_THIGH, RAGDOLL_LEGS_HANG_ANGLE),
    (BONE_LEFT_FOOT, TOE_DOWN_ANGLE),
    (BONE_RIGHT_THIGH, RAGDOLL_LEGS_HANG_ANGLE),
    (BONE_RIGHT_FOOT, TOE_DOWN_ANGLE),
];

/// Joints posed by `flight_pose_system` this frame: (joint, un-posed, posed)
/// local transforms. Restored before the next skeletal animation pass so the
/// pose never accumulates (an animation that is finished or still loading does
/// not rewrite the joints) and never leaks into an animation blend (which starts
/// from the joints' current transforms).
#[derive(Component, Default)]
pub struct FlightPoseRestore {
    joints: Vec<(Entity, Transform, Transform)>,
}

/// Puts back the un-posed joint transforms, unless something else (the skeletal
/// animation, a model respawn) has written the joint since it was posed.
fn restore_unposed_joints(
    restore: &mut FlightPoseRestore,
    query_transform: &mut Query<&mut Transform>,
) {
    for (joint, unposed, posed) in restore.joints.drain(..) {
        if let Ok(mut transform) = query_transform.get_mut(joint) {
            if *transform == posed {
                *transform = unposed;
            }
        }
    }
}

/// Rotation of `bone` in character space (the character entity's local space),
/// composed from the joints' local rotations.
fn character_rotation(local_rotations: &[Quat; SKELETON_BONE_COUNT], bone: usize) -> Quat {
    let mut rotation = local_rotations[bone];
    let mut current = bone;
    while current != BONE_PELVIS {
        current = SKELETON_PARENTS[current];
        rotation = local_rotations[current] * rotation;
    }
    rotation
}

/// Local joint rotations with the flight pose blended in by `blend` (0..=1).
/// Each posed bone ends up pitched by its angle in character space relative to
/// its animated orientation; the other bones keep their local rotation and so
/// follow their nearest posed ancestor.
fn posed_joint_rotations(
    animated: &[Quat; SKELETON_BONE_COUNT],
    blend: f32,
) -> [Quat; SKELETON_BONE_COUNT] {
    let mut posed = *animated;
    for &(bone, pitch) in POSED_BONES.iter() {
        let target = Quat::from_rotation_x(pitch * blend) * character_rotation(animated, bone);
        let parent = if bone == BONE_PELVIS {
            Quat::IDENTITY
        } else {
            character_rotation(&posed, SKELETON_PARENTS[bone])
        };
        posed[bone] = (parent.inverse() * target).normalize();
    }
    posed
}

/// System that applies a flight pose to the player character's skeleton when flying.
///
/// When flying and airborne:
/// - Leans the body forward (pelvis) and lowers it slightly (hanging effect)
/// - Arms dangle straight down, legs hang slightly back, toes point down
/// - Head tilted up to look forward
/// - Blended in by `FlightState::pose_blend`
///
/// When flight ends or the character is not yet airborne the pose blends back out.
///
/// Only the skeleton joints are touched, never the character root transform, so
/// the pose is visual-only and doesn't affect movement direction (which comes from
/// FacingDirection and camera).
///
/// Must run in PostUpdate after `RoseAnimationSystem` (which rewrites the joints)
/// and before `TransformSystems::Propagate`.
pub fn flight_pose_system(
    mut commands: Commands,
    mut query_player: Query<
        (
            Entity,
            &FlightState,
            &SkinnedMesh,
            Option<&mut FlightPoseRestore>,
        ),
        With<PlayerCharacter>,
    >,
    mut query_transform: Query<&mut Transform>,
) {
    for (player_entity, flight_state, skinned_mesh, mut restore) in query_player.iter_mut() {
        // Normally already restored by flight_pose_blend_update_system; this only
        // matters if that system did not run since the last pose.
        if let Some(restore) = restore
            .as_mut()
            .filter(|restore| !restore.joints.is_empty())
        {
            restore_unposed_joints(restore, &mut query_transform);
        }

        let blend = flight_state.pose_blend;
        if blend <= 0.0 || skinned_mesh.joints.len() < SKELETON_BONE_COUNT {
            continue;
        }

        let mut animated = [Quat::IDENTITY; SKELETON_BONE_COUNT];
        let mut joints_found = true;
        for (rotation, joint) in animated.iter_mut().zip(skinned_mesh.joints.iter()) {
            match query_transform.get(*joint) {
                Ok(transform) => *rotation = transform.rotation,
                Err(_) => {
                    joints_found = false;
                    break;
                }
            }
        }
        if !joints_found {
            continue;
        }

        let posed = posed_joint_rotations(&animated, blend);

        let mut applied = Vec::with_capacity(POSED_BONES.len());
        for &(bone, _) in POSED_BONES.iter() {
            let joint = skinned_mesh.joints[bone];
            let Ok(mut transform) = query_transform.get_mut(joint) else {
                continue;
            };

            let unposed = *transform;
            transform.rotation = posed[bone];
            if bone == BONE_PELVIS {
                // The pelvis' parent is the character entity, so its translation
                // is already in character space (+Y up)
                transform.translation.y += RAGDOLL_BODY_HANG_OFFSET * blend;
            }
            applied.push((joint, unposed, *transform));
        }

        match restore {
            Some(mut restore) => restore.joints = applied,
            None => {
                commands
                    .entity(player_entity)
                    .insert(FlightPoseRestore { joints: applied });
            }
        }
    }
}

/// System that updates the FlightState pose_blend value.
/// This runs separately to track the blend state on the FlightState component.
///
/// It also takes last frame's pose off the skeleton (see `FlightPoseRestore`).
/// It runs in Update, before the PostUpdate skeletal animation pass, so the
/// animation always starts from the un-posed joints.
pub fn flight_pose_blend_update_system(
    time: Res<Time>,
    mut query: Query<(&mut FlightState, Option<&mut FlightPoseRestore>), With<PlayerCharacter>>,
    mut query_transform: Query<&mut Transform>,
) {
    let delta_time = time.delta_secs();

    for (mut flight_state, restore) in query.iter_mut() {
        if let Some(mut restore) = restore.filter(|restore| !restore.joints.is_empty()) {
            restore_unposed_joints(&mut restore, &mut query_transform);
        }

        let is_airborne =
            flight_state.is_flying && flight_state.current_speed > AIRBORNE_SPEED_THRESHOLD;

        if is_airborne {
            // Increase pose blend towards 1.0
            flight_state.pose_blend =
                (flight_state.pose_blend + POSE_BLEND_SPEED * delta_time).min(1.0);
        } else {
            // Decrease pose blend towards 0.0
            flight_state.pose_blend =
                (flight_state.pose_blend - POSE_BLEND_SPEED * delta_time).max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An arbitrary non-trivial animated skeleton pose.
    fn animated_pose() -> [Quat; SKELETON_BONE_COUNT] {
        let mut rotations = [Quat::IDENTITY; SKELETON_BONE_COUNT];
        for (index, rotation) in rotations.iter_mut().enumerate() {
            let i = index as f32;
            *rotation = Quat::from_euler(EulerRot::XYZ, 0.1 * i, 0.2 - 0.03 * i, -0.05 * i);
        }
        rotations
    }

    fn same_rotation(a: Quat, b: Quat) -> bool {
        a.abs_diff_eq(b, 1e-4) || a.abs_diff_eq(-b, 1e-4)
    }

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

    // ============================================
    // Skeleton Pose Tests
    // ============================================

    #[test]
    fn test_posed_bones_ancestors_first() {
        // Every posed bone's posed ancestors must be processed before it
        for (position, &(bone, _)) in POSED_BONES.iter().enumerate() {
            let mut current = bone;
            while current != BONE_PELVIS {
                current = SKELETON_PARENTS[current];
                if let Some(ancestor_position) =
                    POSED_BONES.iter().position(|&(posed, _)| posed == current)
                {
                    assert!(ancestor_position < position);
                }
            }
        }
    }

    #[test]
    fn test_zero_blend_keeps_animation() {
        let animated = animated_pose();
        let posed = posed_joint_rotations(&animated, 0.0);
        for bone in 0..SKELETON_BONE_COUNT {
            assert!(same_rotation(posed[bone], animated[bone]));
        }
    }

    #[test]
    fn test_posed_bones_pitch_in_character_space() {
        let animated = animated_pose();
        let posed = posed_joint_rotations(&animated, 1.0);
        for &(bone, pitch) in POSED_BONES.iter() {
            assert!(same_rotation(
                character_rotation(&posed, bone),
                Quat::from_rotation_x(pitch) * character_rotation(&animated, bone),
            ));
        }
    }

    #[test]
    fn test_unposed_bones_follow_posed_ancestor() {
        let animated = animated_pose();
        let posed = posed_joint_rotations(&animated, 1.0);
        // The calf is not posed: it follows the thigh
        let calf = 14;
        assert!(same_rotation(posed[calf], animated[calf]));
        assert!(same_rotation(
            character_rotation(&posed, calf),
            Quat::from_rotation_x(RAGDOLL_LEGS_HANG_ANGLE) * character_rotation(&animated, calf),
        ));
    }

    #[test]
    fn test_forward_pitch_tips_up_axis_forward() {
        // Character space is +Y up, +Z forward: a positive pitch leans forward
        let up = Quat::from_rotation_x(FLIGHT_PITCH_ANGLE) * Vec3::Y;
        assert!(up.z > 0.0);
        // and points the toes (+Z) down
        let toes = Quat::from_rotation_x(TOE_DOWN_ANGLE) * Vec3::Z;
        assert!(toes.y < 0.0);
    }
}
