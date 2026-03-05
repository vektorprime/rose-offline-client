use bevy::core_pipeline::dof::DepthOfFieldMode;
use bevy::ecs::system::SystemParam;
use bevy::prelude::{Color, Local, Query, Res, ResMut, Resource};
use bevy_egui::{egui, EguiContexts};

use crate::{
    audio::SoundGain,
    components::{BirdSettings, DirtDashSettings, FishSettings, Season, SoundCategory, WindSwaySettings},
    graphics::GraphicsSettings,
    render::{SkyMode, SkySettings, StarrySkySettings, ZoneLighting},
    resources::{SeasonSettings, SoundSettings, WaterSettings, ZoneTime, ZoneTimeState},
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
            dof_enabled: false,
            volumetric_fog_enabled: true,
            color_grading_enabled: false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum SettingsPage {
    Sound,
    Sky,
    Stars,
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
            enabled: false,
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
    pub query_sounds: Query<'w, 's, (&'static SoundCategory, &'static mut SoundGain)>,
    pub sky_settings: ResMut<'w, SkySettings>,
    pub starry_sky_settings: ResMut<'w, StarrySkySettings>,
    pub starry_sky_render_settings: ResMut<'w, StarrySkyRenderSettings>,
    pub dof_settings: ResMut<'w, DepthOfFieldSettings>,
    pub zone_lighting: ResMut<'w, ZoneLighting>,
    pub water_settings: ResMut<'w, WaterSettings>,
    pub fish_settings: ResMut<'w, FishSettings>,
    pub bird_settings: ResMut<'w, BirdSettings>,
    pub season_settings: ResMut<'w, SeasonSettings>,
    pub dirt_dash_settings: ResMut<'w, DirtDashSettings>,
    pub wind_sway_settings: Option<ResMut<'w, WindSwaySettings>>,
    pub post_processing_settings: ResMut<'w, PostProcessingSettings>,
    pub graphics_settings: ResMut<'w, GraphicsSettings>,
    pub zone_time: Option<Res<'w, ZoneTime>>,
}

pub fn ui_settings_system(mut params: SettingsSystemParams) {
    let SettingsSystemParams {
        mut egui_context,
        mut ui_state_windows,
        mut ui_state_settings,
        mut sound_settings,
        mut query_sounds,
        mut sky_settings,
        mut starry_sky_settings,
        mut starry_sky_render_settings,
        mut dof_settings,
        mut zone_lighting,
        mut water_settings,
        mut fish_settings,
        mut bird_settings,
        mut season_settings,
        mut dirt_dash_settings,
        wind_sway_settings,
        mut post_processing_settings,
        mut graphics_settings,
        zone_time,
    } = params;

    egui::Window::new("Settings")
        .open(&mut ui_state_windows.settings_open)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sound, "Sound");
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Sky,
                    "Sky",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Stars,
                    "Stars",
                );
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
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Water,
                    "Water",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Fish,
                    "Fish",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Birds,
                    "Birds",
                );
                ui.selectable_value(
                    &mut ui_state_settings.page,
                    SettingsPage::Seasons,
                    "Seasons",
                );
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
            });

            ui.separator();

            match ui_state_settings.page {
                SettingsPage::Sound => {
                    egui::Grid::new("sound_settings_gain")
                        .num_columns(2)
                        .show(ui, |ui| {
                            let mut gain_changed = false;

                            ui.label("Sound:");
                            gain_changed |= ui
                                .checkbox(&mut sound_settings.enabled, "Enabled")
                                .changed();
                            ui.end_row();

                            ui.label("Global Volume:");
                            gain_changed |= ui
                                .add(
                                    egui::Slider::new(&mut sound_settings.global_gain, 0.0..=1.0)
                                        .show_value(true),
                                )
                                .changed();
                            ui.end_row();

                            let mut add_category_slider = |text: &str, category| {
                                ui.label(text);
                                gain_changed |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut sound_settings.gains[category],
                                            0.0..=1.0,
                                        )
                                        .show_value(true),
                                    )
                                    .changed();
                                ui.end_row();
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
                SettingsPage::Sky => {
                    egui::Grid::new("sky_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Time Mode:");
                            let mode_text = match sky_settings.mode {
                                SkyMode::Automatic => "Automatic (Game Time)",
                                SkyMode::Manual => "Manual",
                            };
                            egui::ComboBox::from_label("")
                                .selected_text(mode_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut sky_settings.mode,
                                        SkyMode::Automatic,
                                        "Automatic (Game Time)",
                                    );
                                    ui.selectable_value(
                                        &mut sky_settings.mode,
                                        SkyMode::Manual,
                                        "Manual",
                                    );
                                });
                            ui.end_row();

                            // Only show time slider in manual mode
                            if sky_settings.mode == SkyMode::Manual {
                                ui.label("Time of Day:");
                                ui.add(
                                    egui::Slider::new(&mut sky_settings.manual_time, 0.0..=24.0)
                                        .text("hours")
                                        .show_value(true),
                                );
                                ui.end_row();
                                
                                // Show time description
                                let time_desc = format_time_of_day(sky_settings.manual_time);
                                ui.label("");
                                ui.label(time_desc);
                                ui.end_row();
                            }

                            ui.label("Atmosphere Intensity:");
                            ui.add(
                                egui::Slider::new(&mut sky_settings.atmosphere_intensity, 0.0..=2.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    ui.separator();
                    if sky_settings.mode == SkyMode::Automatic {
                        ui.label("Tip: Time follows game time automatically. Switch to Manual mode to control time yourself.");
                    } else {
                        ui.label("Tip: Drag the time slider to change time of day. 6 = sunrise, 12 = noon, 18 = sunset, 0 = midnight.");
                    }
                }
                SettingsPage::Stars => {
                    egui::Grid::new("starry_sky_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Star Density:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_settings.star_density, 0.0..=1.0)
                                    .text("density")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Star Brightness:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_settings.star_brightness, 0.0..=5.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Moon Phase:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_settings.moon_phase, 0.0..=1.0)
                                    .text("phase")
                                    .show_value(true),
                            );
                            ui.end_row();

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

                            ui.label("Moon Direction X:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_settings.moon_direction.x, -1.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Moon Direction Y:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_settings.moon_direction.y, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Moon Direction Z:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_settings.moon_direction.z, -1.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            // Normalize button
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
                SettingsPage::StarrySkyRender => {
                    egui::Grid::new("starry_sky_render_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("⚠️ GHOSTING DEBUG");
                            ui.label("Change settings to fix ghosting");
                            ui.end_row();
                            
                            ui.separator();
                            ui.end_row();
                            
                            ui.label("Blend Mode:");
                            let blend_text = match starry_sky_render_settings.blend_mode {
                                SkyBlendMode::Additive => "Additive (One)",
                                SkyBlendMode::Alpha => "Alpha (OneMinusSrcAlpha)",
                                SkyBlendMode::PremultipliedAlpha => "Premultiplied Alpha",
                                SkyBlendMode::Multiply => "Multiply",
                            };
                            egui::ComboBox::from_label("")
                                .selected_text(blend_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.blend_mode,
                                        SkyBlendMode::Additive,
                                        "Additive (One) - CURRENT",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.blend_mode,
                                        SkyBlendMode::Alpha,
                                        "Alpha (OneMinusSrcAlpha) - FIX OPTION",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.blend_mode,
                                        SkyBlendMode::PremultipliedAlpha,
                                        "Premultiplied Alpha",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.blend_mode,
                                        SkyBlendMode::Multiply,
                                        "Multiply",
                                    );
                                });
                            ui.end_row();
                            
                            ui.label("Depth Compare:");
                            let depth_text = match starry_sky_render_settings.depth_compare {
                                SkyDepthCompare::Always => "Always (CURRENT)",
                                SkyDepthCompare::Less => "Less",
                                SkyDepthCompare::LessEqual => "LessEqual - FIX OPTION",
                                SkyDepthCompare::Greater => "Greater",
                                SkyDepthCompare::GreaterEqual => "GreaterEqual",
                            };
                            egui::ComboBox::from_label("")
                                .selected_text(depth_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.depth_compare,
                                        SkyDepthCompare::Always,
                                        "Always - Sky ignores depth",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.depth_compare,
                                        SkyDepthCompare::Less,
                                        "Less - Strict depth test",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.depth_compare,
                                        SkyDepthCompare::LessEqual,
                                        "LessEqual - Standard depth test",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.depth_compare,
                                        SkyDepthCompare::Greater,
                                        "Greater",
                                    );
                                    ui.selectable_value(
                                        &mut starry_sky_render_settings.depth_compare,
                                        SkyDepthCompare::GreaterEqual,
                                        "GreaterEqual",
                                    );
                                });
                            ui.end_row();
                            
                            ui.label("Depth Write:");
                            ui.checkbox(&mut starry_sky_render_settings.depth_write_enabled, "Enabled (usually OFF for sky)");
                            ui.end_row();
                            
                            ui.label("Depth Bias:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_render_settings.depth_bias, -10.0..=10.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                            
                            ui.label("Alpha Cutoff:");
                            ui.add(
                                egui::Slider::new(&mut starry_sky_render_settings.alpha_cutoff, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                            
                            ui.label("Force Full Brightness:");
                            ui.checkbox(&mut starry_sky_render_settings.force_full_brightness, "Ignore night factor (DEBUG)");
                            ui.end_row();
                            
                            ui.separator();
                            ui.end_row();
                            
                            // Quick fix buttons
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
                SettingsPage::DepthOfField => {
                    egui::Grid::new("dof_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Depth of Field:");
                            ui.checkbox(&mut dof_settings.enabled, "Enabled");
                            ui.end_row();

                            ui.label("Mode:");
                            let mode_text = match dof_settings.mode {
                                DepthOfFieldMode::Bokeh => "Bokeh",
                                DepthOfFieldMode::Gaussian => "Gaussian",
                            };
                            egui::ComboBox::from_label("")
                                .selected_text(mode_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut dof_settings.mode,
                                        DepthOfFieldMode::Bokeh,
                                        "Bokeh",
                                    );
                                    ui.selectable_value(
                                        &mut dof_settings.mode,
                                        DepthOfFieldMode::Gaussian,
                                        "Gaussian",
                                    );
                                });
                            ui.end_row();

                            ui.label("Focal Distance:");
                            ui.add(
                                egui::Slider::new(&mut dof_settings.focal_distance, 1.0..=500.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Aperture f-stop:");
                            ui.add(
                                egui::Slider::new(&mut dof_settings.aperture_f_stops, 0.05..=5.0)
                                    .text("f")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Depth:");
                            ui.add(
                                egui::Slider::new(&mut dof_settings.max_depth, 100.0..=2000.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Sensor Height:");
                            ui.add(
                                egui::Slider::new(
                                    &mut dof_settings.sensor_height,
                                    0.001..=0.1,
                                )
                                .text("m")
                                .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max CoC Diameter:");
                            ui.add(
                                egui::Slider::new(
                                    &mut dof_settings.max_circle_of_confusion_diameter,
                                    1.0..=128.0,
                                )
                                .text("px")
                                .show_value(true),
                            );
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("Tip: Lower f-stop = more blur. Focal distance = sharp plane.");
                }
                SettingsPage::VolumetricFog => {
                    egui::Grid::new("volumetric_fog_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Volumetric Fog:");
                            ui.checkbox(&mut zone_lighting.volumetric_fog_enabled, "Enabled");
                            ui.end_row();

                            ui.label("Density:");
                            ui.add(
                                egui::Slider::new(&mut zone_lighting.volumetric_density_factor, 0.0..=0.5)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Absorption:");
                            ui.add(
                                egui::Slider::new(&mut zone_lighting.volumetric_absorption, 0.0..=0.5)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Scattering:");
                            ui.add(
                                egui::Slider::new(&mut zone_lighting.volumetric_scattering, 0.0..=0.5)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Scattering Asymmetry:");
                            ui.add(
                                egui::Slider::new(&mut zone_lighting.volumetric_scattering_asymmetry, -1.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("Tip: Lower absorption = brighter scene. Higher scattering = more visible light shafts.");
                }
                SettingsPage::Water => {
                    egui::Grid::new("water_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Foam Intensity:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.foam_intensity, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Foam Threshold:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.foam_threshold, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("SSS Intensity:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.sss_intensity, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Refraction Strength:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.refraction_strength, 0.0..=0.2)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Wave Speed:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.wave_speed, 0.1..=5.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Fresnel Strength:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.fresnel_strength, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Specular Intensity:");
                            ui.add(
                                egui::Slider::new(&mut water_settings.specular_intensity, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("Tip: Adjust foam for wave crests, SSS for light scattering, fresnel for angle reflectivity.");
                }
                SettingsPage::Fish => {
                    egui::Grid::new("fish_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Fish per Water:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.fish_count_per_water, 0..=200)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Depth:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.min_depth, 0.1..=10.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Depth:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.max_depth, 0.1..=10.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Speed:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.min_speed, 0.1..=5.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Speed:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.max_speed, 0.1..=5.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Boundary Margin:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.boundary_margin, 0.5..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Target Reach Dist:");
                            ui.add(
                                egui::Slider::new(&mut fish_settings.target_reach_distance, 0.5..=5.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    // Clamp min/max values to prevent crashes
                    fish_settings.max_depth = fish_settings.max_depth.max(fish_settings.min_depth);
                    fish_settings.max_speed = fish_settings.max_speed.max(fish_settings.min_speed);

                    ui.separator();
                    ui.label("Tip: Fish settings apply when entering a new zone. Set fish count to 0 to disable.");
                }
                SettingsPage::Birds => {
                    egui::Grid::new("bird_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Enabled:");
                            ui.checkbox(&mut bird_settings.enabled, "Enabled");
                            ui.end_row();

                            ui.label("Birds Per 1000 Units:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.birds_per_1000_units, 0.0..=200.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Birds Per Zone:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.min_birds_per_zone, 0..=100)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Birds Per Zone:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.max_birds_per_zone, 50..=500)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Altitude:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.min_altitude, 10.0..=200.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Altitude:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.max_altitude, 10.0..=200.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Speed:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.min_speed, 1.0..=30.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Speed:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.max_speed, 1.0..=30.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Roam Radius Multiplier:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.roam_radius_multiplier, 0.1..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Flap Speed:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.flap_speed, 1.0..=30.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Bob Amplitude:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.bob_amplitude, 0.0..=2.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Bob Speed:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.bob_speed, 0.0..=10.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    // Clamp min/max values to prevent crashes
                    bird_settings.max_altitude = bird_settings.max_altitude.max(bird_settings.min_altitude);
                    bird_settings.max_speed = bird_settings.max_speed.max(bird_settings.min_speed);
                    bird_settings.max_birds_per_zone = bird_settings.max_birds_per_zone.max(bird_settings.min_birds_per_zone);

                    ui.separator();
                    ui.label("Note: Bird count is now relative to zone size. Birds have cartoon appearance with flapping wings.");
                }
                SettingsPage::Seasons => {
                    egui::Grid::new("season_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Weather Effects:");
                            ui.checkbox(&mut season_settings.enabled, "Enabled");
                            ui.end_row();

                            ui.label("Season:");
                            let season_text = match season_settings.current_season {
                                Season::None => "None",
                                Season::Spring => "Spring",
                                Season::Summer => "Summer",
                                Season::Fall => "Fall",
                                Season::Winter => "Winter",
                            };
                            egui::ComboBox::from_label("")
                                .selected_text(season_text)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut season_settings.current_season,
                                        Season::None,
                                        "None",
                                    );
                                    ui.selectable_value(
                                        &mut season_settings.current_season,
                                        Season::Spring,
                                        "Spring",
                                    );
                                    ui.selectable_value(
                                        &mut season_settings.current_season,
                                        Season::Summer,
                                        "Summer",
                                    );
                                    ui.selectable_value(
                                        &mut season_settings.current_season,
                                        Season::Fall,
                                        "Fall",
                                    );
                                    ui.selectable_value(
                                        &mut season_settings.current_season,
                                        Season::Winter,
                                        "Winter",
                                    );
                                });
                            ui.end_row();

                            ui.label("Max Particles:");
                            ui.add(
                                egui::Slider::new(&mut season_settings.max_particles, 1000..=20000)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Spawn Rate:");
                            ui.add(
                                egui::Slider::new(&mut season_settings.spawn_rate, 100.0..=5000.0)
                                    .text("/s")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Wind Strength:");
                            ui.add(
                                egui::Slider::new(&mut season_settings.wind_strength, 0.0..=5.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("Tip: Season changes apply immediately. Disable to turn off all weather effects.");
                }
                SettingsPage::DirtDash => {
                    egui::Grid::new("dust_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Max Particles:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.max_particles, 50..=1000)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Size:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.min_size, 0.0..=0.7)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Size:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.max_size, 0.0..=1.0)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Lifetime:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.min_lifetime, 0.0..=2.0)
                                    .text("s")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Lifetime:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.max_lifetime, 0.0..=2.0)
                                    .text("s")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Min Upward Velocity:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.min_upward_velocity, 0.0..=1.0)
                                    .text("m/s")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Max Upward Velocity:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.max_upward_velocity, 0.0..=1.0)
                                    .text("m/s")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Gravity (float if low):");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.gravity, 0.0..=2.0)
                                    .text("m/s²")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Horizontal Spread:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.horizontal_velocity_factor, 0.0..=0.3)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Drift Speed:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.drift_speed, 0.0..=0.5)
                                    .text("m/s")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Vertical Oscillation:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.vertical_oscillation, 0.0..=0.1)
                                    .text("m")
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("Particle Alpha:");
                            ui.add(
                                egui::Slider::new(&mut dirt_dash_settings.particle_color.w, 0.05..=0.8)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });

                    // Clamp min/max values to prevent crashes
                    dirt_dash_settings.max_size = dirt_dash_settings.max_size.max(dirt_dash_settings.min_size);
                    dirt_dash_settings.max_lifetime = dirt_dash_settings.max_lifetime.max(dirt_dash_settings.min_lifetime);
                    dirt_dash_settings.max_upward_velocity = dirt_dash_settings.max_upward_velocity.max(dirt_dash_settings.min_upward_velocity);

                    ui.separator();
                    ui.label("Tip: Dust particles float near the player when running. Low gravity + low velocity = hovering smoke effect.");
                }
                SettingsPage::WindSway => {
                    if let Some(mut settings) = wind_sway_settings {
                        egui::Grid::new("wind_sway_settings")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Wind Sway:");
                                ui.checkbox(&mut settings.enabled, "Enabled");
                                ui.end_row();

                                ui.label("Global Intensity:");
                                ui.add(
                                    egui::Slider::new(&mut settings.global_intensity, 0.0..=3.0)
                                        .show_value(true),
                                );
                                ui.end_row();

                                ui.label("Grass Speed:");
                                ui.add(
                                    egui::Slider::new(&mut settings.grass_speed, 0.1..=10.0)
                                        .show_value(true),
                                );
                                ui.end_row();

                                ui.label("Grass Amplitude:");
                                ui.add(
                                    egui::Slider::new(&mut settings.grass_amplitude, 0.0..=0.5)
                                        .text("rad")
                                        .show_value(true),
                                );
                                ui.end_row();

                                ui.label("Tree Speed:");
                                ui.add(
                                    egui::Slider::new(&mut settings.tree_speed, 0.1..=10.0)
                                        .show_value(true),
                                );
                                ui.end_row();

                                ui.label("Tree Amplitude:");
                                ui.add(
                                    egui::Slider::new(&mut settings.tree_amplitude, 0.0..=0.5)
                                        .text("rad")
                                        .show_value(true),
                                );
                                ui.end_row();

                                ui.label("Debug Log Count:");
                                ui.checkbox(&mut settings.debug_log_count, "Log entity count");
                                ui.end_row();
                            });

                        ui.separator();
                        ui.label("Tip: Wind sway applies to grass, leaves, bushes, and trees. Amplitude is in radians (0.1 ≈ 5.7°, 0.5 ≈ 28.6°).");
                        ui.label("If no sway is visible, enable 'Debug Log Count' to check if entities have the WindSway component.");
                    } else {
                        ui.label("Wind Sway settings not available.");
                        ui.label("The WindEffectPlugin may not be loaded yet.");
                    }
                }
                SettingsPage::PostProcessing => {
                    egui::Grid::new("post_processing_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("🔍 GHOSTING DEBUG");
                            ui.label("Toggle effects to isolate ghosting cause");
                            ui.end_row();
                            
                            ui.separator();
                            ui.end_row();
                            
                            ui.label("Bloom:");
                            ui.checkbox(&mut post_processing_settings.bloom_enabled, "Enabled");
                            ui.end_row();

                            ui.label("Bloom Intensity:");
                            ui.add(
                                egui::Slider::new(&mut post_processing_settings.bloom_intensity, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();

                            ui.label("SSAO:");
                            ui.checkbox(&mut post_processing_settings.ssao_enabled, "Enabled");
                            ui.end_row();

                            ui.label("Depth of Field:");
                            ui.checkbox(&mut post_processing_settings.dof_enabled, "Enabled");
                            ui.end_row();

                            ui.label("Volumetric Fog:");
                            ui.checkbox(&mut post_processing_settings.volumetric_fog_enabled, "Enabled");
                            ui.end_row();

                            ui.label("Color Grading:");
                            ui.checkbox(&mut post_processing_settings.color_grading_enabled, "Enabled");
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("TIP: Disable effects one by one to find ghosting cause.");
                    ui.label("Bloom is most likely to cause trails with bright HDR content.");
                    ui.label("SSAO without TAA can cause noise/flickering.");
                }
                SettingsPage::Graphics => {
                    use crate::graphics::{GraphicsShadowFilteringMethod, MsaaSamples, ShadowQuality, SsaoQuality, TextureQuality, TonemappingMode, VsyncMode};
                    
                    // === Display Section ===
                    ui.collapsing("Display", |ui| {
                        egui::Grid::new("graphics_display")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("VSync:");
                                egui::ComboBox::from_id_salt("vsync")
                                    .selected_text(graphics_settings.vsync_mode.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.vsync_mode, VsyncMode::Disabled, "Disabled");
                                        ui.selectable_value(&mut graphics_settings.vsync_mode, VsyncMode::Enabled, "Enabled");
                                        ui.selectable_value(&mut graphics_settings.vsync_mode, VsyncMode::Mailbox, "Mailbox (Triple Buffer)");
                                    });
                                ui.end_row();

                                ui.label("Anti-Aliasing:");
                                egui::ComboBox::from_id_salt("msaa")
                                    .selected_text(graphics_settings.msaa_samples.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.msaa_samples, MsaaSamples::X1, "Off");
                                        ui.selectable_value(&mut graphics_settings.msaa_samples, MsaaSamples::X2, "2x MSAA");
                                        ui.selectable_value(&mut graphics_settings.msaa_samples, MsaaSamples::X4, "4x MSAA");
                                        ui.selectable_value(&mut graphics_settings.msaa_samples, MsaaSamples::X8, "8x MSAA");
                                    });
                                ui.end_row();

                                ui.label("View Distance:");
                                ui.add(egui::Slider::new(&mut graphics_settings.view_distance, 100.0..=2000.0)
                                    .text("m")
                                    .show_value(true));
                                ui.end_row();
                            });
                    });

                    // === Shadows Section ===
                    ui.collapsing("Shadows", |ui| {
                        egui::Grid::new("graphics_shadows")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Shadow Quality:");
                                egui::ComboBox::from_id_salt("shadow_quality")
                                    .selected_text(graphics_settings.shadow_quality.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.shadow_quality, ShadowQuality::Off, "Off");
                                        ui.selectable_value(&mut graphics_settings.shadow_quality, ShadowQuality::Low, "Low");
                                        ui.selectable_value(&mut graphics_settings.shadow_quality, ShadowQuality::Medium, "Medium");
                                        ui.selectable_value(&mut graphics_settings.shadow_quality, ShadowQuality::High, "High");
                                        ui.selectable_value(&mut graphics_settings.shadow_quality, ShadowQuality::Ultra, "Ultra");
                                    });
                                ui.end_row();

                                ui.label("Shadow Distance:");
                                ui.add(egui::Slider::new(&mut graphics_settings.shadow_max_distance, 10.0..=400.0)
                                    .text("m")
                                    .show_value(true));
                                ui.end_row();

                                ui.label("Shadow Filtering:");
                                egui::ComboBox::from_id_salt("shadow_filtering")
                                    .selected_text(graphics_settings.shadow_filtering.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.shadow_filtering, GraphicsShadowFilteringMethod::Hardware2x2, "Hardware 2x2");
                                        ui.selectable_value(&mut graphics_settings.shadow_filtering, GraphicsShadowFilteringMethod::Gaussian, "Gaussian");
                                        ui.selectable_value(&mut graphics_settings.shadow_filtering, GraphicsShadowFilteringMethod::Temporal, "Temporal");
                                    });
                                ui.end_row();
                            });
                    });

                    // === Image Adjustments Section ===
                    ui.collapsing("Image Adjustments", |ui| {
                        egui::Grid::new("graphics_image")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Brightness:");
                                ui.add(egui::Slider::new(&mut graphics_settings.brightness, 0.0..=2.0)
                                    .show_value(true));
                                ui.end_row();

                                ui.label("Contrast:");
                                ui.add(egui::Slider::new(&mut graphics_settings.contrast, 0.0..=2.0)
                                    .show_value(true));
                                ui.end_row();

                                ui.label("Saturation:");
                                ui.add(egui::Slider::new(&mut graphics_settings.saturation, 0.0..=2.0)
                                    .show_value(true));
                                ui.end_row();

                                ui.label("Gamma:");
                                ui.add(egui::Slider::new(&mut graphics_settings.gamma, 0.5..=2.5)
                                    .show_value(true));
                                ui.end_row();

                                ui.label("Tonemapping:");
                                egui::ComboBox::from_id_salt("tonemapping")
                                    .selected_text(graphics_settings.tonemapping.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::None, "None");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::Reinhard, "Reinhard");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::ReinhardLuminance, "Reinhard Luminance");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::AcesFitted, "ACES Fitted");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::AgX, "AgX");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::SomewhatBoringDisplayTransform, "Somewhat Boring");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::TonyMcMapface, "TonyMcMapface");
                                        ui.selectable_value(&mut graphics_settings.tonemapping, TonemappingMode::BlenderFilmic, "Blender Filmic");
                                    });
                                ui.end_row();
                            });
                    });

                    // === Effects Section ===
                    ui.collapsing("Effects", |ui| {
                        egui::Grid::new("graphics_effects")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Bloom:");
                                ui.checkbox(&mut graphics_settings.bloom_enabled, "Enabled");
                                ui.end_row();

                                ui.label("Bloom Intensity:");
                                ui.add(egui::Slider::new(&mut graphics_settings.bloom_intensity, 0.0..=1.0)
                                    .show_value(true));
                                ui.end_row();

                                ui.label("Motion Blur:");
                                ui.checkbox(&mut graphics_settings.motion_blur_enabled, "Enabled");
                                ui.end_row();

                                ui.label("Motion Blur Intensity:");
                                ui.add(egui::Slider::new(&mut graphics_settings.motion_blur_intensity, 0.0..=1.0)
                                    .show_value(true));
                                ui.end_row();

                                ui.label("SSAO:");
                                ui.checkbox(&mut graphics_settings.ssao_enabled, "Enabled");
                                ui.end_row();

                                ui.label("SSAO Quality:");
                                egui::ComboBox::from_id_salt("ssao_quality")
                                    .selected_text(graphics_settings.ssao_quality.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.ssao_quality, SsaoQuality::Off, "Off");
                                        ui.selectable_value(&mut graphics_settings.ssao_quality, SsaoQuality::Low, "Low");
                                        ui.selectable_value(&mut graphics_settings.ssao_quality, SsaoQuality::Medium, "Medium");
                                        ui.selectable_value(&mut graphics_settings.ssao_quality, SsaoQuality::High, "High");
                                        ui.selectable_value(&mut graphics_settings.ssao_quality, SsaoQuality::Ultra, "Ultra");
                                    });
                                ui.end_row();

                                ui.label("Depth of Field:");
                                ui.checkbox(&mut graphics_settings.dof_enabled, "Enabled");
                                ui.end_row();
                            });
                    });

                    // === Textures Section ===
                    ui.collapsing("Textures", |ui| {
                        egui::Grid::new("graphics_textures")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Texture Quality:");
                                egui::ComboBox::from_id_salt("texture_quality")
                                    .selected_text(graphics_settings.texture_quality.display_name())
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut graphics_settings.texture_quality, TextureQuality::Low, "Low");
                                        ui.selectable_value(&mut graphics_settings.texture_quality, TextureQuality::Medium, "Medium");
                                        ui.selectable_value(&mut graphics_settings.texture_quality, TextureQuality::High, "High");
                                        ui.selectable_value(&mut graphics_settings.texture_quality, TextureQuality::Ultra, "Ultra");
                                    });
                                ui.end_row();
                            });
                    });

                    // === Ambient Lighting Section ===
                    ui.collapsing("Ambient Lighting", |ui| {
                        egui::Grid::new("graphics_ambient")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.label("Brightness:");
                                ui.add(egui::Slider::new(&mut graphics_settings.ambient_light_brightness, 0.5..=3.0)
                                    .show_value(true));
                                ui.end_row();

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
                                ui.label("Base Intensity:");
                                ui.add(egui::Slider::new(&mut graphics_settings.terrain_light_intensity, 1.0..=20.0)
                                    .show_value(true));
                                ui.end_row();

                                // Calculate and display effective intensity based on time of day
                                if let Some(ref zone_time) = zone_time {
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
            }
        });
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
