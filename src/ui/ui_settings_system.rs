use std::ops::RangeInclusive;

use bevy::ecs::system::SystemParam;
use bevy::prelude::{Color, Local, Query, Res, ResMut, Resource};
use bevy_egui::{egui, EguiContexts};
use bevy_post_process::dof::DepthOfFieldMode;

use crate::{
    audio::SoundGain,
    components::{
        BirdSettings, DirtDashSettings, FishSettings, Season, SoundCategory, WindSwaySettings,
    },
    graphics::GraphicsSettings,
    render::{SkyMode, SkySettings, StarrySkySettings, VolumetricCloudSettings, ZoneLighting},
    resources::{
        BloodEffectConfig, SeasonSettings, SoundSettings, SummerSettings, WaterSettings, ZoneTime,
        ZoneTimeState,
    },
    terrain::TerrainEnhancementSettings,
    ui::UiStateWindows,
};

/// Blend mode for starry sky rendering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkyBlendMode {
    #[default]
    Additive,
    Alpha,
    PremultipliedAlpha,
    Multiply,
}

/// Depth compare function for starry sky
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkyDepthCompare {
    Always,
    #[default]
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

/// Resource for starry sky render settings that affect ghosting.
/// These control the blend mode, depth testing, and other render pipeline settings.
#[derive(Resource, Debug, Clone)]
pub struct StarrySkyRenderSettings {
    /// Blend mode for the starry sky material
    pub blend_mode: SkyBlendMode,
    /// Depth compare function
    pub depth_compare: SkyDepthCompare,
    /// Whether depth writes are enabled
    pub depth_write_enabled: bool,
    /// Depth bias value
    pub depth_bias: f32,
    /// Whether to use NoFrustumCulling
    pub no_frustum_culling: bool,
    /// Alpha cutoff value (for alpha testing)
    pub alpha_cutoff: f32,
    /// Whether to render stars at full brightness (ignore night factor for testing)
    pub force_full_brightness: bool,
}

impl Default for StarrySkyRenderSettings {
    fn default() -> Self {
        Self {
            blend_mode: SkyBlendMode::Additive,
            depth_compare: SkyDepthCompare::Always,
            depth_write_enabled: false,
            depth_bias: 1.0,
            no_frustum_culling: true,
            alpha_cutoff: 0.0,
            force_full_brightness: false,
        }
    }
}

/// Resource for storing post-processing settings that can be modified at runtime.
/// These settings affect potential ghosting artifacts.
#[derive(Resource, Debug, Clone)]
pub struct PostProcessingSettings {
    /// Whether bloom effect is enabled
    pub bloom_enabled: bool,
    /// Bloom intensity (0.0 - 1.0)
    pub bloom_intensity: f32,
    /// Whether SSAO (Screen Space Ambient Occlusion) is enabled
    pub ssao_enabled: bool,
    /// Whether depth of field is enabled
    pub dof_enabled: bool,
    /// Whether volumetric fog is enabled
    pub volumetric_fog_enabled: bool,
    /// Whether color grading is enabled
    pub color_grading_enabled: bool,
}

impl Default for PostProcessingSettings {
    fn default() -> Self {
        Self {
            bloom_enabled: true,
            bloom_intensity: 0.5,
            ssao_enabled: true,
            dof_enabled: true,
            volumetric_fog_enabled: true,
            color_grading_enabled: false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum SettingsPage {
    Sound,
    Blood,
    Sky,
    Stars,
    Clouds,
    StarrySkyRender,
    DepthOfField,
    VolumetricFog,
    Water,
    Fish,
    Birds,
    Seasons,
    DirtDash,
    WindSway,
    PostProcessing,
    Graphics,
    Terrain,
}

pub struct UiStateSettings {
    page: SettingsPage,
}

impl Default for UiStateSettings {
    fn default() -> Self {
        Self {
            page: SettingsPage::Sound,
        }
    }
}

/// Resource for storing depth of field settings that can be modified at runtime.
#[derive(Resource, Debug, Clone)]
pub struct DepthOfFieldSettings {
    /// Whether depth of field effect is enabled
    pub enabled: bool,
    /// The mode of depth of field (Bokeh or Gaussian)
    pub mode: DepthOfFieldMode,
    /// Distance to the focal plane in meters (objects at this distance are sharp)
    pub focal_distance: f32,
    /// Aperture f-stop value (lower = more blur, higher = less blur)
    pub aperture_f_stops: f32,
    /// Sensor height in meters (affects blur characteristics)
    pub sensor_height: f32,
    /// Maximum circle of confusion diameter in pixels
    pub max_circle_of_confusion_diameter: f32,
    /// Maximum depth for the effect (objects beyond this are handled differently)
    pub max_depth: f32,
}

impl Default for DepthOfFieldSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: DepthOfFieldMode::Bokeh,
            focal_distance: 10.0,
            aperture_f_stops: 3.3,
            sensor_height: 0.01866,
            max_circle_of_confusion_diameter: 64.0,
            max_depth: 2000.0,
        }
    }
}

/// Grouped system parameters for ui_settings_system to avoid parameter count limit
#[derive(SystemParam)]
pub struct SettingsSystemParams<'w, 's> {
    pub egui_context: EguiContexts<'w, 's>,
    pub ui_state_windows: ResMut<'w, UiStateWindows>,
    pub ui_state_settings: Local<'s, UiStateSettings>,
    pub sound_settings: ResMut<'w, SoundSettings>,
    pub blood_effect_config: ResMut<'w, BloodEffectConfig>,
    pub query_sounds: Query<'w, 's, (&'static SoundCategory, &'static mut SoundGain)>,
    pub sky_settings: ResMut<'w, SkySettings>,
    pub starry_sky_settings: ResMut<'w, StarrySkySettings>,
    pub volumetric_cloud_settings: ResMut<'w, VolumetricCloudSettings>,
    pub starry_sky_render_settings: ResMut<'w, StarrySkyRenderSettings>,
    pub dof_settings: ResMut<'w, DepthOfFieldSettings>,
    pub zone_lighting: ResMut<'w, ZoneLighting>,
    pub water_settings: ResMut<'w, WaterSettings>,
    pub fish_settings: ResMut<'w, FishSettings>,
    pub bird_settings: ResMut<'w, BirdSettings>,
    pub season_settings: ResMut<'w, SeasonSettings>,
    pub summer_settings: ResMut<'w, SummerSettings>,
    pub dirt_dash_settings: ResMut<'w, DirtDashSettings>,
    pub wind_sway_settings: Option<ResMut<'w, WindSwaySettings>>,
    pub post_processing_settings: ResMut<'w, PostProcessingSettings>,
    pub graphics_settings: ResMut<'w, GraphicsSettings>,
    pub terrain_settings: ResMut<'w, TerrainEnhancementSettings>,
    pub zone_time: Option<Res<'w, ZoneTime>>,
}

fn settings_slider<Num: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Num,
    range: RangeInclusive<Num>,
    suffix: Option<&str>,
) -> egui::Response {
    ui.label(label);
    let slider = egui::Slider::new(value, range);
    let slider = if let Some(suffix) = suffix {
        slider.text(suffix)
    } else {
        slider
    };
    let response = ui.add(slider.show_value(true));
    ui.end_row();
    response
}

