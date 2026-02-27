# Unused Plugins and Features Analysis

This document identifies unused plugins, features, and code that can potentially be cleaned up from the codebase.

## Summary

| Category | Count | Risk Level |
|----------|-------|------------|
| Disabled Plugins | 5 | Low |
| Temporarily Disabled Features | 3 | Medium |
| Unregistered Systems | 7 | Low |
| Unadded Diagnostic Plugins | 3 | Low |
| Dead Code Functions | 24+ | Low |
| Potentially Unused Dependencies | 6 | Medium |

---

## 1. Disabled/Commented-Out Plugins in [`lib.rs`](src/lib.rs)

These plugins are explicitly commented out or disabled:

### Line 891-892: Sky Material Plugins (Replaced by Bevy 0.16 Atmosphere)
```rust
// SkyMaterialPlugin { prepass_enabled: false },  // DISABLED - using Bevy 0.16 Atmosphere instead
// CartoonSkyMaterialPlugin { prepass_enabled: false },  // DISABLED - using Bevy 0.16 Atmosphere instead
```
**Action**: These can be removed along with their source files:
- [`src/render/sky_material.rs`](src/render/sky_material.rs) 
- [`src/render/cartoon_sky_material.rs`](src/render/cartoon_sky_material.rs)
- [`src/render/shaders/sky_material.wgsl`](src/render/shaders/sky_material.wgsl)
- [`src/render/shaders/cartoon_sky_material.wgsl`](src/render/shaders/cartoon_sky_material.wgsl)

### Line 907: RenderDiagnosticsPlugin
```rust
// OPTIONAL: RenderDiagnosticsPlugin is for debugging rendering issues - keep disabled to reduce log noise
// RenderDiagnosticsPlugin,
```
**Action**: Keep for debugging purposes, but consider removing if never used.

### Line 1170: ui_debug_physics_system
```rust
// DISABLED: app.add_systems(Update, ui_debug_physics_system); // Too many parameters for Bevy 0.15
```
**Action**: Either fix for Bevy 0.15+ or remove.

### Line 1176: ui_debug_diagnostics_system
```rust
// DISABLED: app.add_systems(Update, ui_debug_diagnostics_system);
```
**Action**: Remove if not needed.

### Line 800-801: Debug Plugins (Not Added)
```rust
// Disabled: RapierDebugRenderPlugin (debug plugin)
// Disabled: RenderDocPlugin (debug plugin)
```
**Action**: [`src/debug/renderdoc.rs`](src/debug/renderdoc.rs) and the `renderdoc` dependency can be removed if not needed.

---

## 2. Temporarily Disabled Features

### Trail Effect Rendering
**File**: [`src/render/trail_effect.rs`](src/render/trail_effect.rs:51)
```rust
// Trail effect rendering temporarily disabled for Bevy 0.14 migration
// The component definitions are kept for API compatibility
```
**Action**: Either re-enable for current Bevy version or remove entirely.

### Gem Effects
**File**: [`src/model_loader.rs`](src/model_loader.rs:580)
```rust
// Gem effects temporarily disabled (use custom materials)
/*
```
**Action**: Remove dead code block or re-implement.

### Effect Spawning in Zone Loader
**File**: [`src/zone_loader.rs`](src/zone_loader.rs:3045)
```rust
// Effect spawning temporarily disabled (use custom materials)
/*
```
**Action**: Remove dead code block or re-implement.

---

## 3. Unregistered Systems (Exported but Never Added)

These systems are exported from [`src/systems/mod.rs`](src/systems/mod.rs) but never registered with `.add_systems()`:

| System | File | Notes |
|--------|------|-------|
| `debug_render_collider_system` | [`debug_render_collider_system.rs`](src/systems/debug_render_collider_system.rs) | Debug visualization |
| `debug_render_skeleton_system` | [`debug_render_skeleton_system.rs`](src/systems/debug_render_skeleton_system.rs) | Debug visualization |
| `debug_render_directional_light_system` | [`debug_render_directional_light_system.rs`](src/systems/debug_render_directional_light_system.rs) | Debug visualization |
| `flight_command_system` | [`flight_command_system.rs`](src/systems/flight_command_system.rs) | Only `is_fly_command` helper is used |
| `move_speed_command_system` | [`move_speed_command_system.rs`](src/systems/move_speed_command_system.rs) | Only `parse_move_speed_command` helper is used |
| `test_particle_spawn` | [`particle_test.rs`](src/render/particle_test.rs) | Test function |

**Action**: Remove unused systems or register them if needed.

---

## 4. Diagnostic Plugins Not Added to App

These plugins are defined but never added via `.add_plugins()`:

| Plugin | File | Purpose |
|--------|------|---------|
| `ZoneRenderValidationPlugin` | [`zone_render_validation_system.rs`](src/systems/zone_render_validation_system.rs:518) | Diagnose black screen issues |
| `ZoneMemoryProfilerPlugin` | [`zone_memory_profiler_system.rs`](src/systems/zone_memory_profiler_system.rs:394) | Memory leak detection |
| `ZoneMemoryProtectionPlugin` | [`zone_memory_protection_system.rs`](src/systems/zone_memory_protection_system.rs:140) | Emergency memory detection |

