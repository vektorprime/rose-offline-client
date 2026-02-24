use bevy::{
    asset::Handle,
    math::Vec3,
    prelude::{
        Commands, Component, Entity, GlobalTransform, Query, Res, Resource, Transform, With,
    },
    ecs::system::ResMut,
};

use crate::{
    audio::{AudioSource, SoundGain, SoundRadius, SpatialSound},
    components::{PlayerCharacter, SoundCategory},
};

/// Maximum number of concurrent monster sounds allowed
const MAX_CONCURRENT_MONSTER_SOUNDS: usize = 3;

/// A pending monster sound request that will be evaluated for playing
#[derive(Component)]
pub struct PendingMonsterSound {
    pub audio_source: Handle<AudioSource>,
    pub position: Vec3,
    pub sound_radius: Option<f32>,
    pub gain: SoundGain,
    pub category: SoundCategory,
}

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
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
) {
    // Get player position
    let player_position = query_player
        .get_single()
        .map(|transform| transform.translation())
        .unwrap_or(Vec3::ZERO);

    // Sort by distance to player (closest first)
    sound_queue
        .pending_sounds
        .sort_by(|a, b| {
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
        let mut entity_commands = commands.spawn((
            sound_data.category,
            sound_data.gain,
            SpatialSound::new(sound_data.audio_source),
            Transform::from_translation(sound_data.position),
            GlobalTransform::from_translation(sound_data.position),
        ));

        if let Some(radius) = sound_data.sound_radius {
            entity_commands.insert(SoundRadius::new(radius));
        }
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
