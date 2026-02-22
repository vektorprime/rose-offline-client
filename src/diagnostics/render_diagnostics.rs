//! Diagnostic logging for rendering crash investigation
//!
//! This module provides comprehensive diagnostic logging to help identify
//! root causes of two specific rendering crashes:
//!
//! **Crash #1 - PipelineCache index out of bounds:**
//! - Error: `index out of bounds: len is 0 but index is 5`
//! - Location: `bevy_render::render_resource::pipeline_cache.rs:587` in `PipelineCache::get_compute_pipeline`
//! - Called from: `bevy_pbr::render::gpu_preprocess::impl$2::run`
//!
//! **Crash #2 - Missing pipeline binding:**
//! - Error: `Shader global ResourceBinding { group: 3, binding: 0 } is not available in the pipeline layout`
//! - Location: Creating render pipeline labeled `alpha_blend_mesh_pipeline`
//!
//! The diagnostic functions in this module log at key decision points in the rendering pipeline
//! to help trace back to what state the rendering system was in when crashes occur.
//! 
use bevy::{
    asset::{AssetId, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        system::{Local, Res, ResMut},
    },
    log,
    prelude::{App, Plugin, Query, Assets, Image, BevyError, Resource},
    render::{
        render_resource::{
            BindGroupLayout, BindGroupLayoutEntry, BindingType, PipelineCache, ShaderStages,
            TextureSampleType, TextureViewDimension,
        },
        render_asset::RenderAssets,
        ExtractSchedule, Render, RenderApp, RenderSet,
    },
    platform::collections::HashMap,
};
use std::fmt;

/// Diagnostic state tracking for rendering
#[derive(Resource, Default)]
pub struct RenderDiagnosticsState {
    /// Frame counter for correlating logs
    pub frame_count: u64,
    /// Track pipeline cache access patterns
    pub pipeline_cache_accesses: Vec<PipelineCacheAccess>,
    /// Track pipeline creation events
    pub pipeline_creations: Vec<PipelineCreationEvent>,
    /// Track alpha blend mesh setup events
    pub alpha_blend_mesh_events: Vec<AlphaBlendMeshEvent>,
    /// Track shader binding configurations
    pub shader_binding_configs: Vec<ShaderBindingConfig>,
}

/// Record of pipeline cache access
#[derive(Debug, Clone)]
pub struct PipelineCacheAccess {
    pub frame: u64,
    pub pipeline_id: Option<u64>,
    pub pipeline_type: PipelineType,
    pub cache_size: usize,
    pub success: bool,
}

/// Type of pipeline being accessed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelineType {
    Render,
    Compute,
    Unknown,
}

impl fmt::Display for PipelineType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PipelineType::Render => write!(f, "Render"),
            PipelineType::Compute => write!(f, "Compute"),
            PipelineType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Record of pipeline creation
#[derive(Debug, Clone)]
pub struct PipelineCreationEvent {
    pub frame: u64,
    pub pipeline_label: String,
    pub pipeline_type: PipelineType,
    pub bind_group_count: usize,
}

/// Record of alpha blend mesh setup
#[derive(Debug, Clone)]
pub struct AlphaBlendMeshEvent {
    pub frame: u64,
    pub entity: Option<Entity>,
    pub model_id: usize,
    pub material_id: usize,
    pub alpha_enabled: bool,
    pub z_write_enabled: bool,
    pub is_alpha_blended: bool,
    pub two_sided: bool,
    pub is_skin: bool,
    pub material_type: String,
}

/// Record of shader binding configuration
#[derive(Debug, Clone)]
pub struct ShaderBindingConfig {
    pub frame: u64,
    pub pipeline_label: String,
    pub group: u32,
    pub binding: u32,
    pub visibility: String,
    pub binding_type: String,
}

/// Plugin to register diagnostic systems
pub struct RenderDiagnosticsPlugin;

impl Plugin for RenderDiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RenderDiagnosticsState>();
        
        // Add diagnostic systems to main world
        app.add_systems(
            bevy::prelude::Update,
            update_frame_counter,
        );
        
        // Add diagnostic systems to render world
        if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
            render_app.init_resource::<RenderDiagnosticsState>();
            render_app.add_systems(
                ExtractSchedule,
                log_render_state_extraction,
            );
        }
        
        log::info!("[RENDER DIAGNOSTICS] RenderDiagnosticsPlugin registered");
    }
}

/// Update frame counter
pub fn update_frame_counter(mut state: ResMut<RenderDiagnosticsState>) -> Result<(), BevyError> {
    state.frame_count += 1;
    Ok(())
}

