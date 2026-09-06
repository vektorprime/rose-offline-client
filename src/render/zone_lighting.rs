use crate::graphics::{GraphicsSettings, ShadowQuality};
use crate::render::starry_sky_material::MoonLight;
use crate::resources::{ZoneTime, ZoneTimeState};
use bevy::camera::visibility::RenderLayers;
use bevy::{
    asset::{load_internal_asset, weak_handle, Handle},
    ecs::component::Component,
    light::{CascadeShadowConfig, FogVolume, VolumetricLight},
    math::{Vec3, Vec4},
    prelude::{
        App, Color, ColorToComponents, Commands, DetectChanges, Dir3, DirectionalLight, EulerRot,
        FromWorld, GlobalAmbientLight, GlobalTransform, IntoScheduleConfigs, LinearRgba, Plugin,
        Quat, Query, ReflectResource, Res, ResMut, Resource, Shader, Startup, Transform, Update,
        With, Without, World,
    },
    reflect::{Reflect, TypePath},
    render::{
        render_resource::{
            encase, BindGroup, BindGroupEntry, BindGroupLayout, BindGroupLayoutEntry, BindingType,
            Buffer, BufferBindingType, BufferDescriptor, BufferUsages, ShaderSize, ShaderStages,
            ShaderType,
        },
        renderer::{RenderDevice, RenderQueue},
        Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
    },
};

/// Marker component for the volumetric fog volume entity.
/// Used to query the fog volume for modifications or removal.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct VolumetricFogVolume;

/// Marker component for the sky-bounce fill light (no shadows).
/// This is a low-intensity directional light from a fixed high angle that
/// lifts the shadow side of characters so faces/bodies stay readable when
/// backlit by the sun. Intensity is scaled by daylight (0 at night).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SkyFillLight;

/// Daylight window (game hours) shared by the sun path, shadow gating and
/// ambient scaling. 05:00-20:00 gives ~15h of usable light with solar noon
/// at 12.5h; night is 20:00-05:00 (9h).
pub const SUNRISE_HOUR: f32 = 5.0;
pub const SUNSET_HOUR: f32 = 20.0;
pub const SOLAR_NOON_HOUR: f32 = 12.5;
/// Peak solar elevation above the horizon at noon (degrees). The old Euler
/// path peaked at ~29 deg (NdotL ~0.48 on flat ground), which is why mornings
/// looked dim until nearly noon. 68 deg gives NdotL ~0.93 at midday with a
/// broad bright plateau from ~08:00-17:00.
pub const SUN_MAX_ELEVATION_DEG: f32 = 68.0;
/// Peak sun illuminance (lux) applied when the sun is well above the horizon.
/// Raised from 15000: with the old low sun path the effective ground-level
/// light was only ~7200 lux even at its peak.
pub const SUN_MAX_ILLUMINANCE: f32 = 25000.0;
/// Peak sky-fill illuminance (lux) at midday. ~25% of the sun gives a ~4:1
/// key-to-fill ratio: shadows stay visible but faces are readable.
pub const FILL_MAX_ILLUMINANCE: f32 = 6000.0;

/// Runtime-tunable daylight parameters (Settings > Sky). Defaults match the
/// constants above so out-of-the-box visuals are unchanged.
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct DaylightSettings {
    /// Sunrise hour (game hours, 0-24). Default 5.0.
    pub sunrise_hour: f32,
    /// Sunset hour (game hours, 0-24). Must stay above sunrise. Default 20.0.
    pub sunset_hour: f32,
    /// Peak solar elevation at noon (degrees). Default 68.0.
    pub max_elevation_deg: f32,
    /// Peak sun illuminance in lux. Default 25000.0.
    pub sun_illuminance: f32,
    /// Peak sky-fill illuminance in lux. Default 6000.0.
    pub fill_illuminance: f32,
}

impl Default for DaylightSettings {
    fn default() -> Self {
        Self {
            sunrise_hour: SUNRISE_HOUR,
            sunset_hour: SUNSET_HOUR,
            max_elevation_deg: SUN_MAX_ELEVATION_DEG,
            sun_illuminance: SUN_MAX_ILLUMINANCE,
            fill_illuminance: FILL_MAX_ILLUMINANCE,
        }
    }
}

/// Mode for controlling how the time of day is determined.
#[derive(Reflect, Clone, Copy, PartialEq, Debug, Default)]
pub enum SkyMode {
    /// Time of day follows the game's ZoneTime resource automatically
    #[default]
    Automatic,
    /// Time of day is manually controlled by the user via SkySettings.manual_time
    Manual,
}

/// Resource for controlling sky and time-of-day settings.
/// Allows players to manually set the time or let it follow game time automatically.
#[derive(Resource, Reflect, Clone)]
#[reflect(Resource)]
pub struct SkySettings {
    /// Whether time is automatic (follows game time) or manual (user-controlled)
    pub mode: SkyMode,
    /// Manual time value in hours (0-24) when mode is Manual
    pub manual_time: f32,
    /// Multiplier for atmosphere scattering intensity (0.0-2.0)
    /// Values > 1.0 make the sky more dramatic, < 1.0 makes it more subtle
    pub atmosphere_intensity: f32,
}

