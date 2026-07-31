use bevy::{
    prelude::{Color, Component, Handle, Vec3},
};

/// Trail effect component - temporarily disabled rendering, but kept for API compatibility
#[derive(Component)]
pub struct TrailEffect {
    pub colour: Color,
    pub duration: f32, // Seconds as f32
    pub start_offset: Vec3,
    pub end_offset: Vec3,
    pub trail_texture: Handle<bevy::prelude::Image>,
    pub distance_per_point: f32,
}
