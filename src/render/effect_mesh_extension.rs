//! Material extension for effect mesh materials with frame-based animation
//!
//! This extension adds ROSE-specific features to Bevy's StandardMaterial:
//! - Animation texture for frame-based mesh animations
//! - Animation parameters (current frame, total frames, etc.)
//! - The original client's render states (blend equation, depth test/write) for
//!   meshes drawn in the transparent pass, see [`rose_effect_material`]

use bevy::image::Image;
use bevy::material::OpaqueRendererMethod;
use bevy::pbr::{
    ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline,
    StandardMaterial,
};
use bevy::prelude::*;
use bevy::render::render_resource::*;
use bevy_mesh::MeshVertexBufferLayoutRef;
use bevy_shader::{ShaderDefVal, ShaderRef};

/// Animation flags for effect mesh animation
pub const EFFECT_MESH_ANIMATION_FLAG_POSITION: u32 = 0x1;
pub const EFFECT_MESH_ANIMATION_FLAG_NORMAL: u32 = 0x2;
pub const EFFECT_MESH_ANIMATION_FLAG_UV: u32 = 0x4;
pub const EFFECT_MESH_ANIMATION_FLAG_ALPHA: u32 = 0x8;

/// Material extension for ROSE effect mesh materials
///
/// Extends StandardMaterial with:
/// - Animation texture for frame-based mesh animations
/// - Animation parameters for controlling frame playback
#[derive(Asset, AsBindGroup, Reflect, Debug, Clone)]
#[bind_group_data(RoseEffectExtensionKey)]
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

    /// Pipeline state of a mesh drawn in the transparent pass (set by
    /// [`rose_effect_material`]); `None` keeps the base material's pipeline.
    #[reflect(ignore)]
    pub transparent_state: Option<EffectMeshTransparentState>,
}

/// The original client's D3D render states of an effect mesh (EFT file) or a zone
/// morph object (LIST_MORPH_OBJECT.STB).
#[derive(Clone, Copy, Debug)]
pub struct EffectMeshRenderStates {
    /// D3DRS_ALPHABLENDENABLE: blend with the factors below, otherwise replace.
    pub alpha_enabled: bool,
    pub alpha_test_enabled: bool,
    pub two_sided: bool,
    pub depth_test_enabled: bool,
    pub depth_write_enabled: bool,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
    pub blend_op: BlendOperation,
}

/// Pipeline state of an effect mesh drawn in the transparent pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EffectMeshTransparentState {
    /// Blend equation for color and alpha; `None` replaces (alpha blending off).
    pub blend: Option<BlendComponent>,
    /// Discard fragments with alpha below 0.5 (the original's alpha test).
    pub alpha_test: bool,
    pub depth_write_enabled: bool,
    pub depth_test_enabled: bool,
}

/// Pipeline key of [`RoseEffectExtension`].
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct RoseEffectExtensionKey {
    transparent_state: Option<EffectMeshTransparentState>,
}

impl From<&RoseEffectExtension> for RoseEffectExtensionKey {
    fn from(extension: &RoseEffectExtension) -> Self {
        Self {
            transparent_state: extension.transparent_state,
        }
    }
}

/// A D3D blend equation as a wgpu blend component. D3D ignores the factors of
/// D3DBLENDOP_MIN / MAX, while wgpu rejects any factor but One with them.
pub fn d3d_blend_component(
    src_factor: BlendFactor,
    dst_factor: BlendFactor,
    operation: BlendOperation,
) -> BlendComponent {
    if matches!(operation, BlendOperation::Min | BlendOperation::Max) {
        BlendComponent {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::One,
            operation,
        }
    } else {
        BlendComponent {
            src_factor,
            dst_factor,
            operation,
        }
    }
}

