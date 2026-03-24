use bevy::prelude::{Query, ResMut};
use bevy_post_process::bloom::Bloom;
use bevy_egui::{egui, EguiContexts};

use crate::{render::ZoneLighting, ui::UiStateDebugWindows};

pub fn ui_debug_zone_lighting_system(
    mut egui_context: EguiContexts,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
    mut zone_lighting: ResMut<ZoneLighting>,
    mut query_camera: Query<Option<&mut Bloom>>,
) {
    if !ui_state_debug_windows.debug_ui_open {
        return;
    }

    egui::Window::new("Zone Lighting")
        .open(&mut ui_state_debug_windows.zone_lighting_open)
        .show(egui_context.ctx_mut().unwrap(), |ui| {
            egui::Grid::new("zone_ambient_lighting")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Map Ambient Color:");
                    let mut map_ambient_color = [
                        zone_lighting.map_ambient_color.x,
                        zone_lighting.map_ambient_color.y,
                        zone_lighting.map_ambient_color.z,
                    ];
                    ui.color_edit_button_rgb(&mut map_ambient_color);
                    ui.end_row();

                    ui.label("Character Ambient Color:");
                    let mut character_ambient_color = [
                        zone_lighting.character_ambient_color.x,
                        zone_lighting.character_ambient_color.y,
                        zone_lighting.character_ambient_color.z,
                    ];
                    ui.color_edit_button_rgb(&mut character_ambient_color);
                    ui.end_row();

                    ui.label("Character Diffuse Color:");
                    let mut character_diffuse_color = [
                        zone_lighting.character_diffuse_color.x,
                        zone_lighting.character_diffuse_color.y,
                        zone_lighting.character_diffuse_color.z,
                    ];
                    ui.color_edit_button_rgb(&mut character_diffuse_color);
                    ui.end_row();
                });

            ui.separator();

            egui::Grid::new("zone_fog").num_columns(2).show(ui, |ui| {
                ui.label("Color Fog Enabled:");
                ui.checkbox(&mut zone_lighting.color_fog_enabled, "Enabled");
                ui.end_row();

                ui.label("Fog Color:");
                let mut fog_color = [
                    zone_lighting.fog_color.x,
                    zone_lighting.fog_color.y,
                    zone_lighting.fog_color.z,
                ];
                ui.color_edit_button_rgb(&mut fog_color);
                ui.end_row();

                ui.label("Fog Density:");
                ui.add(
                    egui::Slider::new(&mut zone_lighting.fog_density, 0.0..=0.01).show_value(true),
                );
                ui.end_row();

                ui.label("Fog Min Amount:");
                ui.add(
                    egui::Slider::new(&mut zone_lighting.fog_min_density, 0.0..=1.0)
                        .show_value(true),
                );
                ui.end_row();

                ui.label("Fog Max Amount:");
                ui.add(
                    egui::Slider::new(&mut zone_lighting.fog_max_density, 0.0..=1.0)
                        .show_value(true),
                );
                ui.end_row();
            });

            ui.separator();

            egui::Grid::new("zone_alpha_fog")
                .num_columns(2)
                .show(ui, |ui| {
                    ui.label("Alpha Fog Enabled:");
                    ui.checkbox(&mut zone_lighting.alpha_fog_enabled, "Enabled");
                    ui.end_row();

                    ui.label("Alpha Fog Start:");
                    ui.add(
                        egui::Slider::new(&mut zone_lighting.fog_alpha_weight_start, 0.0..=1.0)
                            .show_value(true),
                    );
                    ui.end_row();

                    ui.label("Alpha Fog End:");
                    ui.add(
                        egui::Slider::new(&mut zone_lighting.fog_alpha_weight_end, 0.0..=1.0)
                            .show_value(true),
                    );
                    ui.end_row();
                });

            ui.separator();

            if let Ok(bloom_settings_opt) = query_camera.single_mut() {
                if let Some(mut bloom_settings) = bloom_settings_opt {
                    egui::Grid::new("bloom_settings")
                        .num_columns(2)
                        .show(ui, |ui| {
                            ui.label("Intensity:");
                            ui.add(
                                egui::Slider::new(&mut bloom_settings.intensity, 0.0..=1.0)
                                    .show_value(true),
                            );
                            ui.end_row();
                        });
                }
            }
        });
}
