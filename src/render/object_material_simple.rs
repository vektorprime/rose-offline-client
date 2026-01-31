//! Simplified ObjectMaterial using Bevy's standard MaterialPlugin
//!
//! This removes the custom render pipeline and uses Bevy's built-in material system.
//! Zone lighting is simplified to use standard Bevy lighting or hardcoded values.

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
            SpecializedMeshPipelineError, TextureSampleType, TextureViewDimension,
        },
    },
    utils::Uuid,
};
use std::any::TypeId;

use rose_file_readers::{ZscMaterialBlend, ZscMaterialGlow};

pub const OBJECT_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid {
        type_id: TypeId::of::<Shader>(),
        uuid: Uuid::from_u128(0xb7ebbc00ea16d3c7),
    });

pub struct ObjectMaterialPlugin;

impl Plugin for ObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        // Load the internal asset using the Bevy 0.13 API
        load_internal_asset!(
            app,
            OBJECT_MATERIAL_SHADER_HANDLE.typed::<Shader>(),
            "shaders/object_material_simple.wgsl",
            bevy::render::render_resource::Shader::from_wgsl
        );

        // Register the ObjectMaterial asset type with MaterialPlugin
        app.add_plugins(bevy::pbr::MaterialPlugin::<ObjectMaterial>::default());
    }
}

/// Marker component for face clipping (not implemented in simplified shader)
/// Kept for API compatibility with existing code
#[derive(Copy, Clone, Component, Reflect)]
pub enum ObjectMaterialClipFace {
    First(u32),
    Last(u32),
}

bitflags::bitflags! {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct ObjectMaterialFlags: u32 {
        const ALPHA_MODE_OPAQUE          = (1 << 0);
        const ALPHA_MODE_MASK            = (1 << 1);
        const ALPHA_MODE_BLEND           = (1 << 2);
        const HAS_ALPHA_VALUE            = (1 << 3);
        const SPECULAR                   = (1 << 4);
        const NONE                       = 0;
    }
}

/// Uniform data for ObjectMaterial - matches the shader layout
#[derive(Clone, Copy, Debug, Default, Reflect, bevy::render::render_resource::encase::ShaderType)]
pub struct ObjectMaterialUniformData {
    pub flags: u32,
    pub alpha_cutoff: f32,
    pub alpha_value: f32,
    pub lightmap_uv_offset_x: f32,
    pub lightmap_uv_offset_y: f32,
    pub lightmap_uv_scale: f32,
    // Padding to ensure 16-byte alignment
    pub _padding: f32,
}

