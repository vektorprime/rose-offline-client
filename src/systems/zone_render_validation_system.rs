//! Zone Render Validation System
//!
//! This system validates that zone entities have the required components
//! for rendering and helps diagnose black screen issues.

use bevy::prelude::*;
use bevy::render::{mesh::Mesh, primitives::Aabb, view::Visibility};

use crate::components::{Zone, ZoneObject};
use crate::resources::zone_debug_diagnostics::ZoneDebugDiagnostics;

/// Component to mark entities that failed validation
#[derive(Component, Debug)]
pub struct RenderValidationFailure {
    pub reason: String,
    pub detected_at: std::time::Instant,
}

/// Resource to track render validation statistics
#[derive(Resource, Default, Debug)]
pub struct RenderValidationStats {
    pub total_entities_checked: usize,
    pub entities_with_mesh: usize,
    pub entities_with_material: usize,
    pub entities_with_visibility: usize,
    pub entities_with_transform: usize,
    pub entities_with_global_transform: usize,
    pub entities_failing_validation: usize,
    pub validation_failures_by_reason: std::collections::HashMap<String, usize>,
    pub last_validation_time: Option<std::time::Instant>,
}

impl RenderValidationStats {
    pub fn reset(&mut self) {
        self.total_entities_checked = 0;
        self.entities_with_mesh = 0;
        self.entities_with_material = 0;
        self.entities_with_visibility = 0;
        self.entities_with_transform = 0;
        self.entities_with_global_transform = 0;
        self.entities_failing_validation = 0;
        self.validation_failures_by_reason.clear();
        self.last_validation_time = Some(std::time::Instant::now());
    }

    pub fn log_summary(&self) {
        log::info!("========================================");
        log::info!("RENDER VALIDATION SUMMARY");
        log::info!("========================================");
        log::info!("Total entities checked: {}", self.total_entities_checked);
        log::info!("Entities with Mesh: {}", self.entities_with_mesh);
        log::info!("Entities with Material: {}", self.entities_with_material);
        log::info!("Entities with Visibility: {}", self.entities_with_visibility);
        log::info!("Entities with Transform: {}", self.entities_with_transform);
        log::info!("Entities with GlobalTransform: {}", self.entities_with_global_transform);
        log::info!("Entities failing validation: {}", self.entities_failing_validation);
        
        if !self.validation_failures_by_reason.is_empty() {
            log::warn!("\nValidation failures by reason:");
            for (reason, count) in &self.validation_failures_by_reason {
                log::warn!("  {}: {}", reason, count);
            }
        }
        log::info!("========================================");
    }
}

