use bevy::prelude::Component;

/// Component that stores the target bone index for a skinned mesh.
/// This is used when ZMS mesh files don't have joint weight/index data,
/// to ensure vertices follow the correct bone (the one the mesh is parented to).
#[derive(Component, Debug, Clone, Copy)]
pub struct SkinnedMeshTargetBone(pub u32);
