use bevy::{
    math::Vec3,
    prelude::{
        AssetServer, Commands, EventReader, EventWriter, GlobalTransform, Query, Res, ResMut, Transform, With,
    },
};

use rose_data::SoundId;
use rose_file_readers::VfsPathBuf;
use rose_game_common::components::Npc;

use crate::{
    audio::{queue_monster_sound, MonsterSoundQueue, SpatialSound},
    components::{PlayerCharacter, SoundCategory},
    events::{ChatboxEvent, ClientEntityEvent, SpawnEffectData, SpawnEffectEvent},
    resources::{GameData, SoundCache, SoundSettings},
};

pub fn client_entity_event_system(
    mut commands: Commands,
    mut client_entity_events: EventReader<ClientEntityEvent>,
    mut chatbox_events: EventWriter<ChatboxEvent>,
    mut spawn_effect_events: EventWriter<SpawnEffectEvent>,
    query_player: Query<&PlayerCharacter>,
    query_global_transform: Query<&GlobalTransform>,
    query_npc: Query<(&Npc, &GlobalTransform)>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
    sound_cache: Res<SoundCache>,
    mut sound_queue: ResMut<MonsterSoundQueue>,
    query_player_transform: Query<&GlobalTransform, With<PlayerCharacter>>,
) {
    let is_player = |entity| query_player.contains(entity);

    // Get player position for sound prioritization
    let player_position = query_player_transform
        .get_single()
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::ZERO);

    for event in client_entity_events.read() {
        match *event {
            ClientEntityEvent::Die(entity) => {
                if let Ok((npc, global_transform)) = query_npc.get(entity) {
                    if let Some(npc_data) = game_data.npcs.get_npc(npc.id) {
                        if let Some(sound_data) = npc_data
                            .die_sound_id
                            .and_then(|id| game_data.sounds.get_sound(id))
                        {
                            // Use the monster sound queue for capping
                            queue_monster_sound(
                                &mut commands,
                                &mut sound_queue,
                                player_position,
                                sound_cache.load(sound_data, &asset_server),
                                global_transform.translation(),
                                None,
                                sound_settings.gain(SoundCategory::NpcSounds),
                                SoundCategory::NpcSounds,
                            );
                        }

                        if let Some(die_effect_file_id) = npc_data.die_effect_file_id {
                            spawn_effect_events.write(SpawnEffectEvent::OnEntity(
                                entity,
                                None,
                                SpawnEffectData::with_file_id(die_effect_file_id),
                            ));
                        }
                    }
                }
            }
            ClientEntityEvent::LevelUp(entity, level) => {
                let sound_category = if is_player(entity) {
                    if let Some(level) = level {
                        chatbox_events.write(ChatboxEvent::System(format!(
                            "Congratulations! You are now level {}!",
                            level
                        )));
                    }

                    SoundCategory::PlayerCombat
                } else {
                    SoundCategory::OtherCombat
                };

                if let Ok(global_transform) = query_global_transform.get(entity) {
                    if let Some(sound_data) = game_data.sounds.get_sound(SoundId::new(16).unwrap())
                    {
                        // Level up sounds for player are always played directly
                        // For other entities, use the queue
                        if is_player(entity) {
                            commands.spawn((
                                sound_category,
                                sound_settings.gain(sound_category),
                                SpatialSound::new(sound_cache.load(sound_data, &asset_server)),
                                Transform::from_translation(global_transform.translation()),
                                GlobalTransform::from_translation(global_transform.translation()),
                            ));
                        } else {
                            queue_monster_sound(
                                &mut commands,
                                &mut sound_queue,
                                player_position,
                                sound_cache.load(sound_data, &asset_server),
                                global_transform.translation(),
                                None,
                                sound_settings.gain(sound_category),
                                sound_category,
                            );
                        }
                    }
                }

                spawn_effect_events.write(SpawnEffectEvent::OnEntity(
                    entity,
                    None,
                    SpawnEffectData::with_path(VfsPathBuf::new("3DDATA/EFFECT/LEVELUP_01.EFT")),
                ));
            }
        }
    }
}