impl Default for SkySettings {
    fn default() -> Self {
        Self {
            mode: SkyMode::Automatic,
            manual_time: 12.0, // Default to noon
            atmosphere_intensity: 1.0,
        }
    }
}

pub const ZONE_LIGHTING_SHADER_HANDLE_TYPED: Handle<Shader> =
    weak_handle!("444949d3-2b35-d5d9-0000-000000000000");

fn default_light_transform() -> Transform {
    Transform::from_rotation(Quat::from_euler(
        EulerRot::ZYX,
        0.0,
        std::f32::consts::PI * (2.0 / 3.0),
        -std::f32::consts::PI / 4.0,
    ))
}

#[derive(Default)]
pub struct ZoneLightingPlugin;

impl Plugin for ZoneLightingPlugin {
    fn build(&self, app: &mut App) {
        // bevy::log::info!("[ZONE LIGHTING] Building ZoneLightingPlugin");

        load_internal_asset!(
            app,
            ZONE_LIGHTING_SHADER_HANDLE_TYPED,
            "shaders/zone_lighting.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ZoneLighting>()
            .register_type::<SkySettings>()
            .register_type::<SkyMode>()
            .register_type::<DaylightSettings>()
            .init_resource::<ZoneLighting>()
            .init_resource::<SkySettings>()
            .init_resource::<DaylightSettings>();

        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            // bevy::log::info!("[ZONE LIGHTING] Initializing render app systems");
            render_app
                .add_systems(ExtractSchedule, extract_uniform_data)
                .add_systems(
                    Render,
                    (prepare_uniform_data,).in_set(RenderSystems::Prepare),
                );
        } else {
            bevy::log::error!("[ZONE LIGHTING] FAILED to get render app - lighting will not work!");
        }

        app.add_systems(Startup, spawn_lights).add_systems(
            Update,
            (
                update_volumetric_fog_system,
                update_sun_position_system,
                apply_sky_settings_to_zone_time,
                // Sync + shadows read the fresh sun transform, so they run after it.
                sync_zone_lighting_to_bevy_lights_system.after(update_sun_position_system),
                update_shadows_for_time_of_day_system
                    .after(crate::systems::zone_time_system)
                    .after(crate::graphics::apply_shadow_quality_system)
                    .after(update_sun_position_system),
            ),
        );
        // bevy::log::info!("[ZONE LIGHTING] ZoneLightingPlugin build complete");
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<ZoneLightingUniformMeta>();
    }
}

fn spawn_lights(mut commands: Commands, zone_lighting: Res<ZoneLighting>) {
    // bevy::log::info!("[ZONE LIGHTING] Spawning directional and ambient lights");

    // Bevy 0.14: Use individual components instead of DirectionalLightBundle
    // IMPORTANT: shadow_maps_enabled MUST be true for VolumetricLight to work
    let light_entity = commands
        .spawn((
            DirectionalLight {
                illuminance: SUN_MAX_ILLUMINANCE,
                shadow_maps_enabled: true, // REQUIRED for volumetric lighting
                ..Default::default()
            },
            default_light_transform(),
            CascadeShadowConfig {
                // Medium default: 2 cascades to 100m (matches ShadowQuality::Medium).
                // Previously 4 cascades to 1000m (Ultra) at startup with 30% overlap.
                bounds: vec![50.0, 100.0],
                overlap_proportion: 0.2,
                minimum_distance: 0.1,
            },
            RenderLayers::default(),
            VolumetricLight, // Enable volumetric light shafts for this directional light
        ))
        .id();

    // Sky-bounce fill light: fixed high-angle, no shadows, cool sky tint.
    // Lifts the sun's shadow side (character faces/bodies when backlit).
    // Intensity is driven per-frame by daylight; see update_sun_position_system.
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.75, 0.82, 1.0),
            illuminance: FILL_MAX_ILLUMINANCE,
            shadow_maps_enabled: false,
            ..Default::default()
        },
        // Fixed fill direction: high from the north-ish side so it almost always
        // disagrees with the sun azimuth and fills shadowed faces.
        Transform::IDENTITY.looking_at(Vec3::new(0.35, -0.75, -0.55), Vec3::Y),
        RenderLayers::default(),
        SkyFillLight,
    ));

    // bevy::log::info!("[ZONE LIGHTING] Directional light spawned: entity={:?}, illuminance=15000.0, shadow_maps_enabled=true, VolumetricLight component added", light_entity);

    // Bevy 0.18: AmbientLight is now a component. GlobalAmbientLight is the resource form.
    // Using Bevy default values: Color::WHITE, brightness: 80.0
    commands.insert_resource(GlobalAmbientLight::default());

    //bevy::log::info!("[ZONE LIGHTING] Ambient light inserted: brightness=1.0");

    // Spawn the volumetric fog volume that covers the entire world
    // This enables light shafts/god rays from the directional light
    // Use initial values from ZoneLighting resource
    let density_factor = if zone_lighting.volumetric_fog_enabled {
        zone_lighting.volumetric_density_factor
    } else {
        0.0
    };

    // Use volumetric_fog_color from ZoneLighting for time-of-day integration
    let fog_color = Color::srgb(
        zone_lighting.volumetric_fog_color.x,
        zone_lighting.volumetric_fog_color.y,
        zone_lighting.volumetric_fog_color.z,
    );

    // CRITICAL FIX: Position the fog volume at the center of the game world (5120, 0, -5120)
    // The game world is centered around these coordinates, NOT at origin (0,0,0).
    // If the fog volume is at origin, the camera is ~7200 units away and sees a black box.
    // Scale of 2000.0 means the volume spans from (4120, -1000, -6120) to (6120, 1000, -4120)
    let fog_volume_center = Vec3::new(5120.0, 0.0, -5120.0);
    let fog_volume_scale = 2000.0;

    commands.spawn((
        FogVolume {
            fog_color,
            density_factor,
            absorption: zone_lighting.volumetric_absorption,
            scattering: zone_lighting.volumetric_scattering,
            scattering_asymmetry: zone_lighting.volumetric_scattering_asymmetry,
            ..Default::default()
        },
        Transform::from_translation(fog_volume_center).with_scale(Vec3::splat(fog_volume_scale)),
        VolumetricFogVolume, // Marker component for querying
    ));

    // bevy::log::info!(
    //     "[ZONE LIGHTING] Volumetric fog volume spawned at center=({}), scale={}, density_factor={}",
    //     fog_volume_center, fog_volume_scale, density_factor
    // );
    // bevy::log::info!(
    //     "[ZONE LIGHTING] Fog volume bounds: ({}) to ({})",
    //     fog_volume_center - Vec3::splat(fog_volume_scale / 2.0),
    //     fog_volume_center + Vec3::splat(fog_volume_scale / 2.0)
    // );
}

