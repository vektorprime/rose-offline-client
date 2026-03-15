use bevy::{
    asset::LoadState,
    pbr::{ExtendedMaterial, MeshMaterial3d},
    prelude::{AssetServer, Assets, Component, Deref, DerefMut, Entity, Handle, Query, Res, ResMut, With},
    reflect::Reflect,
    time::Time,
};

use crate::{
    animation::{AnimationState, ZmoAsset},
    components::EffectMesh,
    render::{RoseEffectExtension, EffectMeshAnimationUniform, EFFECT_MESH_ANIMATION_FLAG_POSITION, EFFECT_MESH_ANIMATION_FLAG_NORMAL, EFFECT_MESH_ANIMATION_FLAG_UV, EFFECT_MESH_ANIMATION_FLAG_ALPHA},
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
    mut query: Query<(
        &mut MeshAnimation,
        Entity,
        Option<&MeshMaterial3d<ExtendedMaterial<bevy::pbr::StandardMaterial, RoseEffectExtension>>>
    ), With<EffectMesh>>,
    mut effect_mesh_materials: ResMut<Assets<ExtendedMaterial<bevy::pbr::StandardMaterial, RoseEffectExtension>>>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (mut mesh_animation, entity, material_component) in query.iter_mut() {
        if mesh_animation.completed() {
            continue;
        }

        let zmo_handle = mesh_animation.motion().clone();
        let Some(zmo_asset) = motion_assets.get(&zmo_handle) else {
            if matches!(
                asset_server.get_load_state(&zmo_handle),
                Some(LoadState::Failed(_))
            ) {
                // If asset has failed to load, mark the animation as completed
                mesh_animation.set_completed();
            }
            continue;
        };

        // Advance the animation state
        let anim_state = &mut mesh_animation.0;
        anim_state.advance(zmo_asset, &time);
        
        // If this entity has an effect mesh material, update its animation uniform
        if let Some(material_handle) = material_component.map(|m| m.0.clone()) {
            if let Some(material) = effect_mesh_materials.get_mut(&material_handle) {
                // Only update if there's an animation texture present
                if material.extension.animation_texture.is_some() {
                    update_effect_mesh_animation_material(&mut material.extension.animation_state, zmo_asset, anim_state);
                }
            }
        }
    }
}

/// Update the EffectMeshAnimationUniform based on ZmoAsset animation data and current AnimationState
fn update_effect_mesh_animation_material(
    uniform: &mut EffectMeshAnimationUniform,
    zmo_asset: &ZmoAsset,
    anim_state: &AnimationState,
) {
    // Build flags: bits 0-3 = animation type flags, bits 4-31 = num_frames
    let mut flags: u32 = 0;
    if let Some(texture_data) = &zmo_asset.animation_texture {
        if texture_data.has_position_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_POSITION;
        }
        if texture_data.has_normal_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_NORMAL;
        }
        if texture_data.has_uv1_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_UV;
        }
        if texture_data.has_alpha_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_ALPHA;
        }
        // Get alpha value for current frame if available
        let current_frame = anim_state.current_frame_index();
        if let Some(alpha) = texture_data.alphas.get(current_frame).copied() {
            uniform.alpha = alpha;
        }
    }
    // Pack num_frames into upper bits (bits 4-31)
    flags |= (zmo_asset.num_frames as u32) << 4;
    
    uniform.flags = flags;
    
    // Pack current and next frame indices: lower 16 bits = current, upper 16 bits = next
    let current_frame = anim_state.current_frame_index() as u32 & 0xFFFF;
    let next_frame = anim_state.next_frame_index() as u32 & 0xFFFF;
    uniform.current_next_frame = current_frame | (next_frame << 16);
    
    // Set interpolation weight between frames
    uniform.next_weight = anim_state.current_frame_fract();
}