fn settings_checkbox(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut bool,
    checkbox_label: &str,
) -> egui::Response {
    ui.label(label);
    let response = ui.checkbox(value, checkbox_label);
    ui.end_row();
    response
}

fn settings_combo<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id_salt: &str,
    label: &str,
    selected_text: impl Into<egui::WidgetText>,
    value: &mut T,
    options: &[(T, &str)],
) {
    ui.label(label);
    egui::ComboBox::from_id_salt(id_salt)
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for (option_value, option_text) in options {
                ui.selectable_value(value, option_value.clone(), *option_text);
            }
        });
    ui.end_row();
}

pub fn ui_settings_system(mut params: SettingsSystemParams) {
    let SettingsSystemParams {
        mut egui_context,
        mut ui_state_windows,
        mut ui_state_settings,
        mut sound_settings,
        mut blood_effect_config,
        mut query_sounds,
        mut sky_settings,
        mut starry_sky_settings,
        mut volumetric_cloud_settings,
        mut starry_sky_render_settings,
        mut dof_settings,
        mut zone_lighting,
        mut water_settings,
        mut fish_settings,
        mut bird_settings,
        mut season_settings,
        mut summer_settings,
        mut dirt_dash_settings,
        mut wind_sway_settings,
        mut post_processing_settings,
        mut graphics_settings,
        mut terrain_settings,
        zone_time,
    } = params;

    egui::Window::new("Settings")
        .open(&mut ui_state_windows.settings_open)
        .resizable(false)
        .show(egui_context.ctx_mut().unwrap(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sound, "Sound");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Blood, "Blood");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sky, "Sky");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Stars, "Stars");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Clouds, "Clouds");
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::StarrySkyRender,
                    "Sky Render",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::DepthOfField,
                    "Depth of Field",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::VolumetricFog,
                    "Volumetric Fog",
                );
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Water, "Water");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Fish, "Fish");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Birds, "Birds");
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Seasons, "Seasons");
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::DirtDash,
                    "Dirt Dash",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::WindSway,
                    "Wind Sway",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::PostProcessing,
                    "Post Process",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Graphics,
                    "Graphics",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Terrain,
                    "Terrain",
                );
            });

            ui.separator();

            match ui_state_settings.page {
                SettingsPage::Sound => {
                    render_sound_page(ui, &mut sound_settings, &mut query_sounds);
                }
                SettingsPage::Blood => {
                    render_blood_page(ui, &mut blood_effect_config);
                }
                SettingsPage::Sky => {
                    render_sky_page(ui, &mut sky_settings);
                }
                SettingsPage::Stars => {
                    render_stars_page(ui, &mut starry_sky_settings);
                }
                SettingsPage::Clouds => {
                    render_clouds_page(ui, &mut volumetric_cloud_settings);
                }
                SettingsPage::StarrySkyRender => {
                    render_starry_sky_render_page(ui, &mut starry_sky_render_settings);
                }
                SettingsPage::DepthOfField => {
                    render_depth_of_field_page(ui, &mut dof_settings);
                }
                SettingsPage::VolumetricFog => {
                    render_volumetric_fog_page(ui, &mut zone_lighting);
                }
                SettingsPage::Water => {
                    render_water_page(ui, &mut water_settings);
                }
                SettingsPage::Fish => {
                    render_fish_page(ui, &mut fish_settings);
                }
                SettingsPage::Birds => {
                    render_birds_page(ui, &mut bird_settings);
                }
                SettingsPage::Seasons => {
                    render_seasons_page(ui, &mut season_settings, &mut summer_settings);
                }
                SettingsPage::DirtDash => {
                    render_dirt_dash_page(ui, &mut dirt_dash_settings);
                }
                SettingsPage::WindSway => {
                    render_wind_sway_page(ui, &mut wind_sway_settings);
                }
                SettingsPage::PostProcessing => {
                    render_post_processing_page(ui, &mut post_processing_settings);
                }
                SettingsPage::Graphics => {
                    render_graphics_page(ui, &mut graphics_settings, &zone_time);
                }
                SettingsPage::Terrain => {
                    render_terrain_page(ui, &mut terrain_settings);
                }
            }
        });
}

fn render_sound_page(
    ui: &mut egui::Ui,
    sound_settings: &mut SoundSettings,
    query_sounds: &mut Query<'_, '_, (&'static SoundCategory, &'static mut SoundGain)>,
) {
    egui::Grid::new("sound_settings_gain")
        .num_columns(2)
        .show(ui, |ui| {
            let mut gain_changed = false;

            gain_changed |= settings_checkbox(ui, "Sound:", &mut sound_settings.enabled, "Enabled")
                .changed();
            gain_changed |= settings_slider(
                ui,
                "Global Volume:",
                &mut sound_settings.global_gain,
                0.0..=1.0,
                None,
            )
            .changed();

            let mut add_category_slider = |text: &str, category| {
                gain_changed |= settings_slider(
                    ui,
                    text,
                    &mut sound_settings.gains[category],
                    0.0..=1.0,
                    None,
                )
                .changed();
            };

            add_category_slider("Background Music:", SoundCategory::BackgroundMusic);
            add_category_slider("Player Footsteps:", SoundCategory::PlayerFootstep);
            add_category_slider("Other Footsteps:", SoundCategory::OtherFootstep);
            add_category_slider("Player Combat:", SoundCategory::PlayerCombat);
            add_category_slider("Other Combat:", SoundCategory::OtherCombat);
            add_category_slider("NPC Sounds:", SoundCategory::NpcSounds);

            if gain_changed {
                for (category, mut gain) in query_sounds.iter_mut() {
                    let target_gain = sound_settings.gain(*category);

                    if target_gain != *gain {
                        *gain = target_gain;
                    }
                }
            }
        });
}

