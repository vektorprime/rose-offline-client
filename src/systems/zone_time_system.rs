use bevy::{
    ecs::prelude::{Res, ResMut},
    math::{Vec3, Vec4Swizzles},
    prelude::{Children, Entity, Query, State, Visibility, With},
};

use rose_data::{SkyboxState, WORLD_TICK_DURATION};

use crate::{
    components::NightTimeEffect,
    render::ZoneLighting,
    resources::{AppState, CurrentZone, GameData, WorldTime, ZoneTime, ZoneTimeState},
};

// Note: ZoneLighting is now used from resources::CurrentZone (via zone_lighting.rs)
// It provides all the necessary fields for lighting calculations

// Standard fog colors and densities
const MORNING_FOG_COLOR: Vec3 = Vec3::new(100.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0);
const MORNING_FOG_DENSITY: f32 = 0.0022;

const DAY_FOG_COLOR: Vec3 = Vec3::new(200.0 / 255.0, 200.0 / 255.0, 200.0 / 255.0);
const DAY_FOG_DENSITY: f32 = 0.0018;

const EVENING_FOG_COLOR: Vec3 = Vec3::new(100.0 / 255.0, 100.0 / 255.0, 100.0 / 255.0);
const EVENING_FOG_DENSITY: f32 = 0.0022;

const NIGHT_FOG_COLOR: Vec3 = Vec3::new(10.0 / 255.0, 10.0 / 255.0, 10.0 / 255.0);
const NIGHT_FOG_DENSITY: f32 = 0.0020;

// Volumetric fog colors for time-of-day integration
// These should be bright and subtle - volumetric fog is a light effect, not heavy fog
// Dawn (morning): Warm orange/pink tones (bright)
const VOLUMETRIC_MORNING_COLOR: Vec3 = Vec3::new(1.0, 0.85, 0.7);
// Day: Light blue/white tones (bright, nearly white)
const VOLUMETRIC_DAY_COLOR: Vec3 = Vec3::new(0.9, 0.95, 1.0);
// Dusk (evening): Warm orange/purple tones (bright)
const VOLUMETRIC_EVENING_COLOR: Vec3 = Vec3::new(1.0, 0.7, 0.8);
// Night: Darker blue but still visible (not pitch black)
const VOLUMETRIC_NIGHT_COLOR: Vec3 = Vec3::new(0.3, 0.35, 0.5);

// Volumetric fog density factors for time of day
// Tuned for visible light shafts while maintaining gameplay visibility
// Higher values create more prominent god rays, lower values are more subtle
const VOLUMETRIC_MORNING_DENSITY: f32 = 0.06; // Enhanced morning mist effect
const VOLUMETRIC_DAY_DENSITY: f32 = 0.05; // Balanced for daytime atmosphere
const VOLUMETRIC_EVENING_DENSITY: f32 = 0.06; // Enhanced evening dust particles
const VOLUMETRIC_NIGHT_DENSITY: f32 = 0.03; // Subtle night haze

// TODO: Now that we have Visibility::Inherited, this probably does not need to be recursive ?
fn set_visible_recursive(
    is_visible: bool,
    entity: Entity,
    query_visibility: &mut Query<&mut Visibility>,
    query_children: &Query<&Children>,
) {
    if let Ok(mut visibility) = query_visibility.get_mut(entity) {
        if is_visible {
            *visibility = Visibility::Inherited;
        } else {
            *visibility = Visibility::Hidden;
        }
    }

    if let Ok(children) = query_children.get(entity) {
        for child in children.iter() {
            set_visible_recursive(is_visible, *child, query_visibility, query_children);
        }
    }
}

pub trait SingleLerp {
    fn lerp(self, end: Self, s: f32) -> Self;
}

impl SingleLerp for f32 {
    fn lerp(self, end: Self, s: f32) -> Self {
        self * (1.0 - s) + end * s
    }
}

/// Write a value only when it changed beyond an epsilon.
/// Any `ResMut` write marks the resource changed, so writing identical values
/// every frame defeats every `is_changed()` guard downstream (fog volumes,
/// terrain/water materials, sun position). The epsilon keeps smooth
/// transitions continuous while suppressing redundant writes.
fn write_f32_if_changed(field: &mut f32, value: f32, epsilon: f32) {
    if (*field - value).abs() >= epsilon {
        *field = value;
    }
}

