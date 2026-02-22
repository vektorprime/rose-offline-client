//! Zone Loading Debug Diagnostics
//! 
//! This module provides comprehensive diagnostics for the zone loading system
//! to help diagnose black screen issues and memory leaks.

use std::collections::HashMap;
use bevy::prelude::*;
use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::render::mesh::Mesh3d;

/// Resource for tracking detailed zone loading diagnostics
#[derive(Resource, Default, Debug)]
pub struct ZoneDebugDiagnostics {
    /// Total entities spawned across all zones
    pub total_entities_spawned: usize,
    /// Total entities despawned
    pub total_entities_despawned: usize,
    /// Current active entity count
    pub active_entity_count: usize,
    /// Entities spawned per frame (for detecting runaway spawning)
    pub entities_spawned_this_frame: usize,
    /// Entities by type for debugging visibility issues
    pub entities_by_type: HashMap<String, usize>,
    /// Asset handle reference counts by type
    pub asset_references: HashMap<String, usize>,
    /// Last frame entity count (for detecting leaks)
    pub last_frame_entity_count: usize,
    /// Frames since last spawn (for detecting stalled loading)
    pub frames_since_last_spawn: usize,
    /// System execution counts
    pub system_execution_counts: HashMap<String, usize>,
    /// Zone load events processed
    pub zone_load_events_processed: usize,
    /// Memory tracking samples
    pub memory_samples: Vec<MemorySample>,
    /// Maximum memory samples to keep
    pub max_memory_samples: usize,
}

/// Memory sample for tracking over time
#[derive(Debug, Clone)]
pub struct MemorySample {
    pub timestamp: std::time::Instant,
    pub entity_count: usize,
    pub mesh_count: usize,
    pub material_count: usize,
    pub texture_count: usize,
    pub estimated_memory_mb: f64,
}

/// Component to mark entities for debug tracking
#[derive(Component, Debug)]
pub struct ZoneEntityDebugInfo {
    pub entity_type: String,
    pub spawn_time: std::time::Instant,
    pub has_mesh: bool,
    pub has_material: bool,
    pub has_visibility: bool,
}

impl ZoneDebugDiagnostics {
    pub fn new() -> Self {
        Self {
            max_memory_samples: 100,
            ..Default::default()
        }
    }

    /// Log entity spawn with type tracking
    pub fn log_entity_spawn(&mut self, entity_type: &str, has_mesh: bool, has_material: bool, has_visibility: bool) {
        self.total_entities_spawned += 1;
        self.active_entity_count = self.total_entities_spawned.saturating_sub(self.total_entities_despawned);
        self.entities_spawned_this_frame += 1;
        self.frames_since_last_spawn = 0;
        
        *self.entities_by_type.entry(entity_type.to_string()).or_insert(0) += 1;
        
        log::info!(
            "[ZONE DEBUG] Entity spawned: type={}, has_mesh={}, has_material={}, has_visibility={}, total_active={}",
            entity_type, has_mesh, has_material, has_visibility, self.active_entity_count
        );
    }

    /// Log entity despawn
    pub fn log_entity_despawn(&mut self, entity_type: &str) {
        self.total_entities_despawned += 1;
        self.active_entity_count = self.total_entities_spawned.saturating_sub(self.total_entities_despawned);
        
        if let Some(count) = self.entities_by_type.get_mut(entity_type) {
            *count = count.saturating_sub(1);
        }
        
        log::info!(
            "[ZONE DEBUG] Entity despawned: type={}, total_active={}",
            entity_type, self.active_entity_count
        );
    }

    /// Log system execution
    pub fn log_system_execution(&mut self, system_name: &str) {
        *self.system_execution_counts.entry(system_name.to_string()).or_insert(0) += 1;
    }

    /// Add memory sample
    pub fn add_memory_sample(&mut self, entity_count: usize, mesh_count: usize, material_count: usize, texture_count: usize) {
        // Estimate memory usage (very rough approximation)
        let estimated_memory_mb = 
            (entity_count as f64 * 0.5) +  // ~0.5KB per entity
            (mesh_count as f64 * 2.0) +    // ~2MB per mesh (varies greatly)
            (material_count as f64 * 0.1) + // ~0.1MB per material
            (texture_count as f64 * 5.0);   // ~5MB per texture (varies greatly)

        let sample = MemorySample {
            timestamp: std::time::Instant::now(),
            entity_count,
            mesh_count,
            material_count,
            texture_count,
            estimated_memory_mb,
        };

        self.memory_samples.push(sample);
        
        // Keep only recent samples
        if self.memory_samples.len() > self.max_memory_samples {
            self.memory_samples.remove(0);
        }
    }

