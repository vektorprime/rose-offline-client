use arrayvec::ArrayVec;
use bevy::{
    platform::collections::HashMap,
    prelude::{Handle, Image, Resource, Vec2},
};

use crate::render::WorldUiRect;

pub struct NameTagData {
    pub image: Handle<Image>,
    pub size: Vec2,
    pub rects: ArrayVec<WorldUiRect, 2>, // NPC names are 2 rows
}

#[derive(Default, Resource)]
pub struct NameTagCache {
    pub cache: HashMap<String, NameTagData>,
}
