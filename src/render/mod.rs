use bevy::{
    prelude::{App, Plugin},
    render::{mesh::MeshVertexAttribute, render_resource::VertexFormat},
};

// Simplified render module - using Bevy's built-in materials
// Custom materials removed to use StandardMaterial instead

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

pub mod trail_effect;
pub use trail_effect::*;
pub use trail_effect::TrailEffectRenderPlugin;

pub mod damage_digit_material;
pub use damage_digit_material::*;

pub mod damage_digit_render_data;
pub use damage_digit_render_data::*;

pub mod sky_material;
pub use sky_material::{SkyMaterial, SkyMaterialPlugin};

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

pub const MESH_ATTRIBUTE_UV_1: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv2", 280035324, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_2: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv3", 2422131906, VertexFormat::Float32x2);

pub const MESH_ATTRIBUTE_UV_3: MeshVertexAttribute =
    MeshVertexAttribute::new("Vertex_Uv4", 519697814, VertexFormat::Float32x2);

#[derive(Default)]
pub struct RoseRenderPlugin;

impl Plugin for RoseRenderPlugin {
    fn build(&self, _app: &mut App) {
        bevy::log::info!("[RENDER PLUGIN] RoseRenderPlugin - Materials registered via their own plugins");
    }
}
