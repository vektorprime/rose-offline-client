use bevy::prelude::*;

use crate::components::WindSwaySettings;
use crate::resources::{WindSettings, WindState};

pub fn wind_update_system(
    time: Res<Time>,
    settings: Res<WindSettings>,
    mut wind: ResMut<WindState>,
) {
    // When a fresh server wind broadcast exists (ocean zone), use it as the
    // authoritative source; fall back to local generation otherwise.
    if wind.has_fresh_authoritative_wind(time.elapsed_secs()) {
        let angle = wind.authoritative_angle.unwrap_or(wind.angle);
        let speed = wind.authoritative_speed.unwrap_or(wind.speed);
        let gust_factor = wind.authoritative_gust_factor.unwrap_or(wind.gust_factor);
        wind.angle = angle;
        wind.speed = speed;
        wind.gust_factor = gust_factor;
        wind.direction = Vec2::new(angle.sin(), angle.cos()) * speed;
        return;
    }

    wind.time_accumulator += time.delta_secs();
    let t = wind.time_accumulator;

    let base_angle = (t * settings.direction_drift_speed).sin() * 0.5
        + (t * settings.direction_drift_speed * 0.37).sin() * 0.3
        + (t * settings.direction_drift_speed * 0.13).sin() * 0.2;

    let gust = ((t * settings.gust_frequency * std::f32::consts::TAU).sin() * 0.5 + 0.5).powf(3.0);
    let gust_angle_offset = gust * settings.gust_direction_variance * (t * 2.3).sin();

    wind.angle = base_angle + gust_angle_offset;
    wind.speed = settings.base_speed * (1.0 + gust * (settings.gust_max_multiplier - 1.0));
    wind.gust_factor = gust;
    wind.direction = Vec2::new(wind.angle.sin(), wind.angle.cos()) * wind.speed;
}

pub fn sync_vegetation_wind_system(
    wind: Res<WindState>,
    mut sway_settings: ResMut<WindSwaySettings>,
) {
    sway_settings.global_intensity = (wind.speed / 10.0).clamp(0.05, 0.3);
}
