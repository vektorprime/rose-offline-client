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

/// Hides name tags and chat bubbles when terrain or a zone object (building, wall,
/// decoration) blocks the line of sight between the camera and the tag anchor.
///
/// Characters, NPCs, monsters, item drops and water never occlude tags. Hovered or
/// selected targets keep their name tag visible even behind occluders.
pub fn world_ui_occlusion_system(
    mut commands: Commands,
    app_state: Res<State<AppState>>,
    rapier_context: ReadRapierContext,
    query_camera: Query<
        &GlobalTransform,
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

    let Ok(camera_transform) = query_camera.single() else {
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
        let mut occluded = occlusion_state.is_some_and(|state| state.occluded);

        let should_check = occlusion_state.is_none()
            || (u64::from(entity.index_u32()) % OCCLUSION_STAGGER_FRAMES
                == frame_index % OCCLUSION_STAGGER_FRAMES);

        if should_check {
            let anchor = global_transform.translation();
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
                commands.entity(entity).insert(OcclusionState { occluded });
            }
        }

        let desired_visibility = if let Some(name_tag) = name_tag {
            let is_focused =
                Some(entity) == hovered_name_tag || Some(entity) == selected_name_tag;
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
