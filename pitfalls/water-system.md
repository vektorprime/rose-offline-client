# Water System Pitfalls

This document records water-related issues encountered during development.

---

## Water Material Not Rendering (Fixed 2026-02-19)

### Problem
Water planes were not visible in the game after porting from Bevy 0.11 to Bevy 0.15.

### Root Cause
The water shader (`water_material.wgsl`) was using `view.time` for animation, but in Bevy 0.15.4, the `View` struct no longer has a `time` field. Time is now stored in a separate `Globals` struct accessed via `globals.time`.

**Before (Bevy 0.11):**
```wgsl
#import bevy_pbr::mesh_view_bindings view
// ...
let time = view.time * 10.0;
```

**After (Bevy 0.15.4):**
```wgsl
#import bevy_pbr::mesh_view_bindings::{view, globals}
// ...
let time = globals.time * 10.0;
```

### Solution
1. Updated shader import to include `globals` from `mesh_view_bindings`
2. Changed `view.time` to `globals.time`
3. Changed `view.inverse_view` to `view.view_from_world` (the correct field name in Bevy 0.15.4)

### Files Modified
- `src/render/shaders/water_material.wgsl` - Updated shader imports and time access

### Key Changes in Bevy 0.15.4 WGSL API
| Bevy 0.11 | Bevy 0.15.4 |
|-----------|-------------|
| `view.time` | `globals.time` |
| `view.inverse_view` | `view.view_from_world` |
| `#import bevy_pbr::mesh_view_bindings view` | `#import bevy_pbr::mesh_view_bindings::{view, globals}` |

### Lesson Learned
When porting custom shaders between Bevy versions, check the WGSL struct definitions in the Bevy source code:
- `crates/bevy_render/src/view/view.wgsl` - View struct definition
- `crates/bevy_render/src/globals.wgsl` - Globals struct definition
- `crates/bevy_pbr/src/render/mesh_view_bindings.wgsl` - Available bindings

---

## Water Not Rendering After Bevy 0.16.1 Migration (Fixed 2026-02-22)

### Problem
Water was not loading/rendering at all in the game after upgrading from Bevy 0.15.4 to Bevy 0.16.1.

### Root Cause
Breaking API change in Bevy 0.16's `AsBindGroup` trait. The migration guide states:
> "Bevy will now unconditionally call `AsBindGroup::unprepared_bind_group` for your materials, so you must no longer panic in that function. Instead, return the new `AsBindGroupError::CreateBindGroupDirectly` error, and Bevy will fall back to calling `AsBindGroup::as_bind_group` as before."

### Solution
Changed the return value in `unprepared_bind_group()` at [`src/render/water_material.rs:325`](src/render/water_material.rs:325):

```rust
// Before (broken - infinite retry loop):
Err(AsBindGroupError::RetryNextUpdate)

// After (fixed):
Err(AsBindGroupError::CreateBindGroupDirectly)
```

### Why It Works
`CreateBindGroupDirectly` tells Bevy "I implement `as_bind_group()` directly, call that instead." This allows the water material's custom bind group creation with texture arrays to work properly.

### Files Modified
- `src/render/water_material.rs` (line 325) - Changed `RetryNextUpdate` to `CreateBindGroupDirectly`

### Lesson Learned
When implementing custom materials with `AsBindGroup::as_bind_group()` override in Bevy 0.16+:
1. Always return `Err(AsBindGroupError::CreateBindGroupDirectly)` from `unprepared_bind_group()` - this signals Bevy to use your custom `as_bind_group()` implementation
2. Never return `RetryNextUpdate` from `unprepared_bind_group()` in Bevy 0.16+ - it causes an infinite retry loop since Bevy now calls this method unconditionally

---

## Fish Not Appearing in Water (Fixed 2026-02-19)

### Problem
Fish were not appearing in water areas despite the fish spawning system being implemented and events being sent correctly.

### Root Cause
Fish entities were spawning at local water coordinates but were **not parented to the zone entity**. Since zones have a transform offset of `(5200.0, 0.0, -5200.0)`, the fish were appearing at incorrect world positions.

For example:
- Fish local position: `(410.0, -8.0, 0.0)`
- Expected world position: `(5610.0, -8.0, 0.0)` (local + zone offset)
- Actual world position: `(410.0, -8.0, 0.0)` (no parent, so no transform inheritance)

### Solution
1. Added `zone_entity: Entity` field to `WaterSpawnedEvent` struct
2. Updated `spawn_fish_in_water()` function to accept `zone_entity` parameter
3. Added parenting: `commands.entity(zone_entity).add_child(fish_entity);`
4. Updated `zone_loader.rs` to pass `zone_entity` when sending the event

### Code Changes
```rust
// WaterSpawnedEvent - added zone_entity field
pub struct WaterSpawnedEvent {
    pub water_entity: Entity,
    pub zone_entity: Entity,  // NEW: Required for transform inheritance
    pub water_center: Vec3,
    pub water_half_extents: Vec2,
}

// spawn_fish_in_water - parent fish to zone
fn spawn_fish_in_water(
    water_entity: Entity,
    zone_entity: Entity,  // NEW parameter
    water_center: Vec3,
    // ...
) {
    // ... spawn fish_entity ...
    
    // Parent fish to zone entity so it inherits zone transform
    commands.entity(zone_entity).add_child(fish_entity);
}

// zone_loader.rs - pass zone_entity in event
water_spawned_events.send(WaterSpawnedEvent {
    water_entity,
    zone_entity,  // NEW: Pass the zone entity
    water_center,
    water_half_extents,
});
```

### Files Modified
- `src/components/fish.rs` - Added `zone_entity` field to `WaterSpawnedEvent`
- `src/systems/fish_system.rs` - Updated `spawn_fish_in_water()` to parent fish to zone
- `src/zone_loader.rs` - Pass `zone_entity` when sending `WaterSpawnedEvent`
- `src/ui/ui_settings_system.rs` - Added Fish settings tab to Settings UI

### Lesson Learned
When spawning entities that should appear within a transformed parent (like a zone with offset):
1. **Always parent child entities to the zone** - Without parenting, children won't inherit the parent's transform
2. **Zone offset matters** - Zones are positioned at `(5200.0, 0.0, -5200.0)` to center them in the world
3. **Event data must include parent reference** - Events that trigger entity spawning should include the parent entity reference
4. **Debug with world positions** - When debugging visibility issues, check both local and world positions to identify transform inheritance problems
