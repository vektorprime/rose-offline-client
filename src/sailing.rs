use bevy::prelude::Vec2;

/// Wraps any angle to the `0..TAU` range.
pub fn normalize_angle(angle: f32) -> f32 {
    angle.rem_euclid(std::f32::consts::TAU)
}

/// Returns the shortest signed angular delta from `from` to `to`.
pub fn shortest_angle_delta(from: f32, to: f32) -> f32 {
    let mut diff = (to - from).rem_euclid(std::f32::consts::TAU);
    if diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    diff
}

/// Absolute wind-relative angle in radians, folded into `0..PI`.
pub fn angle_to_wind_abs(heading: f32, wind_angle: f32) -> f32 {
    let angle = normalize_angle(heading - wind_angle);
    if angle > std::f32::consts::PI {
        std::f32::consts::TAU - angle
    } else {
        angle
    }
}

/// ROSE sailing polar curve. Returns a speed factor in the `0..1` range.
pub fn sail_speed_factor(angle_to_wind: f32) -> f32 {
    let angle = angle_to_wind.abs().min(std::f32::consts::PI);
    if angle < 0.78 {
        (angle / 0.78).powf(2.0) * 0.3
    } else if angle < 1.57 {
        let t = (angle - 0.78) / (1.57 - 0.78);
        0.3 + t * 0.7
    } else if angle < 2.36 {
        let t = (angle - 1.57) / (2.36 - 1.57);
        1.0 - t * 0.2
    } else {
        let t = (angle - 2.36) / (std::f32::consts::PI - 2.36);
        0.8 - t * 0.3
    }
}

pub fn optimal_sail_trim(angle_to_wind_abs: f32) -> f32 {
    (angle_to_wind_abs * 0.5).clamp(0.0, std::f32::consts::PI)
}

pub fn trim_efficiency(current_trim: f32, optimal_trim: f32) -> f32 {
    (1.0 - ((current_trim - optimal_trim).abs() / std::f32::consts::PI)).clamp(0.1, 1.0)
}

pub fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
    normalize_angle(from + shortest_angle_delta(from, to) * t.clamp(0.0, 1.0))
}

#[derive(Debug, Clone, Copy)]
pub struct SailingStepInput {
    pub heading: f32,
    pub speed: f32,
    pub sail_trim: f32,
    pub rudder: f32,
    pub trim_input: f32,
    pub max_speed: f32,
    pub wind_angle: f32,
    pub wind_speed: f32,
    pub dt: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct SailingStepOutput {
    pub heading: f32,
    pub speed: f32,
    pub sail_trim: f32,
    pub forward_cm: Vec2,
}

/// Advances the boat's pure sailing state. Position/water/collision remain caller-owned.
pub fn sailing_step(input: SailingStepInput) -> SailingStepOutput {
    let max_speed = input.max_speed.max(0.1);
    let dt = input.dt.max(0.0);

    let turn_rate = (input.speed / max_speed).clamp(0.1, 1.0);
    let heading = normalize_angle(input.heading + input.rudder.clamp(-1.0, 1.0) * turn_rate * dt);
    let sail_trim = (input.sail_trim + input.trim_input.clamp(-1.0, 1.0) * 0.5 * dt)
        .clamp(0.0, std::f32::consts::PI);

    let wind_angle_abs = angle_to_wind_abs(heading, input.wind_angle);
    let target_speed_base =
        max_speed * sail_speed_factor(wind_angle_abs) * (input.wind_speed / 5.0).clamp(0.0, 2.0);
    let target_speed =
        target_speed_base * trim_efficiency(sail_trim, optimal_sail_trim(wind_angle_abs));

    let accel = if target_speed > input.speed { 2.0 } else { 1.5 };
    let speed = (input.speed + (target_speed - input.speed) * accel * dt).clamp(0.0, max_speed);

    let forward = Vec2::new(heading.sin(), heading.cos());
    SailingStepOutput {
        heading,
        speed,
        sail_trim,
        forward_cm: forward * speed * dt * 100.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polar_curve_has_expected_shape() {
        assert!(sail_speed_factor(0.0) < 0.01);
        assert!(sail_speed_factor(std::f32::consts::FRAC_PI_2) > 0.95);
        assert!(sail_speed_factor(std::f32::consts::PI) < 0.55);
    }

    #[test]
    fn angle_interpolation_uses_shortest_arc() {
        let from = 350.0_f32.to_radians();
        let to = 10.0_f32.to_radians();
        let mid = lerp_angle(from, to, 0.5).to_degrees();
        assert!(mid < 1.0 || mid > 359.0);
    }

    #[test]
    fn sailing_step_moves_in_heading_direction() {
        let output = sailing_step(SailingStepInput {
            heading: 0.0,
            speed: 4.0,
            sail_trim: std::f32::consts::FRAC_PI_4,
            rudder: 0.0,
            trim_input: 0.0,
            max_speed: 9.0,
            wind_angle: std::f32::consts::FRAC_PI_2,
            wind_speed: 5.0,
            dt: 1.0,
        });

        assert!(output.forward_cm.y > 0.0);
        assert!(output.forward_cm.x.abs() < 0.001);
    }
}
