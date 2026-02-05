//! Material extension for terrain materials with texture splatting
//!
//! This extension adds ROSE-specific terrain features to Bevy's StandardMaterial:
//! - Multiple terrain textures for texture splatting
//! - Detail texture support
//! - Tile-based texture selection

use bevy::pbr::{MaterialExtension, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::*;

/// Maximum number of terrain textures supported
pub const TERRAIN_MAX_TEXTURES: usize = 4;

/// Material extension for ROSE terrain materials
///
/// Extends StandardMaterial with:
/// - Multiple terrain textures for texture splatting
/// - Detail texture for additional detail
/// - Texture count for shader specialization
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct RoseTerrainExtension {
    /// Terrain texture 0
    #[texture(100, dimension = "2d")]
    #[sampler(101)]
    pub texture0: Option<Handle<Image>>,

    /// Terrain texture 1
    #[texture(102, dimension = "2d")]
    #[sampler(103)]
    pub texture1: Option<Handle<Image>>,

    /// Terrain texture 2
    #[texture(104, dimension = "2d")]
    #[sampler(105)]
    pub texture2: Option<Handle<Image>>,

    /// Terrain texture 3
    #[texture(106, dimension = "2d")]
    #[sampler(107)]
    pub texture3: Option<Handle<Image>>,

    /// Detail texture for additional surface detail
    #[texture(108, dimension = "2d")]
    #[sampler(109)]
    pub detail_texture: Option<Handle<Image>>,

    /// Number of terrain textures actually in use (1-4)
    #[uniform(110)]
    pub texture_count: u32,
}

impl Default for RoseTerrainExtension {
    fn default() -> Self {
        Self {
            texture0: None,
            texture1: None,
            texture2: None,
            texture3: None,
            detail_texture: None,
            texture_count: 0,
        }
    }
}

impl MaterialExtension for RoseTerrainExtension {
    fn fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_TERRAIN_EXTENSION_SHADER_HANDLE.into()
    }
}
