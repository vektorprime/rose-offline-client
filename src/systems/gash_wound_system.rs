//! Gash wound visual systems for damaged entities.
//!
//! This module implements wound visuals that appear on entities when their HP
//! drops below a configurable threshold (default 50%).
//!
//! Wounds are rendered via the [`BloodOverlay`](crate::components::BloodOverlay) component
//! which tracks blood stains on entity models. Blood is painted onto overlay textures
//! and rendered via the [`RoseObjectExtension`](crate::render::RoseObjectExtension)
//! material extension, providing realistic blood appearance that deforms with
//! skeletal animation.
//!
//! ## UV Projection (Fix #2)
//!
//! For entities with a [`CharacterModel`](crate::components::CharacterModel) component,
//! the accurate [`project_world_to_uv()`](crate::systems::uv_projection::project_world_to_uv)
//! function is used to find the correct UV coordinates and material index via
//! triangle-ray intersection with skinned mesh vertex transformation.
//!
//! For entities without `CharacterModel` (e.g., monsters), the cylindrical
//! approximation [`world_pos_to_uv()`] is used as a fallback.

use bevy::{
    mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
    pbr::{ExtendedMaterial, MeshMaterial3d, StandardMaterial},
    prelude::*,
};

use rose_game_common::components::{AbilityValues, HealthPoints};

use crate::{
    components::{BloodOverlay, CharacterModel, Dead, GashWounds, ModelHeight, WoundVisual},
    events::BloodEffectEvent,
    resources::{BloodEffectConfig, BloodOverlayAtlas},
    systems::{
        damage_effects::normalize_or,
        uv_projection::{project_world_to_uv, ProjectionResult},
    },
};

/// Converts a world-space wound position to UV space using a cylindrical approximation.
///
/// This is a fallback for entities that don't have a [`CharacterModel`] component
/// (e.g., monsters). It approximates UV coordinates based on the wound position
/// relative to the entity's bounding box.
///
/// **Note:** This produces inaccurate UVs for ROSE model UV layouts (Root Cause #2).
/// When possible, [`project_world_to_uv()`] should be used instead.
fn world_pos_to_uv(wound_pos: Vec3, model_height: f32) -> Vec2 {
    let body_height = model_height.max(0.8);

    // Map Y position to UV Y (0 = feet, 1 = head)
    let uv_y = (wound_pos.y / body_height).clamp(0.0, 1.0);

    // Map X/Z position to UV X based on angle around the body
    let angle = (wound_pos.z.atan2(wound_pos.x) / std::f32::consts::TAU + 0.5).clamp(0.0, 1.0);

    Vec2::new(angle, uv_y)
}

fn random_local_wound_pose(model_height: f32) -> (Vec3, Vec3) {
    // Distribute wounds across the full body height for proper placement on the model.
    // Model height is the total character height; we place wounds at various heights
    // from the feet (y=0) to the top of the head (y=model_height).
    let body_height = model_height.max(0.8);
    let y = body_height * (0.15 + rand::random::<f32>() * 0.85); // 15%-100% of body height

    // Place wound on the surface of the character's body cylinder.
    // The radial distance from center determines how far from the body center the wound is.
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let radial = 0.15 + rand::random::<f32>() * 0.25; // 0.15-0.40 units from center
    let x = radial * angle.cos();
    let z = radial * angle.sin();

    // Normal points outward from the body center at this position
    let normal = normalize_or(Vec3::new(x, 0.0, z), Vec3::Z);

    (Vec3::new(x, y, z), normal)
}

/// System that monitors HP and shows/hides wounds based on health percentage.
///
/// When an entity's HP drops below the wound visibility threshold (default 50%),
/// blood stains are added to the [`BloodOverlay`] component.
pub fn wound_visibility_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            &HealthPoints,
            &AbilityValues,
            Option<&mut GashWounds>,
            Option<&ModelHeight>,
        ),
        Without<Dead>,
    >,
    mut blood_events: MessageWriter<BloodEffectEvent>,
    config: Res<BloodEffectConfig>,
) {
    if !config.enable_blood || !config.show_wounds {
        return;
    }

    for (entity, hp, ability_values, wounds, model_height) in query.iter_mut() {
        let max_hp = ability_values.get_max_health();
        if max_hp <= 0 {
            continue;
        }

        let health_percent = hp.hp as f32 / max_hp as f32;
        let should_show_wounds = health_percent < config.wound_visibility_threshold;

        if let Some(mut wounds) = wounds {
            if wounds.wounds_visible != should_show_wounds {
                wounds.wounds_visible = should_show_wounds;

                if should_show_wounds && wounds.wound_count < config.max_wounds_per_entity {
                    let target_seed_count = config.max_wounds_per_entity.min(3);
                    let to_add = target_seed_count.saturating_sub(wounds.wound_count);
                    let model_h = model_height.map_or(1.8, |h| h.height);
                    for _ in 0..to_add {
                        let (wound_pos, wound_normal) = random_local_wound_pose(model_h);
                        blood_events.write(BloodEffectEvent::show_wound(
                            entity,
                            wound_pos,
                            wound_normal,
                        ));
                        wounds.wound_count += 1;
                    }
                }
            }
        } else if should_show_wounds {
            // First time showing wounds - create component and blood overlay
            let mut wounds_component = GashWounds::new(entity);
            wounds_component.wounds_visible = true;
            commands.entity(entity).insert(BloodOverlay::new());
            let model_h = model_height.map_or(1.8, |h| h.height);
            let target_seed_count = config.max_wounds_per_entity.min(3).max(1);
            for _ in 0..target_seed_count {
                let (wound_pos, wound_normal) = random_local_wound_pose(model_h);
                blood_events.write(BloodEffectEvent::show_wound(
                    entity,
                    wound_pos,
                    wound_normal,
                ));
            }
            wounds_component.wound_count = target_seed_count;
            commands.entity(entity).insert(wounds_component);
        }
    }
}

