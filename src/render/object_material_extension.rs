//! Material extension for object materials with lightmaps and specular maps
//!
//! This extension adds ROSE-specific features to Bevy's StandardMaterial:
//! - Lightmap support with UV offset and scale
//! - Specular map support

use bevy::pbr::{MaterialExtension, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::*;

/// Material extension for ROSE object materials
///
/// Extends StandardMaterial with:
/// - Lightmap texture and parameters
/// - Specular map texture
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct RoseObjectExtension {
    /// Lightmap parameters: x = offset_x, y = offset_y, z = scale, w = unused
    #[uniform(100)]
    pub lightmap_params: Vec4,

    /// Lightmap texture
    #[texture(101)]
    #[sampler(102)]
    pub lightmap_texture: Option<Handle<Image>>,

    /// Specular map texture
    #[texture(103)]
    #[sampler(104)]
    pub specular_texture: Option<Handle<Image>>,
}

impl Default for RoseObjectExtension {
    fn default() -> Self {
        Self {
            lightmap_params: Vec4::new(0.0, 0.0, 1.0, 0.0),
            lightmap_texture: None,
            specular_texture: None,
        }
    }
}

impl MaterialExtension for RoseObjectExtension {
    fn fragment_shader() -> ShaderRef {
        "shaders/rose_object_extension.wgsl".into()
    }
}
