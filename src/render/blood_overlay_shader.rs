//! Blood overlay shader module.
//!
//! This module provides the shader handle for the blood overlay material extension.

use bevy::{asset::weak_handle, prelude::*};

/// Shader path for the blood overlay extension.
pub const BLOOD_OVERLAY_SHADER_PATH: &str = "shaders/blood_overlay.wgsl";

/// Shader handle for the blood overlay extension (static weak handle).
pub const BLOOD_OVERLAY_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("ee5f6a7b-8c9d-0e1f-2a3b-4c5d6e7f8a9b");