fn render_blood_page(ui: &mut egui::Ui, blood_effect_config: &mut BloodEffectConfig) {
    egui::Grid::new("blood_effect_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Enable Blood:", &mut blood_effect_config.enable_blood, "Enabled");
            settings_checkbox(ui, "Show Wounds:", &mut blood_effect_config.show_wounds, "Enabled");
            settings_slider(ui, "Intensity:", &mut blood_effect_config.intensity, 0.0..=1.5, None);
            settings_slider(ui, "Quality Scale:", &mut blood_effect_config.quality_scale, 0.1..=1.0, None);
            settings_slider(ui, "Max Spatters:", &mut blood_effect_config.max_spatters, 10..=600, None);
            settings_slider(ui, "Spawn Budget / Frame:", &mut blood_effect_config.max_spatters_per_frame, 1..=128, None);
            settings_slider(ui, "Spatter Lifetime:", &mut blood_effect_config.spatter_lifetime, 1.0..=120.0, Some("s"));
            settings_slider(ui, "Fade Start:", &mut blood_effect_config.fade_start_fraction, 0.05..=0.95, None);
            settings_slider(ui, "Spatter Radius:", &mut blood_effect_config.spatter_radius, 0.2..=8.0, Some("m"));
            settings_slider(ui, "Kill Spatters:", &mut blood_effect_config.spatter_count_on_kill, 1..=20, None);
            settings_slider(ui, "Hit Spatters:", &mut blood_effect_config.spatter_count_on_hit, 0..=8, None);
            settings_slider(ui, "Min Spatter Size:", &mut blood_effect_config.min_spatter_size, 0.05..=2.0, None);
            settings_slider(ui, "Max Spatter Size:", &mut blood_effect_config.max_spatter_size, 0.1..=4.0, None);
            settings_slider(ui, "Wound Threshold:", &mut blood_effect_config.wound_visibility_threshold, 0.0..=1.0, None);
            settings_slider(ui, "Max Wounds/Entity:", &mut blood_effect_config.max_wounds_per_entity, 0..=12, None);
            settings_slider(ui, "LOD Near Distance:", &mut blood_effect_config.lod_near_distance, 5.0..=200.0, Some("m"));
            settings_slider(ui, "LOD Far Distance:", &mut blood_effect_config.lod_far_distance, 10.0..=600.0, Some("m"));
            settings_checkbox(ui, "Layered Effects:", &mut blood_effect_config.enable_layered_effects, "Enabled");
            settings_checkbox(ui, "Diagnostics Logging:", &mut blood_effect_config.enable_diagnostics, "Enabled");
        });

    // Clamp dependent ranges
    blood_effect_config.max_spatter_size = blood_effect_config
        .max_spatter_size
        .max(blood_effect_config.min_spatter_size);
    blood_effect_config.wound_max_size =
        blood_effect_config.wound_max_size.max(blood_effect_config.wound_min_size);
    blood_effect_config.lod_far_distance = blood_effect_config
        .lod_far_distance
        .max(blood_effect_config.lod_near_distance + 1.0);

    ui.separator();
    ui.label("Tip: Lower quality scale and spawn budget for large battles.");
    ui.label("LOD distances reduce blood complexity for distant combat.");
}

fn render_sky_page(ui: &mut egui::Ui, sky_settings: &mut SkySettings) {
    egui::Grid::new("sky_settings")
        .num_columns(2)
        .show(ui, |ui| {
            let mode_text = match sky_settings.mode {
                SkyMode::Automatic => "Automatic (Game Time)",
                SkyMode::Manual => "Manual",
            };
            settings_combo(
                ui,
                "sky_mode",
                "Time Mode:",
                mode_text,
                &mut sky_settings.mode,
                &[
                    (SkyMode::Automatic, "Automatic (Game Time)"),
                    (SkyMode::Manual, "Manual"),
                ],
            );

            if sky_settings.mode == SkyMode::Manual {
                settings_slider(ui, "Time of Day:", &mut sky_settings.manual_time, 0.0..=24.0, Some("hours"));

                let time_desc = format_time_of_day(sky_settings.manual_time);
                ui.label("");
                ui.label(time_desc);
                ui.end_row();
            }

            settings_slider(ui, "Atmosphere Intensity:", &mut sky_settings.atmosphere_intensity, 0.0..=2.0, None);
        });

    ui.separator();
    if sky_settings.mode == SkyMode::Automatic {
        ui.label("Tip: Time follows game time automatically. Switch to Manual mode to control time yourself.");
    } else {
        ui.label("Tip: Drag the time slider to change time of day. 6 = sunrise, 12 = noon, 18 = sunset, 0 = midnight.");
    }
}

fn render_stars_page(ui: &mut egui::Ui, starry_sky_settings: &mut StarrySkySettings) {
    egui::Grid::new("starry_sky_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_slider(ui, "Star Density:", &mut starry_sky_settings.star_density, 0.0..=1.0, Some("density"));
            settings_slider(ui, "Star Brightness:", &mut starry_sky_settings.star_brightness, 0.0..=5.0, None);
            settings_slider(ui, "Moon Phase:", &mut starry_sky_settings.moon_phase, 0.0..=1.0, Some("phase"));

            // Moon phase description
            let phase_desc = match starry_sky_settings.moon_phase {
                p if p < 0.05 || p > 0.95 => "New Moon",
                p if p < 0.25 => "Waxing Crescent",
                p if p < 0.35 => "First Quarter",
                p if p < 0.55 => "Waxing Gibbous",
                p if p < 0.65 => "Full Moon",
                p if p < 0.75 => "Waning Gibbous",
                p if p < 0.95 => "Last Quarter",
                _ => "New Moon",
            };
            ui.label("");
            ui.label(phase_desc);
            ui.end_row();

            settings_slider(ui, "Moon Direction X:", &mut starry_sky_settings.moon_direction.x, -1.0..=1.0, None);
            settings_slider(ui, "Moon Direction Y:", &mut starry_sky_settings.moon_direction.y, 0.0..=1.0, None);
            settings_slider(ui, "Moon Direction Z:", &mut starry_sky_settings.moon_direction.z, -1.0..=1.0, None);

            if ui.button("Normalize Moon Direction").clicked() {
                starry_sky_settings.moon_direction = starry_sky_settings.moon_direction.normalize();
            }
            ui.end_row();

            ui.label("Night Factor:");
            ui.label(format!("{:.2} (auto)", starry_sky_settings.night_factor));
            ui.end_row();
        });

    ui.separator();
    ui.label("Tip: Star density 0.15 = sparse (~1,000 stars), 0.60 = dense (~6,000 stars). Changes apply instantly.");
    ui.label("Note: Night factor is controlled by game time. Set to Manual mode in Sky tab and set time to midnight to see stars.");
}

