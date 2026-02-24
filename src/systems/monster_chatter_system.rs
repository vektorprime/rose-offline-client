use bevy::prelude::*;

use rose_game_common::components::Npc;

use crate::{
    components::{MonsterChatter, ClientEntityName, ModelHeight},
    events::{ChatBubbleEvent, ChatBubbleType},
    resources::MonsterChatterPhrases,
};

/// System that makes monsters occasionally say random phrases
pub fn monster_chatter_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query_monsters: Query<(Entity, &mut MonsterChatter, Option<&ClientEntityName>, Option<&ModelHeight>), With<Npc>>,
    mut chat_bubble_events: EventWriter<ChatBubbleEvent>,
    phrases: Res<MonsterChatterPhrases>,
) {
    let delta = time.delta_secs();

    for (entity, mut chatter, name, _model_height) in query_monsters.iter_mut() {
        // Decrease timer
        chatter.time_until_next_chat -= delta;

        // Check if it's time to chat
        if chatter.time_until_next_chat <= 0.0 {
            // Get a random phrase
            let phrase = phrases.get_random_phrase();

            // Get entity name or use default
            let entity_name = name.map(|n| n.name.clone()).unwrap_or_else(|| "Monster".to_string());

            // Send chat bubble event
            chat_bubble_events.write(
                ChatBubbleEvent::new(entity_name, phrase.clone())
                    .with_entity(entity)
                    .with_duration(4.0)
                    .with_color(Color::srgb(0.9, 0.9, 0.9))
                    .with_bubble_type(ChatBubbleType::Monster)
            );

            // Reset timer with random interval
            chatter.time_until_next_chat = rand::random::<f32>() 
                * (chatter.max_interval - chatter.min_interval) 
                + chatter.min_interval;
        }
    }
}

/// System to add MonsterChatter component to newly spawned NPCs
pub fn add_monster_chatter_system(
    mut commands: Commands,
    query_npcs: Query<Entity, (With<Npc>, Without<MonsterChatter>)>,
) {
    for entity in query_npcs.iter() {
        commands.entity(entity).insert(MonsterChatter::default());
    }
}
