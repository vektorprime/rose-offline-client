use bevy::core_pipeline::dof::DepthOfFieldMode;
use bevy::prelude::{Local, Query, ResMut, Resource};
use bevy_egui::{egui, EguiContexts};

use crate::{
    audio::SoundGain,
    components::{BirdSettings, DirtDashSettings, FishSettings, Season, SoundCategory},
    render::ZoneLighting,
    resources::{SeasonSettings, SoundSettings, WaterSettings},
    ui::UiStateWindows,
};

#[derive(Copy, Clone, PartialEq, Debug)]
enum SettingsPage {
    Sound,
    DepthOfField,
    VolumetricFog,
    Water,
    Fish,
    Birds,
    Seasons,
    DirtDash,
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

pub fn ui_settings_system(
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_state_settings: Local<UiStateSettings>,
    mut sound_settings: ResMut<SoundSettings>,
    mut query_sounds: Query<(&SoundCategory, &mut SoundGain)>,
    mut dof_settings: ResMut<DepthOfFieldSettings>,
    mut zone_lighting: ResMut<ZoneLighting>,
    mut water_settings: ResMut<WaterSettings>,
    mut fish_settings: ResMut<FishSettings>,
    mut bird_settings: ResMut<BirdSettings>,
    mut season_settings: ResMut<SeasonSettings>,
    mut dirt_dash_settings: ResMut<DirtDashSettings>,
) {
    egui::Window::new("Settings")
        .open(&mut ui_state_windows.settings_open)
        .resizable(false)
        .show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut ui_state_settings.page, SettingsPage::Sound, "Sound");
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

                            ui.label("Birds Per Zone:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.birds_per_zone, 0..=1000)
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

                            ui.label("Roam Radius:");
                            ui.add(
                                egui::Slider::new(&mut bird_settings.roam_radius, 50.0..=2000.0)
                                    .text("m")
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

                    ui.separator();
                    ui.label("Note: Bird count changes apply when entering a new zone");
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
            }
        });
}
