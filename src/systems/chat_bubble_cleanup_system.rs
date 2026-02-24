use bevy::prelude::*;

use crate::{
    components::{ChatBubbleEntity, ClientEntityName},
};

/// System that cleans up chat bubbles when their target entities are despawned
/// This listens for removed ClientEntityName components as a signal that entities are being removed
pub fn chat_bubble_cleanup_system(
    mut commands: Commands,
    mut removed_names: RemovedComponents<ClientEntityName>,
    query_bubbles: Query<(Entity, &ChatBubbleEntity)>,
) {
    // Collect all target entities that have been removed
    let removed_entities: Vec<Entity> = removed_names.read().collect();

    // Despawn any chat bubbles targeting removed entities
    for (bubble_entity, bubble_marker) in query_bubbles.iter() {
        if removed_entities.contains(&bubble_marker.target_entity) {
            commands.entity(bubble_entity).despawn_recursive();
        }
    }
}

/// Alternative cleanup that runs when parent entity is despawned
/// This uses bevy's hierarchy to detect when parent is gone
pub fn chat_bubble_orphan_cleanup_system(
    mut commands: Commands,
    query_bubbles: Query<(Entity, &ChatBubbleEntity), With<ChatBubbleEntity>>,
    query_targets: Query<Entity>,
) {
    for (bubble_entity, bubble_marker) in query_bubbles.iter() {
        // Check if target entity still exists
        if query_targets.get(bubble_marker.target_entity).is_err() {
            commands.entity(bubble_entity).despawn_recursive();
        }
    }
}
