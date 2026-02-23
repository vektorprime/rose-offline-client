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

    if day_time >= zone_data.night_time || day_time < zone_data.morning_time {
        let state_length = zone_data.morning_time + (zone_data.day_cycle - zone_data.night_time);
        let state_ticks = day_time - zone_data.night_time;

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
    } else if day_time >= zone_data.evening_time {
        let state_length = zone_data.night_time - zone_data.evening_time;
        let state_ticks = day_time - zone_data.evening_time;

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
    } else if day_time >= zone_data.day_time {
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
    } else if day_time >= zone_data.morning_time {
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