/// Log render state extraction from main world to render world
pub fn log_render_state_extraction(
    mut state: ResMut<RenderDiagnosticsState>,
) -> Result<(), BevyError> {
    if state.frame_count % 300 == 0 {
        // Log summary every ~5 seconds at 60fps
        // log::info!(
        //     "[RENDER DIAGNOSTICS] Frame {} - Pipeline cache accesses: {}, Pipeline creations: {}, Alpha blend events: {}, Shader binding configs: {}",
        //     state.frame_count,
        //     state.pipeline_cache_accesses.len(),
        //     state.pipeline_creations.len(),
        //     state.alpha_blend_mesh_events.len(),
        //     state.shader_binding_configs.len(),
        // );
    }
    Ok(())
}

/// Log pipeline cache access attempt
///
/// Call this before accessing PipelineCache to log the attempt
/// This helps diagnose Crash #1 (index out of bounds)
pub fn log_pipeline_cache_access(
    state: &mut RenderDiagnosticsState,
    pipeline_id: Option<u64>,
    pipeline_type: PipelineType,
    cache_size: usize,
) {
    let access = PipelineCacheAccess {
        frame: state.frame_count,
        pipeline_id,
        pipeline_type,
        cache_size,
        success: false, // Will be updated after access
    };
    
    state.pipeline_cache_accesses.push(access.clone());
    
    // Log detailed information for each access
    match pipeline_type {
        PipelineType::Compute => {
            log::warn!(
                "[PIPELINE CACHE] Frame {} - Attempting to access COMPUTE pipeline with ID {:?}, cache size: {}",
                state.frame_count,
                pipeline_id,
                cache_size,
            );
            
            // CRITICAL: Log if trying to access index 5 with empty cache (Crash #1 scenario)
            if let Some(id) = pipeline_id {
                if id == 5 && cache_size == 0 {
                    log::error!(
                        "[PIPELINE CACHE] CRITICAL: Attempting to access compute pipeline ID 5 but cache is EMPTY (size=0). This WILL cause index out of bounds crash!",
                    );
                } else if id >= cache_size as u64 {
                    log::error!(
                        "[PIPELINE CACHE] CRITICAL: Attempting to access compute pipeline ID {} but cache only has {} entries. This WILL cause index out of bounds crash!",
                        id,
                        cache_size,
                    );
                }
            }
        }
        PipelineType::Render => {
            log::info!(
                "[PIPELINE CACHE] Frame {} - Accessing RENDER pipeline with ID {:?}, cache size: {}",
                state.frame_count,
                pipeline_id,
                cache_size,
            );
        }
        PipelineType::Unknown => {
            log::warn!(
                "[PIPELINE CACHE] Frame {} - Accessing UNKNOWN pipeline type with ID {:?}, cache size: {}",
                state.frame_count,
                pipeline_id,
                cache_size,
            );
        }
    }
}

/// Log pipeline creation event
///
/// Call this when creating a new pipeline to log the creation
/// This helps diagnose pipeline-related crashes
pub fn log_pipeline_creation(
    state: &mut RenderDiagnosticsState,
    pipeline_label: &str,
    pipeline_type: PipelineType,
    bind_group_count: usize,
) {
    let event = PipelineCreationEvent {
        frame: state.frame_count,
        pipeline_label: pipeline_label.to_string(),
        pipeline_type,
        bind_group_count,
    };

    state.pipeline_creations.push(event.clone());

    log::info!(
        "[PIPELINE CREATION] Frame {} - Creating {} pipeline '{}', bind group count: {}",
        state.frame_count,
        pipeline_type,
        pipeline_label,
        bind_group_count,
    );

    // CRITICAL: Check for alpha_blend_mesh_pipeline (Crash #2 scenario)
    if pipeline_label.contains("alpha_blend") {
        log::warn!(
            "[PIPELINE CREATION] WARNING: Creating alpha-blend pipeline '{}'. Ensure pipeline layout includes all required shader bindings (especially group 3, binding 0 for Crash #2)",
            pipeline_label,
        );
    }
}

