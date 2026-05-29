use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use crate::components::{BoatState, PlayerCharacter, Position};
use crate::resources::{CurrentZone, WindState};
use crate::systems::find_nearest_shore_position;
use crate::zone_loader::ZoneLoaderAsset;

fn normalize_angle_pi(angle: f32) -> f32 {
    let wrapped = angle.rem_euclid(std::f32::consts::TAU);
    if wrapped > std::f32::consts::PI {
        wrapped - std::f32::consts::TAU
    } else {
        wrapped
    }
}

fn speed_color(speed_ratio: f32) -> egui::Color32 {
    if speed_ratio < 0.15 {
        egui::Color32::from_rgb(180, 50, 50)
    } else if speed_ratio < 0.40 {
        egui::Color32::from_rgb(210, 175, 70)
    } else if speed_ratio < 0.80 {
        egui::Color32::from_rgb(70, 190, 90)
    } else {
        egui::Color32::from_rgb(60, 190, 220)
    }
}

fn draw_wind_compass(ui: &mut egui::Ui, wind_angle: f32, boat_heading: f32, wind_speed: f32) {
    let desired_size = egui::vec2(120.0, 140.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;
    let center = rect.center_top() + egui::vec2(0.0, 58.0);
    let radius = 50.0;

    painter.circle_filled(
        center,
        radius,
        egui::Color32::from_rgba_premultiplied(0, 0, 0, 140),
    );
    painter.circle_stroke(
        center,
        radius,
        egui::Stroke::new(2.0, egui::Color32::LIGHT_GRAY),
    );

    let dirs = [
        ("N", 0.0f32),
        ("E", std::f32::consts::FRAC_PI_2),
        ("S", std::f32::consts::PI),
        ("W", std::f32::consts::PI * 1.5),
    ];
    for (label, angle) in dirs {
        let rotated = angle - boat_heading;
        let pos = center + egui::vec2(rotated.sin(), -rotated.cos()) * (radius + 10.0);
        painter.text(
            pos,
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(10.0),
            egui::Color32::from_rgb(200, 200, 200),
        );
    }

    // Boat heading marker (always up)
    let boat_tip = center + egui::vec2(0.0, -radius);
    painter.add(egui::Shape::convex_polygon(
        vec![
            boat_tip,
            boat_tip + egui::vec2(-5.0, 10.0),
            boat_tip + egui::vec2(5.0, 10.0),
        ],
        egui::Color32::from_rgb(60, 220, 80),
        egui::Stroke::NONE,
    ));

    // Wind arrow relative to boat
    let wind_relative = normalize_angle_pi(wind_angle - boat_heading);
    let dir = egui::vec2(wind_relative.sin(), -wind_relative.cos());
    let arrow_start = center + dir * 10.0;
    let arrow_end = center + dir * (radius - 6.0);
    painter.arrow(
        arrow_start,
        arrow_end - arrow_start,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(220, 70, 70)),
    );

    painter.text(
        rect.center_bottom() + egui::vec2(0.0, -8.0),
        egui::Align2::CENTER_BOTTOM,
        format!("Wind {:.1} m/s", wind_speed),
        egui::FontId::proportional(10.0),
        egui::Color32::from_rgb(210, 210, 210),
    );
}

