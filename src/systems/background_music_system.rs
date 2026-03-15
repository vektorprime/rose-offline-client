use bevy::prelude::{AssetServer, Commands, Entity, Handle, Local, Query, Res};
use rose_data::ZoneId;

use crate::{
    audio::{AudioSource, GlobalSound},
    components::SoundCategory,
    resources::{CurrentZone, GameData, SoundSettings, ZoneTime, ZoneTimeState},
};

const CROSSFADE_DURATION_MS: u64 = 2000;

#[derive(Default)]
pub enum BackgroundMusicState {
    #[default]
    None,
    PlayingDay,
    PlayingNight,
    FadingOut {
        old_entity: Entity,
        new_source: Option<Handle<AudioSource>>,
        timer_ms: u64,
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
    mut query_global_sounds: Query<&mut GlobalSound>,
) {
    if let Some(current_zone) = current_zone {
        if background_music.zone != Some(current_zone.id) {
            if let Some(entity) = background_music.entity.take() {
                commands.entity(entity).despawn();
            }
            background_music.state = BackgroundMusicState::None;

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
        if let BackgroundMusicState::FadingOut {
            old_entity,
            new_source,
            timer_ms,
        } = &mut background_music.state
        {
            *timer_ms += 16; // Approximate frame time
            if *timer_ms >= CROSSFADE_DURATION_MS {
                // Fade complete, despawn old entity
                commands.entity(*old_entity).despawn();
                background_music.state = if let Some(source) = new_source.take() {
                    background_music.entity = Some(
                        commands
                            .spawn((
                                SoundCategory::BackgroundMusic,
                                GlobalSound::new_repeating(source),
                                sound_settings.gain(SoundCategory::BackgroundMusic),
                            ))
                            .id(),
                    );
                    if zone_time.state == ZoneTimeState::Morning
                        || zone_time.state == ZoneTimeState::Day
                    {
                        BackgroundMusicState::PlayingDay
                    } else {
                        BackgroundMusicState::PlayingNight
                    }
                } else {
                    BackgroundMusicState::None
                };
            }
        }

        match zone_time.state {
            ZoneTimeState::Morning | ZoneTimeState::Day => {
                match &background_music.state {
                    BackgroundMusicState::None | BackgroundMusicState::PlayingNight => {
                        let old_entity = background_music.entity.take();
                        let new_source = background_music.day_audio_source.clone();

                        if let Some(old_entity) = old_entity {
                            // Start fading out old music
                            background_music.state = BackgroundMusicState::FadingOut {
                                old_entity,
                                new_source,
                                timer_ms: 0,
                            };
                        } else {
                            // No old music, just start new one
                            if let Some(audio_source) = new_source {
                                background_music.entity = Some(
                                    commands
                                        .spawn((
                                            SoundCategory::BackgroundMusic,
                                            GlobalSound::new_repeating(audio_source),
                                            sound_settings.gain(SoundCategory::BackgroundMusic),
                                        ))
                                        .id(),
                                );
                            }
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

                        if let Some(old_entity) = old_entity {
                            // Start fading out old music
                            background_music.state = BackgroundMusicState::FadingOut {
                                old_entity,
                                new_source,
                                timer_ms: 0,
                            };
                        } else {
                            // No old music, just start new one
                            if let Some(audio_source) = new_source {
                                background_music.entity = Some(
                                    commands
                                        .spawn((
                                            SoundCategory::BackgroundMusic,
                                            GlobalSound::new_repeating(audio_source),
                                            sound_settings.gain(SoundCategory::BackgroundMusic),
                                        ))
                                        .id(),
                                );
                            }
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
