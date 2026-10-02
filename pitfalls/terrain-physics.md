# Terrain and Physics Pitfalls

This document records terrain and physics-related issues encountered during development.

---

## Terrain Adherence Bug - Player Could Not Descend Below Spawn Height (Fixed 2026-02-18)

### Problem
Player character could ascend terrain slopes but could not descend below the elevation coordinate it spawned at. The spawn height was effectively treated as a minimum floor. Additionally, NPCs and monsters were spawning at extremely high elevation (y=9702m) instead of at terrain level.

### Root Causes
1. **Zone Asset Timing**: In `zone_loader.rs`, the zone asset was being sent via `ZoneLoadedFromVfsEvent` BEFORE being added to the `Assets<ZoneLoaderAsset>` collection. This meant `collision_player_system` couldn't access terrain height data when trying to compute ground position.

2. **NPC/Monster Spawn Height**: Server sends `position.z = 0.00` for NPCs/monsters, and spawn code was using arbitrary `+ 10000.0` offset instead of querying terrain height from the zone heightmap.

3. **Bevy Bundle Tuple Limit**: Bevy's `Bundle` trait implementation has a maximum of ~15 components per tuple. Spawn handlers exceeded this limit, causing compilation errors.

### Solution
1. **Zone Asset Timing**: Moved `zone_loader_assets.add(zone_asset)` to occur BEFORE sending `ZoneLoadedFromVfsEvent`.

2. **Terrain Height Helper**: Added `get_spawn_height_from_world()` function that accesses `CurrentZone` and `Assets<ZoneLoaderAsset>` from the World to query terrain height from the zone heightmap.

3. **Deferred Spawning with Split Inserts**: Used `commands.queue()` with closures to spawn entities, splitting component inserts into two phases to avoid tuple size limits:
```rust
// Phase 1: Spawn with core components (~13)
let entity = world.spawn((...core components...)).id();

// Phase 2: Add remaining components
world.entity_mut(entity).insert((...remaining components...));
```

### Files Modified
- `src/zone_loader.rs` (lines 1177-1183) - Zone asset added to Assets before event sent
- `src/systems/game_connection_system.rs` - Added terrain height helper, split spawn handlers
- `src/systems/collision_system.rs` - Re-enabled `collision_height_only_system`

### Key Code Pattern for Deferred Spawning with World Access
```rust
commands.queue(move |world: &mut World| {
    // Get terrain height at spawn position
    let spawn_y = get_spawn_height_from_world(world, position.x, position.y);
    
    // Spawn with core components first
    let entity = world.spawn((
        // ... core components (up to ~14)
    )).id();

    // Add remaining components in a second insert
    world.entity_mut(entity).insert((
        // ... remaining components
        Transform::from_xyz(position.x / 100.0, spawn_y, -position.y / 100.0),
    ));
});
```

### Lesson Learned
1. When using `commands.queue()` with closures, you can access World resources directly via `world.get_resource::<T>()`
2. Bevy's Bundle trait implementation limits tuples to ~15 components - split large spawns into multiple `insert()` calls
3. Zone/level data must be added to Assets collection BEFORE events trigger systems that depend on that data
4. Server position data (especially z/height) may be unreliable - use client-side terrain heightmap for ground placement

---

## NPCs / Bots Stuck Under Elevated Platforms (Fixed 2026-10-02)

### Problem
At the Junon Polis north gate (platform top 1.6 m above terrain), NPCs, vendor bots and the buddy bot Talez sometimes stood on the terrain under the platform (name tag visible, model hidden or only the hat poking through). Intermittent: right after `/mm 2` it failed, on a later visit it worked.

### Root Cause
1. Zone object colliders are built only after the object meshes load, but the server spawns the zone's NPCs right after JoinZoneRequest. With no platform collider the ground ray hit terrain and the NPCs fell to it.
2. Characters (bots, vendors, server-teleported buddy bots) were placed at terrain height; the server's z was ignored and overwritten.
3. Recovery was impossible: the height-only ground ray started only 1 m above the feet (below the 1.6 m top), and the "inside an object" test used an overlap ball, but trimesh colliders are hollow surfaces, so it never touched a face.

### Solution
In `collision_height_only_system` (`src/systems/collision_system.rs`):
- No descending while any `SharedMeshCollider` is still pending (capped at 20 s).
- Characters/NPCs start at the server's height on their first frame when a MOVEABLE object surface lies within a step of it (deferred until colliders exist).
- Stuck-inside lift via one upward ray: a first hit on a trimesh BACK face means the feet are inside that part; lift onto it (not for bridge undersides, parts not reaching the feet, steep tops, or > 5 m while walking).
- Ground ray starts 1.35 m above the feet (the player's step) and uses MOVEABLE only, like the player and the original client.

### Files Modified
- `src/systems/collision_system.rs`, `system-architecture/Physics.md`

### Lesson Learned
Entity heights computed against zone colliders must account for colliders that do not exist yet after a zone load. Trimesh colliders are hollow: use ray hits (and back-face tests) rather than overlap shapes to detect being inside an object. The server z is useful as a hint when an object surface supports it.
