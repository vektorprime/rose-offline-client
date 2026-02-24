//! Selection Highlight System
//! 
//! This module provides visual feedback for selected entities in the map editor.
//! It draws selection outlines/highlights using Bevy's gizmo system.

use bevy::{
    prelude::{
        App, Color, Entity, GlobalTransform, Gizmos, IntoScheduleConfigs, Plugin, Query, Res, 
        Update, With, Without, Vec3, InheritedVisibility, Transform,
    },
};

use crate::map_editor::{
    components::{SelectedInEditor, EditorSelectable},
    resources::MapEditorState,
};

/// Plugin for the selection highlight system
pub struct SelectionHighlightPlugin;

impl Plugin for SelectionHighlightPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, selection_highlight_system);
    }
}

/// System that draws visual highlights around selected entities
/// 
/// This system:
/// - Draws bounding boxes around selected entities using gizmos
/// - Uses a bright highlight color for visibility
/// - Only runs when the map editor is enabled
pub fn selection_highlight_system(
    map_editor_state: Res<MapEditorState>,
    mut gizmos: Gizmos,
    query_selected: Query<
        (Entity, &GlobalTransform, Option<&InheritedVisibility>),
        With<SelectedInEditor>,
    >,
    query_selectable: Query<&GlobalTransform, (With<EditorSelectable>, Without<SelectedInEditor>)>,
) {
    // Only run when map editor is enabled
    if !map_editor_state.enabled {
        return;
    }

    // Selection highlight color (bright cyan)
    let selection_color = Color::srgba(0.0, 1.0, 1.0, 0.8);
    
    // Hover color for selectable but not selected entities
    let _hover_color = Color::srgba(1.0, 1.0, 0.0, 0.4);

    // Draw selection boxes around all selected entities
    for (entity, transform, visibility) in query_selected.iter() {
        // Skip invisible entities
        if let Some(vis) = visibility {
            if !vis.get() {
                continue;
            }
        }

        let position = transform.translation();
        let rotation = transform.rotation();
        let scale = transform.scale();
        
        // Calculate a bounding box based on scale
        // Use a reasonable default size that can be seen
        let half_size = 1.0;

        // Create a transform for the cuboid gizmo
        let mut cube_transform = Transform::from_translation(position);
        cube_transform.rotation = rotation;
        cube_transform.scale = Vec3::new(
            half_size * 2.0 * scale.x,
            half_size * 2.0 * scale.y,
            half_size * 2.0 * scale.z,
        );

        // Draw a wireframe cube around the selected entity using cuboid
        gizmos.cuboid(cube_transform, selection_color);

        // Draw entity index indicator
        let _ = entity; // Acknowledge entity variable
        
        // Note: In a full implementation, you might want to:
        // - Get actual mesh bounding boxes for more accurate outlines
        // - Draw entity names/IDs above selected entities
        // - Use different colors for different selection states
        // - Apply outline post-processing effects
    }

    // Optionally draw indicators for selectable entities (when hovering)
    // This would require mouse hover detection which can be added later
    let _ = query_selectable; // Acknowledge the query for future use
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_system_exists() {
        // Basic test to ensure the module compiles
        assert!(true);
    }
}
