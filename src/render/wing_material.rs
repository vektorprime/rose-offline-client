//! Angelic Wing Material with glow and ethereal effects
//! 
//! This material provides a custom shader for rendering angelic wings with:
//! - Base white/silver color with golden tips
//! - Soft blue/white glow emanating from within
//! - Semi-transparent edges for ethereal look
//! - Animated shimmer/pulse effect

use bevy::{
    asset::{load_internal_asset, weak_handle, Handle},
    pbr::{Material, MaterialPlugin, MaterialPipeline, MeshPipelineKey},
    prelude::*,
    reflect::TypePath,
    render::{
        alpha::AlphaMode,
        mesh::MeshVertexBufferLayoutRef,
        render_resource::{AsBindGroup, RenderPipelineDescriptor, Shader, SpecializedMeshPipelineError},
    },
};

/// Handle for the wing material shader
pub const WING_MATERIAL_SHADER_HANDLE: Handle<Shader> = 
    weak_handle!("a1b2c3d4-e5f6-7890-abcd-ef1234567890");

/// Plugin for the wing material
#[derive(Default)]
pub struct WingMaterialPlugin {
    pub prepass_enabled: bool,
}

impl Plugin for WingMaterialPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            WING_MATERIAL_SHADER_HANDLE,
            "shaders/wing_material.wgsl",
            Shader::from_wgsl
        );

        app.add_plugins(
            MaterialPlugin::<WingMaterial> {
                prepass_enabled: self.prepass_enabled,
                ..Default::default()
            },
        );
        
        // Add system to update time uniform
        app.add_systems(Update, wing_material_time_system);
        
        bevy::log::info!("[WingMaterial] Plugin initialized");
    }
}

/// Custom material for angelic wings with glow effects
#[derive(Asset, Debug, Clone, TypePath, AsBindGroup)]
pub struct WingMaterial {
    /// Base color of the wing (white/silver)
    #[uniform(0)]
    pub base_color: LinearRgba,
    
    /// Glow color (soft blue/white)
    #[uniform(0)]
    pub glow_color: LinearRgba,
    
    /// Intensity of the glow effect (0.0 - 2.0)
    #[uniform(0)]
    pub glow_intensity: f32,
    
    /// Time value for animated effects (updated automatically)
    #[uniform(0)]
    pub time: f32,
    
    /// Speed of the shimmer animation
    #[uniform(0)]
    pub shimmer_speed: f32,
    
    /// Overall alpha transparency
    #[uniform(0)]
    pub alpha: f32,
}

impl Default for WingMaterial {
    fn default() -> Self {
        Self {
            // White/silver base color
            base_color: LinearRgba::new(0.95, 0.95, 1.0, 1.0),
            // Soft blue/white glow
            glow_color: LinearRgba::new(0.6, 0.8, 1.0, 1.0),
            // Moderate glow intensity
            glow_intensity: 0.8,
            // Time starts at 0
            time: 0.0,
            // Shimmer animation speed
            shimmer_speed: 1.0,
            // Slightly transparent for ethereal look
            alpha: 0.9,
        }
    }
}

impl Material for WingMaterial {
    fn fragment_shader() -> bevy::render::render_resource::ShaderRef {
        WING_MATERIAL_SHADER_HANDLE.into()
    }
    
    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }
    
    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: bevy::pbr::MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Enable double-sided rendering for wings
        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            // Don't write depth for transparent wings
            depth_stencil.depth_write_enabled = false;
        }
        
        Ok(())
    }
}

/// System that updates the time uniform for all wing materials
/// This enables the animated shimmer/pulse effect
fn wing_material_time_system(
    time: Res<Time>,
    mut materials: ResMut<Assets<WingMaterial>>,
    query: Query<&MeshMaterial3d<WingMaterial>>,
) {
    let elapsed = time.elapsed_secs();
    
    for handle in query.iter() {
        if let Some(material) = materials.get_mut(&handle.0) {
            material.time = elapsed;
        }
    }
}