/// System to validate zone entities have required render components
/// This helps diagnose black screen issues
/// Uses combined queries to stay within Bevy's system parameter limit
pub fn zone_render_validation_system(
    mut stats: ResMut<RenderValidationStats>,
    // Combined query for Zone entities
    zone_query: Query<(Entity, Option<&Children>), With<Zone>>,
    // Combined query for ZoneObject entities with all render components - using StandardMaterial
    zone_object_query: Query<(Entity, Option<&Children>, Option<&Handle<Mesh>>, Option<&Visibility>, Option<&Transform>, Option<&GlobalTransform>, Option<&Handle<StandardMaterial>>), With<ZoneObject>>,
) {
    // Only run validation every 60 frames (approx 1 second at 60fps)
    static mut FRAME_COUNTER: usize = 0;
    let should_run = unsafe {
        FRAME_COUNTER += 1;
        FRAME_COUNTER % 60 == 0
    };
    
    if !should_run {
        return;
    }

    stats.reset();
    
    log::info!("[RENDER VALIDATION] Starting zone entity validation...");

    // Track zone objects with valid render components
    let mut zone_objects_with_mesh: std::collections::HashSet<Entity> = std::collections::HashSet::new();
    let mut zone_objects_with_material: std::collections::HashSet<Entity> = std::collections::HashSet::new();

    // Validate ZoneObject entities
    log::info!("[RENDER VALIDATION] Checking ZoneObject entities...");
    let mut zone_object_count = 0;
    
    for (entity, children, mesh, visibility, transform, global_transform, material) in zone_object_query.iter() {
        zone_object_count += 1;
        
        // Only validate entities that have meshes (the actual renderable parts)
        if let Some(mesh_handle) = mesh {
            let has_mesh = true;
            let has_material: bool = material.is_some();
            let has_visibility: bool = visibility.is_some();
            let is_visible = matches!(visibility, Some(Visibility::Visible));
            let has_transform: bool = transform.is_some();
            let has_global_transform: bool = global_transform.is_some();
            let mesh_handle: &Handle<Mesh> = mesh_handle;

            if has_mesh { stats.entities_with_mesh += 1; }
            if has_material { stats.entities_with_material += 1; }
            if has_visibility { stats.entities_with_visibility += 1; }
            if has_transform { stats.entities_with_transform += 1; }
            if has_global_transform { stats.entities_with_global_transform += 1; }
            stats.total_entities_checked += 1;

            // Track entities
            zone_objects_with_mesh.insert(entity);
            if has_material {
                zone_objects_with_material.insert(entity);
            }

            // Only log failures, not every entity
            if !has_material || !has_transform || !has_global_transform {
                let mut failures = Vec::new();
                if !has_material { failures.push("Missing Material"); }
                if !has_visibility { failures.push("Missing Visibility"); }
                if !is_visible { failures.push("Not Visible"); }
                if !has_transform { failures.push("Missing Transform"); }
                if !has_global_transform { failures.push("Missing GlobalTransform"); }
                
                stats.entities_failing_validation += 1;
                let reason = failures.join(", ");
                *stats.validation_failures_by_reason.entry(reason.clone()).or_insert(0) += 1;
                
                if !has_material {
                    log::warn!("[RENDER VALIDATION] Entity {:?} MISSING MATERIAL - mesh_id={:?}",
                        entity, mesh_handle.id());
                }
                
                log::warn!("[RENDER VALIDATION] ZoneObject {:?} failed: {}", entity, reason);
            }
        }
        
        // Also track parent ZoneObjects that have children with mesh/material
        if let Some(children) = children {
            for &child in children.iter() {
                // Check if child has mesh/material by querying it
                if let Ok((_, _, child_mesh, _, _, _, child_material)) = zone_object_query.get(child) {
                    let child_mesh: Option<&Handle<Mesh>> = child_mesh;
                    let child_material: Option<&Handle<StandardMaterial>> = child_material;
                    if child_mesh.is_some() {
                        zone_objects_with_mesh.insert(entity);
                    }
                    if child_material.is_some() {
                        zone_objects_with_material.insert(entity);
                    }
                }
            }
        }
    }
    
    log::info!("[RENDER VALIDATION] Checked {} ZoneObject entities", zone_object_count);
    log::info!("[RENDER VALIDATION] ZoneObjects with mesh children: {}, with material children: {}",
        zone_objects_with_mesh.len(), zone_objects_with_material.len());

    // Log summary
    stats.log_summary();

    // If many entities are failing validation, this could explain the black screen
    if stats.entities_failing_validation > stats.total_entities_checked / 2 && stats.total_entities_checked > 0 {
        log::error!("[RENDER VALIDATION] CRITICAL: More than 50% of entities failed validation!");
        log::error!("[RENDER VALIDATION] This explains the black screen issue - entities have no mesh/material!");
    }
}

