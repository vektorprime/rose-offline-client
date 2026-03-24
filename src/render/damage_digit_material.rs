use bevy::{
    prelude::*,
    render::{alpha::AlphaMode, render_resource::*, storage::ShaderStorageBuffer},
    asset::{load_internal_asset, Handle, weak_handle},
    pbr::Material,
};
use bevy_shader::ShaderRef;

pub const DAMAGE_DIGIT_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("6a4b5c6d-7e8f-9a0b-0000-000000000000");

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

    /// Disable prepass - storage buffers incompatible with prepass pipeline
    fn enable_prepass() -> bool {
        false
    }

    /// Transparent materials don't cast shadows
    fn enable_shadows() -> bool {
        false
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

        // Note: prepass and shadows are controlled via enable_prepass() and enable_shadows() methods on Material trait
        app.add_plugins(bevy::pbr::MaterialPlugin::<DamageDigitMaterial>::default());
        bevy::log::info!("[MATERIAL PLUGIN] DamageDigitMaterial plugin built");
    }
}
