use bevy::{log::info, prelude::*};

use rose_game_common::components::Npc;

use crate::{
    components::{MonsterChatter, ClientEntityName, ClientEntity, ClientEntityType, ModelHeight},
    events::{ChatBubbleEvent, ChatBubbleType},
    resources::MonsterChatterPhrases,
};

/// System that makes monsters and NPCs occasionally say random phrases
pub fn monster_chatter_system(
    mut commands: Commands,
    time: Res<Time<Virtual>>,
    mut query_entities: Query<(Entity, &mut MonsterChatter, Option<&ClientEntityName>, Option<&ClientEntity>, Option<&ModelHeight>), With<Npc>>,
    mut chat_bubble_events: EventWriter<ChatBubbleEvent>,
    phrases: Res<MonsterChatterPhrases>,
) {
    let delta = time.delta_secs();
    let mut events_sent = 0;

    for (entity, mut chatter, name, client_entity, _model_height) in query_entities.iter_mut() {
        // Decrease timer
        chatter.time_until_next_chat -= delta;

        // Check if it's time to chat
        if chatter.time_until_next_chat <= 0.0 {
            // Get entity type (default to Monster if no ClientEntity component)
            let entity_type = client_entity
                .map(|ce| ce.entity_type)
                .unwrap_or(ClientEntityType::Monster);

            // Get a random phrase based on entity type
            let phrase = phrases.get_random_phrase(entity_type);

            // Get entity name or use default based on type
            let entity_name = name.map(|n| n.name.clone()).unwrap_or_else(|| {
                match entity_type {
                    ClientEntityType::Npc => "NPC".to_string(),
                    _ => "Monster".to_string(),
                }
            });

            info!("[MONSTER_CHATTER] Sending chat bubble for entity {:?} name='{}' text='{}'",
                entity, entity_name, phrase);
            
            // Determine bubble type based on entity type
            let bubble_type = match entity_type {
                ClientEntityType::Npc => ChatBubbleType::Npc,
                _ => ChatBubbleType::Monster,
            };

            // Send chat bubble event
            chat_bubble_events.write(
                ChatBubbleEvent::new(entity_name, phrase.clone())
                    .with_entity(entity)
                    .with_duration(10.0)
                    .with_color(Color::BLACK)
                    .with_bubble_type(bubble_type)
            );
            events_sent += 1;

            // Reset timer with random interval
            chatter.time_until_next_chat = rand::random::<f32>()
                * (chatter.max_interval - chatter.min_interval)
                + chatter.min_interval;
        }
    }
    
    if events_sent > 0 {
        info!("[MONSTER_CHATTER] Sent {} chat bubble events this frame", events_sent);
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
