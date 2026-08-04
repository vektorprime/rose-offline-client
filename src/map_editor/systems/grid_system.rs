//! Editor Grid System
//!
//! This module provides a visual grid for the map editor at y=0.
//! The grid helps with positioning and alignment of objects.

use bevy::prelude::{
    in_state, App, Color, Gizmos, IntoScheduleConfigs, Plugin, Res, Update, Vec3,
};

use crate::map_editor::resources::{EditorGridSettings, MapEditorState};
use crate::resources::AppState;

/// Plugin for the editor grid system
pub struct EditorGridPlugin;

impl Plugin for EditorGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            grid_render_system.run_if(in_state(AppState::MapEditor)),
        );
    }
}

/// System that renders the editor grid using gizmos
///
/// This system:
/// - Draws a grid at y=0 using Bevy's gizmo system
/// - Respects the grid visibility setting
/// - Uses the grid size from EditorGridSettings
pub fn grid_render_system(
    map_editor_state: Res<MapEditorState>,
    grid_settings: Res<EditorGridSettings>,
    mut gizmos: Gizmos,
) {
    // Only run when map editor is enabled and grid is visible
    if !map_editor_state.enabled || !grid_settings.visible {
        return;
    }

    let cell_size = grid_settings.cell_size;
    let extent = grid_settings.extent;
    let grid_color = grid_settings.color;
    let grid_srgba = grid_color.to_srgba();

    // Draw grid lines along X axis
    let half_extent = extent / 2.0;
    let num_lines = (extent / cell_size) as i32 + 1;

    for i in 0..num_lines {
        let z = -half_extent + (i as f32) * cell_size;

        // Vary line intensity for major lines
        let is_major_line = i % 10 == 0;
        let line_color = if is_major_line {
            Color::srgba(grid_srgba.red, grid_srgba.green, grid_srgba.blue, 0.8)
        } else {
            grid_color
        };

        // Draw line along X axis
        gizmos.line(
            Vec3::new(-half_extent, 0.0, z),
            Vec3::new(half_extent, 0.0, z),
            line_color,
        );
    }

    // Draw grid lines along Z axis
    for i in 0..num_lines {
        let x = -half_extent + (i as f32) * cell_size;

        // Vary line intensity for major lines
        let is_major_line = i % 10 == 0;
        let line_color = if is_major_line {
            Color::srgba(grid_srgba.red, grid_srgba.green, grid_srgba.blue, 0.8)
        } else {
            grid_color
        };

        // Draw line along Z axis
        gizmos.line(
            Vec3::new(x, 0.0, -half_extent),
            Vec3::new(x, 0.0, half_extent),
            line_color,
        );
    }

    // Draw origin axes for reference
    let axis_length = 5.0;

    // X axis (red)
    gizmos.line(
        Vec3::ZERO,
        Vec3::X * axis_length,
        Color::srgba(1.0, 0.0, 0.0, 1.0),
    );

    // Y axis (green)
    gizmos.line(
        Vec3::ZERO,
        Vec3::Y * axis_length,
        Color::srgba(0.0, 1.0, 0.0, 1.0),
    );

    // Z axis (blue)
    gizmos.line(
        Vec3::ZERO,
        Vec3::Z * axis_length,
        Color::srgba(0.0, 0.0, 1.0, 1.0),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grid_system_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