/// Builds an effect mesh / morph object material with the original client's render
/// states. Every `ExtendedMaterial<StandardMaterial, RoseEffectExtension>` should be
/// created through this.
///
/// Alpha-blended meshes and meshes without depth write or depth test go to the
/// transparent pass (drawn back to front), where `RoseEffectExtension::specialize`
/// applies the mesh's own blend equation and depth state, like the original client
/// (and the Bevy 0.11 client's EffectMeshMaterial). Most effect meshes are additive
/// (SrcAlpha / One) over a black texture background: drawn opaque, that background
/// was a black box. Other meshes stay opaque or alpha-masked, forward-rendered (the
/// water reflection camera has no deferred prepass).
///
/// `base` supplies the texture and lighting settings; alpha mode, culling and the
/// render method are set here.
pub fn rose_effect_material(
    base: StandardMaterial,
    animation_texture: Option<Handle<Image>>,
    states: EffectMeshRenderStates,
) -> ExtendedMaterial<StandardMaterial, RoseEffectExtension> {
    let transparent =
        states.alpha_enabled || !states.depth_write_enabled || !states.depth_test_enabled;

    let alpha_mode = if transparent {
        // BLEND_ALPHA: the shader outputs straight (not premultiplied) color and
        // alpha, which is what the D3D blend factors expect.
        AlphaMode::Blend
    } else if states.alpha_test_enabled {
        AlphaMode::Mask(0.5)
    } else {
        AlphaMode::Opaque
    };

    let transparent_state = transparent.then(|| EffectMeshTransparentState {
        blend: states.alpha_enabled.then(|| {
            d3d_blend_component(
                states.src_blend_factor,
                states.dst_blend_factor,
                states.blend_op,
            )
        }),
        alpha_test: states.alpha_test_enabled,
        depth_write_enabled: states.depth_write_enabled,
        depth_test_enabled: states.depth_test_enabled,
    });

    ExtendedMaterial {
        base: StandardMaterial {
            alpha_mode,
            double_sided: states.two_sided,
            cull_mode: if states.two_sided {
                None
            } else {
                Some(Face::Back)
            },
            opaque_render_method: OpaqueRendererMethod::Forward,
            ..base
        },
        extension: RoseEffectExtension {
            animation_texture,
            animation_state: EffectMeshAnimationUniform::default(),
            transparent_state,
        },
    }
}

/// Uniform structure for effect mesh animation state
/// This matches `EffectMeshAnimationState` in `shaders/rose_effect_mesh.wgsl`
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
            transparent_state: None,
        }
    }
}

// The morph runs in every vertex stage the mesh is drawn with (forward pass, depth and
// normal prepass, deferred G-buffer, shadow maps), so depth, shadows and shading follow
// the animation. Fragment stages other than the forward one stay StandardMaterial's.
// The shader applies the animation only when `animation_state` reports frames, so
// meshes without an animation texture are drawn unmodified.
impl MaterialExtension for RoseEffectExtension {
    fn vertex_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_EFFECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_EFFECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn prepass_vertex_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_EFFECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn deferred_vertex_shader() -> ShaderRef {
        crate::render::extension_material_plugin::ROSE_EFFECT_EXTENSION_SHADER_HANDLE.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let Some(state) = key.bind_group_data.transparent_state else {
            return Ok(());
        };

        // Transparent-pass materials have no prepass or shadow pipelines, but keep
        // their depth-only pipelines untouched should one ever be built.
        if descriptor
            .vertex
            .shader_defs
            .contains(&ShaderDefVal::from("PREPASS_PIPELINE"))
        {
            return Ok(());
        }

        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = Some(state.depth_write_enabled);
            if !state.depth_test_enabled {
                depth_stencil.depth_compare = Some(CompareFunction::Always);
            }
        }

        if let Some(fragment) = descriptor.fragment.as_mut() {
            if state.alpha_test {
                fragment
                    .shader_defs
                    .push(ShaderDefVal::from("ROSE_EFFECT_ALPHA_TEST"));
            }

            let blend = state.blend.map(|component| BlendState {
                color: component,
                alpha: component,
            });
            for target in fragment.targets.iter_mut().flatten() {
                target.blend = blend;
            }
        }

        Ok(())
    }
}
