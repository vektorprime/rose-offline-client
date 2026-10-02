use bevy::{
    asset::LoadState,
    camera::visibility::DynamicSkinnedMeshBounds,
    mesh::{skinning::SkinnedMesh, Mesh3d},
    prelude::*,
};

use crate::components::SkinningTarget;

/// Plugin that provides deferred SkinnedMesh insertion for proper skinning support in Bevy 0.16.
///
/// This is REQUIRED for proper skinned mesh rendering.
/// Without this plugin, meshes that need skinning won't get the SkinnedMesh component,
/// causing bind group layout mismatches and rendering failures.
///
/// The problem this solves:
/// 1. At spawn time, we can't know if a mesh has joint attributes (async loading)
/// 2. If we insert SkinnedMesh but mesh lacks joint attributes, it causes bind group mismatch
/// 3. So we spawn with SkinningTarget marker instead
/// 4. This plugin's system waits for mesh to load, checks for joint attributes, and adds SkinnedMesh only if appropriate
pub struct SkinnedMeshFixPlugin;

impl Plugin for SkinnedMeshFixPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, add_skinned_mesh_to_skinning_targets);
    }
}

/// System that adds SkinnedMesh component to entities marked with SkinningTarget
/// IF their mesh has joint attributes (bone_indices/bone_weights).
fn add_skinned_mesh_to_skinning_targets(
    mut commands: Commands,
    query: Query<(Entity, &SkinningTarget, &Mesh3d), Without<SkinnedMesh>>,
    parent_query: Query<&SkinnedMesh>,
    meshes: Res<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
) {
    for (entity, skinning_target, mesh3d) in query.iter() {
        // Get the mesh first: a lock-free lookup that rules out every pending,
        // loading or failed mesh without touching the asset server. A failed
        // mesh keeps its marker on purpose: a later load() of the same path
        // retries it (Bevy re-requests Failed assets), and it must still be
        // skinned if that retry succeeds.
        let Some(mesh) = meshes.get(&mesh3d.0) else {
            continue;
        };

        // Check if mesh is loaded
        let load_state = asset_server.get_load_state(&mesh3d.0);
        if !matches!(load_state, Some(LoadState::Loaded)) {
            continue;
        }

        // Check if mesh has joint attributes
        let has_joint_indices = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some();
        let has_joint_weights = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some();

        if has_joint_indices && has_joint_weights {
            // Mesh has joint attributes - clone SkinnedMesh from parent
            if let Ok(parent_skinned_mesh) = parent_query.get(skinning_target.skinned_mesh_parent) {
                // DynamicSkinnedMeshBounds: Bevy recomputes the Aabb from the joints
                // every frame (from the ZMS loader's skinned mesh bounds), so frustum
                // and GPU occlusion culling follow the animated pose instead of the
                // bind pose. Collider sizing reads the bind-pose mesh bounds instead.
                commands
                    .entity(entity)
                    .insert((parent_skinned_mesh.clone(), DynamicSkinnedMeshBounds));
            } else {
                log::error!(
                    "[SKINNING_FIX] Cannot add SkinnedMesh to entity {:?} - parent {:?} has no SkinnedMesh component!",
                    entity,
                    skinning_target.skinned_mesh_parent
                );
            }
        }

        // Remove the marker regardless - we've processed this entity
        commands.entity(entity).remove::<SkinningTarget>();
    }
}