/// System that updates the FogVolume component from ZoneLighting resource settings.
/// This allows runtime control of volumetric fog parameters through the ZoneLighting resource.
/// Uses change detection to only update when ZoneLighting has been modified.
fn update_volumetric_fog_system(
    zone_lighting: Res<ZoneLighting>,
    mut fog_volume_query: Query<&mut FogVolume, With<VolumetricFogVolume>>,
) {
    // Only proceed if ZoneLighting has changed (change detection)
    if !zone_lighting.is_changed() {
        return;
    }

    // bevy::log::debug!(
    //     "[ZONE LIGHTING] ZoneLighting changed - updating fog volumes (enabled={}, density={}, absorption={}, scattering={})",
    //     zone_lighting.volumetric_fog_enabled,
    //     zone_lighting.volumetric_density_factor,
    //     zone_lighting.volumetric_absorption,
    //     zone_lighting.volumetric_scattering
    // );

    // Update all fog volumes marked with VolumetricFogVolume
    for mut fog_volume in fog_volume_query.iter_mut() {
        if zone_lighting.volumetric_fog_enabled {
            fog_volume.fog_color = Color::srgb(
                zone_lighting.volumetric_fog_color.x,
                zone_lighting.volumetric_fog_color.y,
                zone_lighting.volumetric_fog_color.z,
            );
            fog_volume.density_factor = zone_lighting.volumetric_density_factor;
            fog_volume.absorption = zone_lighting.volumetric_absorption;
            fog_volume.scattering = zone_lighting.volumetric_scattering;
            fog_volume.scattering_asymmetry = zone_lighting.volumetric_scattering_asymmetry;

            // bevy::log::debug!(
            //     "[ZONE LIGHTING] FogVolume updated: density_factor={}, absorption={}, scattering={}",
            //     fog_volume.density_factor, fog_volume.absorption, fog_volume.scattering
            // );
        } else {
            // When disabled, set density to 0 to effectively disable the fog
            fog_volume.density_factor = 0.0;
            // bevy::log::debug!("[ZONE LIGHTING] Volumetric fog disabled - set density_factor to 0");
        }
    }
}

