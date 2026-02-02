use std::collections::VecDeque;

use bevy::{
    prelude::{Color, Component, GlobalTransform, Handle, Vec3},
    time::Time,
};

use crate::resources::RenderConfiguration;

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

#[derive(Copy, Clone, Default)]
struct TrailEffectPoint {
    start: Vec3,
    end: Vec3,
    time: f32,
}

#[derive(Component)]
pub struct TrailEffectPositionHistory {
    history: VecDeque<TrailEffectPoint>,
    catmull_points: [TrailEffectPoint; 4],
    trail_length_excess: f32,
    last_temp_points: usize,
}

impl Default for TrailEffectPositionHistory {
    fn default() -> Self {
        Self {
            history: VecDeque::new(),
            catmull_points: [Default::default(); 4],
            trail_length_excess: 0.0,
            last_temp_points: 0,
        }
    }
}

pub struct TrailEffectRenderPlugin;

impl bevy::app::Plugin for TrailEffectRenderPlugin {
    fn build(&self, _app: &mut bevy::app::App) {
        // Trail effect rendering temporarily disabled for Bevy 0.14 migration
        // The component definitions are kept for API compatibility
    }
}

fn initialise_trail_effects(
    mut commands: bevy::ecs::system::Commands,
    query: bevy::ecs::system::Query<
        bevy::ecs::entity::Entity,
        (
            bevy::ecs::query::With<TrailEffect>,
            bevy::ecs::query::Without<TrailEffectPositionHistory>,
        ),
    >,
) {
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(TrailEffectPositionHistory::default());
    }
}

#[allow(clippy::type_complexity)]
fn update_trail_effects(
    _render_config: bevy::ecs::system::Res<RenderConfiguration>,
    _time: bevy::ecs::system::Res<Time>,
    mut _query: bevy::ecs::system::Query<(
        &TrailEffect,
        &mut TrailEffectPositionHistory,
        &GlobalTransform,
    )>,
) {
    // Trail effect update logic temporarily disabled
}
