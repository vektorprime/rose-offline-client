//! Zone Memory Profiler System
//! 
//! This system provides memory profiling and leak detection for the zone loading pipeline.
//! It tracks asset allocations, entity counts, and detects memory leak patterns.

use std::collections::HashMap;
use bevy::prelude::*;
use bevy::asset::Assets;

use crate::zone_loader::MemoryTrackingResource;
use crate::resources::zone_debug_diagnostics::ZoneDebugDiagnostics;

/// Resource for detailed memory profiling
#[derive(Resource, Default, Debug)]
pub struct ZoneMemoryProfiler {
    /// Snapshot history for detecting trends
    pub snapshots: Vec<MemorySnapshot>,
    /// Maximum snapshots to keep
    pub max_snapshots: usize,
    /// Last snapshot time
    pub last_snapshot_time: Option<std::time::Instant>,
    /// Snapshot interval in seconds
    pub snapshot_interval_secs: u64,
    /// Detected leak alerts
    pub leak_alerts: Vec<LeakAlert>,
    /// Asset handle reference tracking
    pub asset_reference_counts: HashMap<String, usize>,
    /// Peak memory usage
    pub peak_entity_count: usize,
    pub peak_mesh_count: usize,
    pub peak_material_count: usize,
    pub peak_texture_count: usize,
}

/// Memory snapshot at a point in time
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    pub timestamp: std::time::Instant,
    pub entity_count: usize,
    pub mesh_count: usize,
    pub material_count: usize,
    pub texture_count: usize,
    pub asset_handle_count: usize,
    pub memory_tracking: MemoryTrackingSnapshot,
}

impl MemorySnapshot {
    pub fn clone_for_analysis(&self) -> Self {
        Self {
            timestamp: self.timestamp,
            entity_count: self.entity_count,
            mesh_count: self.mesh_count,
            material_count: self.material_count,
            texture_count: self.texture_count,
            asset_handle_count: self.asset_handle_count,
            memory_tracking: self.memory_tracking.clone(),
        }
    }
}

/// Snapshot of MemoryTrackingResource
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryTrackingSnapshot {
    pub mesh_handles_created: usize,
    pub material_handles_created: usize,
    pub texture_handles_created: usize,
    pub unique_asset_paths: usize,
    pub duplicate_asset_requests: usize,
    pub entities_spawned: usize,
    pub entities_despawned: usize,
}

