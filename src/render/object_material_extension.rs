//! Material extension for object materials with lightmaps and specular maps
//!
//! This extension adds ROSE-specific features to Bevy's StandardMaterial:
//! - Lightmap support with UV offset and scale
//! - Specular map support
//! - Blink state uniform for character face blinking (shader integration pending)
//!
//! Note: Zone lighting has been temporarily removed to simplify the rendering
//! pipeline. It can be added back later once basic rendering is confirmed working.

use bevy::pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, MeshPipelineKey, StandardMaterial};
use bevy::prelude::*;
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_image::Image;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_shader::ShaderRef;

/// Material extension for ROSE object materials
///
/// Extends StandardMaterial with:
/// - Lightmap texture and parameters
/// - Specular map texture
/// - Blink state (for character face eye clipping)
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

    /// Blink state for character face eye clipping
    /// 0 = eyes open, 1 = eyes closed (blinking)
    /// Note: Vertex shader integration requires custom pipeline beyond ExtendedMaterial capabilities
    #[uniform(105)]
    pub blink_state: u32,
}

impl Default for RoseObjectExtension {
    fn default() -> Self {
        Self {
            lightmap_params: Vec4::new(0.0, 0.0, 1.0, 0.0),
            lightmap_texture: None,
            specular_texture: None,
            blink_state: 0, // Default to eyes open
        }
    }
}

impl MaterialExtension for RoseObjectExtension {
    fn fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_OBJECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_OBJECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        _descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // No custom specialization needed - use standard Bevy PBR pipeline
        // Zone lighting can be added back later if needed
        Ok(())
    }
}
