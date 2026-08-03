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

---

## Fish Lined Up in Rows / Converged and Static / Huge Perf Hit (Fixed 2026-08-02)

### Problem
1. Fish spawned lined up in visible rows and stayed that way.
2. In many areas fish were all converged in one spot, seemingly not moving, while other areas looked fine.
3. After adding a separation force, the game became much slower while CPU/MEM/GPU usage looked low.

### Root Cause
1. **Spawn pattern**: schools were placed on a fixed 8x8 stratified grid in row-major order. With ~118 schools for 64 cells, schools wrapped around (`% grid_cells_z`) and doubled up; members clustered within ±5% of the water width at one shared depth. Result: visible parallel rows of tight clusters.
2. **Shared school target**: all members of a school converged on the same point (±0.5 m), so clusters collapsed into lines/points.
3. **Wobble bug (the "not moving" look)**: the swim wobble was added to position every frame WITHOUT scaling by `delta`. At 60 fps a fast fish weaved ~2.4 m/s sideways vs 2 m/s forward, so fish vibrated in place with near-zero net progress. Same bug on the vertical wobble.
4. **No separation force**: fish have no collision, so overlapping spawn piles never spread apart.
5. **NaN freeze risk**: `direction / distance` with distance == 0 produces NaN, permanently corrupting the fish transform.
6. **O(n²) separation loop**: naive pairwise neighbor scan over all fish per frame cost tens of ms/frame in the unoptimized debug build (single core pegged — reads as low overall CPU% in Task Manager).

### Solution (src/systems/fish_system.rs)
1. Replaced the 8x8 grid with uniform random school centers (no rows, no wrap).
2. Per-member targets instead of one shared school target — schools disperse naturally.
3. Member spread scales with school size (`0.8 * sqrt(school_size)`), capped to the water plane size; per-member depth jitter (±0.3 m) so schools aren't flat planes.
4. Scaled all wobble additions by `delta` (per-second lateral speed ~0.03 m/s instead of per-frame).
5. Added separation force (radius 0.5 m, push ≤ 2 m/s, frame-rate independent via delta).
6. NaN guard: `!distance.is_finite() || distance < reach` → pick new target.
7. `pick_new_target` guards against zero-size water planes (`extents.max(0.001)`) — an empty `gen_range` range panics and disables the whole movement system.
8. **Perf fix**: separation uses an X-sorted index sweep — the inner scan breaks as soon as the X delta exceeds the 0.5 m radius (O(n·k) instead of O(n²)).

### Lesson Learned
- Any per-frame displacement added in a per-frame system must be scaled by `delta` or it becomes frame-rate-dependent and can dominate real movement.
- Neighborhood queries need a spatial acceleration (sorted sweep, grid, hash) — naive O(n²) pairwise scans are unusable in debug builds with a few hundred entities per area.
- `rng.gen_range(a..b)` panics on empty ranges — guard degenerate extents, since a single system panic disables the system and appears as frozen behavior.

---

## Fish Still Clumped and Stationary After First Fix (Fixed 2026-08-02, user confirmed)

### Problem
After the row/wobble/O(n²) fixes, many areas still showed fish packed together in static clumps while other areas looked realistic.

### Root Cause
1. **Fixed count per water plane**: `fish_count_per_water: 400` applied to every plane regardless of size. Zones build water from many small per-block IFO planes, so a 10x10 m pond received the same 400 fish as a huge lake (~4 fish/m²) — a packed, churning ball whose targets all lie inside the same tiny area, so the clump never goes anywhere. Large planes spread the same 400 thin and looked normal.
2. **Separation skipped fish from different water planes** (`water_center` equality check). Fish where adjacent/overlapping planes meet could stack forever and never push apart.
3. **Separation radius (0.5 m) smaller than large fish** (tuna ~1.1 m long) — big fish interpenetrated.
4. **No distance culling**: every fish in every loaded plane simulated every frame, amplifying the cost of over-populated planes.

### Solution (src/components/fish.rs, src/systems/fish_system.rs, src/ui/ui_settings_system.rs)
1. Area-based count: `fish_per_1000_sqm` (default 50) x plane area, clamped to `[min_fish_per_water=8, max_fish_per_water=150]`; density 0 disables fish. Mirrors the existing `BirdSettings` density pattern.
2. Separation now uses world-space positions (`GlobalTransform` snapshot) with no same-water check, so fish from different planes also separate. Pushes are world directions applied to local translation — valid because zone parents are pure translations.
3. Separation radius raised to 0.9 m; `target_reach_distance` lowered 1.0 -> 0.4 m (fewer no-move re-target frames on small planes).
4. Camera-distance cull: fish beyond `simulation_distance` (default 80 m) are skipped entirely; disabled when no camera exists.
5. Spawn extents guarded (`max(0.001)`) like `pick_new_target` already was; clamp range normalized so misconfigured min/max cannot panic.

### Lesson Learned
- Ambient-creature counts must scale with the area they live in — a fixed count per spawn region over-packs small regions no matter how good the per-fish AI is.
- When separating entities that come from multiple sources/regions, keying the check on the source (water plane) can silently disable separation exactly where sources overlap.

