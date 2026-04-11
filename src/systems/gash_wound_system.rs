//! Gash wound visual systems for damaged entities.
//!
//! This module implements wound visuals that appear on entities when their HP
//! drops below a configurable threshold (default 50%).

use bevy::prelude::*;
use bevy::math::primitives::Rectangle;
use bevy_mesh::skinning::SkinnedMesh;

use rose_game_common::components::{AbilityValues, HealthPoints};

use crate::{
    components::{Dead, GashWounds, ModelHeight, WoundVisual},
    events::BloodEffectEvent,
    resources::{BloodDecalAtlas, BloodEffectConfig, BloodEffectDiagnostics},
};

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let len_sq = value.length_squared();
    if len_sq > 1e-6 {
        value / len_sq.sqrt()
    } else {
        fallback
    }
}

fn tangent_from_normal(normal: Vec3) -> Vec3 {
    let up = if normal.y.abs() > 0.95 { Vec3::X } else { Vec3::Y };
    normalize_or(normal.cross(up), Vec3::X)
}

fn random_local_wound_pose(_model_height: f32) -> (Vec3, Vec3) {
    let y = -0.04 + rand::random::<f32>() * 0.18;
    let angle = rand::random::<f32>() * std::f32::consts::TAU;
    let radial = 0.05 + rand::random::<f32>() * 0.16;
    let x = radial * angle.cos();
    let z = radial * angle.sin();
    let normal = normalize_or(Vec3::new(x, 0.05, z), Vec3::Z);
    (Vec3::new(x, y, z), normal)
}

/// System that monitors HP and shows/hides wounds based on health percentage.
///
/// When an entity's HP drops below the wound visibility threshold (default 50%),
/// wounds are shown. Wounds remain visible until the entity despawns.
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
            // First time showing wounds - create component
            commands.entity(entity).insert(GashWounds::new(entity));
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
        }
    }
}

/// System that processes wound-related blood effect events.
///
/// This handles:
/// - [`BloodEffectEvent::ShowWound`] - Creates wound visual entities
/// - [`BloodEffectEvent::UpdateWoundVisibility`] - Updates visibility based on HP
/// - [`BloodEffectEvent::CleanupWounds`] - Removes wound visuals
pub fn wound_spawn_system(
    mut commands: Commands,
    mut blood_events: MessageReader<BloodEffectEvent>,
    query_wounds: Query<(Entity, &WoundVisual)>,
    query_skinned_mesh: Query<&SkinnedMesh>,
    query_model_height: Query<&ModelHeight>,
    config: Res<BloodEffectConfig>,
    atlas: Res<BloodDecalAtlas>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut diagnostics: ResMut<BloodEffectDiagnostics>,
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
                wound_normal,
            } => {
                let existing_count = query_wounds
                    .iter()
                    .filter(|(_, wound)| wound.parent_entity == *entity)
                    .count();
                if existing_count >= config.max_wounds_per_entity {
                    continue;
                }

                let wound_size = (config.wound_min_size
                    + rand::random::<f32>() * (config.wound_max_size - config.wound_min_size).max(0.001))
                    .max(0.16);

                let wound_texture = if atlas.wound_textures.is_empty() {
                    None
                } else {
                    let idx = rand::random::<usize>() % atlas.wound_textures.len();
                    atlas.wound_textures.get(idx).cloned()
                };

                let normal = normalize_or(*wound_normal, Vec3::Z);
                let tangent = tangent_from_normal(normal);
                let orientation = Quat::from_mat3(&Mat3::from_cols(tangent, normal.cross(tangent), normal));

                let mut attach_entity = *entity;
                let mut local_wound_position = *wound_position;
                if let Ok(skinned_mesh) = query_skinned_mesh.get(*entity) {
                    if !skinned_mesh.joints.is_empty() {
                        let len = skinned_mesh.joints.len();
                        let min_index = (len / 4).min(len.saturating_sub(1));
                        let max_index_exclusive = ((len * 9) / 10).max(min_index + 1).min(len);
                        let range_len = max_index_exclusive.saturating_sub(min_index);
                        let chosen_index = if range_len > 0 {
                            min_index + (rand::random::<usize>() % range_len)
                        } else {
                            len / 2
                        };

                        attach_entity = skinned_mesh.joints[chosen_index];
                        local_wound_position *= 0.22;
                    }
                } else if let Ok(model_height) = query_model_height.get(*entity) {
                    let h = model_height.height.max(0.8);
                    local_wound_position.y = h * (0.35 + rand::random::<f32>() * 0.35);
                }

                let mesh = meshes.add(Mesh::from(Rectangle::new(1.0, 1.0)));
                let material = materials.add(StandardMaterial {
                    base_color_texture: wound_texture,
                    base_color: config.blood_color.with_alpha(0.92),
                    alpha_mode: AlphaMode::Blend,
                    cull_mode: None,
                    unlit: true,
                    emissive: Color::srgb(0.08, 0.0, 0.0).into(),
                    ..default()
                });

                let wound_entity = commands
                    .spawn((
                        Name::new("WoundVisual"),
                        WoundVisual::new(*entity, wound_size),
                        Mesh3d(mesh),
                        MeshMaterial3d(material),
                        Transform::from_translation(local_wound_position + normal * 0.045)
                            .with_rotation(orientation)
                            .with_scale(Vec3::splat(wound_size * 1.35)),
                        Visibility::Visible,
                    ))
                    .id();

                diagnostics.wound_visuals_spawned =
                    diagnostics.wound_visuals_spawned.saturating_add(1);

                // Attach to parent entity
                commands.entity(attach_entity).add_child(wound_entity);
            }
            BloodEffectEvent::UpdateWoundVisibility {
                entity,
                health_percent,
            } => {
                // Update wound visibility based on health
                let should_show = *health_percent < config.wound_visibility_threshold;

                for (wound_entity, wound_visual) in query_wounds.iter() {
                    if wound_visual.parent_entity == *entity {
                        let mut entity_cmds = commands.entity(wound_entity);
                        if should_show {
                            entity_cmds.insert(Visibility::Visible);
                        } else {
                            entity_cmds.insert(Visibility::Hidden);
                        }
                    }
                }
            }
            BloodEffectEvent::CleanupWounds { entity } => {
                // Remove all wound visuals for this entity
                for (wound_entity, wound_visual) in query_wounds.iter() {
                    if wound_visual.parent_entity == *entity {
                        commands.entity(wound_entity).despawn();
                    }
                }
            }
            _ => {}
        }
    }
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
