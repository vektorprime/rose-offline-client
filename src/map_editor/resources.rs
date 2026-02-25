//! Map Editor Resources
//! 
//! This module contains the resource definitions for the map editor system.

use bevy::prelude::*;
use std::collections::HashSet;

/// Maximum number of undo actions to keep in history
const MAX_UNDO_HISTORY: usize = 100;

/// Main resource for map editor state
#[derive(Resource, Default)]
pub struct MapEditorState {
    /// Whether the map editor is enabled
    pub enabled: bool,
    
    /// Currently selected entities (multi-select support)
    pub selected_entities: HashSet<Entity>,
    
    /// Current editor mode
    pub editor_mode: EditorMode,
    
    /// Transform space for gizmos
    pub transform_space: TransformSpace,
    
    /// Whether to snap to grid
    pub snap_to_grid: bool,
    
    /// Grid size for snapping
    pub grid_size: f32,
    
    /// Whether to show the grid
    pub show_grid: bool,
    
    /// Whether the map has unsaved modifications
    pub is_modified: bool,
    
    /// Search filter for model browser
    pub model_browser_search: String,
    
    /// Filter for hierarchy panel
    pub hierarchy_filter: String,
    
    /// Undo stack for editor actions
    pub undo_stack: Vec<EditorAction>,
    
    /// Redo stack for editor actions
    pub redo_stack: Vec<EditorAction>,
}

impl MapEditorState {
    /// Create a new MapEditorState with default values
    pub fn new() -> Self {
        Self {
            enabled: true,
            selected_entities: HashSet::new(),
            editor_mode: EditorMode::default(),
            transform_space: TransformSpace::default(),
            snap_to_grid: true,
            grid_size: 1.0,
            show_grid: true,
            is_modified: false,
            model_browser_search: String::new(),
            hierarchy_filter: String::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
    
    /// Clear all selected entities
    pub fn clear_selection(&mut self) {
        self.selected_entities.clear();
    }
    
    /// Add an entity to the selection
    pub fn select_entity(&mut self, entity: Entity) {
        self.selected_entities.insert(entity);
    }
    
    /// Remove an entity from the selection
    pub fn deselect_entity(&mut self, entity: Entity) {
        self.selected_entities.remove(&entity);
    }
    
    /// Toggle entity selection
    pub fn toggle_entity_selection(&mut self, entity: Entity) {
        if self.selected_entities.contains(&entity) {
            self.selected_entities.remove(&entity);
        } else {
            self.selected_entities.insert(entity);
        }
    }
    
    /// Check if an entity is selected
    pub fn is_entity_selected(&self, entity: Entity) -> bool {
        self.selected_entities.contains(&entity)
    }
    
    /// Get the number of selected entities
    pub fn selection_count(&self) -> usize {
        self.selected_entities.len()
    }
    
    /// Get the first selected entity (if any)
    pub fn first_selected(&self) -> Option<Entity> {
        self.selected_entities.iter().next().copied()
    }
    
    /// Push an action to the undo stack and clear redo stack
    pub fn push_action(&mut self, action: EditorAction) {
        self.undo_stack.push(action);
        
        // Limit undo history size
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }
        
        // Clear redo stack when new action is performed
        self.redo_stack.clear();
        
        // Mark as modified
        self.is_modified = true;
    }
    
    /// Pop an action from the undo stack
    pub fn pop_undo(&mut self) -> Option<EditorAction> {
        self.undo_stack.pop()
    }
    
    /// Push an action to the redo stack
    pub fn push_redo(&mut self, action: EditorAction) {
        self.redo_stack.push(action);
        
        // Limit redo history size
        if self.redo_stack.len() > MAX_UNDO_HISTORY {
            self.redo_stack.remove(0);
        }
    }
    
    /// Pop an action from the redo stack
    pub fn pop_redo(&mut self) -> Option<EditorAction> {
        self.redo_stack.pop()
    }
    
    /// Check if undo is available
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    
    /// Check if redo is available
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
    
    /// Clear all undo/redo history
    pub fn clear_history(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Editor action for undo/redo system
#[derive(Debug, Clone)]
pub enum EditorAction {
    /// Transform was changed for an entity
    TransformEntity {
        entity: Entity,
        old_transform: Transform,
        new_transform: Transform,
    },
    /// Entity was added
    AddEntity {
        entity: Entity,
    },
    /// Entity was deleted (stores data for recreation)
    DeleteEntity {
        entity: Entity,
        transform: Transform,
        entity_type: String,
        serialized_data: String,
    },
    /// Component was modified
    ModifyComponent {
        entity: Entity,
        component_type: String,
        old_value: String,
        new_value: String,
    },
    /// Multiple entities were transformed
    TransformEntities {
        entities: Vec<(Entity, Transform, Transform)>, // (entity, old, new)
    },
    /// Multiple entities were deleted
    DeleteEntities {
        entities: Vec<(Entity, Transform, String, String)>, // (entity, transform, entity_type, serialized_data)
    },
    /// Multiple entities were added
    AddEntities {
        entities: Vec<Entity>,
    },
}

/// Editor mode for the map editor
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorMode {
    #[default]
    Select,
    Translate,
    Rotate,
    Scale,
    Add,
    Delete,
}

impl EditorMode {
    /// Get a display name for the mode
    pub fn display_name(&self) -> &'static str {
        match self {
            EditorMode::Select => "Select",
            EditorMode::Translate => "Translate",
            EditorMode::Rotate => "Rotate",
            EditorMode::Scale => "Scale",
            EditorMode::Add => "Add",
            EditorMode::Delete => "Delete",
        }
    }
}

/// Transform space for gizmos
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransformSpace {
    #[default]
    World,
    Local,
}

impl TransformSpace {
    /// Get a display name for the transform space
    pub fn display_name(&self) -> &'static str {
        match self {
            TransformSpace::World => "World",
            TransformSpace::Local => "Local",
        }
    }
}

/// Selection mode for the map editor
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionMode {
    #[default]
    Single,
    Multi,
    Area,
}

/// Hierarchy filter options
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HierarchyFilter {
    #[default]
    All,
    DecoObjects,
    CnstObjects,
    EventObjects,
    WarpObjects,
    Terrain,
    Water,
    Effects,
    Sounds,
}

impl HierarchyFilter {
    /// Get a display name for the filter
    pub fn display_name(&self) -> &'static str {
        match self {
            HierarchyFilter::All => "All",
            HierarchyFilter::DecoObjects => "Deco Objects",
            HierarchyFilter::CnstObjects => "Cnst Objects",
            HierarchyFilter::EventObjects => "Event Objects",
            HierarchyFilter::WarpObjects => "Warp Objects",
            HierarchyFilter::Terrain => "Terrain",
            HierarchyFilter::Water => "Water",
            HierarchyFilter::Effects => "Effects",
            HierarchyFilter::Sounds => "Sounds",
        }
    }
}

