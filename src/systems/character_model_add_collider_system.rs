use bevy::{
    ecs::query::QueryEntityError,
    math::{Quat, Vec3, Vec3A},
    prelude::{
        Assets, BuildChildren, Commands, Entity, GlobalTransform, Handle, Mesh, Mesh3d, Query, Res,
        Transform, With, Without,
    },
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        primitives::Aabb,
    },
};
use bevy_rapier3d::prelude::{Collider, CollisionGroups};
use log::info;

use crate::components::{
    CharacterModel, CharacterModelPart, ColliderEntity, ColliderParent, ModelHeight, PersonalStore,
    PlayerCharacter, COLLISION_FILTER_CLICKABLE, COLLISION_FILTER_INSPECTABLE,
    COLLISION_GROUP_CHARACTER, COLLISION_GROUP_PHYSICS_TOY, COLLISION_GROUP_PLAYER,
};

pub fn character_model_add_collider_system(
    mut commands: Commands,
    query_add_collider: Query<
        (
            Entity,
            &CharacterModel,
            &SkinnedMesh,
            Option<&PlayerCharacter>,
        ),
        (Without<ColliderEntity>, Without<PersonalStore>),
    >,
    query_aabb: Query<Option<&Aabb>, With<SkinnedMesh>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
) {
    // Add colliders to character models without one
    for (entity, character_model, skinned_mesh, player_character) in query_add_collider.iter() {
        info!("Found CharacterModel entity without collider: {:?}", entity);
        let mut min: Option<Vec3A> = None;
        let mut max: Option<Vec3A> = None;
        let mut all_parts_loaded = true;
        let mut parts_found = 0;
        let mut aabb_data_count = 0;

        // Collect the AABB of Body, Hands, Feet
        for part_entity in character_model.model_parts[CharacterModelPart::Body]
            .1
            .iter()
            .chain(
                character_model.model_parts[CharacterModelPart::Hands]
                    .1
                    .iter(),
            )
            .chain(
                character_model.model_parts[CharacterModelPart::Feet]
                    .1
                    .iter(),
            )
        {
            parts_found += 1;
            match query_aabb.get(*part_entity) {
                Ok(Some(aabb)) => {
                    aabb_data_count += 1;
                    min = Some(min.map_or_else(|| aabb.min(), |min| min.min(aabb.min())));
                    max = Some(max.map_or_else(|| aabb.max(), |max| max.max(aabb.max())));
                }
                Ok(None) | Err(QueryEntityError::NoSuchEntity(_)) => {
                    all_parts_loaded = false;
                    info!("AABB query failed for part entity {:?}: no AABB or no such entity", part_entity);
                    break;
                }
                _ => {}
            }
        }

        info!("Collected {} AABB data points from {} model parts for entity {:?}", aabb_data_count, parts_found, entity);

        let inverse_bindpose = inverse_bindposes.get(&skinned_mesh.inverse_bindposes);
        if min.is_none() || max.is_none() || !all_parts_loaded || inverse_bindpose.is_none() {
            info!("Skipping collider creation for entity {:?}: min={:?}, max={:?}, all_parts_loaded={}, inverse_bindpose={:?}",
                entity, min, max, all_parts_loaded, inverse_bindpose.is_some());
            // Try again next frame
            continue;
        }
        let min = Vec3::from(min.unwrap());
        let max = Vec3::from(max.unwrap());
        let root_bone_inverse_bindpose = Transform::from_matrix(inverse_bindpose.unwrap()[0]);

        let local_bound_center = 0.5 * (min + max);
        let half_extents = 0.5 * (max - min);
        let root_bone_offset = root_bone_inverse_bindpose.transform_point(local_bound_center);

        let collider_entity = commands
            .spawn((
                Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
                ColliderParent::new(entity),
                CollisionGroups::new(
                    if player_character.is_some() {
                        COLLISION_GROUP_PLAYER
                    } else {
                        COLLISION_GROUP_CHARACTER
                    },
                    COLLISION_FILTER_INSPECTABLE
                        | COLLISION_FILTER_CLICKABLE
                        | COLLISION_GROUP_PHYSICS_TOY,
                ),
                Transform::from_translation(root_bone_offset)
                    .with_rotation(Quat::from_axis_angle(Vec3::Z, std::f32::consts::PI / 2.0)),
                GlobalTransform::default(),
            ))
            .id();

        commands
            .entity(skinned_mesh.joints[0])
            .add_child(collider_entity);

        commands.entity(entity).insert((
            ColliderEntity::new(collider_entity),
            ModelHeight::new(0.65 + half_extents.y * 2.0),
        ));

        info!("Created collider for entity {:?}: half_extents=({:.2}, {:.2}, {:.2}), root_bone_offset=({:.2}, {:.2}, {:.2})",
            entity, half_extents.x, half_extents.y, half_extents.z, root_bone_offset.x, root_bone_offset.y, root_bone_offset.z);
    }
}