/// System that syncs ZoneLighting resource values to Bevy's built-in lights
/// This ensures that both custom shaders and standard PBR materials use the same lighting
///
/// IMPORTANT: This system respects GraphicsSettings for ambient light, allowing user
/// brightness/color adjustments to take effect while still using zone lighting as a base.
fn sync_zone_lighting_to_bevy_lights_system(
    mut zone_lighting: ResMut<ZoneLighting>,
    mut ambient_light: ResMut<GlobalAmbientLight>,
    // Sun only: with the moon spawned there are 2 directional lights, so an
    // unfiltered single_mut() always errors and the sync body never runs.
    mut query_directional_light: Query<
        (&mut DirectionalLight, &GlobalTransform),
        (With<VolumetricLight>, Without<MoonLight>),
    >,
    graphics_settings: Option<Res<crate::graphics::GraphicsSettings>>,
) {
    // Determine the ambient light color to use:
    // 1. Start with zone lighting's map_ambient_color as the base
    // 2. If GraphicsSettings exists, multiply by the user's ambient color and brightness
    // 3. Scale brightness by daylight so the sun's shadow side stays readable at
    //    midday without washing out the night (night ~1x, full day ~3x).
    let map_ambient = zone_lighting.map_ambient_color;

    // Daylight factor from the live sun transform (self-consistent with the
    // sun path): -forward.y is the sun-height sine (1 = overhead, <=0 = set).
    // Falls back to 1.0 if the sun query below fails.
    let mut daylight_factor = 1.0;
    if let Ok((_, transform)) = query_directional_light.single() {
        let sun_height: f32 = -transform.forward().y;
        daylight_factor = (sun_height / 0.35).clamp(0.0, 1.0);
    }
    // Smoothstep for a gentle ramp instead of a hard switch.
    let daylight_smooth = daylight_factor * daylight_factor * (3.0 - 2.0 * daylight_factor);
    let daylight_boost = 1.0 + 2.0 * daylight_smooth;

    let (final_color, final_brightness) = if let Some(settings) = graphics_settings {
        // Get the user's ambient color preference as linear RGB
        let user_color = settings.ambient_light_color.to_linear();

        // Blend zone ambient color with user's ambient color
        let blended_r = map_ambient.x * user_color.red;
        let blended_g = map_ambient.y * user_color.green;
        let blended_b = map_ambient.z * user_color.blue;

        // Apply user's brightness multiplier (base 80.0 is Bevy's default),
        // scaled up during the day so backlit faces stay visible.
        let brightness = 80.0 * settings.ambient_light_brightness * daylight_boost;

        (
            Color::from(LinearRgba::new(blended_r, blended_g, blended_b, 1.0)),
            brightness,
        )
    } else {
        // No graphics settings, use zone lighting defaults with Bevy's default brightness
        (
            Color::from(LinearRgba::new(
                map_ambient.x,
                map_ambient.y,
                map_ambient.z,
                1.0,
            )),
            80.0 * daylight_boost,
        )
    };

    // Write only when the computed values actually differ, so the global
    // ambient resource is not marked changed for the whole render graph every
    // frame (GlobalAmbientLight writes force re-evaluation of ambient terms).
    if ambient_light.color != final_color || ambient_light.brightness != final_brightness {
        ambient_light.color = final_color;
        ambient_light.brightness = final_brightness;
    }

    if let Ok((mut light, transform)) = query_directional_light.single_mut() {
        // Sync directional light color from zone_lighting.character_diffuse_color
        let char_diffuse = zone_lighting.character_diffuse_color;
        let new_light_color = Color::from(LinearRgba::new(
            char_diffuse.x,
            char_diffuse.y,
            char_diffuse.z,
            1.0,
        ));
        if light.color != new_light_color {
            light.color = new_light_color;
        }

        // Update zone_lighting.light_direction from the actual light transform
        // This ensures custom shaders (like terrain) stay in sync with the sun position
        let current_dir: Dir3 = transform.forward();
        if zone_lighting.light_direction != *current_dir {
            zone_lighting.light_direction = *current_dir;
        }
    }
}

/// Solar elevation model shared by the sun path, shadow gating and fill.
/// Returns the sun direction (scene -> sun, normalized) and the elevation
/// sine (NdotL on flat ground: 1.0 = overhead, 0.0 = horizon, <0 = below).
pub fn sun_direction_for_hour(time_hours: f32, daylight: &DaylightSettings) -> (Vec3, f32) {
    let t = time_hours.rem_euclid(24.0);
    let sunrise = daylight.sunrise_hour.clamp(0.0, 24.0);
    let sunset = daylight.sunset_hour.clamp(0.0, 24.0).max(sunrise + 1.0);
    let day_length = (sunset - sunrise).max(1.0);
    let night_length = (24.0 - day_length).max(1.0);
    let max_elev = daylight.max_elevation_deg.clamp(5.0, 89.0);

    let (elevation_deg, azimuth_t) = if t >= sunrise && t < sunset {
        // Day: 0 at sunrise -> 1 at noon -> 0 at sunset. The pow broadens the
        // high-sun plateau so ~08:00-17:00 stays bright instead of spiking.
        let progress = (t - sunrise) / day_length;
        let elevation = (progress * std::f32::consts::PI).sin().powf(0.6) * max_elev;
        (elevation, progress)
    } else {
        // Night: sun swings below the horizon (west -> east return path).
        let night_t = if t >= sunset {
            t - sunset
        } else {
            (24.0 - sunset) + t
        };
        let progress = (night_t / night_length).clamp(0.0, 1.0);
        let elevation = -(progress * std::f32::consts::PI).sin() * 30.0;
        (elevation, 1.0 - progress)
    };

    let elev_rad = elevation_deg.to_radians();
    // East (-X) at sunrise -> West (+X) at sunset, with a slight south (+Z)
    // bias so faces get modelled light rather than flat top-down noon.
    let x = -(1.0 - azimuth_t * 2.0) * elev_rad.cos();
    let sun_dir = Vec3::new(x, elev_rad.sin(), 0.35 * elev_rad.cos() + 0.15).normalize();
    (sun_dir, elev_rad.sin())
}

