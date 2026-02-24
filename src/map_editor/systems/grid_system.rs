//! Editor Grid System
//! 
//! This module provides a visual grid for the map editor at y=0.
//! The grid helps with positioning and alignment of objects.

use bevy::{
    prelude::{
        App, Commands, Entity, Gizmos, IntoScheduleConfigs, Plugin, Query, Res, ResMut, 
        Update, Vec3, Color, With, Without, InheritedVisibility, Transform, Mesh, 
        MeshMaterial3d, StandardMaterial, Assets, Handle, Component,
    },
};

use crate::map_editor::{
    components::{EditorGrid, EditorOnly},
    resources::{MapEditorState, EditorGridSettings},
};

/// Plugin for the editor grid system
pub struct EditorGridPlugin;

impl Plugin for EditorGridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, grid_render_system);
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

    // Draw grid lines along X axis
    let half_extent = extent / 2.0;
    let num_lines = (extent / cell_size) as i32 + 1;

    for i in 0..num_lines {
        let z = -half_extent + (i as f32) * cell_size;
        
        // Vary line intensity for major lines
        let is_major_line = i % 10 == 0;
        let line_color = if is_major_line {
            Color::srgba(grid_color.to_srgba().red, grid_color.to_srgba().green, grid_color.to_srgba().blue, 0.8)
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
            Color::srgba(grid_color.to_srgba().red, grid_color.to_srgba().green, grid_color.to_srgba().blue, 0.8)
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

/// System to spawn a mesh-based grid (alternative to gizmo grid)
/// 
/// This is useful if you want a more permanent grid that doesn't
/// rely on gizmos. Currently not used, but available for future use.
#[allow(dead_code)]
pub fn grid_spawn_system(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    grid_settings: Res<EditorGridSettings>,
    query_grid: Query<Entity, With<EditorGrid>>,
) {
    // Check if grid already exists
    if !query_grid.is_empty() {
        return;
    }

    // Create a grid mesh
    // For now, we use gizmos instead, but this could be used for a mesh-based grid
    let _ = (commands, meshes, materials, grid_settings);
}

/// System to update grid visibility based on settings
pub fn grid_visibility_system(
    grid_settings: Res<EditorGridSettings>,
    mut query_grid: Query<&mut InheritedVisibility, With<EditorGrid>>,
) {
    // Update visibility of mesh-based grid entities
    for mut visibility in query_grid.iter_mut() {
        // Note: InheritedVisibility is read-only, we would need Visibility component
        // to change visibility. This is a placeholder for mesh-based grid.
        let _ = visibility;
    }
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