/// Model category for the model browser
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelCategory {
    #[default]
    All,
    Deco,
    Cnst,
    Event,
    Special,
}

impl ModelCategory {
    /// Get a display name for the category
    pub fn display_name(&self) -> &'static str {
        match self {
            ModelCategory::All => "All",
            ModelCategory::Deco => "Decoration",
            ModelCategory::Cnst => "Construction",
            ModelCategory::Event => "Event",
            ModelCategory::Special => "Special",
        }
    }
}

/// Information about a single model available for placement
#[derive(Debug, Clone)]
pub struct ModelInfo {
    /// Unique ID from the ZSC file
    pub id: u32,
    /// Display name (derived from mesh path or ID)
    pub name: String,
    /// Path to the primary mesh
    pub mesh_path: String,
    /// Category of the model (Deco, Cnst, Event, Special)
    pub category: ModelCategory,
    /// Number of parts in the model
    pub part_count: usize,
}

impl ModelInfo {
    /// Create a new ModelInfo with basic information
    pub fn new(id: u32, name: String, mesh_path: String, category: ModelCategory, part_count: usize) -> Self {
        Self {
            id,
            name,
            mesh_path,
            category,
            part_count,
        }
    }
    
    /// Get a short name for display (just the file name without extension)
    pub fn short_name(&self) -> &str {
        // Extract just the file name from the path
        if let Some(pos) = self.name.rfind('/') {
            &self.name[pos + 1..]
        } else if let Some(pos) = self.name.rfind('\\') {
            &self.name[pos + 1..]
        } else {
            &self.name
        }
    }
}

/// Resource containing all available models organized by category
#[derive(Resource, Default, Debug)]
pub struct AvailableModels {
    /// Decoration models (from zsc_deco.txt)
    pub deco_models: Vec<ModelInfo>,
    /// Construction models (from zsc_cnst.txt)
    pub cnst_models: Vec<ModelInfo>,
    /// Event object models (from zsc_event_object.txt)
    pub event_models: Vec<ModelInfo>,
    /// Special/warp object models
    pub special_models: Vec<ModelInfo>,
}

impl AvailableModels {
    /// Get total count of all models
    pub fn total_count(&self) -> usize {
        self.deco_models.len() + self.cnst_models.len() + self.event_models.len() + self.special_models.len()
    }
    
