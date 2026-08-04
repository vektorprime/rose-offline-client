use bevy::{
    asset::Handle,
    ecs::system::ResMut,
    math::Vec3,
    prelude::{Commands, Component, Query, Resource, With},
};

use crate::{
    audio::{spawn_spatial_sound, AudioSource, SoundGain, AUDIBLE_CUTOFF},
    components::SoundCategory,
};

/// Maximum number of monster sounds to spawn per frame
const MAX_CONCURRENT_MONSTER_SOUNDS: usize = 3;

/// Maximum number of concurrently active monster one-shot sounds
const MAX_ACTIVE_MONSTER_SOUNDS: usize = 64;

/// Marker for spatial sounds spawned from the monster sound queue, used to count active sounds
#[derive(Component)]
pub struct MonsterSound;

/// Resource to track active monster sounds in the current frame
#[derive(Resource, Default)]
pub struct MonsterSoundQueue {
    pub pending_sounds: Vec<PendingMonsterSoundData>,
}

#[derive(Clone)]
pub struct PendingMonsterSoundData {
    pub audio_source: Handle<AudioSource>,
    pub position: Vec3,
    pub sound_radius: Option<f32>,
    pub gain: SoundGain,
    pub category: SoundCategory,
    pub distance_to_player: f32,
}

/// System that processes pending monster sounds and spawns only the closest ones
/// This should run after all sound request systems but before the spatial_sound_system
pub fn process_monster_sound_queue_system(
    mut commands: Commands,
    mut sound_queue: ResMut<MonsterSoundQueue>,
    query_active_monster_sounds: Query<(), With<MonsterSound>>,
) {
    // Sort by distance to player (closest first)
    sound_queue.pending_sounds.sort_by(|a, b| {
        a.distance_to_player
            .partial_cmp(&b.distance_to_player)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Cap the number of concurrently active monster sounds
    let mut active_sounds = query_active_monster_sounds.iter().count();

    // Only spawn the closest N sounds, refusing once the concurrency cap is reached
    for sound_data in sound_queue
        .pending_sounds
        .drain(..)
        .take(MAX_CONCURRENT_MONSTER_SOUNDS)
    {
        if active_sounds >= MAX_ACTIVE_MONSTER_SOUNDS {
            break;
        }

        let SoundGain::Ratio(gain) = sound_data.gain;
        let entity = spawn_spatial_sound(
            &mut commands,
            sound_data.audio_source,
            sound_data.position,
            gain,
            sound_data.sound_radius,
            sound_data.category,
            false,
        );
        commands.entity(entity).insert(MonsterSound);
        active_sounds += 1;
    }
}

/// Helper function to add a monster sound to the queue instead of spawning directly
pub fn queue_monster_sound(
    _commands: &mut Commands,
    sound_queue: &mut ResMut<MonsterSoundQueue>,
    player_position: Vec3,
    audio_source: Handle<AudioSource>,
    position: Vec3,
    sound_radius: Option<f32>,
    gain: SoundGain,
    category: SoundCategory,
) {
    let distance_to_player = position.distance(player_position);

    // Sounds beyond the audible cutoff are never queued
    if distance_to_player > AUDIBLE_CUTOFF {
        return;
    }

    sound_queue.pending_sounds.push(PendingMonsterSoundData {
        audio_source,
        position,
        sound_radius,
        gain,
        category,
        distance_to_player,
    });
}
