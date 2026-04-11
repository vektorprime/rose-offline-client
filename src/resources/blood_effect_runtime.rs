use bevy::prelude::{Entity, Resource};

/// Runtime state for blood effects, including entity pooling.
#[derive(Resource, Default, Debug)]
pub struct BloodEffectRuntime {
    pub spatter_pool: Vec<Entity>,
    pub wound_pool: Vec<Entity>,
}

/// Lightweight diagnostics counters for blood effects.
#[derive(Resource, Default, Debug)]
pub struct BloodEffectDiagnostics {
    pub spatter_events: u64,
    pub active_spatters_spawned: u64,
    pub pooled_spatters_reused: u64,
    pub pooled_spatters_returned: u64,
    pub wound_visuals_spawned: u64,
    pub wound_visuals_reused: u64,
    pub mist_spawned: u64,
    pub droplets_spawned: u64,
    pub accum_time_secs: f32,
}

