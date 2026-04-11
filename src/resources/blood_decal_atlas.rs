use bevy::{prelude::*, reflect::Reflect};

/// Cached texture atlas handles used by blood-related effects.
///
/// The textures are generated once and reused for all blood spatter and wound visuals.
#[derive(Resource, Reflect, Default, Clone, Debug)]
#[reflect(Resource)]
pub struct BloodDecalAtlas {
    /// Procedural blood spatter texture variants.
    pub spatter_textures: Vec<Handle<Image>>,
    /// Procedural wound texture variants.
    pub wound_textures: Vec<Handle<Image>>,
}

