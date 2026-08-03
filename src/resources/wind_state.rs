use bevy::prelude::*;

/// Global wind state used by sailing and visual systems.
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct WindState {
    /// Wind direction in Position-space XY plane, scaled by speed.
    pub direction: Vec2,
    /// Wind speed in m/s.
    pub speed: f32,
    /// Wind angle in radians. 0 = +Y in Position space.
    pub angle: f32,
    /// 0..1 gust intensity.
    pub gust_factor: f32,
    /// Simulation time accumulator.
    pub time_accumulator: f32,
    /// Authoritative wind angle from the server, if a broadcast was received.
    pub authoritative_angle: Option<f32>,
    /// Authoritative wind speed from the server, if a broadcast was received.
    pub authoritative_speed: Option<f32>,
    /// Authoritative gust factor from the server, if a broadcast was received.
    pub authoritative_gust_factor: Option<f32>,
    /// Wall-clock time (elapsed_secs) of the last authoritative broadcast.
    pub authoritative_last_update: f32,
}

impl Default for WindState {
    fn default() -> Self {
        let speed: f32 = 5.0;
        let angle: f32 = 0.0;
        Self {
            direction: Vec2::new(0.0, speed),
            speed,
            angle,
            gust_factor: 0.0,
            time_accumulator: 0.0,
            authoritative_angle: None,
            authoritative_speed: None,
            authoritative_gust_factor: None,
            authoritative_last_update: -10.0,
        }
    }
}

impl WindState {
    /// Returns true when a server wind broadcast is fresh enough to override
    /// the locally generated wind.
    pub fn has_fresh_authoritative_wind(&self, elapsed_secs: f32) -> bool {
        elapsed_secs - self.authoritative_last_update < 2.0
            && self.authoritative_angle.is_some()
            && self.authoritative_speed.is_some()
            && self.authoritative_gust_factor.is_some()
    }
}

/// Wind simulation tuning values.
#[derive(Resource, Reflect, Clone, Debug)]
#[reflect(Resource)]
pub struct WindSettings {
    pub base_speed: f32,
    pub direction_drift_speed: f32,
    pub gust_frequency: f32,
    pub gust_max_multiplier: f32,
    pub gust_direction_variance: f32,
}

impl Default for WindSettings {
    fn default() -> Self {
        Self {
            base_speed: 5.0,
            direction_drift_speed: 0.05,
            gust_frequency: 0.1,
            gust_max_multiplier: 1.5,
            gust_direction_variance: 0.3,
        }
    }
}