/// System that updates the directional light rotation based on SkySettings and ZoneTime.
/// This creates a dynamic day/night cycle where the sun position changes with time.
///
/// When SkySettings.mode is Automatic, the sun follows the game's ZoneTime.
/// When SkySettings.mode is Manual, the sun position is controlled by SkySettings.manual_time.
///
/// Sun path (see SUNRISE_HOUR/SUNSET_HOUR):
/// - Sunrise (~5:00): Sun at horizon in the East
/// - Solar noon (~12:30): Sun at ~68 deg elevation (broad 08:00-17:00 plateau)
/// - Sunset (~20:00): Sun at horizon in the West
/// - Night (20:00-05:00): Sun below horizon
fn update_sun_position_system(
    zone_time: Res<crate::resources::ZoneTime>,
    sky_settings: Res<SkySettings>,
    daylight: Res<DaylightSettings>,
    current_zone: Option<Res<crate::resources::CurrentZone>>,
    game_data: Res<crate::resources::GameData>,
    // Sun only: moon_light_follow_camera_system owns the moon transform; without
    // this filter the sun rotation overwrote the moon every time change (order-dependent).
    // The sky fill light has its own fixed direction and must not be touched here.
    mut sun_query: Query<
        &mut Transform,
        (
            With<DirectionalLight>,
            Without<MoonLight>,
            Without<SkyFillLight>,
        ),
    >,
    mut fill_query: Query<
        &mut DirectionalLight,
        (With<SkyFillLight>, Without<MoonLight>),
    >,
) {
    // Determine if we should update based on mode and what changed.
    // Daylight slider moves must refresh the sun even when the clock hasn't ticked.
    let should_update = match sky_settings.mode {
        SkyMode::Automatic => {
            zone_time.is_changed() || sky_settings.is_changed() || daylight.is_changed()
        }
        SkyMode::Manual => sky_settings.is_changed() || daylight.is_changed(),
    };

    if !should_update {
        return;
    }

    // Get the time value in HOURS (0-24) based on mode
    let time_hours = match sky_settings.mode {
        SkyMode::Automatic => {
            // Convert ZoneTime.time (ticks) to hours (0-24)
            // Need to get day_cycle from zone data for proper conversion
            const DEFAULT_DAY_CYCLE: f32 = 160.0; // Standard 24-hour day cycle
            let day_cycle = if let Some(current_zone) = current_zone {
                if let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) {
                    // SAFETY: Ensure day_cycle is never zero to prevent division by zero
                    let cycle = zone_data.day_cycle as f32;
                    if cycle > 0.0 {
                        cycle
                    } else {
                        DEFAULT_DAY_CYCLE
                    }
                } else {
                    DEFAULT_DAY_CYCLE
                }
            } else {
                DEFAULT_DAY_CYCLE
            };

            // Convert ticks to hours: (ticks / day_cycle) * 24
            (zone_time.time as f32 / day_cycle) * 24.0
        }
        SkyMode::Manual => {
            // Use manual time from SkySettings (already in hours)
            sky_settings.manual_time
        }
    };

    let (sun_dir, elevation_sin) = sun_direction_for_hour(time_hours, &daylight);
    // Light shines along forward (-Z): aim forward at -sun_dir so the sun sits
    // at +sun_dir. looking_at handles the near-vertical noon case gracefully.
    let new_rotation = Transform::IDENTITY
        .looking_at(-sun_dir, Vec3::Y)
        .rotation;
    for mut transform in sun_query.iter_mut() {
        // Only write when the rotation actually changed (it only moves when the
        // tick-based time advances), so the shadow-casting directional light is
        // not dirtied every frame.
        if transform.rotation != new_rotation {
            transform.rotation = new_rotation;
        }
    }

    // Sky fill follows daylight: full at midday, off when the sun is down.
    // smoothstep(0, 0.35) reaches full well before noon so faces read clearly
    // through the whole bright plateau, and fades out across dusk.
    let daylight_t = (elevation_sin / 0.35).clamp(0.0, 1.0);
    let fill_peak = daylight.fill_illuminance.max(0.0);
    let fill_illuminance = fill_peak * daylight_t * daylight_t * (3.0 - 2.0 * daylight_t);
    for mut fill in fill_query.iter_mut() {
        if (fill.illuminance - fill_illuminance).abs() > 1.0 {
            fill.illuminance = fill_illuminance;
        }
    }
}