fn render_clouds_page(ui: &mut egui::Ui, volumetric_cloud_settings: &mut VolumetricCloudSettings) {
    egui::Grid::new("volumetric_cloud_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Enabled:", &mut volumetric_cloud_settings.enabled, "");
            settings_slider(ui, "Cloud Count:", &mut volumetric_cloud_settings.cloud_count, 10..=200, Some("clouds"));
            settings_slider(ui, "Cluster Size Min:", &mut volumetric_cloud_settings.cluster_size_min, 2..=5, Some("blobs"));
            settings_slider(ui, "Cluster Size Max:", &mut volumetric_cloud_settings.cluster_size_max, 2..=8, Some("blobs"));
            settings_slider(ui, "Radius Min:", &mut volumetric_cloud_settings.cloud_radius_min, 10.0..=400.0, Some("units"));
            settings_slider(ui, "Radius Max:", &mut volumetric_cloud_settings.cloud_radius_max, 20.0..=700.0, Some("units"));
            settings_slider(ui, "Height Min:", &mut volumetric_cloud_settings.cloud_height_min, 100.0..=1000.0, Some("units"));
            settings_slider(ui, "Height Max:", &mut volumetric_cloud_settings.cloud_height_max, 200.0..=1500.0, Some("units"));
            settings_slider(ui, "Spawn Radius:", &mut volumetric_cloud_settings.cloud_spawn_radius, 500.0..=10000.0, Some("units"));

            ui.separator();
            ui.end_row();

            settings_slider(ui, "Density:", &mut volumetric_cloud_settings.density, 0.0..=1.0, None);
            settings_slider(ui, "Opacity:", &mut volumetric_cloud_settings.opacity, 0.0..=1.0, None);
            settings_slider(ui, "Brightness:", &mut volumetric_cloud_settings.brightness, 0.5..=3.0, None);
            settings_slider(ui, "Noise Scale:", &mut volumetric_cloud_settings.noise_scale, 0.005..=0.05, None);
            settings_slider(ui, "Noise Octaves:", &mut volumetric_cloud_settings.noise_octaves, 1..=6, None);
            settings_slider(ui, "TOD Response:", &mut volumetric_cloud_settings.tod_response, 0.0..=1.0, None);

            ui.separator();
            ui.end_row();

            settings_slider(ui, "Drift Speed X:", &mut volumetric_cloud_settings.drift_speed.x, -50.0..=50.0, None);
            settings_slider(ui, "Drift Speed Y:", &mut volumetric_cloud_settings.drift_speed.y, -50.0..=50.0, None);
            settings_slider(ui, "Drift Speed Z:", &mut volumetric_cloud_settings.drift_speed.z, -50.0..=50.0, None);

            // Ensure a valid min/max range for cluster size.
            if volumetric_cloud_settings.cluster_size_max
                < volumetric_cloud_settings.cluster_size_min
            {
                volumetric_cloud_settings.cluster_size_max =
                    volumetric_cloud_settings.cluster_size_min;
            }
        });

    ui.separator();
    ui.label("Tip: Increase cloud count and spawn radius for more coverage. Adjust density and opacity for whiter clouds.");
    ui.label("Note: Cloud settings apply instantly. Structural changes (count/size/height/spawn radius/enabled) respawn clouds immediately.");
}

fn render_starry_sky_render_page(
    ui: &mut egui::Ui,
    starry_sky_render_settings: &mut StarrySkyRenderSettings,
) {
    egui::Grid::new("starry_sky_render_settings")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("⚠️ GHOSTING DEBUG");
            ui.label("Change settings to fix ghosting");
            ui.end_row();

            ui.separator();
            ui.end_row();

            let blend_text = match starry_sky_render_settings.blend_mode {
                SkyBlendMode::Additive => "Additive (One)",
                SkyBlendMode::Alpha => "Alpha (OneMinusSrcAlpha)",
                SkyBlendMode::PremultipliedAlpha => "Premultiplied Alpha",
                SkyBlendMode::Multiply => "Multiply",
            };
            settings_combo(
                ui,
                "starry_sky_blend_mode",
                "Blend Mode:",
                blend_text,
                &mut starry_sky_render_settings.blend_mode,
                &[
                    (SkyBlendMode::Additive, "Additive (One) - CURRENT"),
                    (
                        SkyBlendMode::Alpha,
                        "Alpha (OneMinusSrcAlpha) - FIX OPTION",
                    ),
                    (SkyBlendMode::PremultipliedAlpha, "Premultiplied Alpha"),
                    (SkyBlendMode::Multiply, "Multiply"),
                ],
            );

            let depth_text = match starry_sky_render_settings.depth_compare {
                SkyDepthCompare::Always => "Always (CURRENT)",
                SkyDepthCompare::Less => "Less",
                SkyDepthCompare::LessEqual => "LessEqual - FIX OPTION",
                SkyDepthCompare::Greater => "Greater",
                SkyDepthCompare::GreaterEqual => "GreaterEqual",
            };
            settings_combo(
                ui,
                "starry_sky_depth_compare",
                "Depth Compare:",
                depth_text,
                &mut starry_sky_render_settings.depth_compare,
                &[
                    (SkyDepthCompare::Always, "Always - Sky ignores depth"),
                    (SkyDepthCompare::Less, "Less - Strict depth test"),
                    (
                        SkyDepthCompare::LessEqual,
                        "LessEqual - Standard depth test",
                    ),
                    (SkyDepthCompare::Greater, "Greater"),
                    (SkyDepthCompare::GreaterEqual, "GreaterEqual"),
                ],
            );

            settings_checkbox(ui, "Depth Write:", &mut starry_sky_render_settings.depth_write_enabled, "Enabled (usually OFF for sky)");
            settings_slider(ui, "Depth Bias:", &mut starry_sky_render_settings.depth_bias, -10.0..=10.0, None);
            settings_slider(ui, "Alpha Cutoff:", &mut starry_sky_render_settings.alpha_cutoff, 0.0..=1.0, None);
            settings_checkbox(ui, "Force Full Brightness:", &mut starry_sky_render_settings.force_full_brightness, "Ignore night factor (DEBUG)");

            ui.separator();
            ui.end_row();

            ui.label("Quick Fixes:");
            ui.end_row();

            if ui.button("Fix: Alpha Blend + LessEqual Depth").clicked() {
                starry_sky_render_settings.blend_mode = SkyBlendMode::Alpha;
                starry_sky_render_settings.depth_compare = SkyDepthCompare::LessEqual;
            }
            ui.end_row();

            if ui.button("Reset: Additive + Always Depth").clicked() {
                starry_sky_render_settings.blend_mode = SkyBlendMode::Additive;
                starry_sky_render_settings.depth_compare = SkyDepthCompare::Always;
            }
            ui.end_row();
        });

    ui.separator();
    ui.label("TIP: Try 'Alpha Blend + LessEqual Depth' to fix ghosting.");
    ui.label("Additive blending (dst=One) accumulates color which may cause trails.");
    ui.label("Always depth compare may cause sky to render over models incorrectly.");
    ui.label("Changes require app restart to take effect (pipeline recreation).");
}

fn render_depth_of_field_page(ui: &mut egui::Ui, dof_settings: &mut DepthOfFieldSettings) {
    egui::Grid::new("dof_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Depth of Field:", &mut dof_settings.enabled, "Enabled");

            let mode_text = match dof_settings.mode {
                DepthOfFieldMode::Bokeh => "Bokeh",
                DepthOfFieldMode::Gaussian => "Gaussian",
            };
            settings_combo(
                ui,
                "dof_mode",
                "Mode:",
                mode_text,
                &mut dof_settings.mode,
                &[
                    (DepthOfFieldMode::Bokeh, "Bokeh"),
                    (DepthOfFieldMode::Gaussian, "Gaussian"),
                ],
            );

            settings_slider(ui, "Focal Distance:", &mut dof_settings.focal_distance, 1.0..=500.0, Some("m"));
            settings_slider(ui, "Aperture f-stop:", &mut dof_settings.aperture_f_stops, 0.05..=5.0, Some("f"));
            settings_slider(ui, "Max Depth:", &mut dof_settings.max_depth, 100.0..=2000.0, Some("m"));
            settings_slider(ui, "Sensor Height:", &mut dof_settings.sensor_height, 0.001..=0.1, Some("m"));
            settings_slider(ui, "Max CoC Diameter:", &mut dof_settings.max_circle_of_confusion_diameter, 1.0..=128.0, Some("px"));
        });

    ui.separator();
    ui.label("Tip: Lower f-stop = more blur. Focal distance = sharp plane.");
}

