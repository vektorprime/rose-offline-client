use bevy::prelude::Component;

/// Bevy-native world-space damage number (replaces the custom
/// `DamageDigitMaterial` GPU pipeline). A parent entity carrying this
/// component owns one quad child per digit; `damage_number_billboard_system`
/// faces the parent at the camera and `damage_number_animate_system` floats
/// it up and despawns it (children go along via hierarchy cascade).
#[derive(Component)]
pub struct DamageNumber {
    /// Seconds left before despawn.
    pub remaining: f32,
    /// World units risen per second.
    pub rise_speed: f32,
}

impl DamageNumber {
    pub fn new(lifetime: f32, rise_speed: f32) -> Self {
        Self {
            remaining: lifetime,
            rise_speed,
        }
    }
}
