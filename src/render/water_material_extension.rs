//! Material extension for water materials with animated waves
//!
//! This extension adds ROSE-specific water features to Bevy's StandardMaterial:
//! - UV animation for wave movement
//! - Water texture

use bevy::pbr::{MaterialExtension, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::*;

/// Material extension for ROSE water materials
///
/// Extends StandardMaterial with:
/// - UV animation parameters for wave movement
/// - Water texture
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct RoseWaterExtension {
    /// UV animation parameters: x = speed_u, y = speed_v, z = time_offset, w = unused
    #[uniform(100)]
    pub uv_animation_params: Vec4,

    /// Water texture
    #[texture(101, dimension = "2d")]
    #[sampler(102)]
    pub water_texture: Option<Handle<Image>>,
}

impl Default for RoseWaterExtension {
    fn default() -> Self {
        Self {
            uv_animation_params: Vec4::new(0.0, 0.0, 0.0, 0.0),
            water_texture: None,
        }
    }
}

impl MaterialExtension for RoseWaterExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/rose_water_extension.wgsl".into()
    }
}
