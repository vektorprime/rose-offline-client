//! Gash wound visual systems for damaged entities.
//!
//! This module implements wound visuals that appear on entities when their HP
//! drops below a configurable threshold (default 50%).

use bevy::prelude::*;

use rose_game_common::components::{AbilityValues, HealthPoints};

use crate::{
    components::{Dead, GashWounds, WoundVisual},
    events::BloodEffectEvent,
    resources::BloodEffectConfig,
};

/// System that monitors HP and shows/hides wounds based on health percentage.
///
/// When an entity's HP drops below the wound visibility threshold (default 50%),
/// wounds are shown. Wounds remain visible until the entity despawns.
pub fn wound_visibility_system(
    mut commands: Commands,
    mut query: Query<
        (Entity, &HealthPoints, &AbilityValues, Option<&mut GashWounds>),
        Without<Dead>,
    >,
    mut blood_events: EventWriter<BloodEffectEvent>,
    config: Res<BloodEffectConfig>,
) {
    if !config.enable_blood || !config.show_wounds {
        return;
    }

    for (entity, hp, ability_values, wounds) in query.iter_mut() {
        let max_hp = ability_values.get_max_health();
        if max_hp <= 0 {
            continue;
        }

        let health_percent = hp.hp as f32 / max_hp as f32;
        let should_show_wounds = health_percent < config.wound_visibility_threshold;

        if let Some(mut wounds) = wounds {
            if wounds.wounds_visible != should_show_wounds {
                wounds.wounds_visible = should_show_wounds;

                if should_show_wounds && wounds.wound_count < 3 {
                    // Add new wound visual
                    blood_events.write(BloodEffectEvent::show_wound(
                        entity,
                        Vec3::ZERO, // Will be calculated based on hit direction
                        Vec3::Z,
                    ));
                    wounds.wound_count += 1;
                }
            }
        } else if should_show_wounds {
            // First time showing wounds - create component
            commands.entity(entity).insert(GashWounds::new(entity));

            blood_events.write(BloodEffectEvent::show_wound(
                entity,
                Vec3::ZERO,
                Vec3::Z,
            ));
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
    mut blood_events: EventReader<BloodEffectEvent>,
    query_wounds: Query<(Entity, &WoundVisual)>,
    config: Res<BloodEffectConfig>,
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
                // Create a simple wound visual as a child entity
                // In a full implementation, this would create a mesh/texture overlay
                let wound_entity = commands
                    .spawn((
                        Name::new("WoundVisual"),
                        WoundVisual::new(*entity),
                        Transform::from_translation(*wound_position),
                        Visibility::Visible,
                    ))
                    .id();

                // Attach to parent entity
                commands.entity(*entity).add_child(wound_entity);
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