/// System to check if assets are actually loaded
pub fn asset_loading_validation_system(
    asset_server: Res<AssetServer>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
    material_meshes: Query<(Entity, &Handle<Mesh>), With<ZoneObject>>,
) {
    use bevy::asset::LoadState;

    static mut FRAME_COUNTER: usize = 0;
    unsafe {
        FRAME_COUNTER += 1;
        if FRAME_COUNTER % 120 != 0 {
            return;
        }
    }

    log::info!("[ASSET VALIDATION] Checking asset load states...");

    let mut not_loaded_meshes = 0;

    // Check mesh load states
    for (entity, mesh_handle) in material_meshes.iter() {
        if let Some(state) = asset_server.get_load_state(mesh_handle) {
            match state {
                LoadState::NotLoaded => {
                    not_loaded_meshes += 1;
                    log::warn!("[ASSET VALIDATION] Entity {:?} mesh not loaded", entity);
                }
                LoadState::Loading => {
                    log::debug!("[ASSET VALIDATION] Entity {:?} mesh still loading", entity);
                }
                LoadState::Loaded => {}
                LoadState::Failed => {
                    log::error!("[ASSET VALIDATION] Entity {:?} mesh load FAILED", entity);
                }
                _ => {}
            }
        }
    }

    if not_loaded_meshes > 0 {
        log::warn!(
            "[ASSET VALIDATION] Assets still loading: {} meshes",
            not_loaded_meshes
        );
    } else {
        log::info!("[ASSET VALIDATION] All checked assets are loaded");
    }

    log::info!("[ASSET VALIDATION] Mesh assets in storage: {}", meshes.len());
    log::info!("[ASSET VALIDATION] Image assets in storage: {}", images.len());
}

/// System to validate camera configuration
pub fn camera_validation_system(
    camera_query: Query<(Entity, &Camera, Option<&Transform>), With<Camera3d>>,
) {
    static mut FRAME_COUNTER: usize = 0;
    unsafe {
        FRAME_COUNTER += 1;
        if FRAME_COUNTER % 120 != 0 {
            return;
        }
    }

    log::info!("[CAMERA VALIDATION] Checking camera configuration...");

    let camera_count = camera_query.iter().count();
    log::info!("[CAMERA VALIDATION] Found {} 3D cameras", camera_count);

    for (entity, camera, transform) in camera_query.iter() {
        log::info!("[CAMERA VALIDATION] Camera {:?}:", entity);
        log::info!("  - Is active: {}", camera.is_active);
        log::info!("  - Order: {}", camera.order);
        
        if let Some(transform) = transform {
            log::info!("  - Position: {:?}", transform.translation);
            log::info!("  - Looking at: (check forward vector)");
        } else {
            log::warn!("  - WARNING: No Transform component!");
        }

        if !camera.is_active {
            log::warn!("[CAMERA VALIDATION] WARNING: Camera {:?} is not active!", entity);
        }
    }

    if camera_count == 0 {
        log::error!("[CAMERA VALIDATION] CRITICAL: No 3D cameras found! This explains the black screen!");
    }
}

