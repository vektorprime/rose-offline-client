//! Material extension for effect mesh materials with frame-based animation
//!
//! This extension adds ROSE-specific features to Bevy's StandardMaterial:
//! - Animation texture for frame-based mesh animations
//! - Animation parameters (current frame, total frames, etc.)

use bevy::pbr::{MaterialExtension, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::*;

/// Material extension for ROSE effect mesh materials
///
/// Extends StandardMaterial with:
/// - Animation texture for frame-based mesh animations
/// - Animation parameters for controlling frame playback
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct RoseEffectExtension {
    /// Animation texture containing frame data
    #[texture(100)]
    #[sampler(101)]
    pub animation_texture: Option<Handle<Image>>,
}

impl Default for RoseEffectExtension {
    fn default() -> Self {
        Self {
            animation_texture: None,
        }
    }
}

impl MaterialExtension for RoseEffectExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/rose_effect_extension.wgsl".into()
    }
}