fn render_volumetric_fog_page(ui: &mut egui::Ui, zone_lighting: &mut ZoneLighting) {
    egui::Grid::new("volumetric_fog_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Volumetric Fog:", &mut zone_lighting.volumetric_fog_enabled, "Enabled");
            settings_slider(ui, "Density:", &mut zone_lighting.volumetric_density_factor, 0.0..=0.5, None);
            settings_slider(ui, "Absorption:", &mut zone_lighting.volumetric_absorption, 0.0..=0.5, None);
            settings_slider(ui, "Scattering:", &mut zone_lighting.volumetric_scattering, 0.0..=0.5, None);
            settings_slider(ui, "Scattering Asymmetry:", &mut zone_lighting.volumetric_scattering_asymmetry, -1.0..=1.0, None);
        });

    ui.separator();
    ui.label("Tip: Lower absorption = brighter scene. Higher scattering = more visible light shafts.");
}

fn render_water_page(ui: &mut egui::Ui, water_settings: &mut WaterSettings) {
    egui::Grid::new("water_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_slider(ui, "Foam Intensity:", &mut water_settings.foam_intensity, 0.0..=1.0, None);
            settings_slider(ui, "Foam Threshold:", &mut water_settings.foam_threshold, 0.0..=1.0, None);
            settings_slider(ui, "SSS Intensity:", &mut water_settings.sss_intensity, 0.0..=1.0, None);
            settings_slider(ui, "Refraction Strength:", &mut water_settings.refraction_strength, 0.0..=0.2, None);
            settings_slider(ui, "Wave Speed:", &mut water_settings.wave_speed, 0.1..=5.0, None);
            settings_slider(ui, "Fresnel Strength:", &mut water_settings.fresnel_strength, 0.0..=1.0, None);
            settings_slider(ui, "Specular Intensity:", &mut water_settings.specular_intensity, 0.0..=1.0, None);

            // === NEW DEPTH SETTINGS ===
            settings_slider(ui, "Min Depth:", &mut water_settings.min_depth, 0.1..=5.0, Some("m"));
            settings_slider(ui, "Max Depth:", &mut water_settings.max_depth, 1.0..=40.0, Some("m"));
            settings_slider(ui, "Shallow Threshold:", &mut water_settings.shallow_threshold, 0.5..=10.0, Some("m"));
            settings_slider(ui, "Bottom Visibility:", &mut water_settings.bottom_visibility, 0.0..=1.0, None);
            settings_slider(ui, "Wave Amplitude:", &mut water_settings.wave_amplitude, 0.1..=2.0, None);
            settings_slider(ui, "Wave Frequency:", &mut water_settings.wave_frequency, 0.5..=5.0, None);
            settings_slider(ui, "Wave Layers:", &mut water_settings.wave_layers, 1..=4, None);
            settings_slider(ui, "Caustics Intensity:", &mut water_settings.caustics_intensity, 0.0..=1.0, None);
            settings_slider(ui, "Caustics Scale:", &mut water_settings.caustics_scale, 0.01..=1.0, None);
            settings_slider(ui, "Caustics Speed:", &mut water_settings.caustics_speed, 0.1..=2.0, None);

            // === PLANAR REFLECTION SETTINGS ===
            settings_checkbox(ui, "Reflections:", &mut water_settings.reflection_enabled, "Enabled");
            settings_slider(ui, "Reflection Resolution:", &mut water_settings.reflection_scale, 0.25..=1.0, None);
            settings_checkbox(ui, "Debug Show Reflection:", &mut water_settings.debug_show_reflection, "Show raw reflection texture");
        });

    ui.separator();
    ui.label("Tip: Depth settings control shallow-to-deep water color transition. Wave settings control surface detail. Reflections render the scene from a mirrored camera (higher resolution = higher cost).");
}

fn render_fish_page(ui: &mut egui::Ui, fish_settings: &mut FishSettings) {
    egui::Grid::new("fish_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_slider(ui, "Fish per 1000 m²:", &mut fish_settings.fish_per_1000_sqm, 0.0..=500.0, None);
            settings_slider(ui, "Min Fish per Water:", &mut fish_settings.min_fish_per_water, 0..=100, None);
            settings_slider(ui, "Max Fish per Water:", &mut fish_settings.max_fish_per_water, 0..=1000, None);
            settings_slider(ui, "Min Depth:", &mut fish_settings.min_depth, 0.1..=10.0, Some("m"));
            settings_slider(ui, "Max Depth:", &mut fish_settings.max_depth, 0.1..=10.0, Some("m"));
            settings_slider(ui, "Min Speed:", &mut fish_settings.min_speed, 0.1..=5.0, None);
            settings_slider(ui, "Max Speed:", &mut fish_settings.max_speed, 0.1..=5.0, None);
            settings_slider(ui, "Boundary Margin:", &mut fish_settings.boundary_margin, 0.5..=1.0, None);
            settings_slider(ui, "Target Reach Dist:", &mut fish_settings.target_reach_distance, 0.1..=5.0, Some("m"));
            settings_slider(ui, "Simulation Distance:", &mut fish_settings.simulation_distance, 10.0..=500.0, Some("m"));
        });

    // Clamp min/max values to prevent crashes
    fish_settings.max_depth = fish_settings.max_depth.max(fish_settings.min_depth);
    fish_settings.max_speed = fish_settings.max_speed.max(fish_settings.min_speed);
    fish_settings.max_fish_per_water = fish_settings
        .max_fish_per_water
        .max(fish_settings.min_fish_per_water);

    ui.separator();
    ui.label("Tip: Fish count scales with water plane area (density × area, clamped to min/max). Settings apply when entering a new zone. Set density to 0 to disable fish.");
}

fn render_birds_page(ui: &mut egui::Ui, bird_settings: &mut BirdSettings) {
    egui::Grid::new("bird_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Enabled:", &mut bird_settings.enabled, "Enabled");
            settings_slider(ui, "Birds Per 1000 Units:", &mut bird_settings.birds_per_1000_units, 0.0..=200.0, None);
            settings_slider(ui, "Min Birds Per Zone:", &mut bird_settings.min_birds_per_zone, 0..=100, None);
            settings_slider(ui, "Max Birds Per Zone:", &mut bird_settings.max_birds_per_zone, 50..=500, None);
            settings_slider(ui, "Min Altitude:", &mut bird_settings.min_altitude, 10.0..=200.0, Some("m"));
            settings_slider(ui, "Max Altitude:", &mut bird_settings.max_altitude, 10.0..=200.0, Some("m"));
            settings_slider(ui, "Min Speed:", &mut bird_settings.min_speed, 1.0..=30.0, None);
            settings_slider(ui, "Max Speed:", &mut bird_settings.max_speed, 1.0..=30.0, None);
            settings_slider(ui, "Roam Radius Multiplier:", &mut bird_settings.roam_radius_multiplier, 0.1..=1.0, None);
            settings_slider(ui, "Flap Speed:", &mut bird_settings.flap_speed, 1.0..=30.0, None);
            settings_slider(ui, "Bob Amplitude:", &mut bird_settings.bob_amplitude, 0.0..=2.0, None);
            settings_slider(ui, "Bob Speed:", &mut bird_settings.bob_speed, 0.0..=10.0, None);
        });

    // Clamp min/max values to prevent crashes
    bird_settings.max_altitude = bird_settings.max_altitude.max(bird_settings.min_altitude);
    bird_settings.max_speed = bird_settings.max_speed.max(bird_settings.min_speed);
    bird_settings.max_birds_per_zone = bird_settings
        .max_birds_per_zone
        .max(bird_settings.min_birds_per_zone);

    ui.separator();
    ui.label("Note: Bird count is now relative to zone size. Birds have cartoon appearance with flapping wings.");
}