fn draw_speed_gauge(ui: &mut egui::Ui, speed: f32, max_speed: f32) {
    let desired_size = egui::vec2(250.0, 40.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;

    let bar_rect = egui::Rect::from_min_size(
        rect.min + egui::vec2(0.0, 4.0),
        egui::vec2(rect.width(), 20.0),
    );
    let speed_ratio = if max_speed > 0.0 {
        (speed / max_speed).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let fill_width = bar_rect.width() * speed_ratio;

    painter.rect_filled(
        bar_rect,
        4.0,
        egui::Color32::from_rgba_premultiplied(0, 0, 0, 140),
    );
    if fill_width > 1.0 {
        let fill_rect =
            egui::Rect::from_min_size(bar_rect.min, egui::vec2(fill_width, bar_rect.height()));
        painter.rect_filled(fill_rect, 4.0, speed_color(speed_ratio));
    }
    painter.rect_stroke(
        bar_rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 220, 220)),
        egui::StrokeKind::Outside,
    );

    let speed_knots = speed * 1.943_844;
    painter.text(
        bar_rect.center(),
        egui::Align2::CENTER_CENTER,
        format!("{:.1} m/s  ({:.0} kt)", speed, speed_knots),
        egui::FontId::proportional(12.0),
        egui::Color32::WHITE,
    );

    painter.text(
        bar_rect.left_bottom() + egui::vec2(0.0, 14.0),
        egui::Align2::LEFT_BOTTOM,
        "0",
        egui::FontId::proportional(10.0),
        egui::Color32::GRAY,
    );
    painter.text(
        bar_rect.right_bottom() + egui::vec2(0.0, 14.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{:.1}", max_speed),
        egui::FontId::proportional(10.0),
        egui::Color32::GRAY,
    );
}

fn draw_trim_indicator(ui: &mut egui::Ui, current_trim: f32, optimal_trim: f32) {
    let desired_size = egui::vec2(90.0, 90.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;
    let center = rect.center();
    let radius = 30.0;

    let bg = egui::Color32::from_rgba_premultiplied(0, 0, 0, 140);
    painter.circle_filled(center, radius + 14.0, bg);

    let segments = 28;
    let mut points = Vec::with_capacity((segments + 1) as usize);
    for i in 0..=segments {
        let t = i as f32 / segments as f32;
        let angle = std::f32::consts::PI * (1.0 - t);
        points.push(center + egui::vec2(angle.cos(), -angle.sin()) * radius);
    }
    painter.add(egui::Shape::line(
        points,
        egui::Stroke::new(2.0, egui::Color32::from_rgb(180, 180, 180)),
    ));

    let current_norm = (current_trim / std::f32::consts::PI).clamp(0.0, 1.0);
    let optimal_norm = (optimal_trim / std::f32::consts::PI).clamp(0.0, 1.0);

    let current_angle = std::f32::consts::PI * (1.0 - current_norm);
    let optimal_angle = std::f32::consts::PI * (1.0 - optimal_norm);

    let trim_error = (current_trim - optimal_trim).abs();
    let trim_color = if trim_error <= 0.2 {
        egui::Color32::from_rgb(60, 220, 80)
    } else if trim_error <= 0.5 {
        egui::Color32::from_rgb(220, 190, 60)
    } else {
        egui::Color32::from_rgb(220, 90, 90)
    };

    let current_pos = center + egui::vec2(current_angle.cos(), -current_angle.sin()) * radius;
    let optimal_pos = center + egui::vec2(optimal_angle.cos(), -optimal_angle.sin()) * radius;

    painter.circle_filled(optimal_pos, 4.0, egui::Color32::from_rgb(100, 160, 255));
    painter.circle_filled(current_pos, 5.0, trim_color);
    painter.text(
        rect.center_bottom() + egui::vec2(0.0, -6.0),
        egui::Align2::CENTER_BOTTOM,
        "Trim",
        egui::FontId::proportional(10.0),
        egui::Color32::WHITE,
    );
}

pub fn ui_sailing_hud_system(
    mut egui_ctx: EguiContexts,
    wind: Res<WindState>,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    boat_query: Query<(&BoatState, &Position), With<PlayerCharacter>>,
) {
    let Ok((boat, position)) = boat_query.single() else {
        return;
    };

    if !boat.active {
        return;
    }

    let ctx = egui_ctx.ctx_mut().unwrap();

    if ctx.wants_pointer_input() {
        return;
    }

    let angle_to_wind = (boat.heading - wind.angle).rem_euclid(std::f32::consts::TAU);
    let angle_to_wind_abs = if angle_to_wind > std::f32::consts::PI {
        std::f32::consts::TAU - angle_to_wind
    } else {
        angle_to_wind
    };
    let optimal_trim = angle_to_wind_abs * 0.5;

    egui::Area::new(egui::Id::new("sailing_hud_compass"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-12.0, 12.0))
        .interactable(false)
        .show(ctx, |ui| {
            draw_wind_compass(ui, wind.angle, boat.heading, wind.speed);
        });

    egui::Area::new(egui::Id::new("sailing_hud_speed"))
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -14.0))
        .interactable(false)
        .show(ctx, |ui| {
            draw_speed_gauge(ui, boat.speed, boat.max_speed);
        });

    egui::Area::new(egui::Id::new("sailing_hud_trim"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-12.0, -20.0))
        .interactable(false)
        .show(ctx, |ui| {
            draw_trim_indicator(ui, boat.sail_trim, optimal_trim);
        });

    let mut prompts = Vec::new();
    if angle_to_wind_abs < 0.78 {
        prompts.push("Luffing! Turn away from wind".to_string());
    }
    let near_shore = current_zone
        .as_ref()
        .and_then(|zone| zone_loader_assets.get(&zone.handle))
        .and_then(|zone_data| {
            find_nearest_shore_position(position.position, zone_data, boat.water_height_cm)
        })
        .is_some();
    if near_shore {
        prompts.push("Press E to disembark".to_string());
    } else {
        prompts.push("Sail closer to shore to disembark".to_string());
    }

    egui::Window::new("Sailing Prompts")
        .anchor(egui::Align2::LEFT_TOP, [12.0, 12.0])
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .movable(false)
        .show(ctx, |ui| {
            for prompt in prompts.iter() {
                ui.label(prompt);
            }
            ui.separator();
            ui.label("A/D steer, W/S sail trim, E disembark");
        });
}
