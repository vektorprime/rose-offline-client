use bevy::{
    asset::LoadState,
    prelude::{AssetServer, Assets, Component, Deref, DerefMut, Handle, Query, Res},
    reflect::Reflect,
    time::Time,
};

use crate::{
    animation::{AnimationState, ZmoAsset},
    render::{RoseEffectExtension},
};

#[derive(Component, Reflect, Deref, DerefMut)]
pub struct MeshAnimation(AnimationState);

impl MeshAnimation {
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self {
        Self(AnimationState::repeat(motion, limit))
    }

    pub fn once(motion: Handle<ZmoAsset>) -> Self {
        Self(AnimationState::once(motion))
    }

    pub fn with_start_delay(mut self, start_delay: f32) -> Self {
        self.set_start_delay(start_delay);
        self
    }
}

pub fn mesh_animation_system(
    mut query_animations: Query<&mut MeshAnimation>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for mut mesh_animation in query_animations.iter_mut() {
        let mesh_animation: &mut MeshAnimation = &mut mesh_animation;
        if mesh_animation.completed() {
            continue;
        }

        let zmo_handle = mesh_animation.motion();
        let Some(zmo_asset) = motion_assets.get(zmo_handle) else {
            if matches!(
                asset_server.get_load_state(zmo_handle),
                Some(LoadState::Failed(_))
            ) {
                // If asset has failed to load, mark the animation as completed
                mesh_animation.set_completed();
            }

            continue;
        };

        let animation = &mut mesh_animation.0;
        animation.advance(zmo_asset, &time);

        // TODO: Effect mesh animation rendering state removed with old material system
        // This functionality needs to be reimplemented with new ExtendedMaterial pattern
        // The EffectMeshAnimationRenderState component was used to pass animation data to shaders
        // through the RoseEffectExtension material extension
    }
}
