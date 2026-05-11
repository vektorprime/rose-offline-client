//! Sailing Zone Management
//!
//! Configuration and management for sailing zones.

use bevy::prelude::*;

/// Configuration for sailing zones
#[derive(Resource, Debug, Clone)]
pub struct SailingZoneConfig {
    /// Zone ID for sailing zones
    pub zone_id: u16,
    /// Whether sailing features are enabled
    pub enabled: bool,
    /// Water level in cm
    pub water_level_cm: f32,
    /// Sea floor depth in cm
    pub sea_floor_cm: f32,
    /// Maximum boat speed multiplier
    pub max_speed_multiplier: f32,
    /// Wind effect multiplier
    pub wind_multiplier: f32,
    /// Storm frequency (minutes between storms)
    pub storm_frequency_minutes: f32,
    /// Treasure respawn time (seconds)
    pub treasure_respawn_seconds: f32,
}

impl Default for SailingZoneConfig {
    fn default() -> Self {
        Self {
            zone_id: 200,
            enabled: true,
            water_level_cm: 0.0,
            sea_floor_cm: -500.0,
            max_speed_multiplier: 1.0,
            wind_multiplier: 1.0,
            storm_frequency_minutes: 5.0,
            treasure_respawn_seconds: 300.0,
        }
    }
}

/// Check if current zone is a sailing zone
pub fn is_sailing_zone(current_zone: &Res<crate::resources::CurrentZone>) -> bool {
    current_zone.id == 200
}

/// Get sailing zone configuration
pub fn get_sailing_config(config: &Res<SailingZoneConfig>) -> &SailingZoneConfig {
    config
}
