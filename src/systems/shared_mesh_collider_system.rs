//! Trimesh colliders shared between all entities that use the same mesh.

use std::collections::HashMap;

use bevy::{
    asset::{AssetId, AssetServer, Assets, LoadState},
    mesh::Mesh,
    prelude::{Commands, Component, Entity, Mesh3d, Query, Res, ResMut, Resource},
};
use bevy_rapier3d::prelude::{Collider, ComputedColliderShape, TriMeshFlags};

/// Requests a trimesh collider built from the entity's `Mesh3d` once the mesh loads.
///
/// Replaces bevy_rapier's `AsyncCollider(ComputedColliderShape::TriMesh(flags))`
/// for zone objects. That builds a full trimesh (vertex merge, topology,
/// pseudo-normals, BVH) separately for every entity, while a zone has thousands of
/// object parts but only hundreds of distinct meshes. Here the unscaled shape is
/// built once per (mesh, flags) and shared: rapier shapes are reference-counted, and
/// bevy_rapier still applies each entity's own scale when it creates the collider,
/// exactly as before.
#[derive(Component, Clone, Copy, Debug)]
pub struct SharedMeshCollider(pub TriMeshFlags);

/// Unscaled colliders built so far, keyed by mesh asset and trimesh flags.
#[derive(Resource, Default)]
pub struct MeshColliderCache {
    colliders: HashMap<(AssetId<Mesh>, TriMeshFlags), Collider>,
}

/// Turns [`SharedMeshCollider`] requests into `Collider`s. Runs before
/// `PhysicsSet::SyncBackend`, where bevy_rapier picks the new colliders up the same
/// frame (the slot `AsyncCollider` was processed in).
pub fn shared_mesh_collider_system(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<MeshColliderCache>,
    query: Query<(Entity, &Mesh3d, &SharedMeshCollider)>,
) {
    if query.is_empty() {
        return;
    }

    // Drop shapes whose mesh is gone (e.g. the previous zone's).
    cache
        .colliders
        .retain(|(mesh_id, _), _| meshes.contains(*mesh_id));

    for (entity, mesh_3d, request) in query.iter() {
        let key = (mesh_3d.id(), request.0);
        let collider = if let Some(collider) = cache.colliders.get(&key) {
            collider.clone()
        } else if let Some(mesh) = meshes.get(&mesh_3d.0) {
            match Collider::from_bevy_mesh(mesh, &ComputedColliderShape::TriMesh(request.0)) {
                Some(collider) => {
                    cache.colliders.insert(key, collider.clone());
                    collider
                }
                None => {
                    // AsyncCollider logged this and retried every frame forever.
                    log::error!(
                        "[COLLIDER] Unable to generate trimesh collider from mesh {:?}",
                        mesh_3d.id()
                    );
                    commands.entity(entity).remove::<SharedMeshCollider>();
                    continue;
                }
            }
        } else {
            // Not loaded yet; stop waiting if it never will be.
            if matches!(
                asset_server.get_load_state(mesh_3d.id()),
                Some(LoadState::Failed(_))
            ) {
                commands.entity(entity).remove::<SharedMeshCollider>();
            }
            continue;
        };

        commands
            .entity(entity)
            .insert(collider)
            .remove::<SharedMeshCollider>();
    }
}
