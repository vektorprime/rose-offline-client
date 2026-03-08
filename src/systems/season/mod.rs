use bevy::prelude::*;

mod fall_system;
mod season_manager;
mod spring_system;
mod summer_system;
mod winter_system;

pub use fall_system::*;
pub use season_manager::*;
pub use spring_system::*;
pub use summer_system::*;
pub use winter_system::*;

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
                    fall_system::fall_particle_system,
                    spring_system::spring_rain_system,
                    // DEPRECATED: CPU-based grass systems - replaced by GPU-based procedural grass
                    // summer_system::summer_vegetation_system,  // Deprecated: CPU grass spawning
                    // summer_system::vegetation_sway_system,    // Deprecated: CPU grass animation
                    summer_system::spawn_procedural_grass_system, // Polls for terrain entities
                    summer_system::sync_grass_wind_system,        // Sync wind settings to procedural grass
                    summer_system::grass_visibility_system,       // Control grass visibility based on season
                    summer_system::cleanup_grass_on_season_change, // Remove grass when season changes
                    summer_system::cleanup_grass_on_zone_change,   // Remove grass when zone changes
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
