use bevy::prelude::*;

/// Component attached to a chat bubble entity to track its lifetime
#[derive(Component, Reflect)]
pub struct ChatBubble {
    /// The entity this bubble is attached to
    pub target_entity: Entity,
    /// The text being displayed
    pub text: String,
    /// Time remaining before fade-out starts
    pub remaining_time: f32,
    /// Total display time for calculating fade
    pub total_time: f32,
    /// Time when fade-out begins as fraction of total_time (e.g., 0.8 means fade starts at 80% of total time)
    pub fade_start_fraction: f32,
}

impl ChatBubble {
    pub fn new(target_entity: Entity, text: String, duration: f32) -> Self {
        Self {
            target_entity,
            text,
            remaining_time: duration,
            total_time: duration,
            fade_start_fraction: 0.8, // Fade starts at 80% of total time
        }
    }

    /// Returns a value between 0.0 and 1.0 representing the fade alpha
    /// 1.0 means fully visible, 0.0 means fully faded
    pub fn get_fade_alpha(&self) -> f32 {
        let fade_start_time = self.total_time * self.fade_start_fraction;
        if self.remaining_time >= fade_start_time {
            1.0
        } else {
            (self.remaining_time / fade_start_time).clamp(0.0, 1.0)
        }
    }
}

/// Marker component for the parent entity containing chat bubble parts
#[derive(Component)]
pub struct ChatBubbleEntity {
    pub target_entity: Entity,
}

/// Marker component for chat bubble text rects
#[derive(Component)]
pub struct ChatBubbleText;

/// Marker component for chat bubble background rects
#[derive(Component)]
pub struct ChatBubbleBackground;

/// Component that enables monsters to display random chat phrases
#[derive(Component, Reflect)]
pub struct MonsterChatter {
    /// Time until next chat message
    pub time_until_next_chat: f32,
    /// Minimum time between chats in seconds
    pub min_interval: f32,
    /// Maximum time between chats in seconds
    pub max_interval: f32,
}

impl MonsterChatter {
    pub fn new(min_interval: f32, max_interval: f32) -> Self {
        Self {
            time_until_next_chat: rand::random::<f32>() * (max_interval - min_interval) + min_interval,
            min_interval,
            max_interval,
        }
    }
}

impl Default for MonsterChatter {
    fn default() -> Self {
        Self::new(30.0, 120.0) // Default: 30-120 seconds between chats
    }
}
