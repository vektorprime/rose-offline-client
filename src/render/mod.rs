use bevy::{
    prelude::{App, Plugin},
    render::{mesh::MeshVertexAttribute, render_resource::VertexFormat},
};

// Custom terrain material with texture array support
pub mod terrain_material;
pub use terrain_material::{
    TerrainMaterial, TerrainMaterialPlugin, TERRAIN_MATERIAL_MAX_TEXTURES,
};

// Custom water material with animated texture array support
pub mod water_material;
pub use water_material::{
    WaterMaterial, WaterMaterialPlugin, WaterAnimationTime, WATER_MATERIAL_NUM_TEXTURES,
};

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

pub mod particle_test;
pub use particle_test::test_particle_spawn;

pub mod zone_lighting;
pub use zone_lighting::ZoneLighting;
pub use zone_lighting::ZoneLightingPlugin;
pub use zone_lighting::VolumetricFogVolume;

pub mod trail_effect;
pub use trail_effect::*;
pub use trail_effect::TrailEffectRenderPlugin;

pub mod damage_digit_material;
pub use damage_digit_material::*;

pub mod damage_digit_render_data;
pub use damage_digit_render_data::*;

pub mod sky_material;
pub use sky_material::{SkyMaterial, SkyMaterialPlugin};

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
pub use underwater_effect::{
    UnderwaterEffectPlugin, UnderwaterSettings, CameraUnderwaterState,
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
        
        bevy::log::info!("[RENDER PLUGIN] RoseRenderPlugin - Materials registered via their own plugins");
    }
}
