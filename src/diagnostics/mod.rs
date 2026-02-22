//! Diagnostic systems for rendering crash investigation
//!
//! This module provides optional debugging for rendering pipeline investigation.
//! NOTE: SkinnedMeshFixPlugin has been moved to render::skinned_mesh_fix as it is
//! REQUIRED for proper skinned mesh rendering, not optional diagnostics.

pub mod render_diagnostics;

pub use render_diagnostics::{
    RenderDiagnosticsPlugin,
    log_pipeline_cache_access,
    log_pipeline_creation,
    log_alpha_blend_mesh_setup,
    log_alpha_blend_mesh_setup_simple,
    log_shader_binding_config,
    log_render_state,
};