    /// Get models for a specific category
    pub fn get_models(&self, category: ModelCategory) -> &[ModelInfo] {
        match category {
            ModelCategory::Deco => &self.deco_models,
            ModelCategory::Cnst => &self.cnst_models,
            ModelCategory::Event => &self.event_models,
            ModelCategory::Special => &self.special_models,
            ModelCategory::All => &[], // Use all_models() iterator instead
        }
    }
    
    /// Get mutable models for a specific category
    pub fn get_models_mut(&mut self, category: ModelCategory) -> &mut Vec<ModelInfo> {
        match category {
            ModelCategory::Deco => &mut self.deco_models,
            ModelCategory::Cnst => &mut self.cnst_models,
            ModelCategory::Event => &mut self.event_models,
            ModelCategory::Special => &mut self.special_models,
            ModelCategory::All => &mut self.deco_models, // Fallback
        }
    }
    
    /// Find a model by ID across all categories
    pub fn find_by_id(&self, id: u32, category: ModelCategory) -> Option<&ModelInfo> {
        let models = self.get_models(category);
        models.iter().find(|m| m.id == id)
    }
    
    /// Check if any models are loaded
    pub fn is_empty(&self) -> bool {
        self.deco_models.is_empty()
            && self.cnst_models.is_empty()
            && self.event_models.is_empty()
            && self.special_models.is_empty()
    }
}

/// Resource to track the currently selected model for placement
#[derive(Resource, Default, Debug)]
pub struct SelectedModel {
    /// The selected model info (if any)
    pub model: Option<ModelInfo>,
    /// Whether the model browser panel is visible
    pub browser_visible: bool,
    /// Currently selected category tab in the browser
    pub selected_category: ModelCategory,
    /// Scroll position in the model list
    pub scroll_position: f32,
    /// Search filter text for the model list
    pub search_filter: String,
    /// Flag to indicate that a placement is pending (user clicked "Add to Zone")
    pub pending_placement: bool,
}

impl SelectedModel {
    /// Create a new SelectedModel with default values
    pub fn new() -> Self {
        Self {
            model: None,
            browser_visible: true,
            selected_category: ModelCategory::Deco,
            scroll_position: 0.0,
            search_filter: String::new(),
            pending_placement: false,
        }
    }
    
    /// Select a model for placement
    pub fn select(&mut self, model: ModelInfo) {
        self.model = Some(model);
    }
    
    /// Clear the selected model
    pub fn clear(&mut self) {
        self.model = None;
    }
    
    /// Check if a model is selected
    pub fn is_selected(&self) -> bool {
        self.model.is_some()
    }
    
    /// Toggle browser visibility
    pub fn toggle_browser(&mut self) {
        self.browser_visible = !self.browser_visible;
    }
    
    /// Check if there's a pending placement and clear the flag
    pub fn take_pending_placement(&mut self) -> bool {
        let pending = self.pending_placement;
        self.pending_placement = false;
        pending
    }
}

/// Resource to track grid settings
#[derive(Resource, Clone)]
pub struct EditorGridSettings {
    /// Whether the grid is visible
    pub visible: bool,
    /// Grid cell size
    pub cell_size: f32,
    /// Grid extent (total size)
    pub extent: f32,
    /// Grid color
    pub color: Color,
}

impl Default for EditorGridSettings {
    fn default() -> Self {
        Self {
            visible: true,
            cell_size: 1.0,
            extent: 100.0,
            color: Color::srgba(0.5, 0.5, 0.5, 0.5),
        }
    }
}

/// Type of zone object for deletion tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneObjectType {
    Deco,
    Cnst,
    Event,
    Warp,
    Sound,
    Effect,
    Animated,
}

/// Resource to track deleted zone objects for save system
/// 
/// When objects are deleted in the editor, they're removed from the Bevy world.
/// The save system pre-populates from existing IFO data which still contains the deleted objects.
/// This resource tracks deletions so the save system can remove them from the export data.
#[derive(Resource, Default, Debug)]
pub struct DeletedZoneObjects {
    /// List of deleted objects: (block_x, block_y, ifo_object_id, object_type)
    pub objects: Vec<(u32, u32, usize, ZoneObjectType)>,
}

impl DeletedZoneObjects {
    /// Add a deleted object to the tracking list
    pub fn add(&mut self, block_x: u32, block_y: u32, ifo_object_id: usize, object_type: ZoneObjectType) {
        self.objects.push((block_x, block_y, ifo_object_id, object_type));
    }
    
    /// Clear all tracked deletions (call after successful save)
    pub fn clear(&mut self) {
        self.objects.clear();
    }
    
    /// Check if there are any tracked deletions
    pub fn is_empty(&self) -> bool {
        self.objects.is_empty()
    }
    
    /// Get the count of tracked deletions
    pub fn len(&self) -> usize {
        self.objects.len()
    }
}
