use bevy::{
    asset::RenderAssetUsages,
    math::{Vec3Swizzles, Vec4},
    pbr::MeshMaterial3d,
    prelude::{Commands, Entity, GlobalTransform, Query, ResMut, Assets, Mesh, Mesh3d},
    render::storage::ShaderStorageBuffer,
    mesh::PrimitiveTopology,
};

use crate::{
    animation::TransformAnimation, 
    components::DamageDigits, 
    render::DamageDigitRenderData,
    resources::PendingDamageDigitMaterial,
    render::DamageDigitMaterial,
};

/// System to handle entities with PendingDamageDigitMaterial component
/// Creates the actual DamageDigitMaterial with storage buffers
pub fn create_damage_digit_material_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &PendingDamageDigitMaterial,
    )>,
    mut materials: ResMut<Assets<DamageDigitMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let entity_count = query.iter().count();
    log::info!("[CREATE_DAMAGE_DIGIT_MATERIAL] Found {} entities with PendingDamageDigitMaterial", entity_count);
    
    for (entity, pending) in query.iter() {
        log::info!("[CREATE_DAMAGE_DIGIT_MATERIAL] Processing entity {:?} with texture {:?}", entity, pending.texture);
        
        // Create empty storage buffers for positions, sizes, and uvs
        // These will be populated by damage_digit_render_system
        let positions_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<Vec4>::new()));
        let sizes_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<bevy::prelude::Vec2>::new()));
        let uvs_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<Vec4>::new()));
        log::info!("[CREATE_DAMAGE_DIGIT_MATERIAL] Created storage buffers for entity {:?}: positions={:?}, sizes={:?}, uvs={:?}", entity, positions_buffer, sizes_buffer, uvs_buffer);
        
        let material = materials.add(DamageDigitMaterial {
            positions: positions_buffer,
            sizes: sizes_buffer,
            uvs: uvs_buffer,
            texture: pending.texture.clone(),
        });
        log::info!("[CREATE_DAMAGE_DIGIT_MATERIAL] Created DamageDigitMaterial for entity {:?} with handle {:?}", entity, material);
        
        // Create a unique mesh for this entity with enough vertices for max digits
        // Max 10 digits * 6 vertices per quad = 60 vertices
        // Use MAIN_WORLD | RENDER_WORLD to allow access from both worlds
        // CRITICAL: Mesh needs actual vertex positions for Bevy's render pipeline to work
        // The shader uses @builtin(vertex_index) to procedurally generate vertices,
        // but the mesh still needs vertex data defined
        let max_digit_count = 10;
        let vertex_count = max_digit_count * 6;
        let vertex_positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; vertex_count];
        let mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertex_positions);
        let mesh_handle = meshes.add(mesh);
        log::info!("[CREATE_DAMAGE_DIGIT_MATERIAL] Created mesh with {} vertices for entity {:?}", vertex_count, entity);
        
        // Remove the pending marker and add the actual material and mesh
        commands.entity(entity)
            .remove::<PendingDamageDigitMaterial>()
            .insert(MeshMaterial3d(material))
            .insert(Mesh3d(mesh_handle));
        log::info!("[CREATE_DAMAGE_DIGIT_MATERIAL] Replaced PendingDamageDigitMaterial with MeshMaterial3d and Mesh3d for entity {:?}", entity);
    }
}

