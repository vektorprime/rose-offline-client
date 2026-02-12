use bevy::{
    prelude::*,
    render::{alpha::AlphaMode, render_resource::*, storage::ShaderStorageBuffer},
    asset::{load_internal_asset, Handle},
    pbr::Material,
};

pub const DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x6a4b5c6d7e8f9a0b);

#[derive(Debug, Clone, Asset, TypePath, AsBindGroup)]
pub struct DamageDigitMaterial {
    #[storage(0, read_only)]
    pub positions: Handle<ShaderStorageBuffer>,
    
    #[storage(1, read_only)]
    pub sizes: Handle<ShaderStorageBuffer>,
    
    #[storage(2, read_only)]
    pub uvs: Handle<ShaderStorageBuffer>,
    
    #[texture(3)]
    #[sampler(4)]
    pub texture: Handle<Image>,
}

impl Material for DamageDigitMaterial {
    fn vertex_shader() -> ShaderRef {
        DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE.into()
    }

    // Return Default to signal no custom prepass shader - this prevents Bevy from
    // processing prepass shaders that expect storage buffers when the prepass
    // pipeline only provides uniform buffers
    fn prepass_vertex_shader() -> ShaderRef {
        ShaderRef::Default
    }

    fn prepass_fragment_shader() -> ShaderRef {
        ShaderRef::Default
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

        app.add_plugins(bevy::pbr::MaterialPlugin::<DamageDigitMaterial> {
            prepass_enabled: false,  // Disable prepass - storage buffers incompatible with prepass pipeline
            shadows_enabled: false,  // Transparent materials don't cast shadows
            ..Default::default()
        });
        bevy::log::info!("[MATERIAL PLUGIN] DamageDigitMaterial plugin built");
    }
}
