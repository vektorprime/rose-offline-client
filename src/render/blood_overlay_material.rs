//! Material extension for blood overlay rendering on model textures.
//!
//! This extension adds a blood overlay layer to standard materials, allowing
//! blood stains to be rendered on top of model textures. The overlay is sampled
//! from a per-entity blood texture and blended with the base material color.

use bevy::pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_shader::ShaderRef;

/// Material extension for blood overlay rendering.
///
/// Extends StandardMaterial with a blood overlay texture that contains
/// pre-computed blood stains. The overlay is blended with the base material
/// color in the fragment shader.
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
pub struct BloodOverlayExtension {
    /// Blood overlay texture containing all blood stains for this entity.
    /// The texture is a small RGBA image where each stain is painted at its
    /// UV position with the appropriate color and alpha.
    #[texture(200)]
    #[sampler(201)]
    pub blood_overlay_texture: Option<Handle<Image>>,

    /// Uniform data for blood overlay rendering.
    /// - x: blood intensity multiplier (0.0 = no blood, 1.0 = full intensity)
    /// - y: texture width (for UV calculations)
    /// - z: texture height (for UV calculations)
    /// - w: unused
    #[uniform(202, BloodOverlayUniform)]
    pub blood_params: Vec4,
}

impl Default for BloodOverlayExtension {
    fn default() -> Self {
        Self {
            blood_overlay_texture: None,
            blood_params: Vec4::new(1.0, 256.0, 256.0, 0.0),
        }
    }
}

/// Uniform data for blood overlay extension.
#[derive(Clone, Default, bevy::render::render_resource::ShaderType)]
pub struct BloodOverlayUniform {
    /// Blood intensity multiplier.
    pub intensity: f32,
    /// Texture width.
    pub texture_width: f32,
    /// Texture height.
    pub texture_height: f32,
    /// Unused padding.
    pub _padding: f32,
}

impl MaterialExtension for BloodOverlayExtension {
    fn fragment_shader() -> ShaderRef {
        crate::render::blood_overlay_shader::BLOOD_OVERLAY_SHADER_HANDLE.into()
    }

    fn deferred_fragment_shader() -> ShaderRef {
        crate::render::blood_overlay_shader::BLOOD_OVERLAY_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        _descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // No custom specialization needed
        Ok(())
    }
}
