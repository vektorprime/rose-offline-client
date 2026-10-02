use bevy::camera::primitives::{Frustum, Sphere};
use bevy::ecs::world::EntityWorldMut;
use bevy::math::Vec3A;
use bevy::prelude::{
    Camera3d, Commands, Entity, GlobalTransform, Local, Or, Query, Res, State, Visibility, With,
    Without,
};
use bevy_rapier3d::{
    plugin::context::systemparams::ReadRapierContext,
    prelude::{CollisionGroups, QueryFilter},
};

use crate::{
    components::{
        ChatBubbleEntity, NameTag, NameTagEntity, OcclusionState, COLLISION_FILTER_INSPECTABLE,
        COLLISION_GROUP_ZONE_OBJECT, COLLISION_GROUP_ZONE_TERRAIN,
    },
    resources::{AppState, NameTagSettings, SelectedTarget},
};

/// Raycast checks are distributed across this many frames so that only a fraction
/// of the visible tags pay for a raycast each frame. Every tag is re-checked once
/// every N frames; newly spawned tags (no occlusion state yet) are checked immediately.
const OCCLUSION_STAGGER_FRAMES: u64 = 4;

/// Small distance epsilon so an anchor resting exactly against a wall is not
/// reported as occluded by that wall.
const OCCLUSION_EPSILON: f32 = 0.05;

/// Radius around a tag anchor, at its distance from the camera, that must touch the
/// camera frustum before the tag's line of sight is ray-cast. Tags are drawn at a
/// fixed pixel size, so their world-space reach grows with distance: 0.25 m per
/// metre is ~300 px at 1080p and the 45 degree FOV, more than a name tag with its
/// health bar, and also covers a frame of camera turn (the frustum is last frame's).
const TAG_VIEW_RADIUS_BASE: f32 = 2.0;
const TAG_VIEW_RADIUS_PER_METER: f32 = 0.25;

/// Hides name tags and chat bubbles when terrain or a zone object (building, wall,
/// decoration) blocks the line of sight between the camera and the tag anchor.
///
/// Characters, NPCs, monsters, item drops and water never occlude tags. Hovered or
/// selected targets keep their name tag visible even behind occluders.
///
/// Tags outside the camera view skip the ray cast (most tags around the player are
/// off screen): their stored result is dropped, so the first frame they are back in
/// view checks them again immediately.
pub fn world_ui_occlusion_system(
    mut commands: Commands,
    app_state: Res<State<AppState>>,
    rapier_context: ReadRapierContext,
    query_camera: Query<
        (&GlobalTransform, &Frustum),
        (With<Camera3d>, Without<crate::render::WaterReflectionCamera>),
    >,
    mut query_tags: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&NameTag>,
            Option<&ChatBubbleEntity>,
            Option<&OcclusionState>,
            &mut Visibility,
        ),
        Or<(With<NameTag>, With<ChatBubbleEntity>)>,
    >,
    query_name_tag_entity: Query<&NameTagEntity>,
    selected_target: Res<SelectedTarget>,
    name_tag_settings: Res<NameTagSettings>,
    mut frame_counter: Local<u64>,
) {
    if *app_state.get() != AppState::Game {
        return;
    }

    let Ok(rapier_context) = rapier_context.single() else {
        return;
    };

    let Ok((camera_transform, camera_frustum)) = query_camera.single() else {
        return;
    };

    let camera_position = camera_transform.translation();

    let hovered_name_tag = selected_target
        .hover
        .and_then(|entity| query_name_tag_entity.get(entity).ok())
        .map(|name_tag_entity| name_tag_entity.0);
    let selected_name_tag = selected_target
        .selected
        .and_then(|entity| query_name_tag_entity.get(entity).ok())
        .map(|name_tag_entity| name_tag_entity.0);

    // Only zone objects (buildings/decorations) and terrain block line of sight.
    // The INSPECTABLE membership bit is accepted by every terrain/object collider filter.
    let occluder_groups = CollisionGroups::new(
        COLLISION_FILTER_INSPECTABLE,
        COLLISION_GROUP_ZONE_OBJECT | COLLISION_GROUP_ZONE_TERRAIN,
    );

    let frame_index = *frame_counter;
    *frame_counter = frame_counter.wrapping_add(1);

    for (entity, global_transform, name_tag, chat_bubble, occlusion_state, mut visibility) in
        query_tags.iter_mut()
    {
        let is_focused = Some(entity) == hovered_name_tag || Some(entity) == selected_name_tag;

        // A focused name tag is always shown and a non-show_all, unfocused one is
        // always hidden, so only chat bubbles and show_all unfocused tags use the ray.
        let occlusion_matters = match name_tag {
            Some(name_tag) => !is_focused && name_tag_settings.show_all[name_tag.name_tag_type],
            None => chat_bubble.is_some(),
        };

        let mut occluded = occlusion_state.is_some_and(|state| state.occluded);

        let anchor = global_transform.translation();
        let in_view = || {
            let radius =
                TAG_VIEW_RADIUS_BASE + TAG_VIEW_RADIUS_PER_METER * anchor.distance(camera_position);
            camera_frustum.intersects_sphere(
                &Sphere {
                    center: Vec3A::from(anchor),
                    radius,
                },
                false,
            )
        };

        if occlusion_matters && !in_view() {
            // Not drawn: skip the ray, and forget the stale result so the tag is
            // checked immediately when it comes back into view.
            if occlusion_state.is_some() {
                commands
                    .entity(entity)
                    .queue_silenced(|mut tag_entity: EntityWorldMut| {
                        tag_entity.remove::<OcclusionState>();
                    });
            }
        } else if occlusion_matters {
            let should_check = occlusion_state.is_none()
                || (u64::from(entity.index_u32()) % OCCLUSION_STAGGER_FRAMES
                    == frame_index % OCCLUSION_STAGGER_FRAMES);

            if should_check {
                let delta = anchor - camera_position;
                let distance = delta.length();

                occluded = if distance > OCCLUSION_EPSILON {
                    rapier_context
                        .cast_ray(
                            camera_position,
                            delta / distance,
                            distance - OCCLUSION_EPSILON,
                            false,
                            QueryFilter::new().groups(occluder_groups),
                        )
                        .is_some()
                } else {
                    false
                };

                if occlusion_state.is_none_or(|state| state.occluded != occluded) {
                    commands.entity(entity).queue_silenced(
                        move |mut tag_entity: EntityWorldMut| {
                            tag_entity.insert(OcclusionState { occluded });
                        },
                    );
                }
            }
        } else if occlusion_state.is_some() {
            // Skip the raycast while it cannot change visibility, and drop the
            // stored result so the tag is re-checked immediately (never with a
            // stale result) as soon as occlusion matters again.
            commands
                .entity(entity)
                .queue_silenced(|mut tag_entity: EntityWorldMut| {
                    tag_entity.remove::<OcclusionState>();
                });
        }

        let desired_visibility = if let Some(name_tag) = name_tag {
            let base_visible = name_tag_settings.show_all[name_tag.name_tag_type] || is_focused;
            if base_visible && (is_focused || !occluded) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        } else if chat_bubble.is_some() {
            if occluded {
                Visibility::Hidden
            } else {
                Visibility::Inherited
            }
        } else {
            continue;
        };

        if *visibility != desired_visibility {
            *visibility = desired_visibility;
        }
    }
}
