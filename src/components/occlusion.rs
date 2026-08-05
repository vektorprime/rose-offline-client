use bevy::prelude::Component;

/// Tracks the line-of-sight occlusion state of a world UI element
/// (name tag root or chat bubble root), updated by `world_ui_occlusion_system`.
#[derive(Component, Clone, Copy, Default)]
pub struct OcclusionState {
    pub occluded: bool,
}