/// System that applies SkySettings manual time to ZoneTime.debug_overwrite_time
/// This bridges the UI settings to the zone time system
///
/// When SkySettings.mode is Manual, this system converts manual_time (0-24 hours)
/// to ticks and sets ZoneTime.debug_overwrite_time, which causes zone_time_system
/// to use the manual time instead of the game's world time.
///
/// When SkySettings.mode is Automatic, this system clears debug_overwrite_time
/// so the game time is used normally.
fn apply_sky_settings_to_zone_time(
    sky_settings: Res<SkySettings>,
    current_zone: Option<Res<crate::resources::CurrentZone>>,
    game_data: Res<crate::resources::GameData>,
    mut zone_time: ResMut<crate::resources::ZoneTime>,
) {
    // Only update if sky_settings changed
    if !sky_settings.is_changed() {
        return;
    }

    // Need current zone to get day_cycle
    let Some(current_zone) = current_zone else {
        return;
    };

    let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) else {
        return;
    };

    match sky_settings.mode {
        SkyMode::Manual => {
            // Convert manual_time (hours 0-24) to ticks
            // day_cycle represents the total ticks for a full 24-hour day
            let manual_time_hours = sky_settings.manual_time.clamp(0.0, 24.0);
            let tick_value = ((manual_time_hours / 24.0) * zone_data.day_cycle as f32) as u32;

            zone_time.debug_overwrite_time = Some(tick_value);

            // Log once when manual mode is enabled
            // if zone_time.debug_overwrite_time.is_some() {
            //     log::info!(
            //         "[SKY SETTINGS] Manual time enabled: {:.1} hours -> {} ticks (day_cycle: {})",
            //         manual_time_hours,
            //         tick_value,
            //         zone_data.day_cycle
            //     );
            // }
        }
        SkyMode::Automatic => {
            // Clear the override to use game time
            zone_time.debug_overwrite_time = None;
            log::info!("[SKY SETTINGS] Automatic time enabled - following game time");
        }
    }
}

/// System that adjusts shadow settings based on the live sun elevation.
/// The sun stays on (with shadows) whenever it is above the horizon, so light
/// lasts through Evening until ~20:00 instead of cutting out at 17:00.
/// Illuminance ramps smoothly with elevation: soft dawn/dusk, full midday.
///
/// Shadow State by Time (moon shadows stay off in all states for perf):
/// | Time State    | Sun Shadows            | Moon Shadows |
/// |---------------|------------------------|--------------|
/// | Morning/Day   | Enabled (sun is up)    | Disabled     |
/// | Evening (sun) | Enabled while up       | Disabled     |
/// | Evening/Night | Disabled (sun is down) | Disabled     |
pub fn update_shadows_for_time_of_day_system(
    zone_time: Res<ZoneTime>,
    daylight: Res<DaylightSettings>,
    mut sun_query: Query<
        (&mut DirectionalLight, &GlobalTransform),
        (With<VolumetricLight>, Without<MoonLight>, Without<SkyFillLight>),
    >,
    mut moon_query: Query<&mut DirectionalLight, With<MoonLight>>,
    graphics_settings: Option<Res<GraphicsSettings>>,
) {
    use bevy::ecs::change_detection::DetectChanges;
    // Only re-evaluate when the time state actually changed (or a daylight slider
    // moved). Previously this wrote shadow_maps_enabled/illuminance every frame,
    // invalidating the shadow-map cache and forcing cascade re-renders even for
    // a static scene.
    if !zone_time.is_changed() && !daylight.is_changed() {
        return;
    }

    // Check if shadows are enabled in graphics settings
    // If shadows are disabled by quality settings, don't override
    let shadow_maps_enabled_by_settings = graphics_settings
        .map(|g| g.shadow_quality != ShadowQuality::Off)
        .unwrap_or(true);

    if !shadow_maps_enabled_by_settings {
        return; // Shadows disabled in settings, nothing to do
    }

    // Moon illuminance still follows the named state (it is a state proxy for
    // "how dark is the sky"), while the sun follows its live elevation so the
    // Evening dusk keeps sunlight until the disk actually sets.
    let (moon_shadows, moon_illuminance) = match zone_time.state {
        ZoneTimeState::Morning => (false, 500.0),
        ZoneTimeState::Day => (false, 0.0),
        ZoneTimeState::Evening => (false, 800.0),
        ZoneTimeState::Night => (false, 3000.0),
    };

    // Write only on actual change to avoid dirtying the light every frame.
    for (mut light, transform) in sun_query.iter_mut() {
        // Sun height sine from the live transform (set by
        // update_sun_position_system, which runs before us in Update order).
        let sun_height: f32 = -transform.forward().y;
        let sun_up = sun_height > 0.02;
        // Smooth dawn/dusk ramp: 0 at the horizon, full once ~14 deg up.
        let ramp = (sun_height / 0.25).clamp(0.0, 1.0);
        let smooth = ramp * ramp * (3.0 - 2.0 * ramp);
        let sun_illuminance = daylight.sun_illuminance.max(0.0) * smooth;
        let sun_shadows = sun_up;
        if light.shadow_maps_enabled != sun_shadows {
            light.shadow_maps_enabled = sun_shadows;
        }
        if (light.illuminance - sun_illuminance).abs() > f32::EPSILON {
            light.illuminance = sun_illuminance;
        }
    }

    // Apply to moon light (shadows stay off in all states per table; illuminance follows night).
    for mut light in moon_query.iter_mut() {
        if light.shadow_maps_enabled != moon_shadows {
            light.shadow_maps_enabled = moon_shadows;
        }
        if (light.illuminance - moon_illuminance).abs() > f32::EPSILON {
            light.illuminance = moon_illuminance;
        }
    }
}

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct ZoneLighting {
    pub map_ambient_color: Vec3,
    pub character_ambient_color: Vec3,
    pub character_diffuse_color: Vec3,
    pub light_direction: Vec3,

    pub color_fog_enabled: bool,
    pub fog_color: Vec3,
    pub fog_density: f32,
    pub fog_min_density: f32,
    pub fog_max_density: f32,

    pub alpha_fog_enabled: bool,
    pub fog_alpha_weight_start: f32,
    pub fog_alpha_weight_end: f32,
    // Height-based fog parameters
    pub fog_min_height: f32,
    pub fog_max_height: f32,
    pub fog_height_density: f32,
    // Time of day parameters
    pub time_of_day: f32,
    pub day_color: Vec3,
    pub night_color: Vec3,
    // Volumetric fog settings
    pub volumetric_fog_enabled: bool,
    pub volumetric_fog_color: Vec3,
    pub volumetric_density_factor: f32,
    pub volumetric_absorption: f32,
    pub volumetric_scattering: f32,
    pub volumetric_scattering_asymmetry: f32,
}

