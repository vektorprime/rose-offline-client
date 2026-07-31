//! Terrain enhancement module for procedural noise overlay and other terrain modifications.
//!
//! This module provides systems for adding realistic terrain variation using
//! procedural noise applied to the heightmap during terrain mesh generation.

mod noise_overlay;

pub use noise_overlay::{
    get_thread_local_noise, init_thread_local_noise, GlobalTerrainNoise,
    TerrainEnhancementPlugin, TerrainEnhancementSettings,
};
