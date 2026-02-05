use bevy::{
    math::Vec3,
    pbr::{ExtendedMaterial, StandardMaterial},
    prelude::{
        AssetServer, Assets, Changed, Commands, DespawnRecursiveExt, Entity, Query, Res, ResMut,
        Transform,
    },
    render::{
        alpha::AlphaMode,
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        storage::ShaderStorageBuffer,
    },
};

use crate::render::object_material_extension::RoseObjectExtension;
use enum_map::EnumMap;

use rose_game_common::components::Npc;

use crate::{
    components::{ClientEntityName, DummyBoneOffset, ModelHeight, NpcModel, RemoveColliderCommand},
    model_loader::ModelLoader,
    render::{ParticleMaterial, RoseEffectExtension},
    resources::GameData,
};

pub fn npc_model_update_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &Npc,
            &Transform,
            Option<&mut NpcModel>,
            Option<&mut SkinnedMesh>,
            Option<&mut DummyBoneOffset>,
        ),
        Changed<Npc>,
    >,
    asset_server: Res<AssetServer>,
    model_loader: Res<ModelLoader>,
    mut effect_mesh_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>>,
    mut particle_materials: ResMut<Assets<ParticleMaterial>>,
    mut standard_materials: ResMut<Assets<bevy::pbr::StandardMaterial>>,
    mut object_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, RoseObjectExtension>>>,
    mut skinned_mesh_inverse_bindposes_assets: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    mut meshes: ResMut<Assets<bevy::prelude::Mesh>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    game_data: Res<GameData>,
) {
    for (
        entity,
        npc,
        transform,
        mut current_npc_model,
        mut current_skinned_mesh,
        current_dummy_bone_offset,
    ) in query.iter_mut()
    {
        if let Some(previous_npc_model) = current_npc_model.as_mut() {
            if npc.id == previous_npc_model.npc_id {
                // NPC model has not changed
                continue;
            }

            // Despawn model parts
            for part_entity in previous_npc_model.model_parts.drain(..) {
                commands.entity(part_entity).despawn_recursive();
            }

            // Despawn model skeleton
            if let Some(current_skinned_mesh) = current_skinned_mesh.as_mut() {
                for bone_entity in current_skinned_mesh.joints.drain(..) {
                    commands.entity(bone_entity).despawn_recursive();
                }
            }

            // Remove the old model collider and height
            commands
                .entity(entity)
                .remove_and_despawn_collider()
                .remove::<ModelHeight>();
        }

        let (npc_model, skinned_mesh, dummy_bone_offset) =
            if let Some((npc_model, skinned_mesh, dummy_bone_offset)) = model_loader
                .spawn_npc_model(
                    &mut commands,
                    &asset_server,
                    &mut standard_materials,
                    &mut object_materials,
                    &mut skinned_mesh_inverse_bindposes_assets,
                    &mut particle_materials,
                    &mut effect_mesh_materials,
                    &mut meshes,
                    &mut storage_buffers,
                    entity,
                    npc.id,
                )
            {
                (npc_model, skinned_mesh, dummy_bone_offset)
            } else {
                // CRITICAL FIX: NPC model data not found - do NOT add SkinnedMesh component
                // This prevents bind group mismatch errors when NPC has no skeleton
                log::warn!(
                    "[SKINNED_MESH_FIX] NPC {} model data not found, spawning as non-skinned entity to prevent bind group mismatch",
                    npc.id.get()
                );
                // Insert empty model so we do not retry every frame.
                // Note: We do NOT insert SkinnedMesh here, only NpcModel
                let empty_npc_model = NpcModel {
                    npc_id: npc.id,
                    model_parts: Vec::new(),
                    action_motions: EnumMap::default(),
                    root_bone_position: Vec3::ZERO,
                };

                let mut entity_commands = commands.entity(entity);

                // Update scale
                if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                    entity_commands.insert(transform.with_scale(Vec3::new(
                        npc_data.scale,
                        npc_data.scale,
                        npc_data.scale,
                    )));
                }

                // Update ClientEntityName
                entity_commands.insert(ClientEntityName::new(
                    game_data
                        .npcs
                        .get_npc(npc.id)
                        .map(|npc_data| npc_data.name.to_string())
                        .unwrap_or_else(|| format!("??? [{}]", npc.id.get())),
                ));

                // Update model without SkinnedMesh
                if let Some(mut current_npc_model) = current_npc_model {
                    *current_npc_model = empty_npc_model;
                } else {
                    entity_commands.insert(empty_npc_model);
                }

                // Remove any existing SkinnedMesh and DummyBoneOffset components
                entity_commands
                    .remove::<SkinnedMesh>()
                    .remove::<DummyBoneOffset>();

                continue;
            };

        let mut entity_commands = commands.entity(entity);

        // Update scale
        if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
            entity_commands.insert(transform.with_scale(Vec3::new(
                npc_data.scale,
                npc_data.scale,
                npc_data.scale,
            )));
        }

        // Update ClientEntityName
        entity_commands.insert(ClientEntityName::new(
            game_data
                .npcs
                .get_npc(npc.id)
                .map(|npc_data| npc_data.name.to_string())
                .unwrap_or_else(|| format!("??? [{}]", npc.id.get())),
        ));

        // Update model
        if let Some(mut current_npc_model) = current_npc_model {
            *current_npc_model = npc_model;
        } else {
            entity_commands.insert(npc_model);
        }

        if let Some(mut current_skinned_mesh) = current_skinned_mesh {
            *current_skinned_mesh = skinned_mesh;
        } else {
            entity_commands.insert(skinned_mesh);
        }

        if let Some(mut current_dummy_bone_offset) = current_dummy_bone_offset {
            *current_dummy_bone_offset = dummy_bone_offset;
        } else {
            entity_commands.insert(dummy_bone_offset);
        }
    }
}
