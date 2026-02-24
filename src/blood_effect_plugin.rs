//! Blood Effects Plugin for the Rose Online client.
//!
//! This plugin provides blood visual effects including:
//! - Blood spatter decals on terrain when entities are killed
//! - Gash wounds appearing on entities when HP drops below 50%
//!
//! # Usage
//!
//! Add the plugin to your Bevy app:
//!
//! ```ignore
//! use bevy::prelude::*;
//! use crate::blood_effect_plugin::BloodEffectPlugin;
//!
//! App::new()
//!     .add_plugins(BloodEffectPlugin)
//!     .run();
//! ```
//!
//! # Configuration
//!
//! The plugin uses [`BloodEffectConfig`](crate::resources::BloodEffectConfig) resource
//! for configuration. Insert this resource before adding the plugin to customize behavior:
//!
//! ```ignore
//! app.insert_resource(BloodEffectConfig::high_intensity())
//!    .add_plugins(BloodEffectPlugin);
//! ```

use bevy::{core_pipeline::prepass::DepthPrepass, prelude::*};

use crate::{
    events::BloodEffectEvent,
    resources::BloodEffectConfig,
    systems::{BloodSpatterPlugin, GashWoundPlugin},
};

/// Plugin that registers all blood effect systems and resources.
///
/// This plugin adds:
/// - [`BloodEffectConfig`] resource with default settings
/// - [`BloodEffectEvent`] event for triggering blood effects
/// - Blood spatter systems for decal rendering
/// - Gash wound systems for entity wound visuals
///
/// # Camera Requirements
///
/// For blood spatter decals to render, the camera must have [`DepthPrepass`]:
///
/// ```ignore
/// commands.spawn((
///     Camera3d::default(),
///     DepthPrepass, // Required for forward decals
/// ));
/// ```
pub struct BloodEffectPlugin;

impl Plugin for BloodEffectPlugin {
    fn build(&self, app: &mut App) {
        // Register the config resource with defaults
        app.init_resource::<BloodEffectConfig>();

        // Register the blood effect event
        app.add_event::<BloodEffectEvent>();

        // Add the sub-plugins
        app.add_plugins((BloodSpatterPlugin, GashWoundPlugin));

        log::info!("Blood effect plugin initialized");
    }
}

impl BloodEffectPlugin {
    /// Creates a plugin with blood effects disabled.
    pub fn disabled() -> Self {
        Self
    }

    /// Creates a plugin with low intensity settings.
    pub fn low_intensity() -> Self {
        Self
    }

    /// Creates a plugin with high intensity settings.
    pub fn high_intensity() -> Self {
        Self
    }
}
