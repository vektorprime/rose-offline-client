//! Culling bounds for meshes whose vertices are not where their `Mesh` says.
//!
//! Bevy's `calculate_bounds` builds an `Aabb` from the mesh's vertex positions.
//! Two kinds of ROSE meshes draw somewhere else:
//! - GPU particles: the mesh is a placeholder of zero positions, and the particle
//!   shader builds world-space quads from the material's storage buffers.
//! - Morph-animated meshes (zone animated objects, effect meshes): the ZMO vertex
//!   animation replaces the vertex positions in the shader.
//!
//! These entities carry `NoAutoAabb` and get their `Aabb` here. The `Aabb` matters
//! twice: CPU and GPU frustum culling, and GPU occlusion culling on the main camera,
//! which tests every mesh's `Aabb` against the depth pyramid. Occlusion culling also
//! runs for `NoFrustumCulling` meshes, and without an `Aabb` Bevy uses an infinite
//! box, whose projected corners are NaN, so whether the mesh survives the test
//! depends on the GPU driver.

use bevy::{
    asset::{AssetId, Assets},
    camera::primitives::{Aabb, MeshAabb},
    camera::visibility::NoAutoAabb,
    math::{Vec3, Vec3A},
    prelude::{Commands, Component, Entity, GlobalTransform, Mesh, Mesh3d, Query, Res, With},
};

use crate::{
    animation::{MeshAnimation, ZmoAsset},
    render::ParticleRenderData,
};

/// Relative slack added to a rebuilt particle box, so a drifting particle cloud does
/// not rewrite its `Aabb` (and re-extract the mesh for rendering) every frame.
const PARTICLE_AABB_SLACK: f32 = 0.25;
/// Minimum slack (local units) added to a rebuilt particle box.
const PARTICLE_AABB_MIN_SLACK: f32 = 0.25;
/// Below this |determinant| the particle transform cannot be inverted reliably
/// (an emitter scaled to ~0 by its ZMO scale channel); the last box is kept.
const PARTICLE_MIN_DETERMINANT: f32 = 1.0e-9;

/// Keeps each particle sequence's `Aabb` around its live particles.
///
/// `particle_sequence_system` stores world-space positions and the shader ignores
/// the entity transform, so the box is built in world space and moved into the
/// entity's local space with the transform propagated this frame, which is the one
/// culling applies to it. A sequence without live particles keeps a point box at its
/// emitter, so it is still culled with it.
///
/// Runs in `VisibilitySystems::CalculateBounds` (after transform propagation, before
/// the visibility checks).
pub fn update_particle_aabb_system(
    mut query: Query<(&ParticleRenderData, &GlobalTransform, &mut Aabb)>,
) {
    query
        .par_iter_mut()
        .for_each(|(render_data, global_transform, mut aabb)| {
            let Some(target) = particle_local_aabb(render_data, global_transform) else {
                return;
            };

            if aabb_contains(&aabb, &target) && !aabb_much_larger(&aabb, &target) {
                return;
            }

            *aabb = Aabb {
                center: target.center,
                half_extents: target.half_extents * (1.0 + PARTICLE_AABB_SLACK)
                    + Vec3A::splat(PARTICLE_AABB_MIN_SLACK),
            };
        });
}

