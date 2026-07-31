use crate::components::{Season, SeasonMarker};
use crate::resources::SeasonSettings;
use bevy::prelude::*;

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
