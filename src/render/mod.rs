use bevy::{
    mesh::MeshVertexAttribute,
    prelude::{App, Plugin},
    render::render_resource::VertexFormat,
};

// Custom terrain material with texture array support
pub mod terrain_material;
pub use terrain_material::{TerrainMaterial, TerrainMaterialPlugin, TERRAIN_MATERIAL_MAX_TEXTURES};

// Custom water material with animated texture array support
pub mod water_material;
pub use water_material::{WaterMaterial, WaterMaterialPlugin};

/// Custom vertex attribute for terrain tile info
/// Encoded as u32: layer1_id (bits 0-7) | layer2_id (bits 8-15) | rotation (bits 16-23)
pub const TERRAIN_MESH_ATTRIBUTE_TILE_INFO: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_TileInfo", 988347822, VertexFormat::Uint32);

pub mod world_ui;
pub use world_ui::{WorldUiRect, WorldUiRenderPlugin};

pub mod particle_material;
pub use particle_material::*;

pub mod particle_render_data;
pub use particle_render_data::*;

pub mod particle_debug;
pub use particle_debug::{debug_particle_rendering, particle_performance_monitor};

pub mod zone_lighting;
pub use zone_lighting::SkyMode;
pub use zone_lighting::SkySettings;
pub use zone_lighting::VolumetricFogVolume;
pub use zone_lighting::ZoneLighting;
pub use zone_lighting::ZoneLightingPlugin;

pub mod trail_effect;
pub use trail_effect::TrailEffectRenderPlugin;
pub use trail_effect::*;

pub mod damage_digit_material;
pub use damage_digit_material::*;

pub mod damage_digit_render_data;
pub use damage_digit_render_data::*;

// Angelic wing material with glow effects
pub mod wing_material;
pub use wing_material::{WingMaterial, WingMaterialPlugin};

pub mod object_material_extension;
pub use object_material_extension::*;

pub mod terrain_material_extension;
pub use terrain_material_extension::*;

pub mod water_material_extension;
pub use water_material_extension::*;

pub mod effect_mesh_extension;
pub use effect_mesh_extension::*;

pub mod extension_material_plugin;
pub use extension_material_plugin::ExtensionMaterialPlugin;
pub use extension_material_plugin::RoseObjectMaterialPlugin;

// Skinned mesh fix for Bevy 0.16 - REQUIRED for proper skinned mesh rendering
pub mod skinned_mesh_fix;
pub use skinned_mesh_fix::SkinnedMeshFixPlugin;

// Underwater rendering effect
pub mod underwater_effect;
pub use underwater_effect::{CameraUnderwaterState, UnderwaterEffectPlugin, UnderwaterSettings};

// Procedural starry sky material
pub mod starry_sky_material;
pub use starry_sky_material::{
    create_starry_sky_mesh, moon_light_follow_camera_system, sky_sphere_follow_camera_system,
    toggle_atmosphere_based_on_time, update_starry_sky_night_factor, update_starry_sky_system,
    AtmosphereState, MoonLight, StarrySky, StarrySkyMaterial, StarrySkyMaterialPlugin,
    StarrySkySettings,
};

// Procedural cloud material (2D plane-based)
pub mod cloud_material;
pub use cloud_material::{
    cloud_layer_follow_camera_system, spawn_cloud_layer, update_cloud_lighting_system,
    update_cloud_material_system, CloudLayer, CloudMaterial, CloudMaterialPlugin, CloudSettings,
};

// 3D volumetric cloud material (fluffy cumulus style)
pub mod volumetric_cloud;
pub use volumetric_cloud::{
    despawn_volumetric_clouds, spawn_volumetric_clouds, update_volumetric_cloud_lighting_system,
    update_volumetric_cloud_material_system, VolumetricCloud, VolumetricCloudMaterial,
    VolumetricCloudPlugin, VolumetricCloudSettings,
};

pub const MESH_ATTRIBUTE_UV_1: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv2", 280035324, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_2: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv3", 2422131906, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_3: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv4", 519697814, VertexFormat::Float32x2);

#[derive(Default)]
pub struct RoseRenderPlugin;

impl Plugin for RoseRenderPlugin {
    fn build(&self, app: &mut App) {
        bevy::log::info!("[RENDER PLUGIN] RoseRenderPlugin - Registering material plugins");

        // Register the terrain material plugin
        app.add_plugins(TerrainMaterialPlugin);

        // Register the water material plugin for animated water rendering
        app.add_plugins(WaterMaterialPlugin);

        bevy::log::info!(
            "[RENDER PLUGIN] RoseRenderPlugin - Materials registered via their own plugins"
        );
    }
}
