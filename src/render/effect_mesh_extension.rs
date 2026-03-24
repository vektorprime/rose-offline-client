//! Material extension for effect mesh materials with frame-based animation
//!
//! This extension adds ROSE-specific features to Bevy's StandardMaterial:
//! - Animation texture for frame-based mesh animations
//! - Animation parameters (current frame, total frames, etc.)

use bevy::pbr::{MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline, StandardMaterial};
use bevy::prelude::*;
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::image::Image;
use bevy::render::render_resource::*;
use bevy_shader::ShaderRef;

/// Animation flags for effect mesh animation
pub const EFFECT_MESH_ANIMATION_FLAG_POSITION: u32 = 0x1;
pub const EFFECT_MESH_ANIMATION_FLAG_NORMAL: u32   = 0x2;
pub const EFFECT_MESH_ANIMATION_FLAG_UV: u32       = 0x4;
pub const EFFECT_MESH_ANIMATION_FLAG_ALPHA: u32    = 0x8;

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
    
    /// Animation state uniforms (flags, current_next_frame, next_weight, alpha)
    /// Flags: bits 0-3 = animation flags, bits 4-31 = num_frames
    /// current_next_frame: lower 16 bits = current frame, upper 16 bits = next frame
    #[uniform(102)]
    pub animation_state: EffectMeshAnimationUniform,
}

/// Uniform structure for effect mesh animation state
/// This matches the shader's AnimationState struct
#[derive(Clone, Copy, Debug, Default, Reflect, ShaderType)]
pub struct EffectMeshAnimationUniform {
    /// Flags: bits 0-3 = animation flags (position/normal/uv/alpha), bits 4-31 = num_frames
    pub flags: u32,
    /// Lower 16 bits = current frame index, upper 16 bits = next frame index
    pub current_next_frame: u32,
    /// Interpolation weight between current and next frame (0.0 - 1.0)
    pub next_weight: f32,
    /// Animated alpha value (when alpha animation is enabled)
    pub alpha: f32,
}

impl Default for RoseEffectExtension {
    fn default() -> Self {
        Self {
            animation_texture: None,
            animation_state: EffectMeshAnimationUniform::default(),
        }
    }
}

impl MaterialExtension for RoseEffectExtension {
    fn vertex_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_EFFECT_EXTENSION_SHADER_HANDLE.into()
    }
    
    fn fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_EFFECT_EXTENSION_SHADER_HANDLE.into()
    }
    
    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Always enable HAS_ANIMATION_TEXTURE shader define
        // The shader checks num_frames > 0 before applying animation, so it's safe to always include
        // Note: bind_group_data type is () for AsBindGroup, so we can't directly check animation_texture
        // The shader will handle the case when animation_texture is None gracefully
        descriptor.vertex.shader_defs.push("HAS_ANIMATION_TEXTURE".into());
        
        Ok(())
    }
}