/// Log alpha blend mesh setup
///
/// Call this when setting up alpha-blended meshes
/// This helps diagnose Crash #2 (missing pipeline binding)
pub fn log_alpha_blend_mesh_setup(
    state: &mut RenderDiagnosticsState,
    entity: Option<Entity>,
    model_id: usize,
    material_id: usize,
    alpha_enabled: bool,
    z_write_enabled: bool,
    two_sided: bool,
    is_skin: bool,
) {
    let is_alpha_blended = alpha_enabled && !z_write_enabled;
    let material_type = format!("model_id={}, material_id={}", model_id, material_id);

    let event = AlphaBlendMeshEvent {
        frame: state.frame_count,
        entity,
        model_id,
        material_id,
        alpha_enabled,
        z_write_enabled,
        is_alpha_blended,
        two_sided,
        is_skin,
        material_type: material_type.clone(),
    };

    state.alpha_blend_mesh_events.push(event.clone());

    if is_alpha_blended {
        log::info!(
            "[ALPHA BLEND MESH] Frame {} - Entity {:?}: Material '{}' is alpha-blended (alpha_enabled={}, z_write_enabled={}, two_sided={}, is_skin={})",
            state.frame_count,
            entity,
            material_type,
            alpha_enabled,
            z_write_enabled,
            two_sided,
            is_skin,
        );

        log::warn!(
            "[ALPHA BLEND MESH] WARNING: Alpha-blended mesh detected. Ensure pipeline layout includes all required shader bindings (especially group 3, binding 0 for Crash #2)",
        );
    } else {
        log::debug!(
            "[ALPHA BLEND MESH] Frame {} - Entity {:?}: Material '{}' is NOT alpha-blended (alpha_enabled={}, z_write_enabled={})",
            state.frame_count,
            entity,
            material_type,
            alpha_enabled,
            z_write_enabled,
        );
    }
}

/// Log alpha blend mesh setup (simplified version without state)
///
/// Call this when setting up alpha-blended meshes in functions without access to Bevy resources
/// This helps diagnose Crash #2 (missing pipeline binding)
pub fn log_alpha_blend_mesh_setup_simple(
    model_id: usize,
    material_id: usize,
    alpha_enabled: bool,
    z_write_enabled: bool,
    two_sided: bool,
    is_skin: bool,
) {
    let is_alpha_blended = alpha_enabled && !z_write_enabled;
    let material_type = format!("model_id={}, material_id={}", model_id, material_id);

    if is_alpha_blended {
        log::info!(
            "[ALPHA BLEND MESH] Material '{}' is alpha-blended (alpha_enabled={}, z_write_enabled={}, two_sided={}, is_skin={})",
            material_type,
            alpha_enabled,
            z_write_enabled,
            two_sided,
            is_skin,
        );

        log::warn!(
            "[ALPHA BLEND MESH] WARNING: Alpha-blended mesh detected. Ensure pipeline layout includes all required shader bindings (especially group 3, binding 0 for Crash #2)",
        );
    } else {
        log::debug!(
            "[ALPHA BLEND MESH] Material '{}' is NOT alpha-blended (alpha_enabled={}, z_write_enabled={})",
            material_type,
            alpha_enabled,
            z_write_enabled,
        );
    }
}

/// Log shader binding configuration
///
/// Call this when configuring shader bindings
/// This helps diagnose missing binding errors
pub fn log_shader_binding_config(
    state: &mut RenderDiagnosticsState,
    pipeline_label: &str,
    group: u32,
    binding: u32,
    visibility: ShaderStages,
    binding_type: &BindingType,
) {
    let visibility_str = format!("{:?}", visibility);
    let binding_type_str = format!("{:?}", binding_type);

    let config = ShaderBindingConfig {
        frame: state.frame_count,
        pipeline_label: pipeline_label.to_string(),
        group,
        binding,
        visibility: visibility_str.clone(),
        binding_type: binding_type_str.clone(),
    };

    state.shader_binding_configs.push(config);

    log::info!(
        "[SHADER BINDING] Frame {} - Pipeline '{}': Group {}, Binding {} - visibility={}, type={}",
        state.frame_count,
        pipeline_label,
        group,
        binding,
        visibility_str,
        binding_type_str,
    );

    // CRITICAL: Check for group 3, binding 0 (Crash #2 scenario)
    if group == 3 && binding == 0 {
        log::warn!(
            "[SHADER BINDING] CRITICAL: Configuring binding at group 3, binding 0. This is the binding that was missing in Crash #2!",
        );
        log::warn!(
            "[SHADER BINDING] Ensure this binding is included in the pipeline layout for '{}'",
            pipeline_label,
        );
    }
}