/// ENHANCED DIAGNOSTIC: System to inspect actual mesh data and validate it's renderable
/// This checks vertex counts, indices, and required attributes
pub fn mesh_inspection_system(
    meshes: Res<Assets<Mesh>>,
    mesh_query: Query<(Entity, &Handle<Mesh>), With<ZoneObject>>,
) {
    use bevy::render::mesh::VertexAttributeValues;
    
    static mut FRAME_COUNTER: usize = 0;
    static mut INSPECT_INDEX: usize = 0;
    
    let should_run = unsafe {
        FRAME_COUNTER += 1;
        FRAME_COUNTER % 30 == 0
    };
    
    if !should_run {
        return;
    }
    
    let mesh_list: Vec<_> = mesh_query.iter().collect();
    if mesh_list.is_empty() {
        return;
    }
    
    // Inspect up to 10 meshes per frame (round-robin)
    const MAX_INSPECT_PER_FRAME: usize = 10;
    let inspect_start = unsafe { INSPECT_INDEX };
    let inspect_end = (inspect_start + MAX_INSPECT_PER_FRAME).min(mesh_list.len());
    
    for i in inspect_start..inspect_end {
        let (entity, mesh_handle) = mesh_list[i];
        
        if let Some(mesh) = meshes.get(mesh_handle) {
            // Check for required attributes
            let has_positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).is_some();
            let has_normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some();
            let has_uvs = mesh.attribute(Mesh::ATTRIBUTE_UV_0).is_some();
            let indices_count = mesh.indices().map(|i| i.len()).unwrap_or(0);
            
            // Validate vertex count
            let vertex_count = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
                .map(|attr| match attr {
                    VertexAttributeValues::Float32x3(v) => v.len(),
                    _ => 0,
                })
                .unwrap_or(0);
            
            // Log any issues
            if vertex_count == 0 {
                log::error!("[MESH INSPECTION] Entity {:?} has mesh with 0 vertices! Handle: {:?}", 
                    entity, mesh_handle);
            } else if indices_count == 0 {
                log::error!("[MESH INSPECTION] Entity {:?} has mesh with 0 indices! Vertices: {}", 
                    entity, vertex_count);
            } else if !has_positions {
                log::error!("[MESH INSPECTION] Entity {:?} mesh missing POSITION attribute!", entity);
            } else if vertex_count > 0 && vertex_count < 3 {
                log::warn!("[MESH INSPECTION] Entity {:?} has suspiciously low vertex count: {}", 
                    entity, vertex_count);
            }
            
            // Log detailed info for first few meshes each cycle
            if i < 3 {
                log::info!("[MESH INSPECTION] Entity {:?}: {} vertices, {} indices, positions={}, normals={}, uvs={}", 
                    entity, vertex_count, indices_count, has_positions, has_normals, has_uvs);
            }
        } else {
            log::warn!("[MESH INSPECTION] Entity {:?} has mesh handle but asset not loaded! Handle: {:?}", 
                entity, mesh_handle);
        }
    }
    
    unsafe {
        INSPECT_INDEX = if inspect_end >= mesh_list.len() { 0 } else { inspect_end };
    }
}

/// ENHANCED DIAGNOSTIC: System to validate material assets are loaded and valid
/// Checks StandardMaterial used by zone objects
pub fn material_validation_system(
    standard_materials: Res<Assets<StandardMaterial>>,
    standard_mat_query: Query<(Entity, &Handle<StandardMaterial>), With<ZoneObject>>,
) {
    static mut FRAME_COUNTER: usize = 0;
    unsafe {
        FRAME_COUNTER += 1;
        if FRAME_COUNTER % 60 != 0 {
            return;
        }
    }
    
    let mut not_loaded_count = 0;
    let mut total_checked = 0;
    
    // Validate StandardMaterials
    for (entity, mat_handle) in standard_mat_query.iter() {
        total_checked += 1;
        if standard_materials.get(mat_handle).is_none() {
            not_loaded_count += 1;
            log::warn!("[MATERIAL VALIDATION] Entity {:?} StandardMaterial not loaded! Handle: {:?}",
                entity, mat_handle);
        }
    }
    
    if not_loaded_count > 0 {
        log::warn!("[MATERIAL VALIDATION] {}/{} materials not loaded!", not_loaded_count, total_checked);
    } else if total_checked > 0 {
        log::info!("[MATERIAL VALIDATION] All {} materials are loaded", total_checked);
    }
}

/// ENHANCED DIAGNOSTIC: System to trace entity counts per frame
/// Tracks spawn/despawn rates and detects abnormal growth
#[derive(Resource, Default)]
pub struct EntityFrameTracer {
    pub frame_count: u64,
    pub last_entity_count: usize,
    pub spawn_count_this_frame: usize,
    pub despawn_count_this_frame: usize,
}

