use bevy::{
    ecs::query::QueryItem, 
    prelude::*, 
    reflect::Reflect, 
    render::{extract_component::{ExtractComponent, ExtractComponentPlugin}, render_resource::ShaderType},
};

/// Controls whether character face eyes are rendered or clipped for blinking
#[derive(Component, Reflect, Default, Clone, Copy, Debug)]
pub enum BlinkClip {
    #[default]
    EyesOpen,   // Render full face including eye mesh
    EyesClosed, // Clip eye vertices to hide eyes (blink)
}

impl BlinkClip {
    pub fn as_u32(&self) -> u32 {
        match self {
            BlinkClip::EyesOpen => 0,
            BlinkClip::EyesClosed => 1,
        }
    }
}

/// Render-extracted blink state component for shader uniform access.
/// This is automatically extracted to the render thread via ExtractComponent derive macro.
#[derive(Component, Clone, Copy, Debug, Default, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[extract_component_filter(With<Mesh3d>)] // Only apply to mesh entities
pub struct BlinkClipState(pub u32);

/// Uniform buffer structure passed to the shader (for future custom pipeline use)
#[derive(Debug, Clone, ShaderType, Copy)]
pub struct BlinkUniform {
    pub state: u32, // 0 = eyes open, 1 = eyes closed
}

impl Default for BlinkUniform {
    fn default() -> Self {
        Self { state: 0 }
    }
}

/// Plugin that sets up the blink component extraction system
pub struct BlinkClipPlugin;

impl Plugin for BlinkClipPlugin {
    fn build(&self, app: &mut App) {
        // Register type and extract to render world
        app.register_type::<BlinkClipState>()
            .add_plugins(ExtractComponentPlugin::<BlinkClipState>::default())
            .add_systems(Update, sync_blink_clip_to_state);
    }
}

/// System that runs before render extraction to sync BlinkClip -> BlinkClipState
/// This ensures mesh entities have BlinkClipState for proper extraction
pub fn sync_blink_clip_to_state(
    mut commands: Commands,
    query: Query<(Entity, &BlinkClip), With<Mesh3d>>,
) {
    for (entity, blink_clip) in query.iter() {
        let state = BlinkClipState(blink_clip.as_u32());
        commands.entity(entity).insert(state);
    }
}

/// System to update existing BlinkClipState when BlinkClip changes
pub fn update_blink_clip_state(
    mut commands: Commands,
    query: Query<(Entity, &BlinkClip), With<BlinkClipState>>,
) {
    for (entity, blink_clip) in query.iter() {
        let state = BlinkClipState(blink_clip.as_u32());
        commands.entity(entity).insert(state);
    }
}