/// System that processes wound-related blood effect events and adds blood stains
/// to the [`BloodOverlay`] component instead of spawning separate quad entities.
///
/// This handles:
/// - [`BloodEffectEvent::ShowWound`] - Adds blood stain to the entity's overlay
/// - [`BloodEffectEvent::UpdateWoundVisibility`] - Updates visibility based on HP
/// - [`BloodEffectEvent::CleanupWounds`] - Removes blood stains
///
/// ## UV Projection Strategy (Fix #2)
///
/// For entities with a [`CharacterModel`] component, this system uses
/// [`project_world_to_uv()`] to find accurate UV coordinates via triangle-ray
/// intersection with skinned mesh vertex transformation. The returned
/// `material_index` is used to identify which mesh entity was hit, enabling
/// per-material stain tracking (Fix #1).
///
/// For entities without `CharacterModel`, the cylindrical approximation
/// [`world_pos_to_uv()`] is used as a fallback.
pub fn wound_spawn_system(
    mut commands: Commands,
    mut blood_events: MessageReader<BloodEffectEvent>,
    mut query_targets: Query<(
        Option<&ModelHeight>,
        Option<&mut BloodOverlay>,
        Option<&mut GashWounds>,
        Option<&CharacterModel>,
        Option<&GlobalTransform>,
    )>,
    meshes: Res<Assets<Mesh>>,
    inverse_bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    transforms: Query<&GlobalTransform>,
    mesh_query: Query<&Mesh3d>,
    skinned_mesh_query: Query<&SkinnedMesh>,
    // Query to resolve material_index → mesh entity from CharacterModel
    query_materials: Query<(
        Entity,
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::render::RoseObjectExtension>>,
    )>,
    config: Res<BloodEffectConfig>,
    atlas: Res<BloodOverlayAtlas>,
) {
    if !config.enable_blood || !config.show_wounds {
        blood_events.clear();
        return;
    }

    for event in blood_events.read() {
        match event {
            BloodEffectEvent::ShowWound {
                entity,
                wound_position,
                wound_normal: _,
            } => {
                let Ok((
                    model_height,
                    mut overlay_opt,
                    _wounds_opt,
                    character_model_opt,
                    global_transform_opt,
                )) = query_targets.get_mut(*entity)
                else {
                    continue;
                };

                // Get model height for UV conversion fallback
                let model_h = model_height.map_or(1.8, |h| h.height);

                // Determine wound size and variant
                let wound_size = (config.wound_min_size
                    + rand::random::<f32>()
                        * (config.wound_max_size - config.wound_min_size).max(0.001))
                .max(0.16);

                let variant_count = atlas.blood_stains.len().max(1);
                let stain_variant = rand::random::<usize>() % variant_count;

                // Try accurate UV projection first (Fix #2)
                // For entities with CharacterModel, use project_world_to_uv() which
                // does proper triangle-ray intersection with skinned mesh vertex transformation.
                let projection_result = if let Some(character_model) = character_model_opt {
                    // Convert wound_position to world space if we have a global transform
                    let world_pos = if let Some(gt) = global_transform_opt {
                        gt.transform_point(*wound_position)
                    } else {
                        *wound_position
                    };

                    project_world_to_uv(
                        world_pos,
                        *entity,
                        &meshes,
                        &inverse_bindposes,
                        &transforms,
                        &mesh_query,
                        &skinned_mesh_query,
                        character_model,
                    )
                } else {
                    None
                };

                if let Some(overlay) = overlay_opt.as_deref_mut() {
                    if overlay.stain_count() < config.max_wounds_per_entity {
                        if let Some(proj) = projection_result {
                            // Accurate UV projection succeeded — add stain for the specific material
                            // Resolve material_index to the actual mesh entity
                            let material_entity = resolve_material_entity(
                                *entity,
                                proj.material_index,
                                character_model_opt,
                                &query_materials,
                            );

                            if let Some(mat_entity) = material_entity {
                                overlay.add_stain_for_material(
                                    proj.uv,
                                    wound_size,
                                    stain_variant,
                                    mat_entity,
                                );
                            } else {
                                // Fallback: add without material association
                                overlay.add_stain(proj.uv, wound_size, stain_variant);
                            }
                        } else {
                            // Fallback to cylindrical approximation for non-character entities
                            let uv_pos = world_pos_to_uv(*wound_position, model_h);
                            overlay.add_stain(uv_pos, wound_size, stain_variant);
                        }
                    }
                } else {
                    // Create new BloodOverlay with the first stain
                    let mut new_overlay = BloodOverlay::new();
                    if new_overlay.stain_count() < config.max_wounds_per_entity {
                        if let Some(proj) = projection_result {
                            let material_entity = resolve_material_entity(
                                *entity,
                                proj.material_index,
                                character_model_opt,
                                &query_materials,
                            );

                            if let Some(mat_entity) = material_entity {
                                new_overlay.add_stain_for_material(
                                    proj.uv,
                                    wound_size,
                                    stain_variant,
                                    mat_entity,
                                );
                            } else {
                                new_overlay.add_stain(proj.uv, wound_size, stain_variant);
                            }
                        } else {
                            let uv_pos = world_pos_to_uv(*wound_position, model_h);
                            new_overlay.add_stain(uv_pos, wound_size, stain_variant);
                        }
                    }
                    commands.entity(*entity).insert(new_overlay);
                }
            }
            BloodEffectEvent::UpdateWoundVisibility {
                entity,
                health_percent,
            } => {
                let should_show = *health_percent < config.wound_visibility_threshold;

                if let Ok((_, _, Some(mut wounds), _, _)) = query_targets.get_mut(*entity) {
                    wounds.wounds_visible = should_show;
                }
            }
            BloodEffectEvent::CleanupWounds { entity } => {
                if let Ok((_, overlay_opt, wounds_opt, _, _)) = query_targets.get_mut(*entity) {
                    if let Some(mut overlay) = overlay_opt {
                        overlay.clear_stains();
                    }
                    if let Some(mut wounds) = wounds_opt {
                        wounds.wounds_visible = false;
                        wounds.wound_count = 0;
                    }
                }
            }
            _ => {}
        }
    }
}

