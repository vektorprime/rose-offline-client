use bevy::prelude::*;

use crate::{
    components::{ChatBubble, ChatBubbleEntity, ChatBubbleText, ChatBubbleBackground},
    render::WorldUiRect,
};

/// System that updates chat bubble lifetimes and handles fade-out effects
pub fn chat_bubble_update_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query_bubbles: Query<(Entity, &mut ChatBubble), With<ChatBubbleEntity>>,
    query_children: Query<&Children, With<ChatBubbleEntity>>,
    // Use Without<> to make queries disjoint and avoid Bevy error B0001
    mut query_text_rects: Query<&mut WorldUiRect, (With<ChatBubbleText>, Without<ChatBubbleBackground>)>,
    mut query_bg_rects: Query<&mut WorldUiRect, (With<ChatBubbleBackground>, Without<ChatBubbleText>)>,
) {
    let delta = time.delta_secs();

    for (bubble_entity, mut chat_bubble) in query_bubbles.iter_mut() {
        // Update remaining time
        chat_bubble.remaining_time -= delta;

        // Check if bubble should be despawned
        if chat_bubble.remaining_time <= 0.0 {
            commands.entity(bubble_entity).despawn_recursive();
            continue;
        }

        // Calculate fade alpha
        let fade_alpha = chat_bubble.get_fade_alpha();

        // Update child rects if we can get them
        if let Ok(children) = query_children.get(bubble_entity) {
            for child in children.iter() {
                // Update text rect
                if let Ok(mut rect) = query_text_rects.get_mut(child) {
                    let base_color = rect.color;
                    let srgba = base_color.to_srgba();
                    rect.color = Color::srgba(srgba.red, srgba.green, srgba.blue, srgba.alpha * fade_alpha);
                }

                // Update background rect
                if let Ok(mut rect) = query_bg_rects.get_mut(child) {
                    let base_color = rect.color;
                    let srgba = base_color.to_srgba();
                    rect.color = Color::srgba(srgba.red, srgba.green, srgba.blue, srgba.alpha * fade_alpha);
                }
            }
        }
    }
}