impl AsBindGroupShaderType<ObjectMaterialUniformData> for ObjectMaterial {
    fn as_bind_group_shader_type(&self, _images: &bevy::render::render_asset::RenderAssets<bevy::render::texture::Image>) -> ObjectMaterialUniformData {
        let mut flags = ObjectMaterialFlags::NONE;
        let mut alpha_cutoff = 0.5;
        let mut alpha_value = 1.0;

        if self.specular_texture.is_some() {
            flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE | ObjectMaterialFlags::SPECULAR;
            alpha_cutoff = 1.0;
        } else {
            if self.alpha_enabled {
                flags |= ObjectMaterialFlags::ALPHA_MODE_BLEND;

                if let Some(alpha_ref) = self.alpha_test {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_MASK;
                    alpha_cutoff = alpha_ref;
                }
            } else {
                flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
            }

            if let Some(material_alpha_value) = self.alpha_value {
                if material_alpha_value == 1.0 {
                    flags |= ObjectMaterialFlags::ALPHA_MODE_OPAQUE;
                } else {
                    flags |= ObjectMaterialFlags::HAS_ALPHA_VALUE;
                    alpha_value = material_alpha_value;
                }
            }
        }

        ObjectMaterialUniformData {
            flags: flags.bits(),
            alpha_cutoff,
            alpha_value,
            lightmap_uv_offset_x: self.lightmap_uv_offset.x,
            lightmap_uv_offset_y: self.lightmap_uv_offset.y,
            lightmap_uv_scale: self.lightmap_uv_scale,
            _padding: 0.0,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, Reflect, Hash, PartialEq)]
pub enum ObjectMaterialBlend {
    #[default]
    Normal,
    Lighten,
}

impl From<ZscMaterialBlend> for ObjectMaterialBlend {
    fn from(zsc: ZscMaterialBlend) -> Self {
        match zsc {
            ZscMaterialBlend::Normal => ObjectMaterialBlend::Normal,
            ZscMaterialBlend::Lighten => ObjectMaterialBlend::Lighten,
        }
    }
}

#[derive(Copy, Clone, Debug, Reflect, PartialEq)]
pub enum ObjectMaterialGlow {
    Simple(bevy::math::Vec3),
    Light(bevy::math::Vec3),
    Texture(bevy::math::Vec3),
    TextureLight(bevy::math::Vec3),
    Alpha(bevy::math::Vec3),
}

impl From<ZscMaterialGlow> for ObjectMaterialGlow {
    fn from(zsc: ZscMaterialGlow) -> Self {
        match zsc {
            ZscMaterialGlow::Simple(value) => {
                ObjectMaterialGlow::Simple(bevy::math::Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Light(value) => {
                ObjectMaterialGlow::Light(bevy::math::Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Texture(value) => {
                ObjectMaterialGlow::Texture(bevy::math::Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::TextureLight(value) => {
                ObjectMaterialGlow::TextureLight(bevy::math::Vec3::new(value.x, value.y, value.z))
            }
            ZscMaterialGlow::Alpha(value) => {
                ObjectMaterialGlow::Alpha(bevy::math::Vec3::new(value.x, value.y, value.z))
            }
        }
    }
}

/// Simplified ObjectMaterial using Bevy's standard Material trait
///
/// This uses AsBindGroup derive for automatic bind group management
/// and integrates with Bevy's standard material pipeline.
#[derive(Asset, AsBindGroup, Debug, Clone, Reflect, PartialEq)]
#[bind_group_data(ObjectMaterialKey)]
#[uniform(0, ObjectMaterialUniformData)]
pub struct ObjectMaterial {
    #[texture(1)]
    #[sampler(2)]
    pub base_texture: Option<Handle<bevy::render::texture::Image>>,

    #[texture(3)]
    #[sampler(4)]
    pub lightmap_texture: Option<Handle<bevy::render::texture::Image>>,

    #[texture(5)]
    #[sampler(6)]
    pub specular_texture: Option<Handle<bevy::render::texture::Image>>,

    pub lightmap_uv_offset: bevy::math::Vec2,
    pub lightmap_uv_scale: f32,
    pub alpha_value: Option<f32>,
    pub alpha_enabled: bool,
    pub alpha_test: Option<f32>,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub skinned: bool,
    pub blend: ObjectMaterialBlend,
    pub glow: Option<ObjectMaterialGlow>,
}

impl Default for ObjectMaterial {
    fn default() -> Self {
        Self {
            base_texture: None,
            alpha_value: None,
            alpha_enabled: false,
            alpha_test: None,
            two_sided: false,
            z_test_enabled: true,
            z_write_enabled: true,
            specular_texture: None,
            skinned: false,
            blend: ObjectMaterialBlend::Normal,
            glow: None,
            lightmap_texture: None,
            lightmap_uv_offset: bevy::math::Vec2::new(0.0, 0.0),
            lightmap_uv_scale: 1.0,
        }
    }
}

impl Material for ObjectMaterial {
    fn vertex_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        OBJECT_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        if self.specular_texture.is_some() {
            AlphaMode::Opaque
        } else if self.alpha_enabled {
            if self.alpha_test.is_some() {
                AlphaMode::Mask(self.alpha_test.unwrap_or(0.5))
            } else {
                AlphaMode::Blend
            }
        } else {
            // alpha_enabled is false
            if let Some(alpha_value) = self.alpha_value {
                if alpha_value == 1.0 {
                    AlphaMode::Opaque
                } else {
                    AlphaMode::Blend
                }
            } else {
                AlphaMode::Opaque
            }
        }
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Add LIGHTMAP_UV shader def if material has lightmap
        if key.bind_group_data.has_lightmap {
            descriptor.vertex.shader_defs.push("LIGHTMAP_UV".into());
            if let Some(ref mut fragment) = descriptor.fragment {
                fragment.shader_defs.push("LIGHTMAP_UV".into());
            }
        }

        // Apply depth/stencil configuration
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.depth_write_enabled = !key.bind_group_data.alpha_enabled;
        }

        // Apply face culling based on two_sided flag
        descriptor.primitive.cull_mode = if key.bind_group_data.two_sided {
            None
        } else {
            Some(bevy::render::render_resource::Face::Back)
        };

        Ok(())
    }
}

/// Key data for material specialization
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ObjectMaterialKey {
    pub has_lightmap: bool,
    pub two_sided: bool,
    pub z_test_enabled: bool,
    pub z_write_enabled: bool,
    pub alpha_enabled: bool,
}

impl From<&ObjectMaterial> for ObjectMaterialKey {
    fn from(material: &ObjectMaterial) -> Self {
        ObjectMaterialKey {
            has_lightmap: material.lightmap_texture.is_some(),
            two_sided: material.two_sided,
            z_test_enabled: material.z_test_enabled,
            z_write_enabled: material.z_write_enabled,
            alpha_enabled: material.alpha_enabled || material.alpha_value.map(|v| v < 1.0).unwrap_or(false),
        }
    }
}
