//! Angelic Wing Material
//! 
//! Simplified version using StandardMaterial directly to avoid shader compilation issues.
//! The wings will be rendered with a semi-transparent white material.

use bevy::{
    pbr::StandardMaterial,
    prelude::*,
};

/// Plugin for the wing material
#[derive(Default)]
pub struct WingMaterialPlugin;

impl Plugin for WingMaterialPlugin {
    fn build(&self, _app: &mut App) {
        // StandardMaterial is already registered by Bevy's DefaultPlugins
        // No need to add MaterialPlugin<StandardMaterial> again
        
        bevy::log::info!("[WingMaterial] Plugin initialized (using StandardMaterial)");
    }
}

/// Type alias for wing material - using StandardMaterial directly
pub type WingMaterial = StandardMaterial;

/// Helper function to create a wing material with default settings
pub fn create_wing_material(
    materials: &mut ResMut<Assets<StandardMaterial>>,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: Color::srgba(0.95, 0.95, 1.0, 0.85),
        alpha_mode: bevy::render::alpha::AlphaMode::Blend,
        unlit: false,
        cull_mode: None, // Double-sided
        perceptual_roughness: 0.3,
        metallic: 0.1,
        ..Default::default()
    })
}