pub fn damage_digit_render_system(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &GlobalTransform,
        &TransformAnimation,
        &DamageDigits,
        &mut DamageDigitRenderData,
        &MeshMaterial3d<DamageDigitMaterial>,
        Option<&Mesh3d>,
    )>,
    mut materials: ResMut<Assets<DamageDigitMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let entity_count = query.iter().count();
    log::info!("[DAMAGE_DIGIT_RENDER] Processing {} damage digit entities", entity_count);
    
    for (entity, global_transform, animation, damage_digits, mut damage_digit_render_data, material_handle, mesh_handle) in
        query.iter_mut()
    {
        log::info!("[DAMAGE_DIGIT_RENDER] Processing entity {:?} with damage={}", entity, damage_digits.damage);
        
        let damage_digit_render_data: &mut DamageDigitRenderData = &mut damage_digit_render_data;
        damage_digit_render_data.clear();
        log::info!("[DAMAGE_DIGIT_RENDER] Cleared render data for entity {:?}", entity);

        let animation: &TransformAnimation = animation;
        if animation.completed() {
            // Animation completed, despawn
            log::info!("[DAMAGE_DIGIT_RENDER] Animation completed for entity {:?}, despawning", entity);
            commands.entity(entity).despawn();
            continue;
        }
        log::info!("[DAMAGE_DIGIT_RENDER] Animation not completed for entity {:?}", entity);

        let global_transform: &GlobalTransform = global_transform;
        let (scale, _, translation) = global_transform.to_scale_rotation_translation();
        log::info!("[DAMAGE_DIGIT_RENDER] Transform: scale={:?}, translation={:?}", scale, translation);
        
        let mut digit_count: usize;
        if damage_digits.damage == 0 {
            // Miss, split over 4 digits
            log::info!("[DAMAGE_DIGIT_RENDER] Damage is 0 (miss), adding 4 digit sprites");
            digit_count = 4;
            for digit in 0..4 {
                damage_digit_render_data.add(
                    translation,
                    -1.5 + digit as f32,
                    0.4 * scale.xy(),
                    Vec4::new(digit as f32 / 4.0, 0.0, (digit + 1) as f32 / 4.0, 1.0),
                );
            }
        } else {
            // First count the number of digits
            let mut damage = damage_digits.damage;
            digit_count = 0;
            while damage > 0 {
                digit_count += 1;
                damage /= 10;
            }
            log::info!("[DAMAGE_DIGIT_RENDER] Damage {} has {} digits", damage_digits.damage, digit_count);

            // Add digits to render data
            let number_offset = (digit_count - 1) as f32 / 2.0;
            let mut digit_offset = 0.0;
            let mut damage = damage_digits.damage;
            while damage > 0 {
                let digit = damage % 10;
                log::info!("[DAMAGE_DIGIT_RENDER] Adding digit {} at offset {}", digit, number_offset - digit_offset);
                damage_digit_render_data.add(
                    translation,
                    number_offset - digit_offset,
                    0.4 * scale.xy(),
                    Vec4::new(digit as f32 / 10.0, 0.0, (digit + 1) as f32 / 10.0, 1.0),
                );
                digit_offset += 1.0;
                damage /= 10;
            }
        }
        
        log::info!("[DAMAGE_DIGIT_RENDER] Render data has {} positions, {} sizes, {} UVs",
            damage_digit_render_data.positions.len(),
            damage_digit_render_data.sizes.len(),
            damage_digit_render_data.uvs.len());

        // Note: Mesh vertex count is not updated dynamically
        // The shader uses @builtin(vertex_index) to procedurally generate vertices
        // The mesh was created with enough vertices for max digits (10 * 6 = 60)
        // Only the storage buffers need to be updated with actual digit data
        
        // Update the storage buffers with new render data
        if let Some(material) = materials.get_mut(&material_handle.0) {
            log::info!("[DAMAGE_DIGIT_RENDER] Found material for entity {:?}, updating storage buffers", entity);
            // Store old buffer handles to prevent memory leak
            let old_positions = material.positions.clone();
            let old_sizes = material.sizes.clone();
            let old_uvs = material.uvs.clone();
            
            // Create new storage buffers with updated data
            let positions_buffer = storage_buffers.add(ShaderStorageBuffer::from(damage_digit_render_data.positions.clone()));
            let sizes_buffer = storage_buffers.add(ShaderStorageBuffer::from(damage_digit_render_data.sizes.clone()));
            let uvs_buffer = storage_buffers.add(ShaderStorageBuffer::from(damage_digit_render_data.uvs.clone()));
            log::info!("[DAMAGE_DIGIT_RENDER] Created new storage buffers: positions={:?}, sizes={:?}, uvs={:?}", positions_buffer, sizes_buffer, uvs_buffer);
            
            // Update material with new buffer handles
            material.positions = positions_buffer;
            material.sizes = sizes_buffer;
            material.uvs = uvs_buffer;
            
            // Remove old buffers to prevent memory leak
            storage_buffers.remove(&old_positions);
            storage_buffers.remove(&old_sizes);
            storage_buffers.remove(&old_uvs);
            log::info!("[DAMAGE_DIGIT_RENDER] Updated material and removed old buffers for entity {:?}", entity);
        } else {
            log::warn!("[DAMAGE_DIGIT_RENDER] Could NOT find material for entity {:?} with handle {:?}", entity, material_handle.0);
        }
    }
}
