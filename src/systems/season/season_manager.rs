use bevy::prelude::*;
use crate::components::{Season, SeasonMarker, WeatherParticle};
use crate::resources::SeasonSettings;

/// Cleans up season entities when season changes
pub fn season_cleanup_system(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    query: Query<(Entity, &SeasonMarker)>,
) {
    if settings.is_changed() {
        for (entity, marker) in query.iter() {
            if marker.0 != settings.current_season {
                commands.entity(entity).despawn();
            }
        }
    }
}

/// Spawns particles based on current season
#[allow(dead_code)]
pub fn spawn_season_particles(
    mut commands: Commands,
    settings: Res<SeasonSettings>,
    particle_count: Query<(), With<WeatherParticle>>,
    camera_query: Query<&Transform, With<Camera3d>>,
    time: Res<Time>,
) {
    if !settings.enabled || settings.current_season == Season::None {
        return;
    }

    let _current_count = particle_count.iter().len();
    let _camera_transform = camera_query.get_single();
    let _elapsed = time.elapsed_secs_f64();

    // Implementation depends on season - spawning handled in individual systems
}
