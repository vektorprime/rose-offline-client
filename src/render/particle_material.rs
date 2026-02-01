use bevy::{
    prelude::*,
    render::render_resource::*,
    asset::{load_internal_asset, Handle},
};

pub const PARTICLE_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x5f3c4d5e6f7a8b9c);

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct ParticleMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Material for ParticleMaterial {
    fn vertex_shader() -> ShaderRef {
        PARTICLE_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        PARTICLE_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
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

        app.add_plugins(MaterialPlugin::<ParticleMaterial>::default());
    }
}
