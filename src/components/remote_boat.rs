use bevy::prelude::*;

/// Render/interpolation state for boats controlled by remote/server data.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
pub struct RemoteBoatState {
    pub previous_position_cm: Vec3,
    pub target_position_cm: Vec3,
    pub previous_heading: f32,
    pub target_heading: f32,
    pub target_speed: f32,
    pub sail_trim: f32,
    pub update_age: f32,
    pub update_interval: f32,
    /// Wall-clock time (elapsed_secs) of the last authoritative SailState
    /// update. While fresh, heading/speed/trim come from the server packet
    /// instead of position-delta estimation.
    pub last_authoritative_at: f32,
}

impl RemoteBoatState {
    pub fn from_position(position_cm: Vec3, heading: f32) -> Self {
        Self {
            previous_position_cm: position_cm,
            target_position_cm: position_cm,
            previous_heading: heading,
            target_heading: heading,
            target_speed: 0.0,
            sail_trim: std::f32::consts::FRAC_PI_4,
            update_age: 0.0,
            update_interval: 0.1,
            last_authoritative_at: -10.0,
        }
    }
}
