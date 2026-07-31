use bevy::prelude::{
    AssetServer, Commands, Entity, GlobalTransform, MessageReader, MessageWriter, Query, Res,
    Transform,
};

use rose_data::ItemType;

use crate::{
    audio::SpatialSound,
    components::{PlayerCharacter, SoundCategory},
    events::{SpawnEffectData, SpawnEffectEvent, UseItemEvent},
    resources::{GameData, SoundCache, SoundSettings},
};

pub fn use_item_event_system(
    mut commands: Commands,
    mut events: MessageReader<UseItemEvent>,
    mut spawn_effect_events: MessageWriter<SpawnEffectEvent>,
    mut query: Query<(Entity, &GlobalTransform, Option<&PlayerCharacter>)>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
    sound_cache: Res<SoundCache>,
) {
    for UseItemEvent { entity, item } in events.read() {
        let (user_entity, user_global_transform, user_is_player) =
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
    }
}
