use bevy::prelude::*;
use bevy::transform::TransformSystem;

/// Diagnostic system to check if transform propagation is running
/// This helps diagnose why GlobalTransform is not being computed from Transform
pub fn transform_propagation_diagnostics(
    transforms: Query<(Entity, &Transform, &GlobalTransform, Option<&ChildOf>, Option<&Name>)>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only log every 60 frames to avoid spam
    if *frame_count % 60 != 0 {
        return;
    }

    info!("========================================");
    info!("[TRANSFORM PROPAGATION DIAGNOSTICS] Frame {}", *frame_count);
    info!("========================================");

    let total_entities = transforms.iter().count();
    info!("[TRANSFORM PROPAGATION] Total entities with Transform and GlobalTransform: {}", total_entities);

    let mut matching_count = 0;
    let mut mismatched_count = 0;
    let mut logged = 0;

    for (entity, local_transform, global_transform, parent, name) in transforms.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("<unnamed>");
        let local_pos = local_transform.translation;
        let global_pos = global_transform.translation();

        // For root entities, local and global should match.
        // For children, we'd need to check against parent, but for now let's just check if Global is NOT identity if Local is NOT identity.
        let is_root = parent.is_none();
        let positions_match = if is_root {
            (local_pos - global_pos).length() < 0.01
        } else {
            // For children, we just check if GlobalTransform is being updated at all (not just identity)
            // This is a weak check but better than assuming it should match Local.
            global_pos.length() > 0.001 || local_pos.length() < 0.001
        };

        if logged < 10 {
            info!("[TRANSFORM PROPAGATION] Entity {:?} ('{}') [Root: {}]:", entity, name_str, is_root);
            info!("[TRANSFORM PROPAGATION]   Local Transform: ({:.2}, {:.2}, {:.2})",
                local_pos.x, local_pos.y, local_pos.z);
            info!("[TRANSFORM PROPAGATION]   Global Transform: ({:.2}, {:.2}, {:.2})",
                global_pos.x, global_pos.y, global_pos.z);
            info!("[TRANSFORM PROPAGATION]   Valid propagation: {}", positions_match);
            
            logged += 1;
        }

        if positions_match {
            matching_count += 1;
        } else {
            mismatched_count += 1;
            warn!("[TRANSFORM PROPAGATION]   WARNING: Local and Global transforms do NOT match!");
            warn!("[TRANSFORM PROPAGATION]   This indicates transform propagation is NOT running!");
        }

        logged += 1;
    }

    info!("[TRANSFORM PROPAGATION] Summary:");
    info!("[TRANSFORM PROPAGATION]   Matching transforms: {}", matching_count);
    info!("[TRANSFORM PROPAGATION]   Mismatched transforms: {}", mismatched_count);

    if mismatched_count > 0 && total_entities > 0 {
        error!("[TRANSFORM PROPAGATION] CRITICAL: Transform propagation is NOT working!");
        error!("[TRANSFORM PROPAGATION]   Bevy's TransformSystem::TransformPropagate is not computing GlobalTransform from Transform!");
        error!("[TRANSFORM PROPAGATION]   Possible causes:");
        error!("[TRANSFORM PROPAGATION]     1. System ordering conflicts preventing TransformSystem::TransformPropagate from running");
        error!("[TRANSFORM PROPAGATION]     2. TransformSystem::TransformPropagate is disabled or removed");
        error!("[TRANSFORM PROPAGATION]     3. Entities are missing Transform or GlobalTransform components");
        error!("[TRANSFORM PROPAGATION]   Check system ordering in PostUpdate schedule");
    } else if matching_count == total_entities && total_entities > 0 {
        info!("[TRANSFORM PROPAGATION] ✓ Transform propagation is working correctly!");
    }

    info!("========================================");
}

/// System to log all PostUpdate systems that are registered
/// This helps identify if TransformSystem::TransformPropagate is in the schedule
pub fn post_update_systems_diagnostics(
    // This is a placeholder - in a real implementation we'd need to access the schedule
    // For now, we'll just log that this diagnostic is running
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    // Only log once at startup
    if *frame_count == 1 {
        info!("========================================");
        info!("[POST UPDATE SYSTEMS DIAGNOSTICS] PostUpdate schedule check");
        info!("========================================");
        info!("[POST UPDATE SYSTEMS] This diagnostic is running in PostUpdate schedule");
        info!("[POST UPDATE SYSTEMS] Bevy's TransformSystem::TransformPropagate should run in PostUpdate");
        info!("[POST UPDATE SYSTEMS] If transform propagation is not working, check:");
        info!("[POST UPDATE SYSTEMS]   1. Is TransformPlugin included in DefaultPlugins?");
        info!("[POST UPDATE SYSTEMS]   2. Are custom systems blocking TransformSystem::TransformPropagate?");
        info!("[POST UPDATE SYSTEMS]   3. Is there explicit system ordering that prevents propagation?");
        info!("========================================");
    }
}