**Action**: These are diagnostic tools - keep for debugging but consider moving to a separate debug feature flag.

---

## 5. Dead Code (Functions with #[allow(dead_code)])

Found 24+ instances of functions marked with `#[allow(dead_code)]`:

### Season Systems
- [`spawn_flower_system`](src/systems/season/spring_system.rs:134) - Spring flower spawning
- [`spawn_season_particles`](src/systems/season/season_manager.rs:22) - Season particle spawning
- [`fall_particle_spawn_system`](src/systems/season/fall_system.rs:11) - Fall leaf particles

### Map Editor Helpers
- [`get_collision_part_mut`](src/map_editor/ui/properties_panel.rs:1057) - Future use helper
- [`grid_spawn_system`](src/map_editor/systems/grid_system.rs:121) - Grid spawning not used
- [`is_alt_pressed`](src/map_editor/systems/keyboard_shortcuts_system.rs:142) - Alt key check
- [`keyboard_shortcuts_help_system`](src/map_editor/systems/keyboard_shortcuts_system.rs:332) - Help overlay
- [`try_load_zsc_from_vfs`](src/map_editor/systems/load_models_system.rs:237) - Legacy loader
- [`model_preview_system`](src/map_editor/systems/model_placement_system.rs:465) - Preview placement
- [`object_list_item`](src/map_editor/ui/hierarchy_panel.rs:278) - UI helper
- [`format_zone_object_label`](src/map_editor/ui/hierarchy_panel.rs:319) - UI helper

### Audio Helpers
- [`SoundGain`](src/audio/mod.rs:21) - Decibel/Ratio enum methods
- [`SpatialControlHandle`](src/audio/spatial_sound.rs:21) - Spatial sound control
- [`SpatialSound`](src/audio/spatial_sound.rs:48) - Spatial sound methods
- [`ControlHandle`](src/audio/global_sound.rs:16) - Global sound control
- [`GlobalSound`](src/audio/global_sound.rs:40) - Global sound methods

### Diagnostics
- [`diagnostic_mesh_material_system`](src/diagnostics/render_diagnostics.rs:475)
- [`diagnostic_gpu_image_system`](src/diagnostics/render_diagnostics.rs:508)
- [`check_pipeline_cache_health`](src/diagnostics/render_diagnostics.rs:542)

### Other
- [`ability_values_add_value`](src/bundles/ability_values.rs:117)
- [`ability_values_set_value`](src/bundles/ability_values.rs:380)
- [`WithTransform`](src/events/spawn_effect_event.rs:50) - Event variant

**Action**: Review each and either use or remove.

---

## 6. Potentially Unused Dependencies

Based on code searches, these dependencies in [`Cargo.toml`](Cargo.toml) may not be used:

| Dependency | Search Pattern | Found |
|------------|----------------|-------|
| `futures-io` | `use futures_io::` | 0 |
| `directories` | `use directories::` | 0 |
| `lru` | `use lru::` | 0 |
| `mint` | `use mint::` | 0 |
| `crossbeam-channel` | `use crossbeam_channel::` | 0 |
| `dashmap` | `use dashmap::` | 0 |

**Action**: Verify with `cargo-udeps` or manual review before removing. Some may be transitive dependencies or used in macros.

---

## 7. Unused Diagnostic Function

### diagnose_camera_extraction_state
**File**: [`src/lib.rs:1897`](src/lib.rs:1897)
```rust
/// Camera extraction diagnostic system for Bevy 0.15.4
/// Logs camera state to verify extraction conditions are met
fn diagnose_camera_extraction_state(...)
```
**Action**: This function is defined but never registered. Remove or register it.

---

## 8. Test/Diagnostic Code in Production

### spawn_test_cube
**File**: [`src/lib.rs:1880`](src/lib.rs:1880)
```rust
/// Test cube spawn system for Bevy 0.14.2 rendering isolation test
/// This creates a simple red cube using StandardMaterial to verify core rendering works
fn spawn_test_cube(...)
```
Registered at PostStartup - creates a red cube at position (5100, 75, -5100).

**Action**: Remove for production builds or add behind a debug flag.

---

## Recommended Cleanup Actions

### High Priority (Safe to Remove)
1. Remove disabled sky material plugins and their shader files
2. Remove `diagnose_camera_extraction_state` function
3. Remove unused debug render systems

### Medium Priority (Review First)
1. Remove temporarily disabled features (trail effect, gem effects) or re-enable
2. Review and remove unused dependencies from Cargo.toml
3. Remove `spawn_test_cube` or add debug flag

### Low Priority (Keep for Now)
1. Dead code functions - may be planned for future use
2. Diagnostic plugins - useful for debugging
3. Season system dead code - may be re-enabled

---

## Verification Commands

Before removing any code, verify with:

```bash
# Check for unused dependencies
cargo install cargo-udeps
cargo +nightly udeps

# Check for dead code
cargo clippy -- -W dead_code

# Search for usage
rg "function_name" src/
```
