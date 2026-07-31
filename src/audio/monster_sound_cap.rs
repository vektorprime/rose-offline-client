use bevy::{
    asset::Handle,
    ecs::system::ResMut,
    math::Vec3,
    prelude::{Commands, Resource},
};

use crate::{
    audio::{spawn_spatial_sound, AudioSource, SoundGain},
    components::SoundCategory,
};

/// Maximum number of concurrent monster sounds allowed
const MAX_CONCURRENT_MONSTER_SOUNDS: usize = 3;

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
) {
    // Sort by distance to player (closest first)
    sound_queue.pending_sounds.sort_by(|a, b| {
        a.distance_to_player
            .partial_cmp(&b.distance_to_player)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Only spawn the closest N sounds
    for sound_data in sound_queue
        .pending_sounds
        .drain(..)
        .take(MAX_CONCURRENT_MONSTER_SOUNDS)
    {
        let SoundGain::Ratio(gain) = sound_data.gain;
        spawn_spatial_sound(
            &mut commands,
            sound_data.audio_source,
            sound_data.position,
            gain,
            sound_data.sound_radius,
            sound_data.category,
            false,
        );
    }

    // Clear any remaining sounds that didn't make the cut
    sound_queue.pending_sounds.clear();
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

    sound_queue.pending_sounds.push(PendingMonsterSoundData {
        audio_source,
        position,
        sound_radius,
        gain,
        category,
        distance_to_player,
    });
}
