use bevy::prelude::*;
use bevy::diagnostic::FrameTimeDiagnosticsPlugin;
use crate::render::ParticleMaterial;
use crate::render::ParticleRenderData;

/// Debug system to verify particle rendering components
pub fn debug_particle_rendering(
    query: Query<(
        Entity, 
        &ParticleRenderData, 
        Option<&Mesh3d>, 
        Option<&MeshMaterial3d<ParticleMaterial>>,
        &Transform,
    )>,
    materials: Res<Assets<ParticleMaterial>>,
    storage_buffers: Res<Assets<bevy::render::storage::ShaderStorageBuffer>>,
) {
    for (entity, data, mesh, material, transform) in query.iter() {
        info!("=== Particle Entity {:?} ===", entity);
        info!("  Particles: {}", data.positions.len());
        info!("  Has Mesh3d: {}", mesh.is_some());
        info!("  Has Material: {}", material.is_some());
        info!("  Position: {:?}", transform.translation);
        
        if let Some(mat_handle) = material {
            if let Some(mat) = materials.get(&mat_handle.0) {
                let pos_ok = storage_buffers.contains(&mat.positions);
                let size_ok = storage_buffers.contains(&mat.sizes);
                let color_ok = storage_buffers.contains(&mat.colors);
                let tex_ok = storage_buffers.contains(&mat.textures);
                
                info!("  Buffers: pos={}, size={}, color={}, tex={}", 
                    pos_ok, size_ok, color_ok, tex_ok);
            }
        }
    }
}

/// Performance monitoring system for particles
pub fn particle_performance_monitor(
    query: Query<&ParticleRenderData>,
    materials: Res<Assets<ParticleMaterial>>,
    diagnostics: Res<bevy::diagnostic::DiagnosticsStore>,
    mut last_log: Local<f32>,
    time: Res<Time>,
) {
    *last_log += time.delta_secs();
    
    // Log every 5 seconds
    if *last_log < 5.0 {
        return;
    }
    *last_log = 0.0;
    
    // Count total particles
    let mut total_particles = 0;
    let mut particle_systems = 0;
    
    for data in query.iter() {
        total_particles += data.positions.len();
        particle_systems += 1;
    }
    
    // Get FPS
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d: &bevy::diagnostic::Diagnostic| d.smoothed());
    
    info!("[Particle Stats] Systems: {}, Total particles: {}", 
        particle_systems, total_particles);
    
    if let Some(fps) = fps {
        info!("[Particle Stats] FPS: {:.1}", fps);
        
        if fps < 30.0 && total_particles > 5000 {
            warn!("⚠ Low FPS with {} particles - consider reducing count", total_particles);
        }
    }
}