/// Log comprehensive render state
///
/// Call this periodically to capture the current rendering state
/// This provides a snapshot of the system when crashes occur
pub fn log_render_state(
    state: &RenderDiagnosticsState,
    pipeline_cache: Option<&PipelineCache>,
    additional_context: &str,
) {
    log::info!(
        "[RENDER STATE] Frame {} - Rendering state snapshot{}",
        state.frame_count,
        if additional_context.is_empty() {
            String::new()
        } else {
            format!(": {}", additional_context)
        },
    );
    
    log::info!(
        "[RENDER STATE]   Pipeline cache accesses: {} (last 5: {:?})",
        state.pipeline_cache_accesses.len(),
        state.pipeline_cache_accesses.iter().rev().take(5).collect::<Vec<_>>(),
    );
    
    log::info!(
        "[RENDER STATE]   Pipeline creations: {} (last 5: {:?})",
        state.pipeline_creations.len(),
        state.pipeline_creations.iter().rev().take(5).collect::<Vec<_>>(),
    );
    
    log::info!(
        "[RENDER STATE]   Alpha blend mesh events: {} (last 5: {:?})",
        state.alpha_blend_mesh_events.len(),
        state.alpha_blend_mesh_events.iter().rev().take(5).collect::<Vec<_>>(),
    );
    
    log::info!(
        "[RENDER STATE]   Shader binding configs: {} (last 5: {:?})",
        state.shader_binding_configs.len(),
        state.shader_binding_configs.iter().rev().take(5).collect::<Vec<_>>(),
    );
    
    // Log recent failed accesses
    let failed_accesses: Vec<_> = state.pipeline_cache_accesses
        .iter()
        .filter(|a| !a.success)
        .rev()
        .take(10)
        .collect();
    
    if !failed_accesses.is_empty() {
        log::error!(
            "[RENDER STATE]   Recent FAILED pipeline cache accesses: {:?}",
            failed_accesses,
        );
    }
}

/// Diagnostic system to log mesh and material state
///
/// This runs periodically to capture entity/component state
#[allow(dead_code)]
pub fn diagnostic_mesh_material_system(
    meshes: Query<(Entity, &bevy::prelude::Mesh3d)>,
    standard_materials: Query<(Entity, &bevy::pbr::MeshMaterial3d<bevy::pbr::StandardMaterial>)>,
    state: Res<RenderDiagnosticsState>,
) {
    if state.frame_count % 600 != 0 {
        // Only log every ~10 seconds at 60fps
        return;
    }
    
    let mesh_count = meshes.iter().count();
    let material_count = standard_materials.iter().count();
    
    log::info!(
        "[MESH MATERIAL DIAGNOSTIC] Frame {} - Mesh entities: {}, StandardMaterial entities: {}",
        state.frame_count,
        mesh_count,
        material_count,
    );
    
    // Log first few mesh entities
    for (entity, _mesh_handle) in meshes.iter().take(5) {
        log::info!(
            "[MESH MATERIAL DIAGNOSTIC]   Mesh entity: {:?}",
            entity,
        );
    }
}

/// Diagnostic system to log GPU image state
///
/// This runs periodically to capture texture loading state
#[allow(dead_code)]
pub fn diagnostic_gpu_image_system(
    images: Res<Assets<Image>>,
    state: Res<RenderDiagnosticsState>,
) {
    if state.frame_count % 600 != 0 {
        // Only log every ~10 seconds at 60fps
        return;
    }
    
    let image_count = images.iter().count();
    
    log::info!(
        "[GPU IMAGE DIAGNOSTIC] Frame {} - Loaded images: {}",
        state.frame_count,
        image_count,
    );
    
    // Log image formats
    let mut format_counts: HashMap<String, usize> = HashMap::default();
    for (_, image) in images.iter() {
        let format_str = format!("{:?}", image.texture_descriptor.format);
        *format_counts.entry(format_str).or_insert(0) += 1;
    }
    
    log::info!(
        "[GPU IMAGE DIAGNOSTIC]   Image format distribution: {:?}",
        format_counts,
    );
}

/// Check pipeline cache for potential issues
///
/// This function checks if the pipeline cache is in a state that could cause crashes
#[allow(dead_code)]
pub fn check_pipeline_cache_health(
    state: &RenderDiagnosticsState,
    pipeline_cache: Option<&PipelineCache>,
) {
    let recent_compute_accesses: Vec<_> = state.pipeline_cache_accesses
        .iter()
        .rev()
        .filter(|a| a.pipeline_type == PipelineType::Compute)
        .take(10)
        .collect();
    
    if !recent_compute_accesses.is_empty() {
        log::info!(
            "[PIPELINE CACHE HEALTH] Recent compute pipeline accesses (last 10): {:?}",
            recent_compute_accesses,
        );
        
        // Check for dangerous patterns
        for access in &recent_compute_accesses {
            if let Some(id) = access.pipeline_id {
                if id >= access.cache_size as u64 {
                    log::error!(
                        "[PIPELINE CACHE HEALTH] DANGER: Compute pipeline ID {} exceeds cache size {}. This WILL cause a crash!",
                        id,
                        access.cache_size,
                    );
                }
            }
        }
    }
}
