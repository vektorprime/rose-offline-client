use std::time::Duration;

use bevy::prelude::{Query, Res, Time};

use crate::components::PassiveRecoveryTime;

const RECOVERY_INTERVAL: Duration = Duration::from_secs(4);

pub fn passive_recovery_system(
    mut query: Query<&mut PassiveRecoveryTime>,
    time: Res<Time>,
) {
    // Server-authoritative recovery:
    // the server applies passive HP/MP regen and sends UpdateHealthPoints/UpdateManaPoints.
    // Client keeps timer progression only and never mutates HP/MP locally.
    for mut passive_recovery_time in query.iter_mut() {
        passive_recovery_time.time += time.delta();

        if passive_recovery_time.time > RECOVERY_INTERVAL {
            passive_recovery_time.time -= RECOVERY_INTERVAL;
        }
    }
}
