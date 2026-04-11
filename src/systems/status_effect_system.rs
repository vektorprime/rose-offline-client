use bevy::ecs::prelude::{Query, Res};
use std::time::Duration;

use rose_data::StatusEffectType;
use rose_game_common::components::{
    AbilityValues, HealthPoints, ManaPoints, StatusEffects, StatusEffectsRegen,
};

use crate::resources::GameData;

pub fn status_effect_system(
    mut query: Query<(
        &AbilityValues,
        &mut HealthPoints,
        Option<&mut ManaPoints>,
        &StatusEffects,
        &mut StatusEffectsRegen,
    )>,
    game_data: Res<GameData>,
    time: Res<bevy::time::Time>,
) {
    for (
        _ability_values,
        mut health_points,
        _mana_points,
        status_effects,
        mut status_effects_regen,
    ) in query.iter_mut()
    {
        let apply_per_second_effect = {
            status_effects_regen.per_second_tick_counter += time.delta();
            if status_effects_regen.per_second_tick_counter > Duration::from_secs(1) {
                status_effects_regen.per_second_tick_counter -= Duration::from_secs(1);
                true
            } else {
                false
            }
        };

        for (status_effect_type, status_effect_slot) in status_effects.active.iter() {
            if let Some(status_effect) = status_effect_slot {
                match status_effect_type {
                    StatusEffectType::Poisoned => {
                        if apply_per_second_effect {
                            if let Some(data) =
                                game_data.status_effects.get_status_effect(status_effect.id)
                            {
                                health_points.hp =
                                    i32::max(health_points.hp - data.apply_per_second_value, 1);
                            }
                        }
                    }
                    StatusEffectType::DecreaseLifeTime => {
                        if apply_per_second_effect {
                            if let Some(data) =
                                game_data.status_effects.get_status_effect(status_effect.id)
                            {
                                if health_points.hp > data.apply_per_second_value {
                                    health_points.hp -= data.apply_per_second_value;
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