impl Default for ZoneLighting {
    fn default() -> Self {
        Self {
            map_ambient_color: Vec3::ONE,
            character_ambient_color: Vec3::ONE,
            character_diffuse_color: Vec3::ONE,
            light_direction: default_light_transform().back().normalize(),
            fog_color: Vec3::new(0.2, 0.2, 0.2),
            color_fog_enabled: true,
            fog_density: 0.0018,
            fog_min_density: 0.0,
            fog_max_density: 0.75,
            alpha_fog_enabled: true,
            fog_alpha_weight_start: 0.85,
            fog_alpha_weight_end: 0.98,
            // Height-based fog parameters
            fog_min_height: -10.0,
            fog_max_height: 50.0,
            fog_height_density: 0.5,
            // Time of day parameters
            time_of_day: 0.5,                      // 0.0 = night, 1.0 = day
            day_color: Vec3::new(0.7, 0.8, 1.0),   // Day fog color (blueish)
            night_color: Vec3::new(0.1, 0.1, 0.3), // Night fog color (dark blue)
            // Volumetric fog settings - tuned for atmospheric depth and light shafts
            volumetric_fog_enabled: false, // Disabled by default - can be enabled in settings
            volumetric_fog_color: Vec3::new(0.85, 0.9, 1.0), // Soft blue-white for atmospheric haze
            volumetric_density_factor: 0.05, // Balanced density for visible light shafts without obscuring gameplay
            volumetric_absorption: 0.1,      // Moderate absorption for depth perception
            volumetric_scattering: 0.11, // Scattering coefficient for balanced light shafts (was 0.5 too high)
            volumetric_scattering_asymmetry: 0.7, // Higher asymmetry for forward-scattering (Mie scattering)
        }
    }
}

#[derive(Clone, ShaderType, Resource)]
pub struct ZoneLightingUniformData {
    // Group 0: 64 bytes (4 vec4)
    pub map_ambient_color: Vec4,
    pub character_ambient_color: Vec4,
    pub character_diffuse_color: Vec4,
    pub light_direction: Vec4,

    // Group 1: 64 bytes (4 vec4)
    pub fog_color: Vec4,
    pub day_color: Vec4,
    pub night_color: Vec4,
    // Pack 4 f32 values into vec4 for alignment: fog_density, fog_min_density, fog_max_density, fog_height_density
    pub fog_params: Vec4,

    // Group 2: 48 bytes (3 vec4)
    // Pack 4 f32 values into vec4 for alignment: fog_min_height, fog_max_height, time_of_day, unused
    pub fog_height_params: Vec4,
    // Pack 2 f32 values with padding: fog_alpha_range_start, fog_alpha_range_end, unused, unused
    pub fog_alpha_params: Vec4,
    pub _padding: Vec4, // Padding to ensure total size is multiple of 16
}

#[derive(Resource)]
pub struct ZoneLightingUniformMeta {
    buffer: Buffer,
    bind_group: BindGroup,
    pub bind_group_layout: BindGroupLayout,
}

impl FromWorld for ZoneLightingUniformMeta {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let buffer = render_device.create_buffer(&BufferDescriptor {
            size: ZoneLightingUniformData::min_size().get(),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            mapped_at_creation: false,
            label: Some("zone_lighting_uniform_buffer"),
        });

        let bind_group_layout = render_device.create_bind_group_layout(
            Some("zone_lighting_uniform_layout"),
            &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(ZoneLightingUniformData::min_size()),
                },
                count: None,
            }],
        );

        let bind_group = render_device.create_bind_group(
            "zone_lighting_uniform_bind_group",
            &bind_group_layout,
            &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        );

        ZoneLightingUniformMeta {
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
}