pub fn entity_count_tracing_system(
    mut tracer: ResMut<EntityFrameTracer>,
    all_entities: Query<Entity>,
) {
    tracer.frame_count += 1;
    
    let current_count = all_entities.iter().count();
    let delta = current_count as i64 - tracer.last_entity_count as i64;
    
    if delta > 0 {
        tracer.spawn_count_this_frame += delta as usize;
    } else if delta < 0 {
        tracer.despawn_count_this_frame += (-delta) as usize;
    }
    
    // Log every 60 frames (approx 1 second at 60fps)
    if tracer.frame_count % 60 == 0 {
        log::info!("[ENTITY TRACE] Frame {}: {} entities (spawned: {}, despawned: {}, delta: {})",
            tracer.frame_count,
            current_count,
            tracer.spawn_count_this_frame,
            tracer.despawn_count_this_frame,
            delta
        );
        
        // Reset counters
        tracer.spawn_count_this_frame = 0;
        tracer.despawn_count_this_frame = 0;
    }
    
    tracer.last_entity_count = current_count;
}

/// DIAGNOSTIC: System to check if mesh entities have AABB components
/// This helps diagnose if missing AABBs are causing frustum culling to mark all meshes as invisible
pub fn aabb_diagnostic_system(
    mesh_query: Query<(Entity, &Handle<Mesh>, Option<&Aabb>, Option<&Visibility>), With<ZoneObject>>,
) {
    static mut FRAME_COUNTER: usize = 0;
    unsafe {
        FRAME_COUNTER += 1;
        if FRAME_COUNTER % 60 != 0 {
            return;
        }
    }

    log::info!("========================================");
    log::info!("AABB COMPONENT DIAGNOSTIC");
    log::info!("========================================");

    let mut total_mesh_entities = 0;
    let mut entities_with_aabb = 0;
    let mut entities_without_aabb = 0;
    let mut visible_entities = 0;
    let mut visible_without_aabb = 0;

    for (entity, mesh_handle, aabb, visibility) in mesh_query.iter() {
        total_mesh_entities += 1;

        let has_aabb = aabb.is_some();
        let is_visible = matches!(visibility, Some(Visibility::Visible));

        if has_aabb {
            entities_with_aabb += 1;
        } else {
            entities_without_aabb += 1;
        }

        if is_visible {
            visible_entities += 1;
            if !has_aabb {
                visible_without_aabb += 1;
            }
        }

        // Log first few entities for debugging
        if total_mesh_entities <= 5 {
            log::info!(
                "Entity {:?}: mesh_id={:?}, has_aabb={}, is_visible={}",
                entity,
                mesh_handle.id(),
                has_aabb,
                is_visible
            );
        }
    }

    log::info!("Total mesh entities: {}", total_mesh_entities);
    log::info!("Entities WITH AABB: {}", entities_with_aabb);
    log::info!("Entities WITHOUT AABB: {}", entities_without_aabb);
    log::info!("Visible entities: {}", visible_entities);
    log::info!("Visible entities WITHOUT AABB: {}", visible_without_aabb);

    if entities_without_aabb > 0 {
        log::warn!(
            "WARNING: {}/{} mesh entities are missing AABB components!",
            entities_without_aabb,
            total_mesh_entities
        );
        log::warn!(
            "This can cause frustum culling to incorrectly mark meshes as invisible!"
        );
    }

    if visible_without_aabb > 0 {
        log::error!(
            "CRITICAL: {}/{} visible entities are missing AABB components!",
            visible_without_aabb,
            visible_entities
        );
        log::error!(
            "This is likely the cause of the black screen - Bevy's frustum culling requires AABBs!"
        );
    }

    log::info!("========================================");
}

/// Plugin to add all render validation systems
pub struct ZoneRenderValidationPlugin;

impl Plugin for ZoneRenderValidationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderValidationStats>()
           .init_resource::<EntityFrameTracer>()
           .add_systems(Update, (
               zone_render_validation_system,
               asset_loading_validation_system,
               camera_validation_system,
               mesh_inspection_system,
               material_validation_system,
               entity_count_tracing_system,
               aabb_diagnostic_system,
           ));
    }
}
