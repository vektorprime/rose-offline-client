//! Plugin to register all material extension shaders and custom material plugins
//!
//! This plugin registers the shaders for RoseObjectExtension, RoseTerrainExtension,
//! RoseWaterExtension, and RoseEffectExtension using Bevy's load_internal_asset! macro.
//!
//! Note: Zone lighting has been temporarily removed to simplify the rendering pipeline.
//! The RoseObjectMaterialPlugin now uses the standard MaterialPlugin without custom
//! draw commands. Zone lighting can be added back later once basic rendering is
//! confirmed working.

use bevy::{
    asset::{load_internal_asset, weak_handle},
    pbr::{
        MaterialExtension, MaterialPlugin, StandardMaterial,
        ExtendedMaterial, MaterialPipeline, MaterialPipelineKey,
    },
    prelude::*,
    render::{
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{RenderPipelineDescriptor, ShaderRef, SpecializedMeshPipelineError},
    },
};

use crate::render::object_material_extension::RoseObjectExtension;

// Shader handles for material extensions
pub const ROSE_OBJECT_EXTENSION_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("8a1b2c3d-4e5f-6789-0000-000000000000");

pub const ROSE_TERRAIN_EXTENSION_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("9b2c3d4e-5f6a-7890-0000-000000000000");

pub const ROSE_WATER_EXTENSION_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("ac3d4e5f-6a7b-89c1-0000-000000000000");

pub const ROSE_EFFECT_EXTENSION_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("bd4e5f6a-7b8c-9d2e-0000-000000000000");

/// Type alias for the RoseObject material type
pub type RoseObjectMaterial = ExtendedMaterial<StandardMaterial, RoseObjectExtension>;

/// Custom MaterialPlugin for RoseObjectMaterial
/// Uses standard Bevy rendering without custom zone lighting
pub struct RoseObjectMaterialPlugin {
    /// Controls if the prepass is enabled for the Material.
    pub prepass_enabled: bool,
    /// Controls if shadows are enabled for the Material.
    pub shadows_enabled: bool,
}

impl Default for RoseObjectMaterialPlugin {
    fn default() -> Self {
        Self {
            // Prepass and shadows enabled for proper depth rendering and shadow mapping
            prepass_enabled: true,
            shadows_enabled: true,
        }
    }
}

impl Plugin for RoseObjectMaterialPlugin {
    fn build(&self, app: &mut App) {
        bevy::log::info!("[ROSE OBJECT MATERIAL PLUGIN] Building standard MaterialPlugin");

        // Use the standard MaterialPlugin for the extended material
        let mut material_plugin = MaterialPlugin::<RoseObjectMaterial>::default();
        material_plugin.prepass_enabled = self.prepass_enabled;
        material_plugin.shadows_enabled = self.shadows_enabled;
        
        app.add_plugins(material_plugin);

        bevy::log::info!("[ROSE OBJECT MATERIAL PLUGIN] Build complete");
    }
}

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