fn render_seasons_page(
    ui: &mut egui::Ui,
    season_settings: &mut SeasonSettings,
    summer_settings: &mut SummerSettings,
) {
    egui::Grid::new("season_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Weather Effects:", &mut season_settings.enabled, "Enabled");

            let season_text = match season_settings.current_season {
                Season::None => "None",
                Season::Spring => "Spring",
                Season::Summer => "Summer",
                Season::Fall => "Fall",
                Season::Winter => "Winter",
            };
            settings_combo(
                ui,
                "season",
                "Season:",
                season_text,
                &mut season_settings.current_season,
                &[
                    (Season::None, "None"),
                    (Season::Spring, "Spring"),
                    (Season::Summer, "Summer"),
                    (Season::Fall, "Fall"),
                    (Season::Winter, "Winter"),
                ],
            );

            settings_slider(ui, "Max Particles:", &mut season_settings.max_particles, 1000..=20000, None);
            settings_slider(ui, "Spawn Rate:", &mut season_settings.spawn_rate, 100.0..=5000.0, Some("/s"));
            settings_slider(ui, "Wind Strength:", &mut season_settings.wind_strength, 0.0..=5.0, None);
        });

    ui.separator();
    ui.label("Procedural Grass Settings (GPU-based)");

    egui::Grid::new("procedural_grass_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_slider(ui, "Grass Density:", &mut summer_settings.grass_density, 5..=100, Some("blades/unit"));
            settings_slider(ui, "Blade Length:", &mut summer_settings.blade_length, 0.5..=3.0, Some("m"));
            settings_slider(ui, "Blade Width:", &mut summer_settings.blade_width, 0.01..=0.2, Some("m"));
            settings_slider(ui, "Blade Tilt:", &mut summer_settings.blade_tilt, 0.0..=1.0, None);
            settings_slider(ui, "Tilt Variance:", &mut summer_settings.blade_tilt_variance, 0.0..=0.5, None);
            settings_slider(ui, "Mid Flexibility:", &mut summer_settings.blade_p1_flexibility, 0.0..=1.0, None);
            settings_slider(ui, "Tip Flexibility:", &mut summer_settings.blade_p2_flexibility, 0.0..=1.0, None);
            settings_slider(ui, "Blade Curve:", &mut summer_settings.blade_curve, 0.0..=30.0, None);
        });

    ui.separator();
    ui.label("Tip: Season changes apply immediately. Disable to turn off all weather effects.");
    ui.label("Procedural grass settings apply when entering a new zone.");
}

fn render_dirt_dash_page(ui: &mut egui::Ui, dirt_dash_settings: &mut DirtDashSettings) {
    egui::Grid::new("dust_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_slider(ui, "Max Particles:", &mut dirt_dash_settings.max_particles, 50..=1000, None);
            settings_slider(ui, "Min Size:", &mut dirt_dash_settings.min_size, 0.0..=0.7, Some("m"));
            settings_slider(ui, "Max Size:", &mut dirt_dash_settings.max_size, 0.0..=1.0, Some("m"));
            settings_slider(ui, "Min Lifetime:", &mut dirt_dash_settings.min_lifetime, 0.0..=2.0, Some("s"));
            settings_slider(ui, "Max Lifetime:", &mut dirt_dash_settings.max_lifetime, 0.0..=2.0, Some("s"));
            settings_slider(ui, "Min Upward Velocity:", &mut dirt_dash_settings.min_upward_velocity, 0.0..=1.0, Some("m/s"));
            settings_slider(ui, "Max Upward Velocity:", &mut dirt_dash_settings.max_upward_velocity, 0.0..=1.0, Some("m/s"));
            settings_slider(ui, "Gravity (float if low):", &mut dirt_dash_settings.gravity, 0.0..=2.0, Some("m/s²"));
            settings_slider(ui, "Horizontal Spread:", &mut dirt_dash_settings.horizontal_velocity_factor, 0.0..=0.3, None);
            settings_slider(ui, "Drift Speed:", &mut dirt_dash_settings.drift_speed, 0.0..=0.5, Some("m/s"));
            settings_slider(ui, "Vertical Oscillation:", &mut dirt_dash_settings.vertical_oscillation, 0.0..=0.1, Some("m"));
            settings_slider(ui, "Particle Alpha:", &mut dirt_dash_settings.particle_color.w, 0.05..=0.8, None);
        });

    // Clamp min/max values to prevent crashes
    dirt_dash_settings.max_size = dirt_dash_settings.max_size.max(dirt_dash_settings.min_size);
    dirt_dash_settings.max_lifetime = dirt_dash_settings
        .max_lifetime
        .max(dirt_dash_settings.min_lifetime);
    dirt_dash_settings.max_upward_velocity = dirt_dash_settings
        .max_upward_velocity
        .max(dirt_dash_settings.min_upward_velocity);

    ui.separator();
    ui.label("Tip: Dust particles float near the player when running. Low gravity + low velocity = hovering smoke effect.");
}

fn render_wind_sway_page(
    ui: &mut egui::Ui,
    wind_sway_settings: &mut Option<ResMut<'_, WindSwaySettings>>,
) {
    if let Some(settings) = wind_sway_settings.as_mut() {
        egui::Grid::new("wind_sway_settings")
            .num_columns(2)
            .show(ui, |ui| {
                settings_checkbox(ui, "Wind Sway:", &mut settings.enabled, "Enabled");
                settings_slider(ui, "Global Intensity:", &mut settings.global_intensity, 0.0..=3.0, None);
                settings_slider(ui, "Grass Speed:", &mut settings.grass_speed, 0.1..=10.0, None);
                settings_slider(ui, "Grass Amplitude:", &mut settings.grass_amplitude, 0.0..=0.5, Some("rad"));
                settings_slider(ui, "Tree Speed:", &mut settings.tree_speed, 0.1..=10.0, None);
                settings_slider(ui, "Tree Amplitude:", &mut settings.tree_amplitude, 0.0..=0.5, Some("rad"));
                settings_checkbox(ui, "Debug Log Count:", &mut settings.debug_log_count, "Log entity count");
            });

        ui.separator();
        ui.label("Tip: Wind sway applies to grass, leaves, bushes, and trees. Amplitude is in radians (0.1 ≈ 5.7°, 0.5 ≈ 28.6°).");
        ui.label("If no sway is visible, enable 'Debug Log Count' to check if entities have the WindSway component.");
    } else {
        ui.label("Wind Sway settings not available.");
        ui.label("The WindEffectPlugin may not be loaded yet.");
    }
}

