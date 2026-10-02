use bevy::prelude::*;

use crate::{
    components::{ChatBubble, ChatBubbleBackground, ChatBubbleEntity, ChatBubbleText},
    render::WorldUiRect,
};

/// Unfaded alpha of a chat bubble rect (text or background), set at spawn.
/// The fade is applied to this every frame; applying it to the rect's current
/// (already faded) alpha would compound and fade the bubble out far too early.
#[derive(Component, Clone, Copy)]
pub struct ChatBubbleBaseAlpha(pub f32);

/// System that updates chat bubble lifetimes and handles fade-out effects
pub fn chat_bubble_update_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query_bubbles: Query<(Entity, &mut ChatBubble), With<ChatBubbleEntity>>,
    query_children: Query<&Children, With<ChatBubbleEntity>>,
    mut query_rects: Query<
        (&mut WorldUiRect, &ChatBubbleBaseAlpha),
        Or<(With<ChatBubbleText>, With<ChatBubbleBackground>)>,
    >,
) {
    let delta = time.delta_secs();

    for (bubble_entity, mut chat_bubble) in query_bubbles.iter_mut() {
        // Update remaining time
        chat_bubble.remaining_time -= delta;

        // Check if bubble should be despawned
        if chat_bubble.remaining_time <= 0.0 {
            commands.entity(bubble_entity).despawn();
            continue;
        }

        // Calculate fade alpha. Fully-opaque phase (first 80% of lifetime) needs
        // no color work.
        let fade_alpha = chat_bubble.get_fade_alpha();
        if (fade_alpha - 1.0).abs() < f32::EPSILON {
            continue;
        }

        // Update the text and background rects from their unfaded alpha
        if let Ok(children) = query_children.get(bubble_entity) {
            for child in children.iter() {
                if let Ok((mut rect, base_alpha)) = query_rects.get_mut(child) {
                    rect.color.set_alpha(base_alpha.0 * fade_alpha);
                }
            }
        }
    }
}
