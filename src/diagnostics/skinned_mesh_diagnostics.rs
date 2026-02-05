use bevy::prelude::*;
use bevy::render::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};

pub struct SkinnedMeshDiagnosticsPlugin;

impl Plugin for SkinnedMeshDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (
            log_skinned_mesh_components,
            check_skinned_mesh_validity,
            check_mesh_vertex_attributes,
        ));
    }
}

fn log_skinned_mesh_components(
    query: Query<(Entity, Option<&Name>, Has<SkinnedMesh>, Has<Mesh3d>)>,
) {
    for (entity, name, has_skinned, has_mesh) in query.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        debug!(
            "Entity {:?} '{}': has_skinned_mesh={}, has_mesh={}",
            entity, name_str, has_skinned, has_mesh
        );
    }
}

fn check_skinned_mesh_validity(
    query: Query<(Entity, Option<&Name>, &SkinnedMesh)>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    for (entity, name, skinned_mesh) in query.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        // Check if inverse bindposes asset exists
        if inverse_bindposes.get(&skinned_mesh.inverse_bindposes).is_none() {
            error!(
                "Entity {:?} '{}' has SkinnedMesh but inverse_bindposes asset is not loaded!",
                entity, name_str
            );
        }
        
        // Check if joints are present
        if skinned_mesh.joints.is_empty() {
            error!(
                "Entity {:?} '{}' has SkinnedMesh with no joints!",
                entity, name_str
            );
        } else {
            debug!(
                "Entity {:?} '{}' has {} joints",
                entity, name_str, skinned_mesh.joints.len()
            );
        }
    }
}

fn check_mesh_vertex_attributes(
    query: Query<(Entity, Option<&Name>, &Mesh3d, Has<SkinnedMesh>)>,
    meshes: Res<Assets<Mesh>>,
) {
    for (entity, name, mesh3d, has_skinned) in query.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        if let Some(mesh) = meshes.get(&mesh3d.0) {
            let has_joint_indices = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some();
            let has_joint_weights = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some();
            
            // Check for mismatches
            if has_skinned && (!has_joint_indices || !has_joint_weights) {
                error!(
                    "MISMATCH: Entity {:?} '{}' has SkinnedMesh component but mesh lacks joint data! \
                    (joint_indices={}, joint_weights={})",
                    entity, name_str, has_joint_indices, has_joint_weights
                );
            }
            
            if !has_skinned && (has_joint_indices || has_joint_weights) {
                error!(
                    "MISMATCH: Entity {:?} '{}' has joint data but no SkinnedMesh component! \
                    (joint_indices={}, joint_weights={})",
                    entity, name_str, has_joint_indices, has_joint_weights
                );
            }
            
            debug!(
                "Entity {:?} '{}': has_skinned={}, has_joint_indices={}, has_joint_weights={}",
                entity, name_str, has_skinned, has_joint_indices, has_joint_weights
            );
        }
    }
}