fn render_post_processing_page(
    ui: &mut egui::Ui,
    post_processing_settings: &mut PostProcessingSettings,
) {
    egui::Grid::new("post_processing_settings")
        .num_columns(2)
        .show(ui, |ui| {
            ui.label("🔍 GHOSTING DEBUG");
            ui.label("Toggle effects to isolate ghosting cause");
            ui.end_row();

            ui.separator();
            ui.end_row();

            settings_checkbox(ui, "Bloom:", &mut post_processing_settings.bloom_enabled, "Enabled");
            settings_slider(ui, "Bloom Intensity:", &mut post_processing_settings.bloom_intensity, 0.0..=1.0, None);
            settings_checkbox(ui, "SSAO:", &mut post_processing_settings.ssao_enabled, "Enabled");
            settings_checkbox(ui, "Depth of Field:", &mut post_processing_settings.dof_enabled, "Enabled");
            settings_checkbox(ui, "Volumetric Fog:", &mut post_processing_settings.volumetric_fog_enabled, "Enabled");
            settings_checkbox(ui, "Color Grading:", &mut post_processing_settings.color_grading_enabled, "Enabled");
        });

    ui.separator();
    ui.label("TIP: Disable effects one by one to find ghosting cause.");
    ui.label("Bloom is most likely to cause trails with bright HDR content.");
    ui.label("SSAO without TAA can cause noise/flickering.");
}

fn render_graphics_page(
    ui: &mut egui::Ui,
    graphics_settings: &mut GraphicsSettings,
    zone_time: &Option<Res<'_, ZoneTime>>,
) {
    use crate::graphics::{GraphicsShadowFilteringMethod, MsaaSamples, ShadowQuality, SsaoQuality, TextureQuality, TonemappingMode, VsyncMode};

    // === Display Section ===
    ui.collapsing("Display", |ui| {
        egui::Grid::new("graphics_display")
            .num_columns(2)
            .show(ui, |ui| {
                settings_combo(
                    ui,
                    "vsync",
                    "VSync:",
                    graphics_settings.vsync_mode.display_name(),
                    &mut graphics_settings.vsync_mode,
                    &[
                        (VsyncMode::Disabled, "Disabled"),
                        (VsyncMode::Enabled, "Enabled"),
                        (VsyncMode::Mailbox, "Mailbox (Triple Buffer)"),
                    ],
                );
                settings_combo(
                    ui,
                    "msaa",
                    "Anti-Aliasing:",
                    graphics_settings.msaa_samples.display_name(),
                    &mut graphics_settings.msaa_samples,
                    &[
                        (MsaaSamples::X1, "Off"),
                        (MsaaSamples::X2, "2x MSAA"),
                        (MsaaSamples::X4, "4x MSAA"),
                        (MsaaSamples::X8, "8x MSAA"),
                    ],
                );
                settings_slider(ui, "View Distance:", &mut graphics_settings.view_distance, 100.0..=2000.0, Some("m"));
            });
    });

    // === Shadows Section ===
    ui.collapsing("Shadows", |ui| {
        egui::Grid::new("graphics_shadows")
            .num_columns(2)
            .show(ui, |ui| {
                settings_combo(
                    ui,
                    "shadow_quality",
                    "Shadow Quality:",
                    graphics_settings.shadow_quality.display_name(),
                    &mut graphics_settings.shadow_quality,
                    &[
                        (ShadowQuality::Off, "Off"),
                        (ShadowQuality::Low, "Low"),
                        (ShadowQuality::Medium, "Medium"),
                        (ShadowQuality::High, "High"),
                        (ShadowQuality::Ultra, "Ultra"),
                    ],
                );
                settings_slider(ui, "Shadow Distance:", &mut graphics_settings.shadow_max_distance, 10.0..=400.0, Some("m"));
                settings_combo(
                    ui,
                    "shadow_filtering",
                    "Shadow Filtering:",
                    graphics_settings.shadow_filtering.display_name(),
                    &mut graphics_settings.shadow_filtering,
                    &[
                        (
                            GraphicsShadowFilteringMethod::Hardware2x2,
                            "Hardware 2x2",
                        ),
                        (GraphicsShadowFilteringMethod::Gaussian, "Gaussian"),
                        (GraphicsShadowFilteringMethod::Temporal, "Temporal"),
                    ],
                );
            });
    });

    // === Image Adjustments Section ===
    ui.collapsing("Image Adjustments", |ui| {
        egui::Grid::new("graphics_image")
            .num_columns(2)
            .show(ui, |ui| {
                settings_slider(ui, "Brightness:", &mut graphics_settings.brightness, 0.0..=2.0, None);
                settings_slider(ui, "Contrast:", &mut graphics_settings.contrast, 0.0..=2.0, None);
                settings_slider(ui, "Saturation:", &mut graphics_settings.saturation, 0.0..=2.0, None);
                settings_slider(ui, "Gamma:", &mut graphics_settings.gamma, 0.5..=2.5, None);
                settings_combo(
                    ui,
                    "tonemapping",
                    "Tonemapping:",
                    graphics_settings.tonemapping.display_name(),
                    &mut graphics_settings.tonemapping,
                    &[
                        (TonemappingMode::None, "None"),
                        (TonemappingMode::Reinhard, "Reinhard"),
                        (
                            TonemappingMode::ReinhardLuminance,
                            "Reinhard Luminance",
                        ),
                        (TonemappingMode::AcesFitted, "ACES Fitted"),
                        (TonemappingMode::AgX, "AgX"),
                        (
                            TonemappingMode::SomewhatBoringDisplayTransform,
                            "Somewhat Boring",
                        ),
                        (TonemappingMode::TonyMcMapface, "TonyMcMapface"),
                        (TonemappingMode::BlenderFilmic, "Blender Filmic"),
                    ],
                );
            });
    });

    // === Effects Section ===
    ui.collapsing("Effects", |ui| {
        egui::Grid::new("graphics_effects")
            .num_columns(2)
            .show(ui, |ui| {
                settings_checkbox(ui, "Bloom:", &mut graphics_settings.bloom_enabled, "Enabled");
                settings_slider(ui, "Bloom Intensity:", &mut graphics_settings.bloom_intensity, 0.0..=1.0, None);
                settings_checkbox(ui, "Motion Blur:", &mut graphics_settings.motion_blur_enabled, "Enabled");
                settings_slider(ui, "Motion Blur Intensity:", &mut graphics_settings.motion_blur_intensity, 0.0..=1.0, None);
                settings_checkbox(ui, "SSAO:", &mut graphics_settings.ssao_enabled, "Enabled");
                settings_combo(
                    ui,
                    "ssao_quality",
                    "SSAO Quality:",
                    graphics_settings.ssao_quality.display_name(),
                    &mut graphics_settings.ssao_quality,
                    &[
                        (SsaoQuality::Off, "Off"),
                        (SsaoQuality::Low, "Low"),
                        (SsaoQuality::Medium, "Medium"),
                        (SsaoQuality::High, "High"),
                        (SsaoQuality::Ultra, "Ultra"),
                    ],
                );
                settings_checkbox(ui, "Depth of Field:", &mut graphics_settings.dof_enabled, "Enabled");
            });
    });

    // === Textures Section ===
    ui.collapsing("Textures", |ui| {
        egui::Grid::new("graphics_textures")
            .num_columns(2)
            .show(ui, |ui| {
                settings_combo(
                    ui,
                    "texture_quality",
                    "Texture Quality:",
                    graphics_settings.texture_quality.display_name(),
                    &mut graphics_settings.texture_quality,
                    &[
                        (TextureQuality::Low, "Low"),
                        (TextureQuality::Medium, "Medium"),
                        (TextureQuality::High, "High"),
                        (TextureQuality::Ultra, "Ultra"),
                    ],
                );
            });
    });

    // === Ambient Lighting Section ===
    ui.collapsing("Ambient Lighting", |ui| {
        egui::Grid::new("graphics_ambient")
            .num_columns(2)
            .show(ui, |ui| {
                settings_slider(ui, "Brightness:", &mut graphics_settings.ambient_light_brightness, 0.5..=3.0, None);

                ui.label("Color:");
                let mut color_array = [
                    graphics_settings.ambient_light_color.to_srgba().red,
                    graphics_settings.ambient_light_color.to_srgba().green,
                    graphics_settings.ambient_light_color.to_srgba().blue,
                ];
                if ui.color_edit_button_rgb(&mut color_array).changed() {
                    graphics_settings.ambient_light_color = Color::srgb(
                        color_array[0],
                        color_array[1],
                        color_array[2],
                    );
                }
                ui.end_row();
            });
    });

    // === Terrain Lighting Section ===
    ui.collapsing("Terrain Lighting", |ui| {
        egui::Grid::new("graphics_terrain")
            .num_columns(2)
            .show(ui, |ui| {
                settings_slider(ui, "Base Intensity:", &mut graphics_settings.terrain_light_intensity, 1.0..=20.0, None);

                // Calculate and display effective intensity based on time of day
                if let Some(zone_time) = zone_time.as_ref() {
                    let time_mult = match zone_time.state {
                        ZoneTimeState::Morning => 2.0,
                        ZoneTimeState::Day => 2.5,
                        ZoneTimeState::Evening => 2.0,
                        ZoneTimeState::Night => 1.0,
                    };
                    let effective = (graphics_settings.terrain_light_intensity * time_mult) / 5.0;
                    let state_name = match zone_time.state {
                        ZoneTimeState::Morning => "Morning",
                        ZoneTimeState::Day => "Day",
                        ZoneTimeState::Evening => "Evening",
                        ZoneTimeState::Night => "Night",
                    };
                    ui.label("Effective Intensity:");
                    ui.label(format!("{:.2} ({:.1}x - {})", effective, time_mult, state_name));
                    ui.end_row();
                } else {
                    ui.label("Effective Intensity:");
                    ui.label("N/A (Zone not loaded)");
                    ui.end_row();
                }
            });
        ui.label("Tip: Effective intensity = (base × time_multiplier) / 5.0. Changes with time of day.");
    });

    ui.separator();

    // Preset buttons
    ui.horizontal(|ui| {
        if ui.button("Low Preset").clicked() {
            *graphics_settings = GraphicsSettings::low_preset();
        }
        if ui.button("Medium Preset").clicked() {
            *graphics_settings = GraphicsSettings::medium_preset();
        }
        if ui.button("High Preset").clicked() {
            *graphics_settings = GraphicsSettings::high_preset();
        }
        if ui.button("Ultra Preset").clicked() {
            *graphics_settings = GraphicsSettings::ultra_preset();
        }
    });

    ui.separator();
    ui.label("Tip: Higher shadow quality improves visual fidelity but reduces FPS.");
    ui.label("Changes to MSAA and VSync may require restart to take full effect.");
}

