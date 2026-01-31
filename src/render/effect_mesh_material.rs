//! Simplified EffectMeshMaterial using Bevy's standard MaterialPlugin
//!
//! This removes the custom render pipeline and uses Bevy's built-in material system.

use bevy::{
    asset::{load_internal_asset, AssetApp, Handle, UntypedHandle, Asset, UntypedAssetId},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{AlphaMode, App, Component, Plugin},
    reflect::{Reflect, TypePath},
    render::{
        mesh::MeshVertexBufferLayout,
        prelude::Shader,
        render_resource::{
            AsBindGroup, AsBindGroupShaderType, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, BlendOperation, BlendFactor,
        },
    },
    utils::Uuid,
};
use std::any::TypeId;

pub const EFFECT_MESH_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid {
        type_id: TypeId::of::<Shader>(),
        uuid: Uuid::from_u128(0x90d5233c3001d33e),
    });

#[derive(Default)]
pub struct EffectMeshMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for EffectMeshMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Load the internal asset using the Bevy 0.13 API
        load_internal_asset!(
            app,
            EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed::<Shader>(),
            "shaders/effect_mesh_material.wgsl",
            bevy::render::render_resource::Shader::from_wgsl
        );

        // Register the EffectMeshMaterial asset type with MaterialPlugin
        app.add_plugins(bevy::pbr::MaterialPlugin::<EffectMeshMaterial>::default());
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct EffectMeshMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE         = (1 << 0);
        const ALPHA_MODE_MASK           = (1 << 1);
        const NONE                      = 0;
    }
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct EffectMeshAnimationFlags: u32 {
        const ANIMATE_POSITION          = (1 << 0);
        const ANIMATE_NORMALS           = (1 << 1);
        const ANIMATE_UV                = (1 << 2);
        const ANIMATE_ALPHA             = (1 << 3);
        const NONE                      = 0;
    }
}

#[derive(Copy, Clone, Default, Component, bevy::render::render_resource::encase::ShaderType, Reflect)]
pub struct EffectMeshAnimationRenderState {
    pub flags: u32,
    pub current_next_frame: u32,
    pub next_weight: f32,
    pub alpha: f32,
}

/// Uniform data for EffectMeshMaterial - matches the shader layout
#[derive(Clone, Copy, Debug, Default, bevy::render::render_resource::encase::ShaderType)]
pub struct EffectMeshMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub _padding: f32,
    pub _padding2: f32,
}

impl AsBindGroupShaderType<EffectMeshMaterialUniformData> for EffectMeshMaterial {
    fn as_bind_group_shader_type(&self, _images: &bevy::render::render_asset::RenderAssets<bevy::render::texture::Image>) -> EffectMeshMaterialUniformData {
        let mut flags = EffectMeshMaterialFlags::NONE;
        if self.alpha_test {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_MASK;
        } else if !self.alpha_enabled {
            flags |= EffectMeshMaterialFlags::ALPHA_MODE_OPAQUE;
        }

        EffectMeshMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff: 0.5,
            _padding: 0.0,
            _padding2: 0.0,
        }
    }
}

/// Simplified EffectMeshMaterial using Bevy's standard Material trait
#[derive(Asset, AsBindGroup, Debug, Clone, PartialEq, TypePath)]
#[bind_group_data(EffectMeshMaterialKey)]
#[uniform(0, EffectMeshMaterialUniformData)]
pub struct EffectMeshMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<bevy::render::texture::Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub animation_texture: Option<Handle<bevy::render::texture::Image>>,

    pub alpha_enabled: bool,
    pub alpha_test: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

impl Default for EffectMeshMaterial {
    fn default() -> Self {
        Self {
            base_texture: None,
            animation_texture: None,
            alpha_enabled: false,
            alpha_test: false,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
            blend_op: BlendOperation::Add,
            src_blend_factor: BlendFactor::SrcAlpha,
            dst_blend_factor: BlendFactor::OneMinusSrcAlpha,
        }
    }
}

impl Material for EffectMeshMaterial {
    fn vertex_shader() -> ShaderRef {
        EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        EFFECT_MESH_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.alpha_enabled || !self.z_write_enabled {
            AlphaMode::Blend
        } else if self.alpha_test {
            AlphaMode::Mask(0.5)
        } else {
            AlphaMode::Opaque
        }
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Apply depth/stencil configuration
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = key.bind_group_data.z_write_enabled;
        }

        // Apply face culling based on two_sided flag
        descriptor.primitive.cull_mode = if key.bind_group_data.two_sided {
            None
        } else {
            Some(bevy::render::render_resource::Face::Back)
        };

        // Apply custom blend state
        if let Some(ref mut fragment) = descriptor.fragment {
            for color_target_state in fragment.targets.iter_mut().filter_map(|x| x.as_mut()) {
                color_target_state.blend = Some(bevy::render::render_resource::BlendState {
                    color: bevy::render::render_resource::BlendComponent {
                        src_factor: key.bind_group_data.src_blend_factor,
                        dst_factor: key.bind_group_data.dst_blend_factor,
                        operation: key.bind_group_data.blend_op,
                    },
                    alpha: bevy::render::render_resource::BlendComponent {
                        src_factor: key.bind_group_data.src_blend_factor,
                        dst_factor: key.bind_group_data.dst_blend_factor,
                        operation: key.bind_group_data.blend_op,
                    },
                });
            }
        }

        Ok(())
    }
}

/// Key data for material specialization
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct EffectMeshMaterialKey {
    pub has_animation_texture: bool,
    pub alpha_enabled: bool,
    pub alpha_test: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub blend_op: BlendOperation,
    pub src_blend_factor: BlendFactor,
    pub dst_blend_factor: BlendFactor,
}

impl From<&EffectMeshMaterial> for EffectMeshMaterialKey {
    fn from(material: &EffectMeshMaterial) -> Self {
        EffectMeshMaterialKey {
            has_animation_texture: material.animation_texture.is_some(),
            alpha_enabled: material.alpha_enabled,
            alpha_test: material.alpha_test,
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            blend_op: material.blend_op,
            src_blend_factor: material.src_blend_factor,
            dst_blend_factor: material.dst_blend_factor,
        }
    }
}
