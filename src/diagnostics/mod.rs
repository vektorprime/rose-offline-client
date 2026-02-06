//! Diagnostic logging systems for rendering crash investigation
//!
//! This module provides comprehensive diagnostic logging to help identify
//! the root causes of rendering crashes, particularly:
//! - PipelineCache index out of bounds errors
//! - Missing pipeline binding errors
//! - Skinned mesh bind group layout mismatches

// pub mod render_diagnostics;
pub mod skinned_mesh_diagnostics;

// pub use render_diagnostics::{
//     RenderDiagnosticsPlugin,
//     log_pipeline_cache_access,
//     log_pipeline_creation,
//     log_alpha_blend_mesh_setup,
//     log_alpha_blend_mesh_setup_simple,
//     log_shader_binding_config,
//     log_render_state,
// };

pub use skinned_mesh_diagnostics::SkinnedMeshDiagnosticsPlugin;
