use bevy::prelude::Component;

/// Marker component for mesh entities that should receive SkinnedMesh component
/// if their mesh has joint attributes (bone_indices/bone_weights).
/// 
/// In Bevy 0.16, we can't insert SkinnedMesh at spawn time because:
/// 1. Mesh loading is async - we don't know if the mesh has joint attributes
/// 2. If SkinnedMesh is inserted but mesh lacks joint attributes, it causes a bind group mismatch
///
/// Instead, we spawn with this marker and a system adds SkinnedMesh after mesh loads
/// if the mesh actually has joint attributes.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkinningTarget {
    /// The parent entity that has the SkinnedMesh component we should clone
    pub skinned_mesh_parent: bevy::prelude::Entity,
}
