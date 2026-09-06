use bevy::prelude::{
    Camera3d, Commands, Entity, GlobalTransform, Query, Res, Time, Transform, With, Without,
};

use crate::{components::DamageNumber, render::WaterReflectionCamera};

/// Face every damage number at the main camera.
///
/// Only the parent is rotated; digit quads keep their local X offsets, so the
/// whole number turns as one rigid card. The reflection camera is excluded so
/// digits don't snap to the mirrored view.
pub fn damage_number_billboard_system(
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<WaterReflectionCamera>)>,
    mut query: Query<&mut Transform, With<DamageNumber>>,
) {
    let Ok(camera_transform) = camera_query.single() else {
        return;
    };
    let camera_rotation = camera_transform.rotation();
    for mut transform in query.iter_mut() {
        transform.rotation = camera_rotation;
    }
}

/// Float damage numbers up and despawn them.
///
/// Matches the old ZMO-driven lifetime (rise, then disappear — the old motion
/// never faded alpha, and `transform_animation_system` doesn't implement alpha
/// either). Children despawn with the parent via hierarchy cascade.
pub fn damage_number_animate_system(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut DamageNumber, &mut Transform)>,
) {
    let delta = time.delta_secs();
    for (entity, mut number, mut transform) in query.iter_mut() {
        number.remaining -= delta;
        if number.remaining <= 0.0 {
            commands.entity(entity).despawn();
            continue;
        }

        transform.translation.y += number.rise_speed * delta;
    }
}
