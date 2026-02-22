use std::time::{Duration, Instant};

use bevy::prelude::{
    AssetServer, Commands, Entity, EventReader, EventWriter, GlobalTransform, Query, Res,
    Transform,
};

use rose_data::ItemType;
use rose_game_common::components::{StatusEffects, StatusEffectsRegen};

use crate::{
    audio::SpatialSound,
    components::{PlayerCharacter, SoundCategory},
    events::{SpawnEffectData, SpawnEffectEvent, UseItemEvent},
    resources::{GameData, SoundCache, SoundSettings},
};

pub fn use_item_event_system(
    mut commands: Commands,
    mut events: EventReader<UseItemEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &mut StatusEffects,
        &mut StatusEffectsRegen,
        Option<&PlayerCharacter>,
    )>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
    sound_cache: Res<SoundCache>,
) {
    for UseItemEvent { entity, item } in events.read() {
        let (user_entity, user_global_transform, mut user_status_effects, mut user_status_effects_regen, user_is_player) =
            if let Ok(user) = query.get_mut(*entity) {
                user
            } else {
                continue;
            };

        if item.item_type != ItemType::Consumable {
            continue;
        }

        let item_data =
            if let Some(item_data) = game_data.items.get_consumable_item(item.item_number) {
                item_data
            } else {
                continue;
            };

        if let Some(effect_file_id) = item_data.effect_file_id {
            spawn_effect_events.write(SpawnEffectEvent::OnEntity(
                user_entity,
                None,
                SpawnEffectData::with_file_id(effect_file_id),
            ));
        }

        if let Some(sound_data) = item_data
            .effect_sound_id
            .and_then(|id| game_data.sounds.get_sound(id))
        {
            let category = if user_is_player.is_some() {
                SoundCategory::PlayerCombat
            } else {
                SoundCategory::OtherCombat
            };

            commands.spawn((
                category,
                sound_settings.gain(category),
                SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                Transform::from_translation(user_global_transform.translation()),
                GlobalTransform::from_translation(user_global_transform.translation()),
            ));
        }

        if let Some((base_status_effect_id, total_potion_value)) = item_data.apply_status_effect {
            if let Some(base_status_effect) = game_data
                .status_effects
                .get_status_effect(base_status_effect_id)
            {
                for (status_effect_data, &potion_value_per_second) in base_status_effect
                    .apply_status_effects
                    .iter()
                    .filter_map(|(id, value)| {
                        game_data
                            .status_effects
                            .get_status_effect(*id)
                            .map(|data| (data, value))
                    })
                {
                    if user_status_effects.can_apply(
                        status_effect_data,
                        status_effect_data.id.get() as i32,
                    ) {
                        user_status_effects.apply_potion(
                            &mut user_status_effects_regen,
                            status_effect_data,
                            Instant::now()
                                + Duration::from_micros(
                                    total_potion_value as u64 * 1000000
                                        / potion_value_per_second as u64,
                                ),
                            total_potion_value,
                            potion_value_per_second,
                        );
                    }
                }
            }
        } else if let Some((_add_ability_type, _add_ability_value)) = item_data.add_ability {
            /*
            TODO:
            ability_values_add_value(
                add_ability_type,
                add_ability_value,
                Some(user.ability_values),
                Some(&mut user.basic_stats),
                Some(&mut user.experience_points),
                Some(&mut user.health_points),
                Some(&mut user.inventory),
                Some(&mut user.mana_points),
                Some(&mut user.skill_points),
                Some(&mut user.stamina),
                Some(&mut user.stat_points),
                Some(&mut user.union_membership),
                user.game_client,
            );
            */
        }
    }
}
