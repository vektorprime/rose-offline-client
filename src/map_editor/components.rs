//! Map Editor Components
//! 
//! This module contains component definitions for the map editor system.

use bevy::prelude::*;

/// Marker component for entities that are currently selected in the editor
#[derive(Component, Default)]
pub struct SelectedInEditor;

/// Component for editor gizmo entities (visual handles for transform manipulation)
#[derive(Component)]
pub struct EditorGizmo {
    /// The entity this gizmo is controlling
    pub target_entity: Entity,
    /// The type of gizmo
    pub gizmo_type: GizmoType,
}

impl EditorGizmo {
    /// Create a new editor gizmo
    pub fn new(target_entity: Entity, gizmo_type: GizmoType) -> Self {
        Self {
            target_entity,
            gizmo_type,
        }
    }
}

/// Type of gizmo for transform manipulation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoType {
    /// Translation gizmo (arrows for moving)
    Translate,
    /// Rotation gizmo (rings for rotating)
    Rotate,
    /// Scale gizmo (boxes for scaling)
    Scale,
}

impl Default for GizmoType {
    fn default() -> Self {
        Self::Translate
    }
}

impl GizmoType {
    /// Get a display name for the gizmo type
    pub fn display_name(&self) -> &'static str {
        match self {
            GizmoType::Translate => "Translate",
            GizmoType::Rotate => "Rotate",
            GizmoType::Scale => "Scale",
        }
    }
}

/// Component for the visual grid in the editor
#[derive(Component)]
pub struct EditorGrid {
    /// Grid cell size
    pub cell_size: f32,
    /// Grid extent (total size)
    pub extent: f32,
}

impl Default for EditorGrid {
    fn default() -> Self {
        Self {
            cell_size: 1.0,
            extent: 100.0,
        }
    }
}

/// Marker component for entities that can be selected in the editor
#[derive(Component, Default)]
pub struct EditorSelectable;

/// Marker component for entities that are currently being previewed (e.g., during placement)
#[derive(Component, Default)]
pub struct EditorPreview;

/// Component for entities that have been modified in the editor
#[derive(Component)]
pub struct EditorModified {
    /// Original transform before modification
    pub original_transform: Transform,
}

/// Marker component for editor-only entities (not part of the actual game world)
#[derive(Component, Default)]
pub struct EditorOnly;

/// Component for entities that represent object handles (e.g., for area selection)
#[derive(Component)]
pub struct EditorHandle {
    /// The entity this handle is associated with
    pub target_entity: Entity,
    /// The handle type
    pub handle_type: HandleType,
}

/// Type of editor handle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleType {
    /// Corner handle for resizing
    Corner,
    /// Edge handle for resizing
    Edge,
    /// Center handle for moving
    Center,
}

impl Default for HandleType {
    fn default() -> Self {
        Self::Center
    }
}
