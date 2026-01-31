//! Zone Memory Protection System
//! 
//! This system provides emergency memory protection by:
//! 1. Detecting when too many entities lack render components
//! 2. Warning about potential memory exhaustion
//! 3. Suggesting cleanup actions

use bevy::prelude::*;

use crate::components::ZoneObject;

/// Resource for tracking entity spawn patterns
#[derive(Resource, Default, Debug)]
pub struct ZoneMemoryProtection {
    /// Total entities checked last frame
    pub last_entity_count: usize,
    /// Entities failing validation last frame
    pub last_failed_count: usize,
    /// Consecutive frames with high failure rate
    pub consecutive_bad_frames: usize,
    /// Whether emergency mode is active
    pub emergency_mode: bool,
    /// Frame counter for throttling
    pub frame_counter: usize,
}

impl ZoneMemoryProtection {
    pub fn update(&mut self, total_entities: usize, failed_entities: usize) -> bool {
        self.frame_counter += 1;
        
        // Only check every 60 frames (~1 second)
        if self.frame_counter % 60 != 0 {
            return false;
        }
        
        let failure_rate = if total_entities > 0 {
            failed_entities as f32 / total_entities as f32
        } else {
            0.0
        };
        
        // Check if we have a high failure rate (>40% entities failing)
        if failure_rate > 0.4 && total_entities > 100 {
            self.consecutive_bad_frames += 1;
            
            if self.consecutive_bad_frames >= 3 && !self.emergency_mode {
                self.emergency_mode = true;
                log::error!("========================================");
                log::error!("EMERGENCY: High entity validation failure rate detected!");
                log::error!("  Total entities: {}", total_entities);
                log::error!("  Failed validation: {}", failed_entities);
                log::error!("  Failure rate: {:.1}%", failure_rate * 100.0);
                log::error!("========================================");
                log::error!("This may cause memory exhaustion!");
                log::error!("Suggested fixes:");
                log::error!("  1. Check that asset paths are correct");
                log::error!("  2. Verify asset files exist in VFS");
                log::error!("  3. Ensure mesh/material handles are valid");
                log::error!("========================================");
                return true;
            }
        } else {
            self.consecutive_bad_frames = 0;
            if self.emergency_mode {
                self.emergency_mode = false;
                log::info!("[MEMORY PROTECTION] Entity validation has recovered");
            }
        }
        
        self.last_entity_count = total_entities;
        self.last_failed_count = failed_entities;
        
        false
    }
}

/// System to monitor entity memory usage
/// Uses a single combined query to stay within Bevy's system parameter limit
pub fn zone_memory_protection_system(
    mut protection: ResMut<ZoneMemoryProtection>,
    zone_objects: Query<(Entity, Option<&Handle<Mesh>>, Option<&Handle<StandardMaterial>>), With<ZoneObject>>,
) {
    let total_entities = zone_objects.iter().count();
    
    // Count entities with missing components
    let mut missing_mesh = 0;
    let mut missing_material = 0;
    
    // Check ZoneObject entities for completeness
    for (entity, mesh, material) in zone_objects.iter() {
        let has_mesh: bool = mesh.is_some();
        let has_material: bool = material.is_some();
        
        if !has_mesh {
            missing_mesh += 1;
        }
        if !has_material {
            missing_material += 1;
        }
    }
    
    let failed_entities = missing_mesh.max(missing_material);
    
    // Update protection state
    let is_emergency = protection.update(total_entities, failed_entities);
    
    // Log periodic status (every 5 seconds)
    if protection.frame_counter % 300 == 0 && total_entities > 0 {
        let failure_rate = if total_entities > 0 {
            (failed_entities as f32 / total_entities as f32) * 100.0
        } else {
            0.0
        };
        
        if failure_rate > 20.0 {
            log::warn!(
                "[MEMORY PROTECTION] {} ZoneObjects, {} missing components ({:.1}% failure rate)",
                total_entities, failed_entities, failure_rate
            );
        } else {
            log::info!(
                "[MEMORY PROTECTION] {} ZoneObjects, {:.1}% healthy",
                total_entities, 100.0 - failure_rate
            );
        }
    }
    
    // In emergency mode, log detailed info every second
    if is_emergency || (protection.emergency_mode && protection.frame_counter % 60 == 0) {
        log::error!(
            "[MEMORY PROTECTION EMERGENCY] Missing mesh: {}, Missing material: {}",
            missing_mesh, missing_material
        );
    }
}

/// Plugin to add memory protection
pub struct ZoneMemoryProtectionPlugin;

impl Plugin for ZoneMemoryProtectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneMemoryProtection>()
           .add_systems(Update, zone_memory_protection_system);
    }
}