    /// Check for memory leak patterns
    pub fn detect_memory_leak(&self) -> Option<String> {
        if self.memory_samples.len() < 10 {
            return None;
        }

        // Check if entity count is consistently growing
        let recent_samples = &self.memory_samples[self.memory_samples.len().saturating_sub(10)..];
        let first_count = recent_samples.first()?.entity_count;
        let last_count = recent_samples.last()?.entity_count;
        
        // If entities grew by more than 50% in last 10 samples, likely a leak
        if last_count > first_count && (last_count - first_count) > (first_count / 2) {
            return Some(format!(
                "Possible memory leak detected: entities grew from {} to {} in recent samples",
                first_count, last_count
            ));
        }

        None
    }

    /// Get growth rate (entities per second)
    pub fn get_entity_growth_rate(&self) -> f64 {
        if self.memory_samples.len() < 2 {
            return 0.0;
        }

        let first = self.memory_samples.first().unwrap();
        let last = self.memory_samples.last().unwrap();
        
        let duration_secs = last.timestamp.duration_since(first.timestamp).as_secs_f64();
        if duration_secs <= 0.0 {
            return 0.0;
        }

        let entity_diff = last.entity_count as f64 - first.entity_count as f64;
        entity_diff / duration_secs
    }

    /// Reset frame counters
    pub fn end_frame(&mut self) {
        self.last_frame_entity_count = self.active_entity_count;
        self.entities_spawned_this_frame = 0;
        self.frames_since_last_spawn += 1;
    }

    /// Log comprehensive summary
    pub fn log_summary(&self) {
        log::info!("========================================");
        log::info!("ZONE DEBUG DIAGNOSTICS SUMMARY");
        log::info!("========================================");
        log::info!("Total entities spawned: {}", self.total_entities_spawned);
        log::info!("Total entities despawned: {}", self.total_entities_despawned);
        log::info!("Active entities: {}", self.active_entity_count);
        log::info!("Entities spawned this frame: {}", self.entities_spawned_this_frame);
        log::info!("Frames since last spawn: {}", self.frames_since_last_spawn);
        
        log::info!("\nEntities by type:");
        for (entity_type, count) in &self.entities_by_type {
            log::info!("  {}: {}", entity_type, count);
        }

        log::info!("\nSystem execution counts:");
        for (system, count) in &self.system_execution_counts {
            log::info!("  {}: {}", system, count);
        }

        if let Some(leak_warning) = self.detect_memory_leak() {
            log::warn!("\n!!! {} !!!", leak_warning);
        }

        let growth_rate = self.get_entity_growth_rate();
        if growth_rate > 10.0 {
            log::warn!("\n!!! High entity growth rate: {:.2} entities/second !!!", growth_rate);
        }

        log::info!("========================================");
    }
}

/// System to update zone debug diagnostics every frame
pub fn zone_debug_diagnostics_system(
    mut diagnostics: ResMut<ZoneDebugDiagnostics>,
    query: Query<(Entity, Option<&ZoneEntityDebugInfo>)>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
) {
    // Update entity counts
    let total_entities = query.iter().count();
    diagnostics.active_entity_count = total_entities;

    // Add memory sample
    diagnostics.add_memory_sample(
        total_entities,
        meshes.len(),
        0, // Would need access to materials
        images.len(),
    );

    // Log summary periodically
    if diagnostics.total_entities_spawned > 0 && diagnostics.total_entities_spawned % 100 == 0 {
        diagnostics.log_summary();
    }

    diagnostics.end_frame();
}

