# Feature Upgrades Tracking

> Last Updated: 2026-02-23

This document tracks the progress of three major feature upgrades being implemented in the rose-offline-client.

---

## Table of Contents

1. [Water Rendering Fix](#1-water-rendering-fix)
2. [Blood System](#2-blood-system)
3. [Bird Fix](#3-bird-fix)

---

## 1. Water Rendering Fix

### Status: Implemented (Pending Testing)

### Goal
Make underwater view realistic - not just a blue top layer. The water should have proper depth-based effects when the camera is submerged.

### Sub-Tasks

| Task | Status | Notes |
|------|--------|-------|
| Analyze current water shader implementation | [x] | Analyzed water_material.wgsl and existing post-processing |
| Research underwater rendering techniques | [x] | Beer-Lambert law, depth-based absorption, caustics |
| Implement depth-based fog/color when camera is underwater | [x] | UnderwaterSettings resource with fog density and color |
| Add caustics or light shafts effect underwater | [x] | Procedural caustics using FBM noise in shader |
| Test with camera transitions (entering/exiting water) | [ ] | Needs runtime testing |
| Performance optimization | [x] | Early-out in shader when above water |

### Implementation Details

**Created Files:**
- [`src/render/underwater_effect.rs`](src/render/underwater_effect.rs) - Main implementation
  - `UnderwaterSettings` resource for configurable parameters
  - `CameraUnderwaterState` component for tracking camera state
  - `detect_underwater_camera` system for detection
  - Post-processing render node integrated at `Node3d::PostProcessing`
  
- [`src/render/shaders/underwater_effect.wgsl`](src/render/shaders/underwater_effect.wgsl) - Shader
  - Beer-Lambert law for light absorption
  - Depth-based color absorption (red fastest, blue penetrates)
  - Procedural caustics using gradient noise and FBM

**Modified Files:**
- [`src/render/mod.rs`](src/render/mod.rs) - Added module registration
- [`src/lib.rs`](src/lib.rs) - Added UnderwaterEffectPlugin registration

### Relevant Files
- [`src/render/water_material.rs`](src/render/water_material.rs)
- [`src/render/water_material_extension.rs`](src/render/water_material_extension.rs)
- [`src/render/shaders/water_material.wgsl`](src/render/shaders/water_material.wgsl)
- [`src/render/shaders/rose_water_extension.wgsl`](src/render/shaders/rose_water_extension.wgsl)
- [`src/render/underwater_effect.rs`](src/render/underwater_effect.rs) - NEW
- [`src/render/shaders/underwater_effect.wgsl`](src/render/shaders/underwater_effect.wgsl) - NEW
- [`plans/water-rendering-analysis.md`](plans/water-rendering-analysis.md)
- [`plans/underwater-rendering-fix.md`](plans/underwater-rendering-fix.md)

### Issues/Findings
- WaterSettings resource already has `water_surface_y` field for water level detection
- Uses Bevy 0.16.1 post-processing patterns with `ViewNodeRunner` and `SpecializedRenderPipeline`

### References
- Bevy water rendering examples
- Underwater rendering techniques in game development

---

## 2. Blood System

### Status: Implemented (Pending Testing)

### Goal
Add visual blood effects to combat:
1. Blood spatter on terrain/objects after killing monsters
2. Gash wounds visible on monsters below 50% HP
3. Wounds remain visible on killed monsters (corpses)

### Sub-Tasks

| Task | Status | Notes |
|------|--------|-------|
| **Blood Spatter** | | |
| Design blood spatter particle/decals system | [x] | Using Bevy 0.16.1 ForwardDecal |
| Create blood spatter material/shader | [x] | StandardMaterial with depth fade |
| Implement spatter on terrain collision | [x] | ForwardDecal projects onto terrain |
| Implement spatter on nearby objects | [x] | ForwardDecal handles all surfaces |
| Add fade/dissolve over time | [x] | Alpha fade with configurable lifetime |
| **Gash Wounds** | | |
| Design wound visual system | [x] | WoundVisual marker + GashWounds component |
| Create wound texture/decals | [x] | Procedural red ellipse textures |
| Track HP percentage on monsters | [x] | Via AbilityValues component |
| Apply wounds when HP < 50% | [x] | wound_visibility_system monitors HP |
| **Corpse Wounds** | | |
| Ensure wounds persist on death | [x] | Wounds attached to parent entity |
| Handle corpse despawn timing | [x] | wound_cleanup_system handles despawn |
| **Integration** | | |
| Integrate with hit detection system | [x] | Listens for Added<Dead> event |
| Performance testing with multiple monsters | [ ] | Needs runtime testing |
| Configure blood settings (optional: disable option) | [x] | BloodEffectConfig resource |

### Implementation Details

**Created Files:**
- [`src/components/blood_effect.rs`](src/components/blood_effect.rs) - Components
  - `BloodSpatter` - Tracks lifetime, alpha, size for spatter decals
  - `GashWounds` - Tracks wound count and parent entity
  - `WoundVisual` - Marker for wound child entities
  - `BloodSpatterConfig` - Configuration for spatter pool

- [`src/events/blood_effect_event.rs`](src/events/blood_effect_event.rs) - Events
  - `BloodEffectEvent` enum with variants:
    - `SpawnSpatter` - Spawn blood spatter at position
    - `ShowWound` - Show wounds on entity
    - `UpdateWoundVisibility` - Update wound visibility based on HP
    - `CleanupWounds` - Remove wound visuals

- [`src/resources/blood_effect_config.rs`](src/resources/blood_effect_config.rs) - Configuration
  - `BloodEffectConfig` resource with:
    - `enable_blood` - Toggle blood effects
    - `max_spatters` - Pool size limit (default: 100)
    - `spatter_lifetime` - Duration before fade
    - `spatter_fade_duration` - Fade out time
    - `wound_hp_threshold` - HP% for wounds (default: 0.5)

- [`src/systems/blood_spatter_system.rs`](src/systems/blood_spatter_system.rs) - Blood spatter logic
  - `blood_spatter_on_death_system` - Listens for `Added<Dead>`, spawns events
  - `blood_spatter_spawn_system` - Processes events, creates ForwardDecal entities
  - `blood_spatter_fade_system` - Fades and removes expired spatters
  - `create_blood_texture()` - Procedural blood texture generation

- [`src/systems/gash_wound_system.rs`](src/systems/gash_wound_system.rs) - Wound logic
  - `wound_visibility_system` - Monitors HP via AbilityValues, shows wounds at threshold
  - `wound_spawn_system` - Processes ShowWound events
  - `wound_cleanup_system` - Cleans up wound visuals when parent despawns

- [`src/blood_effect_plugin.rs`](src/blood_effect_plugin.rs) - Plugin registration
  - `BloodEffectPlugin` - Registers config, events, and sub-plugins
  - `BloodSpatterPlugin` - Registers spatter systems
  - `GashWoundPlugin` - Registers wound systems

**Modified Files:**
- [`src/components/mod.rs`](src/components/mod.rs) - Added blood_effect module
- [`src/events/mod.rs`](src/events/mod.rs) - Added blood_effect_event module
- [`src/resources/mod.rs`](src/resources/mod.rs) - Added blood_effect_config module
- [`src/systems/mod.rs`](src/systems/mod.rs) - Added blood_spatter_system and gash_wound_system
- [`src/lib.rs`](src/lib.rs) - Added BloodEffectPlugin registration

### Relevant Files
- [`src/events/hit_event.rs`](src/events/hit_event.rs)
- [`src/components/dead.rs`](src/components/dead.rs)
- [`src/components/pending_damage_list.rs`](src/components/pending_damage_list.rs)
- [`src/render/particle_material.rs`](src/render/particle_material.rs)
- [`src/render/shaders/particle.wgsl`](src/render/shaders/particle.wgsl)
- [`src/render/terrain_material.rs`](src/render/terrain_material.rs)
- [`src/render/shaders/terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl)
- [`plans/blood-effect-system.md`](plans/blood-effect-system.md) - Detailed architecture plan

### Issues/Findings
- ForwardDecal requires camera to have `DepthPrepass` component (already added)
- Bevy 0.16.1 uses `RenderAssetUsages` parameter for Image::new()
- Procedural textures created at runtime for blood spatters

### Design Decisions
- [x] Decals vs particles for blood spatter → **ForwardDecal** (projects onto all surfaces)
- [x] How many wound overlays per monster → **Configurable** (default 3)
- [x] Blood persistence duration → **Configurable** (default 30 seconds)

---

## 3. Bird Fix

### Status: Not Started

### Goal
Make birds face the direction they are flying. Currently birds may not orient correctly to their movement direction.

### Sub-Tasks

| Task | Status | Notes |
|------|--------|-------|
| Analyze current bird movement system | [ ] | |
| Identify rotation update logic | [ ] | |
| Implement smooth rotation to face movement direction | [ ] | |
| Handle perching/landing transitions | [ ] | |
| Test with multiple birds in flight | [ ] | |
| Verify no jittering or snapping | [ ] | |

### Relevant Files
- [`src/components/bird.rs`](src/components/bird.rs)
- [`src/components/facing_direction.rs`](src/components/facing_direction.rs)
- [`src/components/flight.rs`](src/components/flight.rs)
- [`plans/bird-system-architecture.md`](plans/bird-system-architecture.md)
- [`plans/flying-system-architecture.md`](plans/flying-system-architecture.md)

### Issues/Findings
- 
- 

### Related Systems
- [`src/components/fish.rs`](src/components/fish.rs) - Similar movement patterns, may need same fix

---

## Progress Summary

| Feature | Status | Completion |
|---------|--------|------------|
| Water Rendering Fix | Implemented (Pending Testing) | 85% |
| Blood System | Implemented (Pending Testing) | 90% |
| Bird Fix | Not Started | 0% |

---

## Notes

- Update this document as work progresses
- Mark tasks with `[x]` when complete, `[-]` when in progress
- Add any blocking issues or dependencies in the Issues/Findings section
- Reference any PRs or commits in the relevant sections

---

## Changelog

### 2026-02-24
- **Blood System**: Completed full implementation
  - Created blood effect components, events, and resources
  - Implemented blood spatter system using ForwardDecal
  - Implemented gash wound system with HP monitoring
  - Integrated BloodEffectPlugin into lib.rs
  - Build verified successful
