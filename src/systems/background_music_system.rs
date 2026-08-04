use bevy::prelude::{AssetServer, Commands, Entity, Handle, Local, Query, Res, Time};
use rose_data::ZoneId;

use crate::{
    audio::{AudioSource, GlobalSound, SoundGain},
    components::SoundCategory,
    resources::{CurrentZone, GameData, SoundCache, SoundSettings, ZoneTime, ZoneTimeState},
};

const CROSSFADE_DURATION: f32 = 2.0;

#[derive(Default)]
pub enum BackgroundMusicState {
    #[default]
    None,
    PlayingDay,
    PlayingNight,
    FadingOut {
        old_entity: Entity,
        timer: f32,
        initial_gain: f32,
    },
}

#[derive(Default)]
pub struct BackgroundMusic {
    pub zone: Option<ZoneId>,
    pub entity: Option<Entity>,
    pub day_audio_source: Option<Handle<AudioSource>>,
    pub night_audio_source: Option<Handle<AudioSource>>,
    pub state: BackgroundMusicState,
}

pub fn background_music_system(
    mut commands: Commands,
    mut background_music: Local<BackgroundMusic>,
    asset_server: Res<AssetServer>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    zone_time: Res<ZoneTime>,
    sound_settings: Res<SoundSettings>,
    sound_cache: Res<SoundCache>,
    mut query_gains: Query<&mut SoundGain>,
    time: Res<Time>,
) {
    if let Some(current_zone) = current_zone {
        if background_music.zone != Some(current_zone.id) {
            if let Some(entity) = background_music.entity.take() {
                commands.entity(entity).despawn();
            }
            if let BackgroundMusicState::FadingOut { old_entity, .. } = &background_music.state {
                commands.entity(*old_entity).despawn();
            }
            background_music.state = BackgroundMusicState::None;

            // Drop cached sound handles so decoded sounds from the previous zone can be freed
            sound_cache.clear();

            if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
                background_music.day_audio_source = zone_data
                    .background_music_day
                    .as_ref()
                    .map(|path| asset_server.load(path.path().to_string_lossy().into_owned()));
                background_music.night_audio_source = zone_data
                    .background_music_night
                    .as_ref()
                    .map(|path| asset_server.load(path.path().to_string_lossy().into_owned()));
            } else {
                background_music.day_audio_source = None;
                background_music.night_audio_source = None;
            }

            background_music.zone = Some(current_zone.id);
        }

        // Handle crossfade state
        let mut next_state = None;
        if let BackgroundMusicState::FadingOut {
            old_entity,
            timer,
            initial_gain,
        } = &mut background_music.state
        {
            *timer += time.delta_secs();
            if *timer >= CROSSFADE_DURATION {
                // Fade complete, despawn old entity
                commands.entity(*old_entity).despawn();
                next_state = Some(if background_music.entity.is_some() {
                    if zone_time.state == ZoneTimeState::Morning
                        || zone_time.state == ZoneTimeState::Day
                    {
                        BackgroundMusicState::PlayingDay
                    } else {
                        BackgroundMusicState::PlayingNight
                    }
                } else {
                    BackgroundMusicState::None
                });
            } else if let Ok(mut gain) = query_gains.get_mut(*old_entity) {
                // Ramp the old track's gain down over the crossfade duration
                let factor = (*initial_gain * (1.0 - *timer / CROSSFADE_DURATION)).max(0.0);
                *gain = SoundGain::Ratio(factor);
            }
        }
        if let Some(state) = next_state {
            background_music.state = state;
        }

        match zone_time.state {
            ZoneTimeState::Morning | ZoneTimeState::Day => {
                match &background_music.state {
                    BackgroundMusicState::None | BackgroundMusicState::PlayingNight => {
                        let old_entity = background_music.entity.take();
                        let new_source = background_music.day_audio_source.clone();

                        // Start the new track immediately so it overlaps with the old one
                        background_music.entity = new_source.map(|source| {
                            commands
                                .spawn((
                                    SoundCategory::BackgroundMusic,
                                    GlobalSound::new_repeating(source),
                                    sound_settings.gain(SoundCategory::BackgroundMusic),
                                ))
                                .id()
                        });

                        if let Some(old_entity) = old_entity {
                            // Start fading out old music
                            let initial_gain = query_gains
                                .get_mut(old_entity)
                                .ok()
                                .map_or(1.0, |gain| match *gain {
                                    SoundGain::Ratio(factor) => factor,
                                });
                            background_music.state = BackgroundMusicState::FadingOut {
                                old_entity,
                                timer: 0.0,
                                initial_gain,
                            };
                        } else if background_music.entity.is_some() {
                            background_music.state = BackgroundMusicState::PlayingDay;
                        }
                    }
                    BackgroundMusicState::PlayingDay => {}
                    BackgroundMusicState::FadingOut { .. } => {}
                }
            }
            ZoneTimeState::Evening | ZoneTimeState::Night => {
                match &background_music.state {
                    BackgroundMusicState::None | BackgroundMusicState::PlayingDay => {
                        let old_entity = background_music.entity.take();
                        let new_source = background_music.night_audio_source.clone();

                        // Start the new track immediately so it overlaps with the old one
                        background_music.entity = new_source.map(|source| {
                            commands
                                .spawn((
                                    SoundCategory::BackgroundMusic,
                                    GlobalSound::new_repeating(source),
                                    sound_settings.gain(SoundCategory::BackgroundMusic),
                                ))
                                .id()
                        });

                        if let Some(old_entity) = old_entity {
                            // Start fading out old music
                            let initial_gain = query_gains
                                .get_mut(old_entity)
                                .ok()
                                .map_or(1.0, |gain| match *gain {
                                    SoundGain::Ratio(factor) => factor,
                                });
                            background_music.state = BackgroundMusicState::FadingOut {
                                old_entity,
                                timer: 0.0,
                                initial_gain,
                            };
                        } else if background_music.entity.is_some() {
                            background_music.state = BackgroundMusicState::PlayingNight;
                        }
                    }
                    BackgroundMusicState::PlayingNight => {}
                    BackgroundMusicState::FadingOut { .. } => {}
                }
            }
        }
    } else if let Some(entity) = background_music.entity.take() {
        commands.entity(entity).despawn();
    }
}
