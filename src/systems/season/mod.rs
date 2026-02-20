use bevy::prelude::*;

mod fall_system;
mod season_manager;
mod spring_system;
mod winter_system;

pub use fall_system::*;
pub use season_manager::*;
pub use spring_system::*;
pub use winter_system::*;

pub struct SeasonPlugin;

impl Plugin for SeasonPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<crate::resources::SeasonSettings>()
            .init_resource::<crate::resources::FallSettings>()
            .init_resource::<crate::resources::SpringSettings>()
            .init_resource::<crate::resources::WinterSettings>()
            .add_systems(PreUpdate, crate::resources::setup_season_materials)
            .add_systems(
                Update,
                (
                    season_manager::season_cleanup_system,
                    fall_system::fall_particle_system,
                    spring_system::spring_rain_system,
                    winter_system::winter_snow_system,
                ),
            );
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum SeasonSystemSet {
    Spawn,
    Update,
    Cleanup,
}
