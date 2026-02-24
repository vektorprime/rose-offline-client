//! Map Editor Save System
//! 
//! This module provides save/export functionality for the map editor.
//! It allows exporting modified zones back to IFO format.
//!
//! # Architecture
//!
//! - `ifo_types`: Data structures for IFO file format
//! - `ifo_export`: Binary IFO file writer
//! - `save_system`: Bevy systems for saving zones

pub mod ifo_types;
pub mod ifo_export;
pub mod save_system;

pub use ifo_types::*;
pub use save_system::{SaveZoneEvent, SaveStatus, SavePlugin};
