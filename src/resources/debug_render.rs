use bevy::{
    color::Srgba,
    prelude::{Color, Entity, Resource},
};

/// Resource to track render extraction diagnostics between Main World and Render World
#[derive(Resource, Default)]
pub struct RenderExtractionDiagnostics {
    pub main_world_mesh_count: usize,
    pub last_extracted_count: usize,
    pub meshes_marked_visible: usize,
    pub meshes_with_inherited_visibility: usize,
    pub meshes_with_hidden_visibility: usize,
}

const DEBUG_RENDER_COLOR_LIST: [Color; 8] = [
    Color::Srgba(Srgba::RED),
    Color::Srgba(Srgba::GREEN),
    Color::Srgba(Srgba::BLUE),
    Color::Srgba(Srgba::new(1.0, 1.0, 0.0, 1.0)), // YELLOW
    Color::Srgba(Srgba::new(0.0, 1.0, 1.0, 1.0)), // CYAN
    Color::Srgba(Srgba::new(1.0, 0.0, 1.0, 1.0)), // FUCHSIA/MAGENTA
    Color::Srgba(Srgba::WHITE),
    Color::Srgba(Srgba::BLACK),
];

#[derive(Default, Resource)]
pub struct DebugRenderConfig {
    pub colliders: bool,
    pub skeleton: bool,
    pub bone_up: bool,
    pub directional_light_frustum: bool,
    pub directional_light_frustum_freeze: bool,
}

impl DebugRenderConfig {
    pub fn color_for_entity(&self, entity: Entity) -> Color {
        DEBUG_RENDER_COLOR_LIST[entity.index() as usize % DEBUG_RENDER_COLOR_LIST.len()]
    }
}
