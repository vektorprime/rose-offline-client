//! Cel ("toon") shading for the ROSE object and effect materials and the terrain.
//!
//! - `toon_bands.wgsl`: the band shape (terminator, mid band, hard shadow edge), shared
//!   with `terrain_material.wgsl`.
//! - `toon_lighting.wgsl`: `toon_lighting()`, used instead of Bevy's
//!   `apply_pbr_lighting` by `rose_object_extension.wgsl` and `rose_effect_mesh.wgsl`
//!   when their main-pass fragment shader has the `ROSE_TOON` shader def.
//!
//! The outlines are a separate screen-space pass, see [`crate::render::cel_outline`].

use bevy::{
    asset::{load_internal_asset, weak_handle},
    prelude::{App, Handle, Plugin, Shader},
    render::render_resource::RenderPipelineDescriptor,
};

/// Switches the ROSE object and effect materials between cel lighting (`true`) and
/// Bevy's PBR lighting (`false`). The terrain is always banded.
pub const CEL_SHADING_ENABLED: bool = true;

/// Shader def selecting `toon_lighting()` in the ROSE material shaders.
pub const TOON_SHADER_DEF: &str = "ROSE_TOON";

pub const TOON_BANDS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c3e1a7d2-5b4f-4e8a-0000-000000000000");

pub const TOON_LIGHTING_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("c3e1a7d2-5b4f-4e8a-0000-000000000001");

/// Registers the cel shading shader libraries (`rose::toon_bands`, `rose::toon_lighting`).
pub struct CelShadingPlugin;

impl Plugin for CelShadingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            TOON_BANDS_SHADER_HANDLE,
            "shaders/toon_bands.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            TOON_LIGHTING_SHADER_HANDLE,
            "shaders/toon_lighting.wgsl",
            Shader::from_wgsl
        );
    }
}

/// Adds [`TOON_SHADER_DEF`] to a main-pass pipeline's fragment stage when cel shading
/// is enabled. Call it from a material's `specialize` for non-prepass pipelines only.
pub fn push_toon_shader_def(descriptor: &mut RenderPipelineDescriptor) {
    if !CEL_SHADING_ENABLED {
        return;
    }
    if let Some(fragment) = descriptor.fragment.as_mut() {
        fragment.shader_defs.push(TOON_SHADER_DEF.into());
    }
}
