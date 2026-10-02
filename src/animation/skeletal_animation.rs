use std::f32::consts::FRAC_PI_2;

use bevy::{
    asset::LoadState,
    prelude::{
        AssetServer, Assets, Component, Deref, DerefMut, Entity, Handle, Local, MessageWriter,
        Quat, Query, Res, Transform, Vec3,
    },
    reflect::Reflect,
    tasks::{ComputeTaskPool, ParallelSliceMut},
    time::Time,
};
use bevy_mesh::skinning::SkinnedMesh;

use crate::{
    animation::{AnimationFrameEvent, AnimationState, ZmoAsset},
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

/// Sampled (translation, rotation) of one joint for the current frame.
type BoneSample = (Option<Vec3>, Option<Quat>);

/// Bone sampling work for one animated skinned entity, gathered by the serial pass.
struct BoneSampleJob<'a> {
    joints: &'a [Entity],
    zmo_asset: &'a ZmoAsset,
    current_frame_fract: f32,
    current_frame_index: usize,
    next_frame_index: usize,
    interpolate_weight: Option<f32>,
    /// Index of this job's first joint in the shared sample buffer.
    sample_offset: usize,
}

/// Below this many joints in total the keyframe sampling stays on this thread
/// (task dispatch would cost more than it saves). Results are identical either way.
const PARALLEL_SAMPLING_MIN_BONES: usize = 256;

/// Samples every joint of `job` into `samples` (one entry per joint). Pure: reads
/// only the motion asset, so jobs can run in any order or in parallel.
fn sample_bones(job: &BoneSampleJob, samples: &mut [BoneSample]) {
    for (bone_id, sample) in samples.iter_mut().enumerate() {
        *sample = (
            job.zmo_asset.sample_translation(
                bone_id,
                job.current_frame_fract,
                job.current_frame_index,
                job.next_frame_index,
            ),
            job.zmo_asset.sample_rotation(
                bone_id,
                job.current_frame_fract,
                job.current_frame_index,
                job.next_frame_index,
            ),
        );
    }
}

/// Advances skeletal animations and poses their joints.
///
/// Three passes: (1) serial animation advance + frame events, in query order as
/// before; (2) keyframe sampling, which is pure and runs in parallel; (3) serial
/// writes to the joint transforms in the original entity/joint order, so joints
/// shared between skeletons and the blend (which reads the current transform)
/// behave exactly as with the old single loop.
pub fn skeletal_animation_system(
    mut query_animations: Query<(Entity, &mut SkeletalAnimation, Option<&SkinnedMesh>)>,
    mut query_transform: Query<&mut Transform>,
    mut animation_frame_events: MessageWriter<AnimationFrameEvent>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    time: Res<Time>,
    mut bone_samples: Local<Vec<BoneSample>>,
) {
    let mut jobs: Vec<BoneSampleJob> = Vec::new();
    let mut total_bones = 0;

    for (entity, mut skeletal_animation, skinned_mesh) in query_animations.iter_mut() {
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

        jobs.push(BoneSampleJob {
            joints: &skinned_mesh.joints,
            zmo_asset,
            current_frame_fract: animation.current_frame_fract(),
            current_frame_index: animation.current_frame_index(),
            next_frame_index: animation.next_frame_index(),
            interpolate_weight: animation
                .interpolate_weight()
                .map(|w| (w * FRAC_PI_2).sin()),
            sample_offset: total_bones,
        });
        total_bones += skinned_mesh.joints.len();
    }

    if jobs.is_empty() {
        return;
    }

    let samples = &mut *bone_samples;
    samples.clear();
    samples.resize(total_bones, (None, None));

    if total_bones >= PARALLEL_SAMPLING_MIN_BONES {
        // Give every job its own disjoint slice of the sample buffer.
        let mut work: Vec<(&BoneSampleJob, &mut [BoneSample])> = Vec::with_capacity(jobs.len());
        let mut remaining: &mut [BoneSample] = samples.as_mut_slice();
        for job in &jobs {
            let (job_samples, rest) = std::mem::take(&mut remaining).split_at_mut(job.joints.len());
            work.push((job, job_samples));
            remaining = rest;
        }
        work.par_splat_map_mut(ComputeTaskPool::get(), None, |_, chunk| {
            for (job, job_samples) in chunk.iter_mut() {
                sample_bones(job, job_samples);
            }
        });
    } else {
        for job in &jobs {
            let end = job.sample_offset + job.joints.len();
            sample_bones(job, &mut samples[job.sample_offset..end]);
        }
    }

    for job in &jobs {
        let end = job.sample_offset + job.joints.len();
        for (bone_entity, &(translation, rotation)) in
            job.joints.iter().zip(&samples[job.sample_offset..end])
        {
            let Ok(mut bone_transform) = query_transform.get_mut(*bone_entity) else {
                continue;
            };

            if let Some(translation) = translation {
                if let Some(blend_weight) = job.interpolate_weight {
                    bone_transform.translation =
                        bone_transform.translation.lerp(translation, blend_weight);
                } else {
                    bone_transform.translation = translation;
                }
            }

            if let Some(rotation) = rotation {
                if let Some(blend_weight) = job.interpolate_weight {
                    bone_transform.rotation = bone_transform.rotation.slerp(rotation, blend_weight);
                } else {
                    bone_transform.rotation = rotation;
                }
            }
        }
    }
}
