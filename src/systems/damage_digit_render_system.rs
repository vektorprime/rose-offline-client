use bevy::{
    asset::RenderAssetUsages,
    math::{Vec3Swizzles, Vec4},
    mesh::PrimitiveTopology,
    pbr::MeshMaterial3d,
    prelude::{Assets, Commands, Entity, GlobalTransform, Mesh, Mesh3d, Query, ResMut},
    render::storage::ShaderStorageBuffer,
};

use crate::{
    animation::TransformAnimation, components::DamageDigits, render::DamageDigitMaterial,
    render::DamageDigitRenderData, resources::PendingDamageDigitMaterial,
};

/// System to handle entities with PendingDamageDigitMaterial component
/// Creates the actual DamageDigitMaterial with storage buffers
pub fn create_damage_digit_material_system(
    mut commands: Commands,
    mut query: Query<(Entity, &PendingDamageDigitMaterial)>,
    mut materials: ResMut<Assets<DamageDigitMaterial>>,
    mut storage_buffers: ResMut<Assets<ShaderStorageBuffer>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (entity, pending) in query.iter() {
        // Create empty storage buffers for positions, sizes, and uvs
        // These will be populated by damage_digit_render_system
        let positions_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<Vec4>::new()));
        let sizes_buffer =
            storage_buffers.add(ShaderStorageBuffer::from(Vec::<bevy::prelude::Vec2>::new()));
        let uvs_buffer = storage_buffers.add(ShaderStorageBuffer::from(Vec::<Vec4>::new()));

        let material = materials.add(DamageDigitMaterial {
            positions: positions_buffer,
            sizes: sizes_buffer,
            uvs: uvs_buffer,
            texture: pending.texture.clone(),
        });

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

        // Remove the pending marker and add the actual material and mesh
        commands
            .entity(entity)
            .remove::<PendingDamageDigitMaterial>()
            .insert(MeshMaterial3d(material))
            .insert(Mesh3d(mesh_handle));
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
    for (
        entity,
        global_transform,
        animation,
        damage_digits,
        mut damage_digit_render_data,
        material_handle,
        _mesh_handle,
    ) in query.iter_mut()
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

        let mut digit_count: usize;
        if damage_digits.damage == 0 {
            // Miss, split over 4 digits
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

        // Note: Mesh vertex count is not updated dynamically
        // The shader uses @builtin(vertex_index) to procedurally generate vertices
        // The mesh was created with enough vertices for max digits (10 * 6 = 60)
        // Only the storage buffers need to be updated with actual digit data

        // Update the storage buffers with new render data, in place.
        // Previously 3x add()+remove() per digit per frame (AssetId churn +
        // bind-group rebuilds). set_data() uploads into the existing asset.
        if let Some(material) = materials.get_mut(&material_handle.0) {
            if let Some(buf) = storage_buffers.get_mut(&material.positions) {
                buf.set_data(damage_digit_render_data.positions.clone());
            } else {
                material.positions = storage_buffers.add(ShaderStorageBuffer::from(
                    damage_digit_render_data.positions.clone(),
                ));
            }
            if let Some(buf) = storage_buffers.get_mut(&material.sizes) {
                buf.set_data(damage_digit_render_data.sizes.clone());
            } else {
                material.sizes = storage_buffers.add(ShaderStorageBuffer::from(
                    damage_digit_render_data.sizes.clone(),
                ));
            }
            if let Some(buf) = storage_buffers.get_mut(&material.uvs) {
                buf.set_data(damage_digit_render_data.uvs.clone());
            } else {
                material.uvs = storage_buffers.add(ShaderStorageBuffer::from(
                    damage_digit_render_data.uvs.clone(),
                ));
            }
        } else {
            log::warn!(
                "[DAMAGE_DIGIT_RENDER] Could NOT find material for entity {:?} with handle {:?}",
                entity,
                material_handle.0
            );
        }
    }
}
