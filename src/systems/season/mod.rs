use bevy::prelude::*;

mod season_manager;
mod weather_system;

pub use season_manager::*;
pub use weather_system::*;

pub struct SeasonPlugin;

impl Plugin for SeasonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::resources::SeasonSettings>()
            .init_resource::<crate::resources::FallSettings>()
            .init_resource::<crate::resources::SpringSettings>()
            .init_resource::<crate::resources::SummerSettings>()
            .init_resource::<crate::resources::WinterSettings>()
            .add_systems(PreUpdate, crate::resources::setup_season_materials)
            .add_systems(
                Update,
                (
                    season_manager::season_cleanup_system,
                    weather_system::weather_particle_system,
                ),
            );
    }
}
