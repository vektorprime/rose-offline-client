use bevy::prelude::Resource;
use std::time::Duration;

use rose_data::{WorldTicks, WORLD_TICKS_PER_DAY};

#[derive(Resource)]
pub struct WorldTime {
    pub ticks: WorldTicks,
    pub time_since_last_tick: Duration,
}

impl Default for WorldTime {
    fn default() -> Self {
        // Deterministic start time: midday on a standard 160-tick day.
        // The previous random seed made the time-of-day (and therefore whether
        // the sky renders as the daytime atmosphere or the night star field)
        // change on every launch of the login screen. The real server time
        // replaces this value once the player joins a zone in-game.
        Self::new(WorldTicks(WORLD_TICKS_PER_DAY / 2))
    }
}

impl WorldTime {
    pub fn new(ticks: WorldTicks) -> Self {
        Self {
            ticks,
            time_since_last_tick: Duration::from_secs(0),
        }
    }
}
