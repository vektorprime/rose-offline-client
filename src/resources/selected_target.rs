use bevy::prelude::{Entity, Resource};

use crate::resources::UiCursorType;

#[derive(Default, Resource)]
pub struct SelectedTarget {
    pub selected: Option<Entity>,
    pub hover: Option<Entity>,
    pub cursor_type: UiCursorType,
}
