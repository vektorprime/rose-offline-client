//! Underwater camera state and water volume tracking.
//!
//! The underwater screen effect (fullscreen fog/tint/caustics post-process)
//! was removed; this module only keeps the CPU-side state other systems rely
//! on:
//! - [`UnderwaterVolumes`]: world-space water volumes built from
//!   [`WaterSpawnedEvent`]s (boats, sailing and water reflections read it).
//! - [`CameraUnderwaterState`]: whether the camera is below a water surface
//!   (the water reflection camera is disabled while submerged).
//!
//! The module path is kept so the many `underwater_effect::UnderwaterVolumes`
//! imports keep working.

use bevy::prelude::*;

use crate::{components::WaterSpawnedEvent, resources::WaterSettings};

// =============================================================================
// Components and Resources
// =============================================================================

/// Tracks whether a camera is underwater. Updated by [`detect_underwater_camera`]
/// for every camera that carries it (the main game camera).
#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct CameraUnderwaterState {
    /// Whether the camera is currently underwater
    pub is_underwater: bool,
    /// Y coordinate of the water surface
    pub water_surface_y: f32,
    /// How deep below the surface the camera is (0.0 if above water)
    pub depth_below_surface: f32,
}

/// Runtime-tracked water volume derived from spawned water planes.
#[derive(Debug, Clone)]
pub struct WaterVolume {
    pub water_entity: Entity,
    pub center: Vec3,
    pub half_extents: Vec2,
    pub surface_y: f32,
}

/// Collection of all currently known water volumes in world space.
#[derive(Resource, Default, Debug, Clone)]
pub struct UnderwaterVolumes {
    pub volumes: Vec<WaterVolume>,
}

// =============================================================================
// Plugin
// =============================================================================

/// Plugin that tracks water volumes and the camera's underwater state.
pub struct UnderwaterStatePlugin;

impl Plugin for UnderwaterStatePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<CameraUnderwaterState>()
            .init_resource::<UnderwaterVolumes>()
            .add_systems(
                Update,
                (
                    // After the zone spawn so its commands (zone root with its
                    // Transform) are applied when the spawn messages are read.
                    track_underwater_volumes.after(crate::zone_loader::zone_loaded_from_vfs_system),
                    detect_underwater_camera,
                )
                    .chain(),
            );
    }
}

// =============================================================================
// Systems
// =============================================================================

/// System to detect when camera is underwater
pub fn detect_underwater_camera(
    mut camera_query: Query<(&GlobalTransform, &mut CameraUnderwaterState), With<Camera>>,
    water_settings: Res<WaterSettings>,
    underwater_volumes: Res<UnderwaterVolumes>,
) {
    for (transform, mut underwater_state) in camera_query.iter_mut() {
        let camera_position = transform.translation();

        // Determine if camera is inside any spawned water volume.
        let mut selected_surface_y = water_settings.water_surface_y;
        let mut selected_depth = f32::MAX;
        let mut found_volume = false;

        // Use configurable max depth as the effective underwater volume depth.
        let volume_depth_limit = water_settings.max_depth.max(0.1);

        for volume in underwater_volumes.volumes.iter() {
            let dx = (camera_position.x - volume.center.x).abs();
            let dz = (camera_position.z - volume.center.z).abs();
            let inside_bounds = dx <= volume.half_extents.x && dz <= volume.half_extents.y;
            if !inside_bounds {
                continue;
            }

            let depth_below_surface = volume.surface_y - camera_position.y;
            if depth_below_surface < 0.0 || depth_below_surface > volume_depth_limit {
                continue;
            }

            // Prefer the closest valid surface when overlapping volumes exist.
            if depth_below_surface < selected_depth {
                selected_depth = depth_below_surface;
                selected_surface_y = volume.surface_y;
                found_volume = true;
            }
        }

        // Fallback for maps where no water spawn events were observed.
        if !found_volume && underwater_volumes.volumes.is_empty() {
            let fallback_depth = water_settings.water_surface_y - camera_position.y;
            if fallback_depth >= 0.0 && fallback_depth <= volume_depth_limit {
                selected_depth = fallback_depth;
                selected_surface_y = water_settings.water_surface_y;
                found_volume = true;
            }
        }

        // Write only on change so the state is not flagged changed every frame.
        let new_depth = if found_volume {
            selected_depth.max(0.0)
        } else {
            0.0
        };
        if underwater_state.is_underwater != found_volume {
            underwater_state.is_underwater = found_volume;
        }
        if (underwater_state.water_surface_y - selected_surface_y).abs() > f32::EPSILON {
            underwater_state.water_surface_y = selected_surface_y;
        }
        if (underwater_state.depth_below_surface - new_depth).abs() > f32::EPSILON {
            underwater_state.depth_below_surface = new_depth;
        }
    }
}

/// Tracks water planes from spawn events and stores world-space water volumes.
///
/// Volumes whose water entity no longer exists (despawned with its zone) are
/// dropped. They used to accumulate across zone loads, and because every zone
/// sits at the same world offset, a previous zone's lake could mark the camera
/// as underwater (or move the reflection plane) in the next zone.
pub fn track_underwater_volumes(
    mut water_spawned_events: MessageReader<WaterSpawnedEvent>,
    transforms: Query<&GlobalTransform>,
    zone_transforms: Query<&Transform>,
    mut underwater_volumes: ResMut<UnderwaterVolumes>,
    mut water_settings: ResMut<WaterSettings>,
) {
    let mut volumes_changed = false;

    // Read-only check first: `ResMut` deref would flag the resource changed.
    if underwater_volumes
        .volumes
        .iter()
        .any(|volume| !transforms.contains(volume.water_entity))
    {
        underwater_volumes
            .volumes
            .retain(|volume| transforms.contains(volume.water_entity));
        volumes_changed = true;
    }

    for event in water_spawned_events.read() {
        volumes_changed = true;

        // The zone root has no parent, so its local Transform is its world
        // placement. Its GlobalTransform is still identity in the spawn frame
        // (propagation runs in PostUpdate).
        let zone_translation = zone_transforms
            .get(event.zone_entity)
            .map(|t| t.translation)
            .unwrap_or(Vec3::ZERO);

        let world_center = event.water_center + zone_translation;
        let volume = WaterVolume {
            water_entity: event.water_entity,
            center: world_center,
            half_extents: event.water_half_extents,
            surface_y: world_center.y,
        };

        if let Some(existing) = underwater_volumes
            .volumes
            .iter_mut()
            .find(|v| v.water_entity == event.water_entity)
        {
            *existing = volume;
        } else {
            underwater_volumes.volumes.push(volume);
        }
    }

    if volumes_changed {
        if let Some(first_volume) = underwater_volumes.volumes.first() {
            // Keep legacy/global water surface in sync for systems that still read this value.
            // Compared first: a WaterSettings change re-prepares every water material.
            if water_settings.water_surface_y != first_volume.surface_y {
                water_settings.water_surface_y = first_volume.surface_y;
            }
        }
    }
}
