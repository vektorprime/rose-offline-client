use bevy::{
    prelude::*,
    render::{alpha::AlphaMode, render_resource::*},
    asset::{load_internal_asset, Handle},
};

pub const DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x6a4b5c6d7e8f9a0b);

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct DamageDigitMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Material for DamageDigitMaterial {
    fn vertex_shader() -> ShaderRef {
        DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub struct DamageDigitMaterialPlugin;

impl Plugin for DamageDigitMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE,
            "shaders/damage_digit.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(MaterialPlugin::<DamageDigitMaterial>::default());
        bevy::log::info!("[MATERIAL PLUGIN] DamageDigitMaterial plugin built");
    }
}
