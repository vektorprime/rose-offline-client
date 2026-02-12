use bevy::prelude::*;
use crate::render::ParticleRenderData;
use crate::render::ParticleRenderBillboardType;

/// Test function to spawn a simple particle effect
pub fn test_particle_spawn(mut commands: Commands) {
    let particle_count = 100;
    let mut render_data = ParticleRenderData::new(
        particle_count,
        0, // blend_op
        0, // src_blend_factor
        0, // dst_blend_factor
        ParticleRenderBillboardType::Full, // Billboard type
    );
    
    // Add particles in a circle
    for i in 0..particle_count {
        let angle = (i as f32 / particle_count as f32) * std::f32::consts::TAU;
        let radius = 5.0;
        
        render_data.add(
            Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius), // position
            0.0,                                                          // rotation
            Vec2::new(0.2, 0.2),                                         // size
            Vec4::new(1.0, 0.5, 0.0, 1.0),                              // color (orange)
            Vec4::new(0.0, 0.0, 1.0, 1.0),                              // texture UV (full)
        );
    }
    
    // Spawn entity with particle data
    commands.spawn((
        render_data,
        Transform::default(),
        Visibility::default(),
    ));
    
    info!("✓ Spawned test particle effect with {} particles", particle_count);
}