fn extract_uniform_data(mut commands: Commands, zone_lighting: Extract<Res<ZoneLighting>>) {
    commands.insert_resource(ZoneLightingUniformData {
        map_ambient_color: zone_lighting.map_ambient_color.extend(1.0),
        character_ambient_color: zone_lighting.character_ambient_color.extend(1.0),
        character_diffuse_color: zone_lighting.character_diffuse_color.extend(1.0),
        light_direction: zone_lighting.light_direction.extend(1.0),
        fog_color: zone_lighting.fog_color.extend(1.0),
        day_color: zone_lighting.day_color.extend(1.0),
        night_color: zone_lighting.night_color.extend(1.0),
        // Pack fog params: fog_density, fog_min_density, fog_max_density, fog_height_density
        fog_params: Vec4::new(
            if zone_lighting.color_fog_enabled {
                zone_lighting.fog_density
            } else {
                0.0
            },
            if zone_lighting.color_fog_enabled {
                zone_lighting.fog_min_density
            } else {
                0.0
            },
            if zone_lighting.color_fog_enabled {
                zone_lighting.fog_max_density
            } else {
                0.0
            },
            zone_lighting.fog_height_density,
        ),
        // Pack fog height params: fog_min_height, fog_max_height, time_of_day, unused
        fog_height_params: Vec4::new(
            zone_lighting.fog_min_height,
            zone_lighting.fog_max_height,
            zone_lighting.time_of_day,
            0.0, // unused
        ),
        // Pack fog alpha params: fog_alpha_range_start, fog_alpha_range_end, unused, unused
        fog_alpha_params: Vec4::new(
            if zone_lighting.alpha_fog_enabled {
                zone_lighting.fog_alpha_weight_start
            } else {
                99999999999.0
            },
            if zone_lighting.alpha_fog_enabled {
                zone_lighting.fog_alpha_weight_end
            } else {
                999999999.0
            },
            0.0, // unused
            0.0, // unused
        ),
        _padding: Vec4::ZERO,
    });
}

fn prepare_uniform_data(
    uniform_data: Res<ZoneLightingUniformData>,
    uniform_meta: ResMut<ZoneLightingUniformMeta>,
    render_queue: Res<RenderQueue>,
) {
    let byte_buffer = [0u8; ZoneLightingUniformData::SHADER_SIZE.get() as usize];
    let mut buffer = encase::UniformBuffer::new(byte_buffer);
    buffer.write(uniform_data.as_ref()).unwrap();

    render_queue.write_buffer(&uniform_meta.buffer, 0, buffer.as_ref());
}

/// Calculate cloud lighting parameters based on time of day
pub(crate) fn calculate_cloud_lighting(
    zone_time: &crate::resources::ZoneTime,
    zone_lighting: &crate::render::ZoneLighting,
) -> (Vec3, Vec3, Vec3, f32) {
    use crate::resources::ZoneTimeState;

    // Sun direction varies with time of day
    // Morning: East (low angle), Noon: Up, Evening: West (low angle), Night: Below horizon
    let time_of_day = match zone_time.state {
        ZoneTimeState::Morning => {
            // Sun rises in the east, moves upward
            let t = zone_time.state_percent_complete;
            0.0 + t * 0.5 // 0.0 to 0.5 (sunrise to noon approach)
        }
        ZoneTimeState::Day => {
            // Sun at highest point, slowly descending
            let t = zone_time.state_percent_complete;
            0.5 + t * 0.25 // 0.5 to 0.75 (noon to afternoon)
        }
        ZoneTimeState::Evening => {
            // Sun sets in the west
            let t = zone_time.state_percent_complete;
            0.75 + t * 0.25 // 0.75 to 1.0 (sunset)
        }
        ZoneTimeState::Night => {
            // Sun below horizon
            0.0
        }
    };

    // Calculate sun direction from time
    let sun_angle = time_of_day * std::f32::consts::PI;
    let sun_direction = Vec3::new(
        -sun_angle.cos(), // X: east-west
        sun_angle.sin(),  // Y: up-down
        0.3,              // Z: slight northward tilt
    )
    .normalize();

    // Sun color varies with time of day
    let sun_color = match zone_time.state {
        ZoneTimeState::Morning => {
            // Warm orange/pink sunrise
            let t = zone_time.state_percent_complete;
            Vec3::new(1.0, 0.7 + t * 0.2, 0.5 + t * 0.4) // Orange -> whiter
        }
        ZoneTimeState::Day => {
            // Bright white/yellow daylight
            Vec3::new(1.0, 0.98, 0.95)
        }
        ZoneTimeState::Evening => {
            // Warm orange/red sunset
            let t = zone_time.state_percent_complete;
            Vec3::new(1.0, 0.9 - t * 0.4, 0.8 - t * 0.5) // White -> orange/red
        }
        ZoneTimeState::Night => {
            // Dim moonlight
            Vec3::new(0.2, 0.25, 0.4)
        }
    };

    // Ambient color from zone lighting
    let ambient_color = zone_lighting.map_ambient_color;

    // Time-of-day factor for cloud visibility
    let tod_factor = match zone_time.state {
        ZoneTimeState::Morning => 0.5 + zone_time.state_percent_complete * 0.5,
        ZoneTimeState::Day => 1.0,
        ZoneTimeState::Evening => 1.0 - zone_time.state_percent_complete * 0.5,
        ZoneTimeState::Night => 0.3, // Clouds still slightly visible at night
    };

    (sun_direction, sun_color, ambient_color, tod_factor)
}
