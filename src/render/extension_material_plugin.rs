//! Plugin to register all material extension shaders
//!
//! This plugin registers the shaders for RoseObjectExtension, RoseTerrainExtension,
//! RoseWaterExtension, and RoseEffectExtension using Bevy's load_internal_asset! macro.
//! This ensures the shaders are available at runtime with correct paths.

use bevy::{asset::load_internal_asset, prelude::{App, Plugin}, render::render_resource::Shader};

// Shader handles for material extensions
pub const ROSE_OBJECT_EXTENSION_SHADER_HANDLE: bevy::asset::Handle<Shader> =
    bevy::asset::Handle::weak_from_u128(0x8a1b2c3d4e5f6789);

pub const ROSE_TERRAIN_EXTENSION_SHADER_HANDLE: bevy::asset::Handle<Shader> =
    bevy::asset::Handle::weak_from_u128(0x9b2c3d4e5f6a7890);

pub const ROSE_WATER_EXTENSION_SHADER_HANDLE: bevy::asset::Handle<Shader> =
    bevy::asset::Handle::weak_from_u128(0xac3d4e5f6a7b89c1);

pub const ROSE_EFFECT_EXTENSION_SHADER_HANDLE: bevy::asset::Handle<Shader> =
    bevy::asset::Handle::weak_from_u128(0xbd4e5f6a7b8c9d2e);

/// Plugin that registers all material extension shaders
///
/// This plugin must be added after the MaterialPlugin registrations for the
/// extension materials to ensure the shaders are available when needed.
pub struct ExtensionMaterialPlugin;

impl Plugin for ExtensionMaterialPlugin {
    fn build(&self, app: &mut App) {
        bevy::log::info!("[EXTENSION MATERIAL PLUGIN] Registering material extension shaders...");

        // Register RoseObjectExtension shader
        load_internal_asset!(
            app,
            ROSE_OBJECT_EXTENSION_SHADER_HANDLE,
            "shaders/rose_object_extension.wgsl",
            Shader::from_wgsl
        );

        // Register RoseTerrainExtension shader
        load_internal_asset!(
            app,
            ROSE_TERRAIN_EXTENSION_SHADER_HANDLE,
            "shaders/rose_terrain_extension.wgsl",
            Shader::from_wgsl
        );

        // Register RoseWaterExtension shader
        load_internal_asset!(
            app,
            ROSE_WATER_EXTENSION_SHADER_HANDLE,
            "shaders/rose_water_extension.wgsl",
            Shader::from_wgsl
        );

        // Register RoseEffectExtension shader
        load_internal_asset!(
            app,
            ROSE_EFFECT_EXTENSION_SHADER_HANDLE,
            "shaders/rose_effect_extension.wgsl",
            Shader::from_wgsl
        );

        bevy::log::info!("[EXTENSION MATERIAL PLUGIN] All material extension shaders registered successfully");
    }
}
