use bevy::prelude::*;

/// Component attached to wing entities for rendering and animation
#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct AngelicWings {
    /// Which side this wing is on (left or right)
    pub side: WingSide,
    /// Current flap animation phase - 0 to 2*PI
    pub flap_phase: f32,
    /// Wing spread amount - 0.0 = folded, 1.0 = fully spread
    pub spread_amount: f32,
    /// Glow intensity for the ethereal effect
    pub glow_intensity: f32,
    /// Whether the wing is currently spreading
    pub is_spreading: bool,
}

impl Default for AngelicWings {
    fn default() -> Self {
        Self {
            side: WingSide::Left,
            flap_phase: 0.0,
            spread_amount: 0.0,
            glow_intensity: 0.5,
            is_spreading: false,
        }
    }
}

/// Which side of the character a wing is attached to
#[derive(Clone, Copy, PartialEq, Eq, Debug, Reflect)]
pub enum WingSide {
    Left,
    Right,
}
