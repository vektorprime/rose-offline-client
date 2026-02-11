use bevy::{
    prelude::*,
    render::{
        alpha::AlphaMode,
        mesh::MeshVertexBufferLayoutRef,
        render_resource::*,
    },
    asset::{load_internal_asset, Handle},
    pbr::Material,
};
use bevy::render::storage::ShaderStorageBuffer;

pub const PARTICLE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x5f3c4d5e6f7a8b9c);

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct ParticleMaterial {
    #[storage(0, read_only)]
    pub positions: Handle<ShaderStorageBuffer>,
    #[storage(1, read_only)]
    pub sizes: Handle<ShaderStorageBuffer>,
    #[storage(2, read_only)]
    pub colors: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    pub textures: Handle<ShaderStorageBuffer>,
    #[texture(4)]
    #[sampler(5)]
    pub texture: Handle<Image>,
    /// Blend operation to use for rendering
    /// Stored as u32 for shader compatibility (0=Add, 1=Subtract, 2=ReverseSubtract, 3=Min, 4=Max)
    #[uniform(6)]
    pub blend_op: u32,
    /// Source blend factor
    /// Stored as u32 for shader compatibility (mapped from BlendFactor)
    #[uniform(7)]
    pub src_blend_factor: u32,
    /// Destination blend factor
    /// Stored as u32 for shader compatibility (mapped from BlendFactor)
    #[uniform(8)]
    pub dst_blend_factor: u32,
    /// Billboard type for particle rotation
    /// Stored as u32 for shader compatibility (0=None, 1=YAxis, 2=Full)
    #[uniform(9)]
    pub billboard_type: u32,
}

impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef {
        PARTICLE_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        PARTICLE_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        // Map blend configuration to AlphaMode
        // Since AlphaMode doesn't support custom blend operations/factors directly,
        // we use Blend mode and configure custom blend state in specialize()
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &bevy::pbr::MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Note: We cannot access the material's blend configuration or billboard type in this static method
        // because key.bind_group_data is () (unit type) from AsBindGroup derive.
        //
        // The blend configuration and billboard type are stored as uniforms in the material and will be
        // accessible in the shader. For now, we use the default blend state from AlphaMode::Blend.
        //
        // The shader uses the billboard_type uniform to determine billboard behavior dynamically.
        Ok(())
    }
}

pub struct ParticleMaterialPlugin;

impl Plugin for ParticleMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PARTICLE_MATERIAL_SHADER_HANDLE,
            "shaders/particle.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(bevy::pbr::MaterialPlugin::<ParticleMaterial>::default());
        bevy::log::info!("[MATERIAL PLUGIN] ParticleMaterial plugin built");
    }
}
