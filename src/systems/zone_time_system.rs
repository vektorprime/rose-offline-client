use bevy::{
    ecs::change_detection::DetectChanges,
    ecs::prelude::{Res, ResMut},
    math::{Vec3, Vec4Swizzles},
    prelude::{Children, Entity, Query, Visibility, With},
    render::view::{ColorGrading, ColorGradingGlobal, ColorGradingSection},
};

use rose_data::{SkyboxState, WORLD_TICK_DURATION};

use crate::{
    components::NightTimeEffect,
    render::ZoneLighting,
    resources::{CurrentZone, GameData, WorldTime, ZoneTime, ZoneTimeState},
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
const VOLUMETRIC_MORNING_DENSITY: f32 = 0.06;   // Enhanced morning mist effect
const VOLUMETRIC_DAY_DENSITY: f32 = 0.05;       // Balanced for daytime atmosphere
const VOLUMETRIC_EVENING_DENSITY: f32 = 0.06;   // Enhanced evening dust particles
const VOLUMETRIC_NIGHT_DENSITY: f32 = 0.03;     // Subtle night haze

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

pub fn zone_time_system(
    mut zone_lighting: ResMut<ZoneLighting>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    world_time: Res<WorldTime>,
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
    
    // Debug log time thresholds once when zone changes (or on first run)
    static LAST_LOGGED_ZONE: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
    let zone_id = current_zone.id.get() as u32;
    if LAST_LOGGED_ZONE.load(std::sync::atomic::Ordering::Relaxed) != zone_id {
        LAST_LOGGED_ZONE.store(zone_id, std::sync::atomic::Ordering::Relaxed);
        
        // Calculate expected tick values for standard 24-hour day
        let ticks_per_hour = zone_data.day_cycle as f32 / 24.0;
        
        log::info!("[ZONE_TIME] ========== ZONE TIME THRESHOLDS ==========");
        log::info!("[ZONE_TIME] Zone {} ({})", zone_id, zone_data.name);
        log::info!("[ZONE_TIME]   day_cycle: {} ticks = 24 hours", zone_data.day_cycle);
        log::info!("[ZONE_TIME]   ticks_per_hour: {:.2} ticks", ticks_per_hour);
        log::info!("[ZONE_TIME]");
        log::info!("[ZONE_TIME]   ACTUAL VALUES FROM STB:");
        log::info!("[ZONE_TIME]     morning_time: {} ticks = {:.1} hours ({:02}:{:02})", 
            zone_data.morning_time,
            zone_data.morning_time as f32 / ticks_per_hour,
            (zone_data.morning_time as f32 / ticks_per_hour) as u32,
            ((zone_data.morning_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!("[ZONE_TIME]     day_time: {} ticks = {:.1} hours ({:02}:{:02})", 
            zone_data.day_time,
            zone_data.day_time as f32 / ticks_per_hour,
            (zone_data.day_time as f32 / ticks_per_hour) as u32,
            ((zone_data.day_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!("[ZONE_TIME]     evening_time: {} ticks = {:.1} hours ({:02}:{:02})", 
            zone_data.evening_time,
            zone_data.evening_time as f32 / ticks_per_hour,
            (zone_data.evening_time as f32 / ticks_per_hour) as u32,
            ((zone_data.evening_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!("[ZONE_TIME]     night_time: {} ticks = {:.1} hours ({:02}:{:02})", 
            zone_data.night_time,
            zone_data.night_time as f32 / ticks_per_hour,
            (zone_data.night_time as f32 / ticks_per_hour) as u32,
            ((zone_data.night_time as f32 / ticks_per_hour % 1.0) * 60.0) as u32
        );
        log::info!("[ZONE_TIME]");
        log::info!("[ZONE_TIME]   EXPECTED VALUES (standard 24h day):");
        log::info!("[ZONE_TIME]     morning (6:00): {} ticks", zone_data.day_cycle / 4);
        log::info!("[ZONE_TIME]     day (12:00): {} ticks", zone_data.day_cycle / 2);
        log::info!("[ZONE_TIME]     evening (18:00): {} ticks", 3 * zone_data.day_cycle / 4);
        log::info!("[ZONE_TIME]     night (22:00): {} ticks", 22 * zone_data.day_cycle / 24);
        log::info!("[ZONE_TIME] =============================================");
    }
    let skybox_data = zone_data
        .skybox_id
        .and_then(|id| game_data.skybox.get_skybox_data(id));

    let world_day_time = world_time.ticks.get_world_time();
    let (day_time, partial_tick) = if let Some(overwrite_time) = zone_time.debug_overwrite_time {
        (overwrite_time, 0.0)
    } else {
        (
            world_day_time % zone_data.day_cycle,
            world_time.time_since_last_tick.as_secs_f32() / WORLD_TICK_DURATION.as_secs_f32(),
        )
    };
    
    // Convert day_time to hours for easier debugging (assuming day_cycle represents 24 hours)
    let day_time_hours = (day_time as f32 / zone_data.day_cycle as f32) * 24.0;
    let hours = day_time_hours.floor() as u32;
    let minutes = ((day_time_hours - hours as f32) * 60.0) as u32;
    
    // Log current time every 60 frames (~1 second) for debugging
    static FRAME_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let should_log = frame % 60 == 0;
    
    if should_log {
        log::info!("[ZONE_TIME] ========== CURRENT TIME ==========");
        log::info!("[ZONE_TIME]   tick: {} / {}", day_time, zone_data.day_cycle);
        log::info!("[ZONE_TIME]   game time: {:02}:{:02}", hours, minutes);
        log::info!("[ZONE_TIME]   state: {:?}", zone_time.state);
        log::info!("[ZONE_TIME]   state_percent: {:.1}%", zone_time.state_percent_complete * 100.0);
    }
    
    // Determine time state based on thresholds
    // The game supports two configurations:
    // 1. Wrap-around: night_time > morning_time (e.g., night=133, morning=27)
    //    Night wraps around midnight: 133-160 AND 0-27
    // 2. Non-wrap: night_time < morning_time (e.g., night=0, morning=40)
    //    Night is a simple range: 0-40
    
    let is_night = if zone_data.night_time >= zone_data.morning_time {
        // Night wraps around midnight: time >= night_time OR time < morning_time
        day_time >= zone_data.night_time || day_time < zone_data.morning_time
    } else {
        // Night doesn't wrap: time >= night_time AND time < morning_time
        day_time >= zone_data.night_time && day_time < zone_data.morning_time
    };
    
    // For evening, handle the case where night_time might wrap around
    let is_evening = if zone_data.night_time >= zone_data.evening_time {
        // Normal case: evening_time to night_time is a simple range
        day_time >= zone_data.evening_time && day_time < zone_data.night_time
    } else {
        // Wrap case: evening wraps around (evening_time to day_cycle, then 0 to night_time)
        day_time >= zone_data.evening_time || day_time < zone_data.night_time
    };
    
    // Day and morning are always simple ranges (no wrap-around in standard configs)
    let is_day = day_time >= zone_data.day_time && day_time < zone_data.evening_time;
    let is_morning = day_time >= zone_data.morning_time && day_time < zone_data.day_time;
    
    if should_log {
        log::info!("[ZONE_TIME]   State checks:");
        log::info!("[ZONE_TIME]     is_night: {} (time >= {} || time < {})", is_night, zone_data.night_time, zone_data.morning_time);
        log::info!("[ZONE_TIME]     is_evening: {} (time >= {} && time < {})", is_evening, zone_data.evening_time, zone_data.night_time);
        log::info!("[ZONE_TIME]     is_day: {} (time >= {} && time < {})", is_day, zone_data.day_time, zone_data.evening_time);
        log::info!("[ZONE_TIME]     is_morning: {} (time >= {} && time < {})", is_morning, zone_data.morning_time, zone_data.day_time);
        log::info!("[ZONE_TIME] ======================================");
    }

    if is_night {
        // Calculate state_length and state_ticks, handling wrap-around
        let state_length = if zone_data.night_time >= zone_data.morning_time {
            // Night wraps around: night_time to end of day, then 0 to morning_time
            zone_data.morning_time + (zone_data.day_cycle - zone_data.night_time)
        } else {
            // Night doesn't wrap: simple range from night_time to morning_time
            zone_data.morning_time - zone_data.night_time
        };
        
        // Calculate state_ticks, handling wrap-around
        let state_ticks = if day_time >= zone_data.night_time {
            // We're in the first part of night (after night_time)
            day_time - zone_data.night_time
        } else {
            // We're in the second part of night (before morning_time, after midnight)
            (zone_data.day_cycle - zone_data.night_time) + day_time
        };

        if zone_time.state != ZoneTimeState::Night {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(true, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Night;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        // Update volumetric fog for night time
        zone_lighting.volumetric_fog_color = VOLUMETRIC_NIGHT_COLOR;
        zone_lighting.volumetric_density_factor = VOLUMETRIC_NIGHT_DENSITY;

        if let Some(skybox_data) = skybox_data {
            zone_lighting.map_ambient_color =
                skybox_data.map_ambient_color[SkyboxState::Night].xyz();
            zone_lighting.character_ambient_color =
                skybox_data.character_ambient_color[SkyboxState::Night].xyz();
            zone_lighting.character_diffuse_color =
                skybox_data.character_diffuse_color[SkyboxState::Night].xyz();
            zone_lighting.fog_color = NIGHT_FOG_COLOR;
            zone_lighting.fog_density = NIGHT_FOG_DENSITY;
        }
    } else if is_evening {
        // Calculate state_length and state_ticks, handling wrap-around
        let state_length = if zone_data.night_time >= zone_data.evening_time {
            // Normal case: evening_time to night_time is a simple range
            zone_data.night_time - zone_data.evening_time
        } else {
            // Wrap case: evening_time to end of day, then 0 to night_time
            (zone_data.day_cycle - zone_data.evening_time) + zone_data.night_time
        };
        
        // Calculate state_ticks, handling wrap-around
        let state_ticks = if day_time >= zone_data.evening_time {
            // We're in the first part of evening (after evening_time)
            day_time - zone_data.evening_time
        } else {
            // We're in the second part of evening (before night_time, after midnight)
            (zone_data.day_cycle - zone_data.evening_time) + day_time
        };

        if zone_time.state != ZoneTimeState::Evening {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(true, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Evening;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        // Update volumetric fog for evening/dusk with smooth interpolation
        if zone_time.state_percent_complete < 0.5 {
            // First half: transition from day to evening colors
            zone_lighting.volumetric_fog_color = VOLUMETRIC_DAY_COLOR.lerp(
                VOLUMETRIC_EVENING_COLOR,
                zone_time.state_percent_complete * 2.0,
            );
            zone_lighting.volumetric_density_factor = VOLUMETRIC_DAY_DENSITY
                .lerp(VOLUMETRIC_EVENING_DENSITY, zone_time.state_percent_complete * 2.0);
        } else {
            // Second half: transition from evening to night colors
            zone_lighting.volumetric_fog_color = VOLUMETRIC_EVENING_COLOR.lerp(
                VOLUMETRIC_NIGHT_COLOR,
                (zone_time.state_percent_complete - 0.5) * 2.0,
            );
            zone_lighting.volumetric_density_factor = VOLUMETRIC_EVENING_DENSITY
                .lerp(VOLUMETRIC_NIGHT_DENSITY, (zone_time.state_percent_complete - 0.5) * 2.0);
        }

        if let Some(skybox_data) = skybox_data {
            if zone_time.state_percent_complete < 0.5 {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color[SkyboxState::Day]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Evening],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Day]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Evening],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Day]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Evening],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color =
                    DAY_FOG_COLOR.lerp(EVENING_FOG_COLOR, zone_time.state_percent_complete * 2.0);
                zone_lighting.fog_density = DAY_FOG_DENSITY
                    .lerp(EVENING_FOG_DENSITY, zone_time.state_percent_complete * 2.0);
            } else {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color
                    [SkyboxState::Evening]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Night],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Evening]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Night],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Evening]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Night],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color = EVENING_FOG_COLOR.lerp(
                    NIGHT_FOG_COLOR,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
                zone_lighting.fog_density = EVENING_FOG_DENSITY.lerp(
                    NIGHT_FOG_DENSITY,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
            }
        }
    } else if is_day {
        let state_length = zone_data.evening_time - zone_data.day_time;
        let state_ticks = day_time - zone_data.day_time;

        if zone_time.state != ZoneTimeState::Day {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(false, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Day;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        // Update volumetric fog for day time
        zone_lighting.volumetric_fog_color = VOLUMETRIC_DAY_COLOR;
        zone_lighting.volumetric_density_factor = VOLUMETRIC_DAY_DENSITY;

        if let Some(skybox_data) = skybox_data {
            zone_lighting.map_ambient_color = skybox_data.map_ambient_color[SkyboxState::Day].xyz();
            zone_lighting.character_ambient_color =
                skybox_data.character_ambient_color[SkyboxState::Day].xyz();
            zone_lighting.character_diffuse_color =
                skybox_data.character_diffuse_color[SkyboxState::Day].xyz();
            zone_lighting.fog_color = DAY_FOG_COLOR;
            zone_lighting.fog_density = DAY_FOG_DENSITY;
        }
    } else if is_morning {
        let state_length = zone_data.day_time - zone_data.morning_time;
        let state_ticks = day_time - zone_data.morning_time;

        if zone_time.state != ZoneTimeState::Morning {
            for entity in query_night_effects.iter_mut() {
                set_visible_recursive(false, entity, &mut query_visibility, &query_children);
            }
        }

        zone_time.state = ZoneTimeState::Morning;
        zone_time.state_percent_complete =
            (state_ticks as f32 + partial_tick) / state_length as f32;

        // Update volumetric fog for morning/dawn with smooth interpolation
        if zone_time.state_percent_complete < 0.5 {
            // First half: transition from night to morning colors
            zone_lighting.volumetric_fog_color = VOLUMETRIC_NIGHT_COLOR.lerp(
                VOLUMETRIC_MORNING_COLOR,
                zone_time.state_percent_complete * 2.0,
            );
            zone_lighting.volumetric_density_factor = VOLUMETRIC_NIGHT_DENSITY
                .lerp(VOLUMETRIC_MORNING_DENSITY, zone_time.state_percent_complete * 2.0);
        } else {
            // Second half: transition from morning to day colors
            zone_lighting.volumetric_fog_color = VOLUMETRIC_MORNING_COLOR.lerp(
                VOLUMETRIC_DAY_COLOR,
                (zone_time.state_percent_complete - 0.5) * 2.0,
            );
            zone_lighting.volumetric_density_factor = VOLUMETRIC_MORNING_DENSITY
                .lerp(VOLUMETRIC_DAY_DENSITY, (zone_time.state_percent_complete - 0.5) * 2.0);
        }

        if let Some(skybox_data) = skybox_data {
            if zone_time.state_percent_complete < 0.5 {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color[SkyboxState::Night]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Morning],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Night]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Morning],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Night]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Morning],
                        zone_time.state_percent_complete * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color =
                    NIGHT_FOG_COLOR.lerp(MORNING_FOG_COLOR, zone_time.state_percent_complete * 2.0);
                zone_lighting.fog_density = NIGHT_FOG_DENSITY
                    .lerp(MORNING_FOG_DENSITY, zone_time.state_percent_complete * 2.0);
            } else {
                zone_lighting.map_ambient_color = skybox_data.map_ambient_color
                    [SkyboxState::Morning]
                    .lerp(
                        skybox_data.map_ambient_color[SkyboxState::Day],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_ambient_color = skybox_data.character_ambient_color
                    [SkyboxState::Morning]
                    .lerp(
                        skybox_data.character_ambient_color[SkyboxState::Day],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.character_diffuse_color = skybox_data.character_diffuse_color
                    [SkyboxState::Morning]
                    .lerp(
                        skybox_data.character_diffuse_color[SkyboxState::Day],
                        (zone_time.state_percent_complete - 0.5) * 2.0,
                    )
                    .xyz();
                zone_lighting.fog_color = MORNING_FOG_COLOR.lerp(
                    DAY_FOG_COLOR,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
                zone_lighting.fog_density = MORNING_FOG_DENSITY.lerp(
                    DAY_FOG_DENSITY,
                    (zone_time.state_percent_complete - 0.5) * 2.0,
                );
            }
        }
    }

    zone_time.time = day_time;
}

// Color grading temperature values for time-of-day
// Positive = warmer (redder), Negative = cooler (bluer)
// Values significantly reduced for subtle effect
const COLOR_GRADING_MORNING_TEMPERATURE: f32 = 0.03;  // Subtle warm sunrise tones
const COLOR_GRADING_DAY_TEMPERATURE: f32 = 0.0;        // Neutral daylight
const COLOR_GRADING_EVENING_TEMPERATURE: f32 = 0.04;   // Subtle warm sunset tones
const COLOR_GRADING_NIGHT_TEMPERATURE: f32 = -0.02;    // Subtle cool moonlight

// Saturation values for time-of-day
// Values significantly reduced for subtle effect
const COLOR_GRADING_MORNING_SATURATION: f32 = 1.02;    // Subtle vibrant morning colors
const COLOR_GRADING_DAY_SATURATION: f32 = 1.01;         // Very subtle vibrant daytime
const COLOR_GRADING_EVENING_SATURATION: f32 = 1.03;     // Subtle rich sunset colors
const COLOR_GRADING_NIGHT_SATURATION: f32 = 0.98;       // Subtle muted night colors

/// System to update color grading based on time-of-day
/// This creates dynamic color adjustments for warmer tones at sunrise/sunset
/// and cooler tones at night
pub fn color_grading_time_of_day_system(
    zone_time: Res<ZoneTime>,
    mut query: Query<&mut ColorGrading>,
) {
    // Only update if zone_time has changed
    if !zone_time.is_changed() {
        return;
    }

    for mut color_grading in query.iter_mut() {
        let (temperature, saturation) = match zone_time.state {
            ZoneTimeState::Morning => {
                // Transition from night to morning to day
                let t = zone_time.state_percent_complete;
                if t < 0.5 {
                    // Night to morning
                    let lerp_t = t * 2.0;
                    (
                        COLOR_GRADING_NIGHT_TEMPERATURE.lerp(COLOR_GRADING_MORNING_TEMPERATURE, lerp_t),
                        COLOR_GRADING_NIGHT_SATURATION.lerp(COLOR_GRADING_MORNING_SATURATION, lerp_t),
                    )
                } else {
                    // Morning to day
                    let lerp_t = (t - 0.5) * 2.0;
                    (
                        COLOR_GRADING_MORNING_TEMPERATURE.lerp(COLOR_GRADING_DAY_TEMPERATURE, lerp_t),
                        COLOR_GRADING_MORNING_SATURATION.lerp(COLOR_GRADING_DAY_SATURATION, lerp_t),
                    )
                }
            }
            ZoneTimeState::Day => {
                (
                    COLOR_GRADING_DAY_TEMPERATURE,
                    COLOR_GRADING_DAY_SATURATION,
                )
            }
            ZoneTimeState::Evening => {
                // Transition from day to evening to night
                let t = zone_time.state_percent_complete;
                if t < 0.5 {
                    // Day to evening
                    let lerp_t = t * 2.0;
                    (
                        COLOR_GRADING_DAY_TEMPERATURE.lerp(COLOR_GRADING_EVENING_TEMPERATURE, lerp_t),
                        COLOR_GRADING_DAY_SATURATION.lerp(COLOR_GRADING_EVENING_SATURATION, lerp_t),
                    )
                } else {
                    // Evening to night
                    let lerp_t = (t - 0.5) * 2.0;
                    (
                        COLOR_GRADING_EVENING_TEMPERATURE.lerp(COLOR_GRADING_NIGHT_TEMPERATURE, lerp_t),
                        COLOR_GRADING_EVENING_SATURATION.lerp(COLOR_GRADING_NIGHT_SATURATION, lerp_t),
                    )
                }
            }
            ZoneTimeState::Night => {
                (
                    COLOR_GRADING_NIGHT_TEMPERATURE,
                    COLOR_GRADING_NIGHT_SATURATION,
                )
            }
        };

        // Apply the time-of-day color grading adjustments
        color_grading.global.temperature = temperature;
        color_grading.global.post_saturation = saturation;

        // Also adjust shadow lift based on time of day
        // At night, lift shadows slightly to prevent crushed blacks
        // During day, keep shadows more contrasty
        let shadow_lift = match zone_time.state {
            ZoneTimeState::Night => 0.05,
            ZoneTimeState::Morning | ZoneTimeState::Evening => {
                0.02.lerp(0.05, zone_time.state_percent_complete)
            }
            ZoneTimeState::Day => 0.02,
        };
        color_grading.shadows.lift = shadow_lift;
    }
}
