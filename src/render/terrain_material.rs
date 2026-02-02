//! Simplified TerrainMaterial using Bevy's standard MaterialPlugin
//!
//! This uses a simplified approach that works with Bevy's standard Material trait.

use bevy::{
    asset::{load_internal_asset, Handle, UntypedHandle, Asset, UntypedAssetId},
    pbr::{Material, MaterialPipeline, MaterialPipelineKey},
    prelude::{App, Plugin},
    reflect::TypePath,
    render::{
        alpha::AlphaMode,
        mesh::{MeshVertexBufferLayoutRef, MeshVertexAttribute},
        prelude::Shader,
        render_resource::{
            AsBindGroup, RenderPipelineDescriptor, ShaderRef,
            SpecializedMeshPipelineError, VertexFormat, ShaderDefVal,
        },
    },
};
use std::any::TypeId;
use uuid::Uuid;

pub const TERRAIN_MATERIAL_SHADER_HANDLE: UntypedHandle =
    UntypedHandle::Weak(UntypedAssetId::Uuid { type_id: TypeId::of::<Shader>(), uuid: Uuid::from_u128(0x3d7939250aff89cb) });

pub const TERRAIN_MATERIAL_SHADER_HANDLE_TYPED: Handle<Shader> =
    Handle::weak_from_u128(0x3d7939250aff89cb);

pub const TERRAIN_MESH_ATTRIBUTE_TILE_INFO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileInfo", 3855645392, VertexFormat::Uint32);

// Reduced max textures for simplicity
pub const TERRAIN_MATERIAL_MAX_TEXTURES: usize = 16;

#[derive(Default)]
pub struct TerrainMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for TerrainMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TERRAIN_MATERIAL_SHADER_HANDLE_TYPED,
            "shaders/terrain_material.wgsl",
            bevy::render::render_resource::Shader::from_wgsl
        );

        // Register the TerrainMaterial asset type with MaterialPlugin
        app.add_plugins(bevy::pbr::MaterialPlugin::<TerrainMaterial>::default());
        bevy::log::info!("[MATERIAL PLUGIN] TerrainMaterial plugin built");
    }
}

/// Simplified TerrainMaterial using Bevy's standard Material trait
/// 
/// Uses a Vec of textures - the shader will sample based on tile info
/// Note: Only the first 4 textures are used in the simplified shader
#[derive(Asset, AsBindGroup, Debug, Clone, TypePath)]
#[bind_group_data(TerrainMaterialKey)]
pub struct TerrainMaterial {
    #[texture(0, dimension = "2d")]
    #[sampler(1)]
    pub texture0: Option<Handle<bevy::render::texture::Image>>,
    
    #[texture(2, dimension = "2d")]
    #[sampler(3)]
    pub texture1: Option<Handle<bevy::render::texture::Image>>,
    
    #[texture(4, dimension = "2d")]
    #[sampler(5)]
    pub texture2: Option<Handle<bevy::render::texture::Image>>,
    
    #[texture(6, dimension = "2d")]
    #[sampler(7)]
    pub texture3: Option<Handle<bevy::render::texture::Image>>,
    
    #[texture(8, dimension = "2d")]
    #[sampler(9)]
    pub detail_texture: Option<Handle<bevy::render::texture::Image>>,
    
    /// Total texture count for shader specialization
    pub texture_count: u32,
    
    /// Storage for additional textures (not used in bind group directly)
    #[uniform(10)]
    pub _padding: [f32; 4],
}

impl Default for TerrainMaterial {
    fn default() -> Self {
        Self {
            texture0: None,
            texture1: None,
            texture2: None,
            texture3: None,
            detail_texture: None,
            texture_count: 0,
            _padding: [0.0; 4],
        }
    }
}

impl TerrainMaterial {
    /// Create from a list of textures (simplified - only uses first 4)
    pub fn from_textures(textures: &[Handle<bevy::render::texture::Image>]) -> Self {
        Self {
            texture0: textures.get(0).cloned(),
            texture1: textures.get(1).cloned(),
            texture2: textures.get(2).cloned(),
            texture3: textures.get(3).cloned(),
            detail_texture: None,
            texture_count: textures.len().min(4) as u32,
            _padding: [0.0; 4],
        }
    }
}

impl Material for TerrainMaterial {
    fn vertex_shader() -> ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn fragment_shader() -> ShaderRef {
        TERRAIN_MATERIAL_SHADER_HANDLE.typed().into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Add TEXTURE_COUNT shader def
        let texture_count = key.bind_group_data.texture_count.max(1);
        descriptor.vertex.shader_defs.push(ShaderDefVal::UInt("TEXTURE_COUNT".into(), texture_count));
        if let Some(ref mut fragment) = descriptor.fragment {
            fragment.shader_defs.push(ShaderDefVal::UInt("TEXTURE_COUNT".into(), texture_count));
        }
        Ok(())
    }
}

/// Key data for material specialization
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TerrainMaterialKey {
    pub texture_count: u32,
}

impl From<&TerrainMaterial> for TerrainMaterialKey {
    fn from(material: &TerrainMaterial) -> Self {
        TerrainMaterialKey {
            texture_count: material.texture_count.max(1),
        }
    }
}
