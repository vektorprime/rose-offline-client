//! Sailing physics.
//!
//! The pure sailing math lives in `rose-game-common::sailing` so the client and
//! server share the exact same simulation. This module re-exports it for the
//! client's existing call sites.

pub use rose_game_common::sailing::{
    angle_to_wind_abs, lerp_angle, normalize_angle, optimal_sail_trim, sail_speed_factor,
    sailing_step, shortest_angle_delta, trim_efficiency, SailingStepInput, SailingStepOutput,
};
