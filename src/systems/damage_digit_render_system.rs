use bevy::{
    math::{Vec3Swizzles, Vec4},
    pbr::MeshMaterial3d,
    prelude::{Commands, Entity, GlobalTransform, Query, ResMut, Assets, Handle},
    render::storage::ShaderStorageBuffer,
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
) {
    for (entity, pending) in query.iter() {
        // Create empty storage buffers for positions, sizes, and uvs
        // These will be populated by damage_digit_render_system
        let positions_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<Vec4>::new()));
        let sizes_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<bevy::prelude::Vec2>::new()));
        let uvs_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<Vec4>::new()));
        
        let material = materials.add(DamageDigitMaterial {
            positions: positions_buffer,
            sizes: sizes_buffer,
            uvs: uvs_buffer,
            texture: pending.texture.clone_weak(),
        });
        
        // Remove the pending marker and add the actual material
        commands.entity(entity)
            .remove::<PendingDamageDigitMaterial>()
            .insert(MeshMaterial3d(material));
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
    )>,
    mut materials: ResMut<Assets<DamageDigitMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
) {
    for (entity, global_transform, animation, damage_digits, mut damage_digit_render_data, material_handle) in
        query.iter_mut()
    {
        let damage_digit_render_data: &mut DamageDigitRenderData = &mut damage_digit_render_data;
        damage_digit_render_data.clear();

        let animation: &TransformAnimation = animation;
        if animation.completed() {
            // Animation completed, despawn
            commands.entity(entity).despawn();
            continue;
        }

        let global_transform: &GlobalTransform = global_transform;
        let (scale, _, translation) = global_transform.to_scale_rotation_translation();
        
        if damage_digits.damage == 0 {
            // Miss, split over 4 digits
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
            let mut digit_count = 0;
            while damage > 0 {
                digit_count += 1;
                damage /= 10;
            }

            // Add digits to render data
            let number_offset = (digit_count - 1) as f32 / 2.0;
            let mut digit_offset = 0.0;
            let mut damage = damage_digits.damage;
            while damage > 0 {
                let digit = damage % 10;
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
        
        // Update the storage buffers with new render data
        if let Some(material) = materials.get_mut(&material_handle.0) {
            // Create new storage buffers with updated data
            let positions_buffer = storage_buffers.add(ShaderStorageBuffer::from(damage_digit_render_data.positions.clone()));
            let sizes_buffer = storage_buffers.add(ShaderStorageBuffer::from(damage_digit_render_data.sizes.clone()));
            let uvs_buffer = storage_buffers.add(ShaderStorageBuffer::from(damage_digit_render_data.uvs.clone()));
            
            // Update material with new buffer handles
            material.positions = positions_buffer;
            material.sizes = sizes_buffer;
            material.uvs = uvs_buffer;
        }
    }
}