/// Same as [`write_f32_if_changed`] for Vec3 colors (max per-component delta).
fn write_vec3_if_changed(field: &mut Vec3, value: Vec3, epsilon: f32) {
    let delta = *field - value;
    if delta.x.abs() >= epsilon || delta.y.abs() >= epsilon || delta.z.abs() >= epsilon {
        *field = value;
    }
}

pub fn zone_time_system(
    mut zone_lighting: ResMut<ZoneLighting>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    world_time: Res<WorldTime>,
    app_state: Res<State<AppState>>,
    daylight: Res<crate::render::zone_lighting::DaylightSettings>,
    mut zone_time: ResMut<ZoneTime>,
    mut query_night_effects: Query<Entity, With<NightTimeEffect>>,
    mut query_visibility: Query<&mut Visibility>,
    query_children: Query<&Children>,
) {
    if current_zone.is_none() {
        return;
    }
    let current_zone = current_zone.unwrap();
    let zone_data = game_data.zone_list.get_zone(current_zone.id);
    if zone_data.is_none() {
        return;
    }
    let zone_data = zone_data.unwrap();

    // SAFETY: Ensure day_cycle is never zero to prevent division by zero
    // This can happen if zone data is malformed or not properly loaded
    const MIN_DAY_CYCLE: u32 = 1;
    const DEFAULT_DAY_CYCLE: u32 = 160; // Standard 24-hour day cycle
    let safe_day_cycle = if zone_data.day_cycle < MIN_DAY_CYCLE {
        log::warn!(
            "[ZONE_TIME] WARNING: zone_data.day_cycle={} is invalid, using default {}",
            zone_data.day_cycle,
            DEFAULT_DAY_CYCLE
        );
        DEFAULT_DAY_CYCLE
    } else {
        zone_data.day_cycle
    };

    // Debug log time thresholds once when zone changes (or on first run)
    static LAST_LOGGED_ZONE: std::sync::atomic::AtomicU32 =
        std::sync::atomic::AtomicU32::new(u32::MAX);
    let zone_id = current_zone.id.get() as u32;
    if LAST_LOGGED_ZONE.load(std::sync::atomic::Ordering::Relaxed) != zone_id {
        LAST_LOGGED_ZONE.store(zone_id, std::sync::atomic::Ordering::Relaxed);

        // Calculate expected tick values for standard 24-hour day
        let ticks_per_hour = safe_day_cycle as f32 / 24.0;

        log::info!("[ZONE_TIME] ========== ZONE TIME THRESHOLDS ==========");
        log::info!("[ZONE_TIME] Zone {} ({})", zone_id, zone_data.name);
        log::info!(
            "[ZONE_TIME]   day_cycle: {} ticks = 24 hours (safe_day_cycle: {})",
            zone_data.day_cycle,
            safe_day_cycle
        );
        log::info!("[ZONE_TIME]   ticks_per_hour: {:.2} ticks", ticks_per_hour);
        log::info!("[ZONE_TIME]");
        log::info!("[ZONE_TIME]   ACTUAL VALUES FROM STB:");
        log::info!(
            "[ZONE_TIME]     morning_time: {} ticks = {:.1} hours ({:02}:{:02})",
            zone_data.morning_time,
            zone_data.morning_time as f32 / ticks_per_hour,
            (zone_data.morning_time as f32 / ticks_per_hour) as u32,
            ((zone_data.morning_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!(
            "[ZONE_TIME]     day_time: {} ticks = {:.1} hours ({:02}:{:02})",
            zone_data.day_time,
            zone_data.day_time as f32 / ticks_per_hour,
            (zone_data.day_time as f32 / ticks_per_hour) as u32,
            ((zone_data.day_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!(
            "[ZONE_TIME]     evening_time: {} ticks = {:.1} hours ({:02}:{:02})",
            zone_data.evening_time,
            zone_data.evening_time as f32 / ticks_per_hour,
            (zone_data.evening_time as f32 / ticks_per_hour) as u32,
            ((zone_data.evening_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!(
            "[ZONE_TIME]     night_time: {} ticks = {:.1} hours ({:02}:{:02})",
            zone_data.night_time,
            zone_data.night_time as f32 / ticks_per_hour,
            (zone_data.night_time as f32 / ticks_per_hour) as u32,
            ((zone_data.night_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!("[ZONE_TIME]");
        log::info!("[ZONE_TIME]   EXPECTED VALUES (standard 24h day):");
        log::info!(
            "[ZONE_TIME]     morning (6:00): {} ticks",
            safe_day_cycle / 4
        );
        log::info!("[ZONE_TIME]     day (12:00): {} ticks", safe_day_cycle / 2);
        log::info!(
            "[ZONE_TIME]     evening (18:00): {} ticks",
            3 * safe_day_cycle / 4
        );
        log::info!(
            "[ZONE_TIME]     night (22:00): {} ticks",
            22 * safe_day_cycle / 24
        );
        log::info!("[ZONE_TIME] =============================================");
    }
    let skybox_data = zone_data
        .skybox_id
        .and_then(|id| game_data.skybox.get_skybox_data(id));

    // Menu screens (login, character select) always show a fixed midday sky.
    // Without this override, the startup WorldTime seed made the login screen's
    // time-of-day - and thus whether the sky is the daytime atmosphere or the
    // night star field - change randomly on every launch.
    let world_day_time = if matches!(
        app_state.get(),
        AppState::GameLogin | AppState::GameCharacterSelect
    ) {
        safe_day_cycle / 2
    } else {
        world_time.ticks.get_world_time()
    };
    let (day_time, partial_tick) = if let Some(overwrite_time) = zone_time.debug_overwrite_time {
        (overwrite_time, 0.0)
    } else {
        (
            world_day_time % safe_day_cycle,
            world_time.time_since_last_tick.as_secs_f32() / WORLD_TICK_DURATION.as_secs_f32(),
        )
    };

    // Convert day_time to hours for easier debugging (assuming day_cycle represents 24 hours)
    // Use safe_day_cycle to prevent division by zero
    let day_time_hours = (day_time as f32 / safe_day_cycle as f32) * 24.0;

    // Determine time state from the DaylightSettings sunrise/sunset window.
    // Day bounds (11:00 / 17:00) are fixed midday anchors; morning stretches
    // sunrise->11 and evening stretches 17->sunset so the Sky sliders for
    // sunrise/sunset move both the sun disk and these states together.
    //
    // The zone data values are used for tick calculations but NOT for state determination
    // to ensure consistent day/night cycle across all zones.

    let sunrise = daylight.sunrise_hour.clamp(0.0, 24.0);
    let sunset = daylight.sunset_hour.clamp(0.0, 24.0).max(sunrise + 1.0);
    // Day plateau anchors (fixed): full sun between these hours when inside the window.
    const DAY_START_HOUR: f32 = 11.0;
    const DAY_END_HOUR: f32 = 17.0;

    let is_morning = day_time_hours >= sunrise && day_time_hours < DAY_START_HOUR;
    let is_day = day_time_hours >= DAY_START_HOUR && day_time_hours < DAY_END_HOUR;
    let is_evening = day_time_hours >= DAY_END_HOUR && day_time_hours < sunset;
    let is_night = day_time_hours >= sunset || day_time_hours < sunrise;

    if is_night {
        // Night: sunset->sunrise (wraps around midnight)
        let night_length_hours = (24.0 - sunset + sunrise).max(0.5);

        // Calculate state_ticks in hours
        let state_ticks_hours = if day_time_hours >= sunset {
            // We're in the first part of night (sunset to 24:00)
            day_time_hours - sunset
        } else {
            // We're in the second part of night (0:00 to sunrise)
            (24.0 - sunset) + day_time_hours
        };

        if zone_time.state != ZoneTimeState::Night {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(true, entity, &mut query_visibility, &query_children);
            }
            zone_time.state = ZoneTimeState::Night;
        }
        write_f32_if_changed(
            &mut zone_time.state_percent_complete,
            (state_ticks_hours + partial_tick / 24.0) / night_length_hours,
            1e-3,
        );

        // Update volumetric fog for night time
        write_vec3_if_changed(
            &mut zone_lighting.volumetric_fog_color,
            VOLUMETRIC_NIGHT_COLOR,
            1e-4,
        );
        write_f32_if_changed(
            &mut zone_lighting.volumetric_density_factor,
            VOLUMETRIC_NIGHT_DENSITY,
            1e-4,
        );

        if let Some(skybox_data) = skybox_data {
            write_vec3_if_changed(
                &mut zone_lighting.map_ambient_color,
                skybox_data.map_ambient_color[SkyboxState::Night].xyz(),
                1e-4,
            );
            write_vec3_if_changed(
                &mut zone_lighting.character_ambient_color,
                skybox_data.character_ambient_color[SkyboxState::Night].xyz(),
                1e-4,
            );
            write_vec3_if_changed(
                &mut zone_lighting.character_diffuse_color,
                skybox_data.character_diffuse_color[SkyboxState::Night].xyz(),
                1e-4,
            );
            write_vec3_if_changed(&mut zone_lighting.fog_color, NIGHT_FOG_COLOR, 1e-4);
            write_f32_if_changed(&mut zone_lighting.fog_density, NIGHT_FOG_DENSITY, 1e-4);
        }
    } else if is_evening {
        // Evening: DAY_END_HOUR->sunset dusk transition, sun up until sunset.
        let evening_length_hours = (sunset - DAY_END_HOUR).max(0.5);

        // Calculate state_ticks in hours (DAY_END_HOUR is the start)
        let state_ticks_hours = day_time_hours - DAY_END_HOUR;

        if zone_time.state != ZoneTimeState::Evening {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(true, entity, &mut query_visibility, &query_children);
            }
            zone_time.state = ZoneTimeState::Evening;
        }
        write_f32_if_changed(
            &mut zone_time.state_percent_complete,
            (state_ticks_hours + partial_tick / 24.0) / evening_length_hours,
            1e-3,
        );

        // Update volumetric fog for evening/dusk with smooth interpolation
        if zone_time.state_percent_complete < 0.5 {
            // First half: transition from day to evening colors
            write_vec3_if_changed(
                &mut zone_lighting.volumetric_fog_color,
                VOLUMETRIC_DAY_COLOR.lerp(
                    VOLUMETRIC_EVENING_COLOR,
                    zone_time.state_percent_complete * 2.0,
                ),
                1e-4,
            );
            write_f32_if_changed(
                &mut zone_lighting.volumetric_density_factor,
                VOLUMETRIC_DAY_DENSITY.lerp(
                    VOLUMETRIC_EVENING_DENSITY,
                    zone_time.state_percent_complete * 2.0,
                ),
                1e-4,
            );
        } else {
            // Second half: transition from evening to night colors
            write_vec3_if_changed(
                &mut zone_lighting.volumetric_fog_color,
                VOLUMETRIC_EVENING_COLOR.lerp(
                    VOLUMETRIC_NIGHT_COLOR,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                ),
                1e-4,
            );
            write_f32_if_changed(
                &mut zone_lighting.volumetric_density_factor,
                VOLUMETRIC_EVENING_DENSITY.lerp(
                    VOLUMETRIC_NIGHT_DENSITY,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                ),
                1e-4,
            );
        }

        if let Some(skybox_data) = skybox_data {
            if zone_time.state_percent_complete < 0.5 {
                write_vec3_if_changed(
                    &mut zone_lighting.map_ambient_color,
                    skybox_data.map_ambient_color[SkyboxState::Day]
                        .lerp(
                            skybox_data.map_ambient_color[SkyboxState::Evening],
                            zone_time.state_percent_complete * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_ambient_color,
                    skybox_data.character_ambient_color[SkyboxState::Day]
                        .lerp(
                            skybox_data.character_ambient_color[SkyboxState::Evening],
                            zone_time.state_percent_complete * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_diffuse_color,
                    skybox_data.character_diffuse_color[SkyboxState::Day]
                        .lerp(
                            skybox_data.character_diffuse_color[SkyboxState::Evening],
                            zone_time.state_percent_complete * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.fog_color,
                    DAY_FOG_COLOR.lerp(EVENING_FOG_COLOR, zone_time.state_percent_complete * 2.0),
                    1e-4,
                );
                write_f32_if_changed(
                    &mut zone_lighting.fog_density,
                    DAY_FOG_DENSITY
                        .lerp(EVENING_FOG_DENSITY, zone_time.state_percent_complete * 2.0),
                    1e-4,
                );
            } else {
                write_vec3_if_changed(
                    &mut zone_lighting.map_ambient_color,
                    skybox_data.map_ambient_color[SkyboxState::Evening]
                        .lerp(
                            skybox_data.map_ambient_color[SkyboxState::Night],
                            (zone_time.state_percent_complete - 0.5) * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_ambient_color,
                    skybox_data.character_ambient_color[SkyboxState::Evening]
                        .lerp(
                            skybox_data.character_ambient_color[SkyboxState::Night],
                            (zone_time.state_percent_complete - 0.5) * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_diffuse_color,
                    skybox_data.character_diffuse_color[SkyboxState::Evening]
                        .lerp(
                            skybox_data.character_diffuse_color[SkyboxState::Night],
                            (zone_time.state_percent_complete - 0.5) * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.fog_color,
                    EVENING_FOG_COLOR
                        .lerp(NIGHT_FOG_COLOR, (zone_time.state_percent_complete - 0.5) * 2.0),
                    1e-4,
                );
                write_f32_if_changed(
                    &mut zone_lighting.fog_density,
                    EVENING_FOG_DENSITY
                        .lerp(NIGHT_FOG_DENSITY, (zone_time.state_percent_complete - 0.5) * 2.0),
                    1e-4,
                );
            }
        }
    } else if is_day {
        // Day: DAY_START_HOUR-DAY_END_HOUR (full sun high in the sky)
        let day_length_hours = (DAY_END_HOUR - DAY_START_HOUR).max(0.5);

        // Calculate state_ticks in hours (DAY_START_HOUR is the start)
        let state_ticks_hours = day_time_hours - DAY_START_HOUR;

        if zone_time.state != ZoneTimeState::Day {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(false, entity, &mut query_visibility, &query_children);
            }
            zone_time.state = ZoneTimeState::Day;
        }
        write_f32_if_changed(
            &mut zone_time.state_percent_complete,
            (state_ticks_hours + partial_tick / 24.0) / day_length_hours,
            1e-3,
        );

        // Update volumetric fog for day time
        write_vec3_if_changed(&mut zone_lighting.volumetric_fog_color, VOLUMETRIC_DAY_COLOR, 1e-4);
        write_f32_if_changed(
            &mut zone_lighting.volumetric_density_factor,
            VOLUMETRIC_DAY_DENSITY,
            1e-4,
        );

        if let Some(skybox_data) = skybox_data {
            write_vec3_if_changed(
                &mut zone_lighting.map_ambient_color,
                skybox_data.map_ambient_color[SkyboxState::Day].xyz(),
                1e-4,
            );
            write_vec3_if_changed(
                &mut zone_lighting.character_ambient_color,
                skybox_data.character_ambient_color[SkyboxState::Day].xyz(),
                1e-4,
            );
            write_vec3_if_changed(
                &mut zone_lighting.character_diffuse_color,
                skybox_data.character_diffuse_color[SkyboxState::Day].xyz(),
                1e-4,
            );
            write_vec3_if_changed(&mut zone_lighting.fog_color, DAY_FOG_COLOR, 1e-4);
            write_f32_if_changed(&mut zone_lighting.fog_density, DAY_FOG_DENSITY, 1e-4);
        }
    } else if is_morning {
        // Morning: sunrise->DAY_START_HOUR (sunrise ramp to bright plateau)
        let morning_length_hours = (DAY_START_HOUR - sunrise).max(0.5);

        // Calculate state_ticks in hours (sunrise is the start)
        let state_ticks_hours = day_time_hours - sunrise;

        if zone_time.state != ZoneTimeState::Morning {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(false, entity, &mut query_visibility, &query_children);
            }
            zone_time.state = ZoneTimeState::Morning;
        }
        write_f32_if_changed(
            &mut zone_time.state_percent_complete,
            (state_ticks_hours + partial_tick / 24.0) / morning_length_hours,
            1e-3,
        );

        // Update volumetric fog for morning/dawn with smooth interpolation
        if zone_time.state_percent_complete < 0.5 {
            // First half: transition from night to morning colors
            write_vec3_if_changed(
                &mut zone_lighting.volumetric_fog_color,
                VOLUMETRIC_NIGHT_COLOR.lerp(
                    VOLUMETRIC_MORNING_COLOR,
                    zone_time.state_percent_complete * 2.0,
                ),
                1e-4,
            );
            write_f32_if_changed(
                &mut zone_lighting.volumetric_density_factor,
                VOLUMETRIC_NIGHT_DENSITY.lerp(
                    VOLUMETRIC_MORNING_DENSITY,
                    zone_time.state_percent_complete * 2.0,
                ),
                1e-4,
            );
        } else {
            // Second half: transition from morning to day colors
            write_vec3_if_changed(
                &mut zone_lighting.volumetric_fog_color,
                VOLUMETRIC_MORNING_COLOR.lerp(
                    VOLUMETRIC_DAY_COLOR,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                ),
                1e-4,
            );
            write_f32_if_changed(
                &mut zone_lighting.volumetric_density_factor,
                VOLUMETRIC_MORNING_DENSITY.lerp(
                    VOLUMETRIC_DAY_DENSITY,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                ),
                1e-4,
            );
        }

        if let Some(skybox_data) = skybox_data {
            if zone_time.state_percent_complete < 0.5 {
                write_vec3_if_changed(
                    &mut zone_lighting.map_ambient_color,
                    skybox_data.map_ambient_color[SkyboxState::Night]
                        .lerp(
                            skybox_data.map_ambient_color[SkyboxState::Morning],
                            zone_time.state_percent_complete * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_ambient_color,
                    skybox_data.character_ambient_color[SkyboxState::Night]
                        .lerp(
                            skybox_data.character_ambient_color[SkyboxState::Morning],
                            zone_time.state_percent_complete * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_diffuse_color,
                    skybox_data.character_diffuse_color[SkyboxState::Night]
                        .lerp(
                            skybox_data.character_diffuse_color[SkyboxState::Morning],
                            zone_time.state_percent_complete * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.fog_color,
                    NIGHT_FOG_COLOR
                        .lerp(MORNING_FOG_COLOR, zone_time.state_percent_complete * 2.0),
                    1e-4,
                );
                write_f32_if_changed(
                    &mut zone_lighting.fog_density,
                    NIGHT_FOG_DENSITY
                        .lerp(MORNING_FOG_DENSITY, zone_time.state_percent_complete * 2.0),
                    1e-4,
                );
            } else {
                write_vec3_if_changed(
                    &mut zone_lighting.map_ambient_color,
                    skybox_data.map_ambient_color[SkyboxState::Morning]
                        .lerp(
                            skybox_data.map_ambient_color[SkyboxState::Day],
                            (zone_time.state_percent_complete - 0.5) * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_ambient_color,
                    skybox_data.character_ambient_color[SkyboxState::Morning]
                        .lerp(
                            skybox_data.character_ambient_color[SkyboxState::Day],
                            (zone_time.state_percent_complete - 0.5) * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.character_diffuse_color,
                    skybox_data.character_diffuse_color[SkyboxState::Morning]
                        .lerp(
                            skybox_data.character_diffuse_color[SkyboxState::Day],
                            (zone_time.state_percent_complete - 0.5) * 2.0,
                        )
                        .xyz(),
                    1e-4,
                );
                write_vec3_if_changed(
                    &mut zone_lighting.fog_color,
                    MORNING_FOG_COLOR.lerp(
                        DAY_FOG_COLOR,
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    ),
                    1e-4,
                );
                write_f32_if_changed(
                    &mut zone_lighting.fog_density,
                    MORNING_FOG_DENSITY.lerp(
                        DAY_FOG_DENSITY,
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    ),
                    1e-4,
                );
            }
        }
    }

    // Only write the tick value when it actually advanced (it changes at most
    // once per world tick), so `zone_time.is_changed()` stops being always true
    if zone_time.time != day_time {
        zone_time.time = day_time;
    }
}
