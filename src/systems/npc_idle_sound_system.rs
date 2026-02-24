use bevy::{
    prelude::{AssetServer, ChildSpawnerCommands, Commands, Component, Entity, GlobalTransform, Query, Res, ResMut, Transform, With},
    math::Vec3,
};
use rand::Rng;

use rose_game_common::components::Npc;

use crate::{
    animation::SkeletalAnimation,
    audio::{queue_monster_sound, MonsterSoundQueue, SpatialSound},
    components::{Command, SoundCategory},
    resources::{GameData, SoundCache, SoundSettings},
    components::PlayerCharacter,
};

#[derive(Component, Default)]
pub struct NpcIdleSoundState {
    pub last_idle_loop_count: Option<usize>,
}

pub fn npc_idle_sound_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &Npc,
        &SkeletalAnimation,
        &Command,
        &GlobalTransform,
        Option<&mut NpcIdleSoundState>,
    )>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    sound_settings: Res<SoundSettings>,
    sound_cache: Res<SoundCache>,
    mut sound_queue: ResMut<MonsterSoundQueue>,
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
) {
    let mut rng = rand::thread_rng();
    let gain = sound_settings.gain(SoundCategory::NpcSounds);

    // Get player position for sound prioritization
    let player_position = query_player
        .get_single()
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::ZERO);

    for (entity, npc, skeletal_animation, command, global_transform, idle_sound_state) in
        query.iter_mut()
    {
        if idle_sound_state.is_none() {
            commands.entity(entity).insert(NpcIdleSoundState::default());
            continue;
        }
        let mut idle_sound_state = idle_sound_state.unwrap();

        if !command.is_stop() {
            idle_sound_state.last_idle_loop_count = None;
            continue;
        }

        // There is a 20% chance to play the idle sound, once per animation loop
        if let Some(last_idle_loop_count) = idle_sound_state.last_idle_loop_count {
            if last_idle_loop_count >= skeletal_animation.current_loop_count() {
                continue;
            }
            idle_sound_state.last_idle_loop_count = Some(skeletal_animation.current_loop_count());
        } else {
            idle_sound_state.last_idle_loop_count = Some(skeletal_animation.current_loop_count());
        }

        if rng.gen_range(0..100) < 20 {
            if let Some(sound_data) = game_data
                .npcs
                .get_npc(npc.id)
                .and_then(|npc_data| npc_data.normal_effect_sound_id)
                .and_then(|sound_id| game_data.sounds.get_sound(sound_id))
            {
                // Use the monster sound queue for capping
                queue_monster_sound(
                    &mut commands,
                    &mut sound_queue,
                    player_position,
                    sound_cache.load(sound_data, &asset_server),
                    global_transform.translation(),
                    Some(4.0), // Sound radius
                    gain,
                    SoundCategory::NpcSounds,
                );
            }
        }
    }
}