fn render_terrain_page(ui: &mut egui::Ui, terrain_settings: &mut TerrainEnhancementSettings) {
    egui::Grid::new("terrain_settings")
        .num_columns(2)
        .show(ui, |ui| {
            settings_checkbox(ui, "Terrain Noise:", &mut terrain_settings.noise_enabled, "Enabled");
            settings_slider(ui, "Noise Amplitude:", &mut terrain_settings.noise_amplitude, 0.0..=10.0, Some("units"));
            settings_slider(ui, "Noise Scale:", &mut terrain_settings.noise_scale, 0.001..=0.05, Some("frequency"));
            settings_slider(ui, "Noise Octaves:", &mut terrain_settings.noise_octaves, 1..=8, Some("layers"));
            settings_slider(ui, "Noise Persistence:", &mut terrain_settings.noise_persistence, 0.0..=1.0, None);
            settings_slider(ui, "Noise Seed:", &mut terrain_settings.noise_seed, 0..=1000u32, None);
        });

    ui.separator();
    ui.collapsing("Elevation Zones", |ui| {
        egui::Grid::new("terrain_elevation_settings")
            .num_columns(2)
            .show(ui, |ui| {
                settings_slider(ui, "Valley Threshold:", &mut terrain_settings.elevation_zone_low, 0.0..=20.0, Some("units"));
                settings_slider(ui, "Mountain Threshold:", &mut terrain_settings.elevation_zone_high, 10.0..=100.0, Some("units"));
                settings_slider(ui, "Valley Noise Multiplier:", &mut terrain_settings.valley_noise_multiplier, 0.0..=2.0, None);
                settings_slider(ui, "Mountain Noise Multiplier:", &mut terrain_settings.mountain_noise_multiplier, 0.0..=3.0, None);
                settings_slider(ui, "Transition Smoothness:", &mut terrain_settings.elevation_transition_smoothness, 0.0..=1.0, None);
            });
    });

    ui.separator();
    ui.collapsing("Blend Zones (Near Objects)", |ui| {
        egui::Grid::new("terrain_blend_settings")
            .num_columns(2)
            .show(ui, |ui| {
                settings_checkbox(ui, "Blend Near Objects:", &mut terrain_settings.blend_near_objects, "Enabled");
                settings_slider(ui, "Blend Distance:", &mut terrain_settings.blend_distance, 5.0..=100.0, Some("units"));
                settings_slider(ui, "Blend Curve Power:", &mut terrain_settings.blend_curve_power, 1.0..=4.0, None);
            });
    });

    ui.separator();
    ui.label("⚠️ Note: Terrain mesh is generated when a zone loads.");
    ui.label("Changes take effect on next zone load.");
}

/// Helper function to format time of day as a human-readable string
fn format_time_of_day(hours: f32) -> String {
    let hour = hours.floor() as i32 % 24;
    let minutes = ((hours % 1.0) * 60.0).round() as i32;

    let period = if hour < 6 {
        "Night"
    } else if hour < 8 {
        "Dawn"
    } else if hour < 12 {
        "Morning"
    } else if hour < 14 {
        "Noon"
    } else if hour < 17 {
        "Afternoon"
    } else if hour < 20 {
        "Evening"
    } else {
        "Night"
    };

    format!("{:02}:{:02} ({})", hour, minutes, period)
}
