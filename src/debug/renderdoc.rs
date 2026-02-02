//! RenderDoc integration for GPU debugging
//!
//! Usage:
//! 1. Install RenderDoc from https://renderdoc.org/
//! 2. Set environment variable: RENDERDOC_CAPTURE=1
//! 3. Run the application
//! 4. Press F12 to capture a frame (or use renderdoc UI)

use bevy::prelude::*;
use renderdoc::{RenderDoc, V120};
use std::cell::RefCell;

pub struct RenderDocPlugin;

impl Plugin for RenderDocPlugin {
    fn build(&self, app: &mut App) {
        // Only enable if explicitly requested
        if std::env::var("RENDERDOC_CAPTURE").is_ok() {
            // Check if RenderDoc is available
            match RenderDoc::<V120>::new() {
                Ok(_) => {
                    info!("[RENDERDOC] RenderDoc integration enabled. Press F12 to capture.");
                    app.insert_resource(RenderDocEnabled)
                        .add_systems(Update, handle_renderdoc_capture);
                }
                Err(e) => {
                    warn!("[RENDERDOC] RenderDoc not available: {}. Make sure RenderDoc is injected or app is launched from RenderDoc.", e);
                }
            }
        }
    }
}

/// Marker resource indicating RenderDoc is enabled
#[derive(Resource)]
pub struct RenderDocEnabled;

/// Trigger a RenderDoc capture
pub fn trigger_capture() {
    if let Ok(mut rd) = RenderDoc::<V120>::new() {
        rd.trigger_capture();
        info!("[RENDERDOC] Frame capture triggered!");
    }
}

/// Start a frame capture
pub fn start_frame_capture() {
    if let Ok(mut rd) = RenderDoc::<V120>::new() {
        rd.start_frame_capture(std::ptr::null(), std::ptr::null());
    }
}

/// End a frame capture
pub fn end_frame_capture() {
    if let Ok(mut rd) = RenderDoc::<V120>::new() {
        rd.end_frame_capture(std::ptr::null(), std::ptr::null());
    }
}

fn handle_renderdoc_capture(
    _renderdoc: Res<RenderDocEnabled>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Trigger capture on F12
    if keyboard.just_pressed(KeyCode::F12) {
        trigger_capture();
    }
}

/// Marker to identify render phases in RenderDoc
/// Note: Markers are set via thread-local context for efficiency
pub fn renderdoc_event_marker(_name: &str) {
    // Markers can be added here if needed using thread-local context
    // Currently markers are set automatically during capture
}

/// Thread-local RenderDoc context for use in systems that need repeated access
thread_local! {
    static RENDERDOC_CONTEXT: RefCell<Option<RenderDoc<V120>>> = RefCell::new(None);
}

/// Initialize the thread-local RenderDoc context
pub fn init_thread_local_renderdoc() {
    RENDERDOC_CONTEXT.with(|ctx| {
        if ctx.borrow().is_none() {
            if let Ok(rd) = RenderDoc::<V120>::new() {
                *ctx.borrow_mut() = Some(rd);
            }
        }
    });
}

/// Trigger capture using thread-local context (more efficient for repeated calls)
pub fn trigger_capture_thread_local() {
    RENDERDOC_CONTEXT.with(|ctx| {
        if let Some(ref mut rd) = *ctx.borrow_mut() {
            rd.trigger_capture();
            info!("[RENDERDOC] Frame capture triggered!");
        }
    });
}
