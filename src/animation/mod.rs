use bevy::{
    math::Vec3,
    prelude::{
        App, AssetApp, GlobalTransform, IntoScheduleConfigs, Plugin, PostUpdate, SystemSet,
        ViewVisibility,
    },
    transform::TransformSystems,
};

mod animation_state;
mod camera_animation;
mod mesh_animation;
mod skeletal_animation;
mod transform_animation;
mod zmo_asset_loader;

pub use animation_state::AnimationFrameEvent;
pub use camera_animation::CameraAnimation;
pub use mesh_animation::MeshAnimation;
pub use skeletal_animation::SkeletalAnimation;
pub use transform_animation::TransformAnimation;
pub use zmo_asset_loader::{
    ZmoAsset, ZmoAssetAnimationTexture, ZmoAssetBone, ZmoAssetLoader, ZmoTextureAssetLoader,
};

use animation_state::AnimationState;
use camera_animation::camera_animation_system;
use mesh_animation::mesh_animation_system;
use skeletal_animation::skeletal_animation_system;
use transform_animation::transform_animation_system;

/// Off-screen entities within this distance of the main camera are still animated
/// as a safety margin against visible pop-in at the frustum edge.
const ANIMATION_CULL_MARGIN: f32 = 200.0;

pub(crate) fn should_animate_entity(
    view_visibility: Option<&ViewVisibility>,
    global_transform: Option<&GlobalTransform>,
    camera_position: Option<Vec3>,
) -> bool {
    let Some(view_visibility) = view_visibility else {
        return true;
    };
    if view_visibility.get() {
        return true;
    }
    match (global_transform, camera_position) {
        (Some(transform), Some(camera_position)) => {
            transform.translation().distance_squared(camera_position)
                <= ANIMATION_CULL_MARGIN * ANIMATION_CULL_MARGIN
        }
        _ => false,
    }
}

#[derive(Default)]
pub struct RoseAnimationPlugin;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RoseAnimationSystem;

impl Plugin for RoseAnimationPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ZmoAsset>()
            .register_type::<ZmoAssetAnimationTexture>()
            .register_type::<ZmoAssetBone>()
            .init_asset_loader::<ZmoAssetLoader>()
            .init_asset_loader::<ZmoTextureAssetLoader>();

        app.add_message::<AnimationFrameEvent>();

        app.register_type::<AnimationState>()
            .register_type::<CameraAnimation>()
            .register_type::<MeshAnimation>()
            .register_type::<SkeletalAnimation>()
            .register_type::<TransformAnimation>();

        app.configure_sets(
            PostUpdate,
            RoseAnimationSystem.before(TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            (
                camera_animation_system,
                mesh_animation_system,
                skeletal_animation_system,
                transform_animation_system,
            )
                .in_set(RoseAnimationSystem),
        );
    }
}
