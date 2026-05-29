use std::sync::Arc;

use bevy::prelude::*;
use rand::Rng;

use crate::{
    audio::{AudioSource, AudioSourceDecoded, SoundGain, SoundRadius, SpatialSound},
    components::{BoatState, PlayerCharacter, SoundCategory},
    resources::WindState,
};

const BOAT_SOUND_SAMPLE_RATE: u32 = 22_050;

#[derive(Resource, Clone)]
pub struct BoatSoundAssets {
    pub wind_loop: Handle<AudioSource>,
    pub creak_loop: Handle<AudioSource>,
    pub flap_loop: Handle<AudioSource>,
    pub splash: Handle<AudioSource>,
    pub rope: Handle<AudioSource>,
    pub board: Handle<AudioSource>,
    pub disembark: Handle<AudioSource>,
}

/// Attached to the player entity while sailing, manages looped and one-shot boat sounds.
#[derive(Component)]
pub struct BoatSoundState {
    /// Loop entity for wind-rushing sound.
    pub wind_loop: Option<Entity>,
    /// Loop entity for hull creaking sound.
    pub creak_loop: Option<Entity>,
    /// Loop entity for sail flapping (luffing).
    pub flap_loop: Option<Entity>,
    /// Timer for periodic bow splash one-shots.
    pub splash_timer: Timer,
    /// Last recorded sail trim, used to detect trim changes for rope sounds.
    pub last_sail_trim: f32,
    /// Whether the boat was luffing last frame, used to detect state transitions.
    pub last_luffing: bool,
}

impl Default for BoatSoundState {
    fn default() -> Self {
        Self {
            wind_loop: None,
            creak_loop: None,
            flap_loop: None,
            splash_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
            last_sail_trim: std::f32::consts::FRAC_PI_4,
            last_luffing: false,
        }
    }
}

fn make_audio_source(samples: Vec<f32>) -> AudioSource {
    AudioSource {
        bytes: Arc::new([]),
        decoded: Some(Arc::new(AudioSourceDecoded {
            samples,
            channel_count: 1,
            sample_rate: BOAT_SOUND_SAMPLE_RATE,
        })),
        create_streaming_source_fn: |_| Err(anyhow::anyhow!("decoded placeholder sound")),
    }
}

