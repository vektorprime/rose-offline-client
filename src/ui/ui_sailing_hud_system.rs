use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::components::{BoatState, PlayerCharacter};
use crate::resources::WindState;

pub fn ui_sailing_hud_system(
    mut egui_ctx: EguiContexts,
    wind: Res<WindState>,
    boat_query: Query<&BoatState, With<PlayerCharacter>>,
) {
    let Ok(boat) = boat_query.single() else {
        return;
    };

    if !boat.active {
        return;
    }

    let ctx = egui_ctx.ctx_mut().unwrap();

    egui::Window::new("Sailing HUD")
        .anchor(egui::Align2::RIGHT_TOP, [-8.0, 8.0])
        .title_bar(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Wind / Heading").strong());

            let wind_deg = wind.angle.to_degrees().rem_euclid(360.0);
            let heading_deg = boat.heading.to_degrees().rem_euclid(360.0);
            ui.label(format!("Wind: {:>6.1}°  ({:.1} m/s)", wind_deg, wind.speed));
            ui.label(format!("Boat: {:>6.1}°", heading_deg));
            ui.separator();

            let speed_ratio = (boat.speed / boat.max_speed).clamp(0.0, 1.0);
            ui.label(egui::RichText::new("Speed").strong());
            ui.add(egui::ProgressBar::new(speed_ratio).show_percentage());
            ui.label(format!("{:.2} / {:.2} m/s", boat.speed, boat.max_speed));

            ui.separator();
            ui.label(egui::RichText::new("Sail Trim").strong());
            ui.add(
                egui::ProgressBar::new((boat.sail_trim / std::f32::consts::PI).clamp(0.0, 1.0))
                    .show_percentage(),
            );
            ui.label("A/D steer, W/S sail trim, E disembark");
        });
}