/// Diagnostic system to check child entity visibility components
/// This helps identify why child entities (terrain, objects, water) are not visible
pub fn zone_child_visibility_diagnostic_system(
    zone_query: Query<(Entity, &Visibility, &InheritedVisibility, &ViewVisibility, Option<&GlobalTransform>), With<crate::components::Zone>>,
    child_query: Query<(
        Entity,
        &Visibility,
        &InheritedVisibility,
        &ViewVisibility,
        Option<&GlobalTransform>,
        Option<&Mesh3d>,
        Option<&ChildOf>
    ), Without<crate::components::Zone>>,
    meshes: Res<Assets<Mesh>>,
) {
    // ChildOf is now in bevy::prelude, already imported

    // Only run this diagnostic once after zone is loaded
    // This is controlled by a frame counter in the actual system
    let should_run = std::env::var("RUN_VISIBILITY_DIAGNOSTIC").is_ok();
    if !should_run {
        return;
    }

    log::info!("========================================");
    log::info!("ZONE CHILD ENTITY VISIBILITY DIAGNOSTIC");
    log::info!("========================================");

    // Check zone entity visibility
    for (zone_entity, visibility, inherited_visibility, view_visibility, global_transform) in zone_query.iter() {
        log::info!("ZONE ENTITY: {:?}", zone_entity);
        log::info!("  Visibility: {:?}", visibility);
        log::info!("  InheritedVisibility: {:?}", inherited_visibility);
        log::info!("  ViewVisibility: {:?}", view_visibility);
        if let Some(transform) = global_transform {
            log::info!("  Position: {:?}", transform.translation());
        }
        log::info!("  Has Mesh: false (zone is container only)");
    }

    log::info!("");
    log::info!("CHILD ENTITIES (Terrain, Objects, Water, etc.):");
    log::info!("");

    // Check child entities
    let mut child_count = 0;
    let mut visible_children = 0;
    let mut invisible_children = 0;
    let mut children_without_mesh = 0;
    let mut children_without_view_visibility = 0;

    for (entity, visibility, inherited_visibility, view_visibility, global_transform, mesh_handle, parent) in child_query.iter() {
        child_count += 1;

        // Check if entity has a mesh
        let has_mesh = mesh_handle.is_some();
        if !has_mesh {
            children_without_mesh += 1;
        }

        // Check ViewVisibility
        let is_visible = view_visibility.get();
        if is_visible {
            visible_children += 1;
        } else {
            invisible_children += 1;
        }

        // Check if ViewVisibility component exists
        if view_visibility.get() == false {
            children_without_view_visibility += 1;
        }

        // Log first 10 children for detailed analysis
        if child_count <= 10 {
            log::info!("CHILD ENTITY #{:?}: {:?}", child_count, entity);
            log::info!("  Visibility: {:?}", visibility);
            log::info!("  InheritedVisibility: {:?}", inherited_visibility);
            log::info!("  ViewVisibility: {:?}", view_visibility);
            if let Some(transform) = global_transform {
                log::info!("  Position: {:?}", transform.translation());
            }
            log::info!("  Has Mesh: {}", has_mesh);
            if let Some(parent) = parent {
                log::info!("  Parent: {:?}", parent.get());
            }
                if let Some(mesh) = mesh_handle {
                    log::info!("  Mesh Handle: {:?}", mesh);
                    if let Some(mesh_asset) = meshes.get(mesh) {
                        log::info!("  Mesh Vertices: {}", mesh_asset.count_vertices());
                        log::info!("  Mesh Primitives: {:?}", mesh_asset.primitive_topology());
                    }
                }
            log::info!("");
        }
    }

    log::info!("========================================");
    log::info!("SUMMARY:");
    log::info!("========================================");
    log::info!("Total child entities: {}", child_count);
    log::info!("Visible children (ViewVisibility=true): {}", visible_children);
    log::info!("Invisible children (ViewVisibility=false): {}", invisible_children);
    log::info!("Children without Mesh: {}", children_without_mesh);
    log::info!("Children with ViewVisibility=false: {}", children_without_view_visibility);

    if invisible_children > 0 {
        log::warn!("!!! WARNING: {} child entities are NOT VISIBLE !!!", invisible_children);
    }

    if children_without_mesh > 0 {
        log::warn!("!!! WARNING: {} child entities have NO MESH component !!!", children_without_mesh);
    }

    log::info!("========================================");
}

/// Plugin to add zone debug diagnostics
pub struct ZoneDebugDiagnosticsPlugin;

impl Plugin for ZoneDebugDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneDebugDiagnostics>()
           .add_systems(Update, zone_debug_diagnostics_system)
           .add_systems(Update, zone_child_visibility_diagnostic_system);
    }
}