fn envelope(t: f32, attack: f32, release_start: f32) -> f32 {
    let attack_gain = if attack > 0.0 {
        (t / attack).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let release_gain = if t > release_start {
        (1.0 - (t - release_start) / (1.0 - release_start).max(0.001)).clamp(0.0, 1.0)
    } else {
        1.0
    };
    attack_gain * release_gain
}

fn generated_loop(seconds: f32, mut sample: impl FnMut(f32, usize) -> f32) -> Vec<f32> {
    let total_samples = (BOAT_SOUND_SAMPLE_RATE as f32 * seconds) as usize;
    (0..total_samples)
        .map(|index| sample(index as f32 / BOAT_SOUND_SAMPLE_RATE as f32, index).clamp(-1.0, 1.0))
        .collect()
}

fn generated_one_shot(seconds: f32, mut sample: impl FnMut(f32, usize) -> f32) -> Vec<f32> {
    let total_samples = (BOAT_SOUND_SAMPLE_RATE as f32 * seconds) as usize;
    (0..total_samples)
        .map(|index| {
            let t = index as f32 / BOAT_SOUND_SAMPLE_RATE as f32;
            let n = t / seconds.max(0.001);
            (sample(n, index) * envelope(n, 0.03, 0.45)).clamp(-1.0, 1.0)
        })
        .collect()
}

fn pseudo_noise(index: usize) -> f32 {
    let x = index as u32;
    let hash = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (hash as f32 / u32::MAX as f32) * 2.0 - 1.0
}

pub fn setup_boat_sound_assets(
    mut commands: Commands,
    mut audio_sources: ResMut<Assets<AudioSource>>,
) {
    let wind_loop = audio_sources.add(make_audio_source(generated_loop(1.5, |t, i| {
        let gust = (t * std::f32::consts::TAU * 0.7).sin() * 0.4 + 0.6;
        pseudo_noise(i) * 0.08 * gust + (t * std::f32::consts::TAU * 90.0).sin() * 0.015
    })));

    let creak_loop = audio_sources.add(make_audio_source(generated_loop(2.0, |t, _| {
        let slow = (t * std::f32::consts::TAU * 1.2).sin().max(0.0).powf(8.0);
        let wood = (t * std::f32::consts::TAU * 95.0).sin() * 0.08
            + (t * std::f32::consts::TAU * 143.0).sin() * 0.04;
        wood * slow
    })));

    let flap_loop = audio_sources.add(make_audio_source(generated_loop(0.8, |t, i| {
        let flap = (t * std::f32::consts::TAU * 4.0).sin().abs().powf(2.0);
        pseudo_noise(i) * 0.12 * flap
    })));

    let splash = audio_sources.add(make_audio_source(generated_one_shot(0.5, |n, i| {
        let crackle = pseudo_noise(i) * (1.0 - n).powf(2.0);
        let low = (n * std::f32::consts::TAU * 45.0).sin() * (1.0 - n);
        crackle * 0.32 + low * 0.12
    })));

    let rope = audio_sources.add(make_audio_source(generated_one_shot(0.35, |n, _| {
        let chirp = (n * std::f32::consts::TAU * (120.0 + 240.0 * n)).sin();
        chirp * (1.0 - n).powf(1.8) * 0.25
    })));

    let board = audio_sources.add(make_audio_source(generated_one_shot(0.45, |n, i| {
        let thud = (n * std::f32::consts::TAU * 55.0).sin() * (1.0 - n).powf(3.0);
        thud * 0.35 + pseudo_noise(i) * 0.07 * (1.0 - n)
    })));

    let disembark = audio_sources.add(make_audio_source(generated_one_shot(0.4, |n, i| {
        let step = (n * std::f32::consts::TAU * 70.0).sin() * (1.0 - n).powf(2.0);
        step * 0.25 + pseudo_noise(i) * 0.05 * (1.0 - n)
    })));

    commands.insert_resource(BoatSoundAssets {
        wind_loop,
        creak_loop,
        flap_loop,
        splash,
        rope,
        board,
        disembark,
    });
}

fn spawn_loop_sound(
    commands: &mut Commands,
    parent: Entity,
    handle: Handle<AudioSource>,
    volume: f32,
    sound_radius: f32,
) -> Entity {
    let entity = commands
        .spawn((
            SpatialSound::new_repeating(handle),
            SoundGain::Ratio(volume),
            SoundRadius(sound_radius),
            SoundCategory::PlayerFootstep,
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();

    commands.entity(parent).add_child(entity);
    entity
}

fn spawn_one_shot_sound(
    commands: &mut Commands,
    parent: Entity,
    handle: Handle<AudioSource>,
    volume: f32,
    sound_radius: f32,
) -> Entity {
    let entity = commands
        .spawn((
            SpatialSound::new(handle),
            SoundGain::Ratio(volume),
            SoundRadius(sound_radius),
            SoundCategory::PlayerFootstep,
            Transform::default(),
            GlobalTransform::default(),
        ))
        .id();

    commands.entity(parent).add_child(entity);
    entity
}

/// Ensures a BoatSoundState component exists on the player when BoatState is active.
pub fn ensure_boat_sound_state_system(
    mut commands: Commands,
    sound_assets: Res<BoatSoundAssets>,
    boat_query: Query<(Entity, &BoatState, Option<&BoatSoundState>), With<PlayerCharacter>>,
) {
    for (entity, boat, sound_state) in boat_query.iter() {
        if boat.active && sound_state.is_none() {
            let mut state = BoatSoundState::default();
            state.last_sail_trim = boat.sail_trim;

            state.wind_loop = Some(spawn_loop_sound(
                &mut commands,
                entity,
                sound_assets.wind_loop.clone(),
                0.12,
                12.0,
            ));
            state.creak_loop = Some(spawn_loop_sound(
                &mut commands,
                entity,
                sound_assets.creak_loop.clone(),
                0.10,
                7.0,
            ));
            state.flap_loop = Some(spawn_loop_sound(
                &mut commands,
                entity,
                sound_assets.flap_loop.clone(),
                0.0,
                10.0,
            ));

            spawn_one_shot_sound(&mut commands, entity, sound_assets.board.clone(), 0.35, 7.0);
            commands.entity(entity).insert(state);
        } else if !boat.active && sound_state.is_some() {
            if let Some(state) = sound_state {
                spawn_one_shot_sound(
                    &mut commands,
                    entity,
                    sound_assets.disembark.clone(),
                    0.32,
                    7.0,
                );

                if let Some(entity) = state.wind_loop {
                    commands.entity(entity).despawn();
                }
                if let Some(entity) = state.creak_loop {
                    commands.entity(entity).despawn();
                }
                if let Some(entity) = state.flap_loop {
                    commands.entity(entity).despawn();
                }
            }
            commands.entity(entity).remove::<BoatSoundState>();
        }
    }
}

/// Updates looping boat sound gain based on speed, luffing, and wave motion.
pub fn boat_loop_sound_update_system(
    wind: Res<WindState>,
    mut boat_query: Query<(&BoatState, &mut BoatSoundState), With<PlayerCharacter>>,
    mut sound_query: Query<&mut SoundGain>,
) {
    for (boat, mut sound_state) in boat_query.iter_mut() {
        if !boat.active {
            continue;
        }

        let speed_ratio = (boat.speed / boat.max_speed.max(0.1)).clamp(0.0, 1.0);

        if let Some(wind_entity) = sound_state.wind_loop {
            if let Ok(mut gain) = sound_query.get_mut(wind_entity) {
                *gain = SoundGain::Ratio(0.08 + speed_ratio * 0.38);
            }
        }

        if let Some(creak_entity) = sound_state.creak_loop {
            if let Ok(mut gain) = sound_query.get_mut(creak_entity) {
                let wave_motion = (boat.wave_roll.abs() + boat.wave_pitch.abs()).clamp(0.0, 0.2);
                *gain = SoundGain::Ratio(0.07 + speed_ratio * 0.10 + wave_motion * 0.8);
            }
        }

        let angle_to_wind = (boat.heading - wind.angle).rem_euclid(std::f32::consts::TAU);
        let angle_to_wind_abs = if angle_to_wind > std::f32::consts::PI {
            std::f32::consts::TAU - angle_to_wind
        } else {
            angle_to_wind
        };
        let luff_factor = (1.0 - (angle_to_wind_abs / 0.78).clamp(0.0, 1.0)).clamp(0.0, 1.0);

        if let Some(flap_entity) = sound_state.flap_loop {
            if let Ok(mut gain) = sound_query.get_mut(flap_entity) {
                *gain = SoundGain::Ratio(luff_factor * 0.45);
            }
        }

        sound_state.last_luffing = luff_factor > 0.1;
    }
}

/// Manages one-shot boat sounds: bow splashes and rope tightening.
pub fn boat_one_shot_sound_system(
    time: Res<Time>,
    sound_assets: Res<BoatSoundAssets>,
    mut boat_query: Query<(Entity, &BoatState, &mut BoatSoundState), With<PlayerCharacter>>,
    mut commands: Commands,
) {
    let mut rng = rand::thread_rng();

    for (entity, boat, mut sound_state) in boat_query.iter_mut() {
        if !boat.active {
            continue;
        }

        let speed_ratio = (boat.speed / boat.max_speed.max(0.1)).clamp(0.0, 1.0);

        sound_state.splash_timer.tick(time.delta());
        if speed_ratio > 0.5 && sound_state.splash_timer.just_finished() {
            spawn_one_shot_sound(
                &mut commands,
                entity,
                sound_assets.splash.clone(),
                0.18 + speed_ratio * 0.30,
                9.0,
            );
            let next_splash_secs = rng.gen_range(1.2..=2.8);
            sound_state
                .splash_timer
                .set_duration(std::time::Duration::from_secs_f32(next_splash_secs));
            sound_state.splash_timer.reset();
        }

        let trim_change = (boat.sail_trim - sound_state.last_sail_trim).abs();
        if trim_change > 0.1 {
            spawn_one_shot_sound(&mut commands, entity, sound_assets.rope.clone(), 0.22, 6.0);
            sound_state.last_sail_trim = boat.sail_trim;
        }
    }
}
