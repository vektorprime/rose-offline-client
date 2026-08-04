use bevy::{
    asset::LoadState,
    prelude::{
        AssetServer, Assets, Camera3d, Component, Deref, DerefMut, GlobalTransform, Handle, Query,
        Res, Transform, Vec3, ViewVisibility, With, Without,
    },
    reflect::Reflect,
    time::Time,
};

use crate::{
    animation::{should_animate_entity, AnimationState, ZmoAsset},
    render::WaterReflectionCamera,
};

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct TransformAnimation(AnimationState);

impl TransformAnimation {
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self(AnimationState::repeat(motion, limit))
    }

    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self(AnimationState::once(motion))
    }
}

pub fn transform_animation_system(
    mut query_animations: Query<(
        &mut TransformAnimation,
        Option<&mut Transform>,
        Option<&ViewVisibility>,
        Option<&GlobalTransform>,
    )>,
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<WaterReflectionCamera>)>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    let camera_position = camera_query
        .iter()
        .next()
        .map(|transform| transform.translation());

    for (mut transform_animation, transform, view_visibility, global_transform) in
        query_animations.iter_mut()
    {
        if !should_animate_entity(view_visibility, global_transform, camera_position) {
            continue;
        }

        if transform_animation.completed() {
            continue;
        }

        let zmo_handle = transform_animation.motion();
        let Some(zmo_asset) = motion_assets.get(zmo_handle) else {
            if matches!(
                asset_server.get_load_state(zmo_handle),
                Some(LoadState::Failed(_)) | None
            ) {
                // If the asset has failed to load, mark animation as completed
                transform_animation.set_completed();
            }

            continue;
        };

        let animation = &mut transform_animation.0;
        animation.advance(zmo_asset, &time);

        let Some(mut transform) = transform else {
            continue;
        };
        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();

        if let Some(translation) = zmo_asset.sample_translation(
            0,
            current_frame_fract,
            current_frame_index,
            next_frame_index,
        ) {
            transform.translation = translation;
        }

        if let Some(rotation) = zmo_asset.sample_rotation(
            0,
            current_frame_fract,
            current_frame_index,
            next_frame_index,
        ) {
            transform.rotation = rotation;
        }

        if let Some(scale) = zmo_asset.sample_scale(
            0,
            current_frame_fract,
            current_frame_index,
            next_frame_index,
        ) {
            transform.scale = Vec3::splat(scale);
        }

        // TODO: Znzin supports animation of alpha here, not sure if it is used by anything
    }
}
