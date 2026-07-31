//! Map Editor Components
//!
//! This module contains component definitions for the map editor system.

use bevy::prelude::*;

/// Marker component for entities that are currently selected in the editor
#[derive(Component, Default)]
pub struct SelectedInEditor;

/// Marker component for entities that can be selected in the editor
#[derive(Component, Default)]
pub struct EditorSelectable;
