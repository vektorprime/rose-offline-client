use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::LoadState,
    prelude::{
        AssetServer, Assets, Camera3d, Component, Deref, DerefMut, Entity, GlobalTransform,
        Handle, MessageWriter, Query, Res, Transform, ViewVisibility, With, Without,
    },
    reflect::Reflect,
    time::Time,
};
use bevy_mesh::skinning::SkinnedMesh;

use crate::{
    animation::{should_animate_entity, AnimationFrameEvent, AnimationState, ZmoAsset},
    render::WaterReflectionCamera,
    resources::GameData,
};

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct SkeletalAnimation(AnimationState);

impl SkeletalAnimation {
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self(AnimationState::repeat(motion, limit))
    }

    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self(AnimationState::once(motion))
    }

    pub fn with_animation_speed(mut self, animation_speed: f32) -> Self {
        self.0.set_animation_speed(animation_speed);
        self
    }
}

pub fn skeletal_animation_system(
    mut query_animations: Query<(
        Entity,
        &mut SkeletalAnimation,
        Option<&SkinnedMesh>,
        Option<&ViewVisibility>,
        Option<&GlobalTransform>,
    )>,
    mut query_transform: Query<&mut Transform>,
    camera_query: Query<&GlobalTransform, (With<Camera3d>, Without<WaterReflectionCamera>)>,
    mut animation_frame_events: MessageWriter<AnimationFrameEvent>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    let camera_position = camera_query
        .iter()
        .next()
        .map(|transform| transform.translation());

    for (
        entity,
        mut skeletal_animation,
        skinned_mesh,
        view_visibility,
        global_transform,
    ) in query_animations.iter_mut()
    {
        if !should_animate_entity(view_visibility, global_transform, camera_position) {
            continue;
        }

        if skeletal_animation.completed() {
            continue;
        }

        let zmo_handle = skeletal_animation.motion();
        let zmo_asset = if let Some(zmo_asset) = motion_assets.get(zmo_handle) {
            zmo_asset
        } else {
            if matches!(
                asset_server.get_load_state(zmo_handle),
                Some(LoadState::Failed(_))
            ) {
                // If the asset has failed to load, mark the animation as completed
                skeletal_animation.set_completed();
            }

            continue;
        };

        let animation = &mut skeletal_animation.0;
        animation.advance(zmo_asset, &time);

        animation.iter_animation_events(zmo_asset, |event_id| {
            if let Some(flags) = game_data.animation_event_flags.get(event_id as usize) {
                if !flags.is_empty() {
                    animation_frame_events.write(AnimationFrameEvent::new(entity, *flags));
                }
            }
        });

        let Some(skinned_mesh) = skinned_mesh else {
            continue;
        };

        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();
        let interpolate_weight = animation
            .interpolate_weight()
            .map(|w| (w * FRAC_PI_2).sin());

        for (bone_id, bone_entity) in skinned_mesh.joints.iter().enumerate() {
            let Ok(mut bone_transform) = query_transform.get_mut(*bone_entity) else {
                continue;
            };

            if let Some(translation) = zmo_asset.sample_translation(
                bone_id,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                let translation = if let Some(blend_weight) = interpolate_weight {
                    bone_transform.translation.lerp(translation, blend_weight)
                } else {
                    translation
                };
                if !bone_transform.translation.abs_diff_eq(translation, 1e-5) {
                    bone_transform.translation = translation;
                }
            }

            if let Some(rotation) = zmo_asset.sample_rotation(
                bone_id,
                current_frame_fract,
                current_frame_index,
                next_frame_index,
            ) {
                let rotation = if let Some(blend_weight) = interpolate_weight {
                    bone_transform.rotation.slerp(rotation, blend_weight)
                } else {
                    rotation
                };
                if !bone_transform.rotation.abs_diff_eq(rotation, 1e-4) {
                    bone_transform.rotation = rotation;
                }
            }
        }
    }
}