/// Local-space box around the live particles of one sequence, or `None` when the
/// entity transform is degenerate.
fn particle_local_aabb(
    render_data: &ParticleRenderData,
    global_transform: &GlobalTransform,
) -> Option<Aabb> {
    let world_from_local = global_transform.affine();
    if world_from_local.matrix3.determinant().abs() < PARTICLE_MIN_DETERMINANT {
        return None;
    }

    let mut min = Vec3A::splat(f32::MAX);
    let mut max = Vec3A::splat(f32::MIN);
    for (position, size) in render_data.positions.iter().zip(render_data.sizes.iter()) {
        let center = Vec3A::from(position.truncate());
        // A quad corner lies at most |size| from the particle center, whatever its
        // billboard orientation and rotation.
        let reach = Vec3A::splat(size.length());
        min = min.min(center - reach);
        max = max.max(center + reach);
    }

    if min.cmpgt(max).any() {
        return Some(Aabb::default());
    }

    let world_center = 0.5 * (min + max);
    let world_half_extents = 0.5 * (max - min);

    // Smallest local box enclosing the world box (Arvo's method).
    let local_from_world = world_from_local.inverse();
    let m = local_from_world.matrix3;
    let center = local_from_world.transform_point3a(world_center);
    let half_extents = m.x_axis.abs() * world_half_extents.x
        + m.y_axis.abs() * world_half_extents.y
        + m.z_axis.abs() * world_half_extents.z;

    if !center.is_finite() || !half_extents.is_finite() {
        return None;
    }

    Some(Aabb {
        center,
        half_extents,
    })
}

fn aabb_contains(outer: &Aabb, inner: &Aabb) -> bool {
    outer.min().cmple(inner.min()).all() && outer.max().cmpge(inner.max()).all()
}

/// True when `current` is more than twice (plus 1 unit) as large as needed on any
/// axis, so a burst that has died down does not keep its large box.
fn aabb_much_larger(current: &Aabb, needed: &Aabb) -> bool {
    current
        .half_extents
        .cmpgt(needed.half_extents * 2.0 + Vec3A::ONE)
        .any()
}

/// The assets a morph-animated mesh's `Aabb` was built from, so it is rebuilt only
/// when the mesh or the motion changes, or when the motion finishes loading.
#[derive(Component)]
pub struct MeshAnimationBounds {
    mesh: AssetId<Mesh>,
    motion: AssetId<ZmoAsset>,
    includes_motion: bool,
}

/// Gives morph-animated meshes (`MeshAnimation` + `NoAutoAabb`) an `Aabb` enclosing
/// the mesh and every frame of its ZMO vertex animation
/// (`ZmoAssetAnimationTexture::position_bounds`), so they can be frustum and
/// occlusion culled without popping.
///
/// Runs in `VisibilitySystems::CalculateBounds`; the inserted `Aabb` is applied
/// before the visibility checks of the same frame.
pub fn update_mesh_animation_aabb_system(
    mut commands: Commands,
    meshes: Res<Assets<Mesh>>,
    motions: Res<Assets<ZmoAsset>>,
    query: Query<
        (
            Entity,
            &Mesh3d,
            &MeshAnimation,
            Option<&MeshAnimationBounds>,
        ),
        With<NoAutoAabb>,
    >,
) {
    for (entity, mesh3d, mesh_animation, bounds) in query.iter() {
        let mesh_id = mesh3d.0.id();
        let motion_id = mesh_animation.motion().id();
        let motion = motions.get(motion_id);

        if let Some(bounds) = bounds {
            if bounds.mesh == mesh_id
                && bounds.motion == motion_id
                && (bounds.includes_motion || motion.is_none())
            {
                continue;
            }
        }

        // Until the mesh is loaded there is nothing to draw (and nothing to bound).
        let Some(mesh_aabb) = meshes.get(mesh_id).and_then(|mesh| mesh.compute_aabb()) else {
            continue;
        };

        let aabb = match motion
            .and_then(|motion| motion.animation_texture.as_ref())
            .and_then(|texture| texture.position_bounds)
        {
            Some(morph_aabb) => Aabb::from_min_max(
                Vec3::from(mesh_aabb.min().min(morph_aabb.min())),
                Vec3::from(mesh_aabb.max().max(morph_aabb.max())),
            ),
            None => mesh_aabb,
        };

        commands.entity(entity).try_insert((
            aabb,
            MeshAnimationBounds {
                mesh: mesh_id,
                motion: motion_id,
                includes_motion: motion.is_some(),
            },
        ));
    }
}
