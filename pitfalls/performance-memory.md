# Performance and Memory Pitfalls

This document records performance and memory-related issues encountered during development.

---

## GPU Storage Buffer Memory Leak (Fixed 2026-03-03)

### Problem
Graphics would slow down over time due to a slow memory leak in GPU storage buffers.

### Root Cause
Multiple locations were creating new GPU storage buffers every frame without removing old ones:
1. `src/systems/particle_sequence_system.rs` - Created 4 new buffers per particle per frame
2. `src/systems/damage_digit_render_system.rs` - Created 3 new buffers per frame

The `Assets<ShaderStorageBuffer>::add()` method creates a NEW asset handle each call. When updating materials every frame, the old buffer handles were being overwritten but the underlying GPU resources were never removed from the Assets storage.

### Symptoms
- GPU memory grows linearly with particle effects active
- Frame rate degrades over time (minutes of gameplay)
- `Assets<ShaderStorageBuffer>` count grows unbounded

### Fix
Before creating new storage buffers, store the old handles and remove them after:
```rust
// Store old buffer handles
let old_positions = mat.positions.clone();
// ... create new buffers ...
// Remove old buffers to prevent memory leak
storage_buffers.remove(&old_positions);
```

### Files Modified
- `src/systems/particle_sequence_system.rs` - Added buffer cleanup
- `src/systems/damage_digit_render_system.rs` - Added buffer cleanup

### Lesson Learned
When using `Assets::add()` in a system that runs every frame, you MUST track and remove old assets or they will leak indefinitely. This is especially critical for GPU resources like `ShaderStorageBuffer`.

### Related Patterns to Watch For
- Any `storage_buffers.add()` in update systems
- Any `assets.add()` called every frame
- Unbounded Vec/HashSet growth in long-running systems

---

## Change Detection Fired Every Frame by Writes That Change Nothing (Fixed 2026-09-30)

### Problem
Materials, fog, sun/shadow and cloud systems re-ran every frame even though their guards checked `is_changed()` and "write only on difference". Terrain/water/cloud/blood materials were re-prepared (new bind group, re-specialization) every frame.

### Root Cause
Several Bevy 0.19.1 APIs flag a change on access, not on an actual value change:
- `&mut res.field` through `ResMut` (and `Mut::as_mut()`) is a `DerefMut`, which calls `set_changed()` before any comparison. `zone_time_system`'s `write_f32_if_changed(&mut zone_lighting.x, ..)` helpers therefore flagged `ZoneLighting`/`ZoneTime` every frame; `command_system`'s `command.as_mut()` flagged `Changed<Command>` for every entity.
- `Assets::iter_mut()` queues `AssetEvent::Modified` for every asset it visits, written or not.
- `AssetMut` (from `Assets::get_mut`) queues `Modified` on `DerefMut`, even when the assigned value is identical.
- Passing `&mut ResMut<T>` to an egui page (Settings window) flags `T` every frame the page is visible.

### Fix
- Write through `bypass_change_detection()` and call `set_changed()` only when a helper actually wrote; or compare through `Deref` first; or edit a clone and `set_if_neq` it (settings pages).
- Replace `iter_mut()` with `iter()` to collect the ids that differ, then `get_mut()` only those.
- Guard `as_mut()` with a read-only `matches!` first.

### Files Modified
- `src/systems/zone_time_system.rs`, `src/render/zone_lighting.rs`, `src/render/volumetric_cloud.rs`, `src/render/terrain_material.rs`, `src/lib.rs` (`apply_water_settings`), `src/ui/ui_settings_system.rs`, `src/systems/command_system.rs`, `src/graphics/apply_systems.rs`, `src/systems/blood_overlay_system.rs`, `src/systems/blood_spatter_system.rs`

### Lesson Learned
A "write only if changed" helper must not take `&mut res.field`: the `&mut` itself is the change. When fixing such a flag, check every `is_changed()` consumer, because some only worked because the resource changed every frame. `update_shadows_for_time_of_day_system` also reads `GraphicsSettings` and the sun's `GlobalTransform`, and `update_volumetric_cloud_lighting_system` must also run for freshly spawned clouds, so their gates now include every input they read.

---

## Custom `as_bind_group` Materials Leak Their Bind Group on Every Modification (Fixed 2026-09-30, Bevy 0.19.1)

### Problem
Cloud, starry-sky, volumetric-cloud and water materials leaked a GPU bind group (with its buffers) each time the material asset was modified, which happened every frame.

### Root Cause
In Bevy 0.19.1 a material whose `unprepared_bind_group` returns `AsBindGroupError::CreateBindGroupDirectly` goes through a prepare path that allocates a new bind group slot for each modification without freeing the old one.

### Fix
Implement `unprepared_bind_group` and return `UnpreparedBindGroup { bindings: BindingResources(vec![(0, OwnedBindingResource::Buffer(buffer)), ..]) }` (`OwnedBindingResource::TextureView` / `Sampler` for textures) so Bevy's allocator owns and frees the bind group. `TerrainMaterial` (texture-view array) cannot use this path; it is mitigated by one shared material per zone and write-on-change updates.

### Files Modified
- `src/render/cloud_material.rs`, `src/render/starry_sky_material.rs`, `src/render/volumetric_cloud.rs`, `src/render/water_material.rs`

### Lesson Learned
Do not use `CreateBindGroupDirectly` for materials that are ever modified. Combined with the entry above (identical writes still count as modifications), every frame leaked a bind group.

---

## Shared Model-Part Materials Need Copy-on-Write for Per-Entity Writes (2026-09-30)

### Problem
Model parts (characters, NPCs, monsters, vehicles, item drops) created one `ExtendedMaterial<StandardMaterial, RoseObjectExtension>` per part per spawn, so 20 monsters of one type meant 60-100 bind groups and no batching. Sharing them naively would make blood painted on one monster appear on every monster of that type.

### Fix
`spawn_model` (`src/model_loader.rs`) shares one material per `PartMaterialKey` (texture path, specular image, alpha mode, two-sided) and marks each part with `SharedModelPartMaterial`. `blood_overlay_generate_system` clones the material into a private copy before the first write for such a part, swaps the part's `MeshMaterial3d` to it and removes the marker. Asset caches store `AssetId`s and reuse them via `Assets::get_strong_handle`, so they keep nothing alive.

### Files Modified
- `src/model_loader.rs`, `src/systems/blood_overlay_system.rs`

### Lesson Learned
Any new system that writes per-entity values into a model part's material must copy first when the part has `SharedModelPartMaterial` (see `system-architecture/model-spawning.md`).
