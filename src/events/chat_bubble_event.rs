use bevy::{prelude::{Event, Color, Entity}, reflect::Reflect};

/// Event to trigger spawning a chat bubble
#[derive(Event, Reflect)]
pub struct ChatBubbleEvent {
    /// Entity to attach the bubble to, if known
    pub target_entity: Option<Entity>,
    /// Name of the entity, used for lookup if entity not provided
    pub entity_name: String,
    /// The text to display
    pub text: String,
    /// How long to display the bubble in seconds
    pub duration: f32,
    /// Color of the text
    pub color: Color,
    /// Type of bubble for styling
    pub bubble_type: ChatBubbleType,
}

impl ChatBubbleEvent {
    pub fn new(entity_name: String, text: String) -> Self {
        Self {
            target_entity: None,
            entity_name,
            text,
            duration: 5.0,
            color: Color::WHITE,
            bubble_type: ChatBubbleType::Normal,
        }
    }

    pub fn with_entity(mut self, entity: Entity) -> Self {
        self.target_entity = Some(entity);
        self
    }

    pub fn with_duration(mut self, duration: f32) -> Self {
        self.duration = duration;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_bubble_type(mut self, bubble_type: ChatBubbleType) -> Self {
        self.bubble_type = bubble_type;
        self
    }
}

#[derive(Clone, Copy, Debug, Default, Reflect, PartialEq, Eq)]
pub enum ChatBubbleType {
    #[default]
    Normal,
    Shout,
    Whisper,
    Monster,
    Emote,
}
