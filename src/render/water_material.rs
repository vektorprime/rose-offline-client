use bevy::{
    prelude::*,
    render::{alpha::AlphaMode, render_resource::*},
    asset::{load_internal_asset, Handle},
};

pub const WATER_MATERIAL_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x4e2b3c4d5e6f7a8b);

pub const WATER_MATERIAL_NUM_TEXTURES: usize = 25;

#[derive(Asset, Debug, Clone, TypePath, AsBindGroup)]
pub struct WaterMaterial {
    #[texture(0, dimension = "2d")]
    #[sampler(1)]
    pub texture: Handle<Image>,
}

impl Default for WaterMaterial {
    fn default() -> Self {
        Self {
            texture: Handle::default(),
        }
    }
}

impl Material for WaterMaterial {
    fn vertex_shader() -> ShaderRef {
        WATER_MATERIAL_SHADER_HANDLE.into()
    }

    fn fragment_shader() -> ShaderRef {
        WATER_MATERIAL_SHADER_HANDLE.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
}

pub struct WaterMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Default for WaterMaterialPlugin {
    fn default() -> Self {
        Self {
            prepass_enabled: false,
        }
    }
}

impl Plugin for WaterMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WATER_MATERIAL_SHADER_HANDLE,
            "shaders/water_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(MaterialPlugin::<WaterMaterial>::default());
        bevy::log::info!("[MATERIAL PLUGIN] WaterMaterial plugin built");
    }
}
