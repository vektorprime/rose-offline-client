use bevy::prelude::{Event, Handle};

use rose_data::ZoneId;

// Import ZoneLoaderAsset for use in event
use crate::zone_loader::ZoneLoaderAsset;

#[derive(Event)]
pub struct LoadZoneEvent {
    pub id: ZoneId,
    pub despawn_other_zones: bool,
}

impl LoadZoneEvent {
    pub fn new(id: ZoneId) -> Self {
        Self {
            id,
            despawn_other_zones: true,
        }
    }
}

#[derive(Event)]
pub enum ZoneEvent {
    Loaded(ZoneId),
}

/// Event sent when a zone is loaded from VFS via async task and added to the Assets collection
/// Contains the handle to the asset so systems can look it up
#[derive(Event, Clone)]
pub struct ZoneLoadedFromVfsEvent {
    pub zone_id: ZoneId,
    pub zone_handle: Handle<ZoneLoaderAsset>,
}

impl ZoneLoadedFromVfsEvent {
    pub fn new(zone_id: ZoneId, zone_handle: Handle<ZoneLoaderAsset>) -> Self {
        Self {
            zone_id,
            zone_handle,
        }
    }
}
