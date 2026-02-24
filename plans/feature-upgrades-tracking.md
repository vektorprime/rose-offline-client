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

### Status: Not Started

### Goal
Add visual blood effects to combat:
1. Blood spatter on terrain/objects after killing monsters
2. Gash wounds visible on monsters below 50% HP
3. Wounds remain visible on killed monsters (corpses)

### Sub-Tasks

| Task | Status | Notes |
|------|--------|-------|
| **Blood Spatter** | | |
| Design blood spatter particle/decals system | [ ] | |
| Create blood spatter material/shader | [ ] | |
| Implement spatter on terrain collision | [ ] | |
| Implement spatter on nearby objects | [ ] | |
| Add fade/dissolve over time | [ ] | |
| **Gash Wounds** | | |
| Design wound visual system | [ ] | |
| Create wound texture/decals | [ ] | |
| Track HP percentage on monsters | [ ] | |
| Apply wounds when HP < 50% | [ ] | |
| **Corpse Wounds** | | |
| Ensure wounds persist on death | [ ] | |
| Handle corpse despawn timing | [ ] | |
| **Integration** | | |
| Integrate with hit detection system | [ ] | |
| Performance testing with multiple monsters | [ ] | |
| Configure blood settings (optional: disable option) | [ ] | |

### Relevant Files
- [`src/events/hit_event.rs`](src/events/hit_event.rs)
- [`src/components/dead.rs`](src/components/dead.rs)
- [`src/components/pending_damage_list.rs`](src/components/pending_damage_list.rs)
- [`src/render/particle_material.rs`](src/render/particle_material.rs)
- [`src/render/shaders/particle.wgsl`](src/render/shaders/particle.wgsl)
- [`src/render/terrain_material.rs`](src/render/terrain_material.rs)
- [`src/render/shaders/terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl)

### Issues/Findings
- 
- 

### Design Decisions
- [ ] Decide: Decals vs particles for blood spatter
- [ ] Decide: How many wound overlays per monster
- [ ] Decide: Blood persistence duration

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
| Blood System | Not Started | 0% |
| Bird Fix | Not Started | 0% |

---

## Notes

- Update this document as work progresses
- Mark tasks with `[x]` when complete, `[-]` when in progress
- Add any blocking issues or dependencies in the Issues/Findings section
- Reference any PRs or commits in the relevant sections
