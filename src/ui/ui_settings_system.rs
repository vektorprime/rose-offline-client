use bevy::core_pipeline::dof::DepthOfFieldMode;
use bevy::prelude::{Local, Query, ResMut, Resource};
use bevy_egui::{egui, EguiContexts};

use crate::{
    audio::SoundGain, components::SoundCategory, resources::SoundSettings, ui::UiStateWindows,
};

#[derive(Copy, Clone, PartialEq, Debug)]
enum SettingsPage {
    Sound,
    DepthOfField,
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

pub fn ui_settings_system(
    mut egui_context: EguiContexts,
    mut ui_state_windows: ResMut<UiStateWindows>,
    mut ui_state_settings: Local<UiStateSettings>,
    mut sound_settings: ResMut<SoundSettings>,
    mut query_sounds: Query<(&SoundCategory, &mut SoundGain)>,
    mut dof_settings: ResMut<DepthOfFieldSettings>,
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
            }
        });
}