/// Leak alert with details
#[derive(Debug, Clone)]
pub struct LeakAlert {
    pub detected_at: std::time::Instant,
    pub alert_type: LeakType,
    pub severity: LeakSeverity,
    pub message: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakType {
    EntityLeak,
    AssetLeak,
    MemoryGrowth,
    DuplicateAssetRequests,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeakSeverity {
    Info,
    Warning,
    Critical,
}

impl ZoneMemoryProfiler {
    pub fn new() -> Self {
        Self {
            max_snapshots: 60, // Keep ~1 minute of snapshots at 1 per second
            snapshot_interval_secs: 1,
            ..Default::default()
        }
    }

    /// Take a memory snapshot
    pub fn take_snapshot(
        &mut self,
        entity_count: usize,
        mesh_count: usize,
        material_count: usize,
        texture_count: usize,
        asset_handle_count: usize,
        memory_tracking: &MemoryTrackingResource,
    ) {
        let now = std::time::Instant::now();
        
        // Check if enough time has passed since last snapshot
        if let Some(last) = self.last_snapshot_time {
            if now.duration_since(last).as_secs() < self.snapshot_interval_secs {
                return;
            }
        }
        self.last_snapshot_time = Some(now);

        let snapshot = MemorySnapshot {
            timestamp: now,
            entity_count,
            mesh_count,
            material_count,
            texture_count,
            asset_handle_count,
            memory_tracking: MemoryTrackingSnapshot {
                mesh_handles_created: memory_tracking.mesh_handles_created,
                material_handles_created: memory_tracking.material_handles_created,
                texture_handles_created: memory_tracking.texture_handles_created,
                unique_asset_paths: memory_tracking.unique_asset_paths.len(),
                duplicate_asset_requests: memory_tracking.duplicate_asset_requests,
                entities_spawned: memory_tracking.entities_spawned,
                entities_despawned: memory_tracking.entities_despawned,
            },
        };

        // Update peaks
        self.peak_entity_count = self.peak_entity_count.max(entity_count);
        self.peak_mesh_count = self.peak_mesh_count.max(mesh_count);
        self.peak_material_count = self.peak_material_count.max(material_count);
        self.peak_texture_count = self.peak_texture_count.max(texture_count);

        self.snapshots.push(snapshot);

        // Trim old snapshots
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.remove(0);
        }

        // Analyze for leaks
        self.analyze_for_leaks();
    }

    /// Analyze snapshots for leak patterns
    fn analyze_for_leaks(&mut self) {
        if self.snapshots.len() < 5 {
            return;
        }

        // Extract all data we need before any mutable borrows
        let len = self.snapshots.len();
        let first = self.snapshots[len - 5].clone();
        let last = self.snapshots[len - 1].clone();
        let interval_secs = self.snapshot_interval_secs;

        // Check for entity leak (entities growing without bound)
        let entity_growth = last.entity_count as i64 - first.entity_count as i64;
        if entity_growth > 100 {
            let alert = LeakAlert {
                detected_at: std::time::Instant::now(),
                alert_type: LeakType::EntityLeak,
                severity: LeakSeverity::Critical,
                message: format!(
                    "Entity leak detected: {} entities added in {} seconds",
                    entity_growth, interval_secs * 5
                ),
                suggestion: "Check zone_loader_system for duplicate entity spawning. Verify that despawn logic is working correctly.".to_string(),
            };
            self.add_leak_alert(alert);
        }

        // Check for asset handle leak
        let mesh_growth = last.memory_tracking.mesh_handles_created as i64 - first.memory_tracking.mesh_handles_created as i64;
        if mesh_growth > 50 {
            let alert = LeakAlert {
                detected_at: std::time::Instant::now(),
                alert_type: LeakType::AssetLeak,
                severity: LeakSeverity::Warning,
                message: format!(
                    "Mesh handle leak suspected: {} handles created in {} seconds",
                    mesh_growth, interval_secs * 5
                ),
                suggestion: "Ensure mesh handles are dropped after spawning. Check if zone_loading_assets Vec is being cleared.".to_string(),
            };
            self.add_leak_alert(alert);
        }

        // Check for excessive duplicate requests
        let duplicate_growth = last.memory_tracking.duplicate_asset_requests - first.memory_tracking.duplicate_asset_requests;
        if duplicate_growth > 50 {
            let alert = LeakAlert {
                detected_at: std::time::Instant::now(),
                alert_type: LeakType::DuplicateAssetRequests,
                severity: LeakSeverity::Warning,
                message: format!(
                    "Excessive duplicate asset requests: {} new duplicates",
                    duplicate_growth
                ),
                suggestion: "Asset caching may not be working correctly. Check MaterialCache and ensure cache keys are consistent.".to_string(),
            };
            self.add_leak_alert(alert);
        }

        // Check for memory growth pattern (entities spawned but not despawned)
        let spawned = last.memory_tracking.entities_spawned as i64 - first.memory_tracking.entities_spawned as i64;
        let despawned = last.memory_tracking.entities_despawned as i64 - first.memory_tracking.entities_despawned as i64;
        if spawned > 50 && despawned == 0 {
            let alert = LeakAlert {
                detected_at: std::time::Instant::now(),
                alert_type: LeakType::MemoryGrowth,
                severity: LeakSeverity::Critical,
                message: format!(
                    "Memory growth detected: {} entities spawned, 0 despawned",
                    spawned
                ),
                suggestion: "CRITICAL: Entities are being spawned but never despawned. Check zone change logic and ensure old zones are properly cleaned up.".to_string(),
            };
            self.add_leak_alert(alert);
        }
    }

    fn add_leak_alert(&mut self, alert: LeakAlert) {
        // Only add if we don't have a similar recent alert
        let is_duplicate = self.leak_alerts.iter().rev().take(5).any(|a| {
            a.alert_type == alert.alert_type && 
            a.detected_at.elapsed().as_secs() < 30
        });

        if !is_duplicate {
            match alert.severity {
                LeakSeverity::Critical => log::error!("[MEMORY PROFILER] {}", alert.message),
                LeakSeverity::Warning => log::warn!("[MEMORY PROFILER] {}", alert.message),
                LeakSeverity::Info => log::info!("[MEMORY PROFILER] {}", alert.message),
            }
            log::info!("[MEMORY PROFILER] Suggestion: {}", alert.suggestion);
            
            self.leak_alerts.push(alert);
            
            // Keep only recent alerts
            if self.leak_alerts.len() > 20 {
                self.leak_alerts.remove(0);
            }
        }
    }

    /// Get growth rate in entities per second
    pub fn get_entity_growth_rate(&self) -> f64 {
        if self.snapshots.len() < 2 {
            return 0.0;
        }

        let first = self.snapshots.first().unwrap();
        let last = self.snapshots.last().unwrap();

        let duration = last.timestamp.duration_since(first.timestamp).as_secs_f64();
        if duration <= 0.0 {
            return 0.0;
        }

        (last.entity_count as f64 - first.entity_count as f64) / duration
    }

    /// Log comprehensive report
    pub fn log_report(&self) {
        log::info!("========================================");
        log::info!("ZONE MEMORY PROFILER REPORT");
        log::info!("========================================");
        
        if let Some(latest) = self.snapshots.last() {
            log::info!("Current snapshot:");
            log::info!("  Entities: {}", latest.entity_count);
            log::info!("  Meshes: {}", latest.mesh_count);
            log::info!("  Materials: {}", latest.material_count);
            log::info!("  Textures: {}", latest.texture_count);
            log::info!("  Asset handles created: mesh={}, material={}, texture={}",
                latest.memory_tracking.mesh_handles_created,
                latest.memory_tracking.material_handles_created,
                latest.memory_tracking.texture_handles_created);
            log::info!("  Unique asset paths: {}", latest.memory_tracking.unique_asset_paths);
            log::info!("  Duplicate requests: {}", latest.memory_tracking.duplicate_asset_requests);
            log::info!("  Spawned: {}, Despawned: {}",
                latest.memory_tracking.entities_spawned,
                latest.memory_tracking.entities_despawned);
        }

        log::info!("\nPeak usage:");
        log::info!("  Entities: {}", self.peak_entity_count);
        log::info!("  Meshes: {}", self.peak_mesh_count);
        log::info!("  Materials: {}", self.peak_material_count);
        log::info!("  Textures: {}", self.peak_texture_count);

        log::info!("\nGrowth rate: {:.2} entities/second", self.get_entity_growth_rate());

        if !self.leak_alerts.is_empty() {
            log::warn!("\nActive leak alerts: {}", self.leak_alerts.len());
            for alert in self.leak_alerts.iter().rev().take(5) {
                let level = match alert.severity {
                    LeakSeverity::Critical => "CRITICAL",
                    LeakSeverity::Warning => "WARNING",
                    LeakSeverity::Info => "INFO",
                };
                log::warn!("  [{}] {}", level, alert.message);
            }
        }

        log::info!("========================================");
    }
}

/// System to run memory profiling
pub fn zone_memory_profiler_system(
    mut profiler: ResMut<ZoneMemoryProfiler>,
    memory_tracking: Res<MemoryTrackingResource>,
    diagnostics: Res<ZoneDebugDiagnostics>,
    all_entities: Query<Entity>,
    meshes: Res<Assets<Mesh>>,
    images: Res<Assets<Image>>,
) {
    // Count various asset types
    let mesh_count = meshes.len();
    let texture_count = images.len();
    
    // Estimate material count from diagnostics
    let material_count = diagnostics.entities_by_type.values().sum();
    
    // Take snapshot
    profiler.take_snapshot(
        all_entities.iter().count(),
        mesh_count,
        material_count,
        texture_count,
        memory_tracking.mesh_handles_created + memory_tracking.material_handles_created + memory_tracking.texture_handles_created,
        &memory_tracking,
    );

    // Log report periodically
    static mut FRAME_COUNTER: usize = 0;
    unsafe {
        FRAME_COUNTER += 1;
        if FRAME_COUNTER % 300 == 0 { // Every 5 seconds at 60fps
            profiler.log_report();
        }
    }
}

/// System to check for command buffer accumulation
/// This can happen if Commands are not being flushed properly
pub fn command_buffer_validation_system(
    diagnostics: Res<ZoneDebugDiagnostics>,
) {
    static mut LAST_SPAWN_COUNT: usize = 0;
    static mut FRAME_COUNTER: usize = 0;
    
    unsafe {
        FRAME_COUNTER += 1;
        
        // Check every 60 frames
        if FRAME_COUNTER % 60 != 0 {
            return;
        }

        // If entities are being spawned but not showing up in the world,
        // it could be a command buffer issue
        if diagnostics.entities_spawned_this_frame > 0 {
            log::debug!(
                "[COMMAND BUFFER] Entities spawned this frame: {}",
                diagnostics.entities_spawned_this_frame
            );
        }

        LAST_SPAWN_COUNT = diagnostics.total_entities_spawned;
    }
}

/// Plugin to add memory profiling systems
pub struct ZoneMemoryProfilerPlugin;

impl Plugin for ZoneMemoryProfilerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ZoneMemoryProfiler>()
           .add_systems(Update, (
               zone_memory_profiler_system,
               command_buffer_validation_system,
           ));
    }
}