/// Resolves a `material_index` from [`ProjectionResult`] to the actual mesh entity
/// by looking up the [`CharacterModel`] part entities and finding the one with a material.
fn resolve_material_entity(
    parent_entity: Entity,
    material_index: usize,
    character_model_opt: Option<&CharacterModel>,
    query_materials: &Query<(
        Entity,
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, crate::render::RoseObjectExtension>>,
    )>,
) -> Option<Entity> {
    let character_model = character_model_opt?;

    // Iterate through model parts to find the one matching material_index
    for (part_enum, (_, part_entities)) in character_model.model_parts.iter() {
        if part_enum as usize == material_index {
            // Return the first mesh entity in this part that has a material
            for &mesh_entity in part_entities {
                if query_materials.get(mesh_entity).is_ok() {
                    return Some(mesh_entity);
                }
            }
        }
    }

    // If exact index match failed, try the first material entity we can find
    // among all part entities (fallback for mismatched indices)
    for (_, (_, part_entities)) in character_model.model_parts.iter() {
        for &mesh_entity in part_entities {
            if query_materials.get(mesh_entity).is_ok() {
                return Some(mesh_entity);
            }
        }
    }

    None
}

/// System that cleans up wound visuals when their parent entity despawns.
///
/// This prevents orphaned wound entities from remaining in the scene.
pub fn wound_cleanup_system(
    mut commands: Commands,
    query_wound_visuals: Query<(Entity, &WoundVisual)>,
    query_parents: Query<(), Without<Dead>>,
) {
    for (wound_entity, wound_visual) in query_wound_visuals.iter() {
        // If parent entity no longer exists, clean up the wound
        if query_parents.get(wound_visual.parent_entity).is_err() {
            commands.entity(wound_entity).despawn();
        }
    }
}

/// Plugin that registers all gash wound systems.
pub struct GashWoundPlugin;

impl Plugin for GashWoundPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PostUpdate,
            (
                wound_visibility_system,
                wound_spawn_system,
                wound_cleanup_system,
            )
                .chain(),
        );
    }
}
