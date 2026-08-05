# Optimization Review 04 — Physics, Collision, Movement, Terrain Adherence, Flight, Sailing, Monster Separation

**Repo**: `rose-offline-client` (single crate, Bevy 0.18.1, bevy_rapier3d 0.33.0)
**Date**: 2026-08-03
**Type**: Research + report only. No `.rs` files were modified, no build was run, no game was launched.

---

## 1. Scope Summary

Architecture docs that existed in `system-architecture/` before this review (read in full):

| Doc | Status | Relevance |
|---|---|---|
| `system-architecture/Physics.md` | existed, read | Rapier integration, scene-query patterns, collision groups, system ordering |
| `system-architecture/Transform.md` | existed, read | Coordinate conversions, transform propagation |
| `system-architecture/monster-collision-system.md` | existed, read | Design plan for monster separation (O(n²) explicitly accepted; "acceptable for <20 monsters" — now at risk) |
| `system-architecture/flying-system-architecture.md` | existed, read | Flight system design, wind particles, pose system |
| `system-architecture/ECS.md`, `Render.md` etc. | existed, not read (out of scope) | — |

Pitfalls read: `pitfalls/index.md`, `pitfalls/terrain-physics.md`, `pitfalls/flying.md`, `pitfalls/water-system.md`. The water-system pitfall documents the exact O(n²) → X-sorted-sweep fix already applied to fish — that pattern is the reference solution for the same bug in `monster_separation_system`.

Files analyzed (all in `src/`): `systems/collision_system.rs`, `systems/monster_separation_system.rs`, `systems/update_position_system.rs`, `systems/facing_direction_system.rs`, `systems/move_speed_set_system.rs`, `systems/move_speed_command_system.rs`, `systems/passive_recovery_system.rs`, `systems/flight_movement_system.rs`, `systems/flight_pose_system.rs`, `systems/flight_toggle_system.rs`, `systems/flight_command_system.rs`, `systems/sailing_movement_system.rs`, `systems/boat_buoyancy_system.rs`, `systems/boat_wake_system.rs`, `systems/sail_camera_system.rs`, `systems/remote_boat_system.rs`, `systems/boat_spawn_system.rs`, `systems/character_model_add_collider_system.rs`, `systems/npc_model_add_collider_system.rs`, `systems/game_connection_system.rs` (spawn/MoveEntity paths), `components/collision.rs`, `components/monster_separation.rs`, `components/position.rs`, `components/flight.rs`, `components/boat.rs`, `components/boat_wake.rs`, `components/remote_boat.rs`, `components/facing_direction.rs`, `components/vehicle.rs`, `components/vehicle_model.rs`, `resources/flight_settings.rs`, `resources/water_settings.rs`, `resources/world_rates.rs`, `zone_loader/spawning/terrain.rs`, `terrain/noise_overlay.rs`, `zone_loader.rs` (`get_terrain_height`), `sailing.rs`, `zone_content/boats.rs`, `zone_content/monsters.rs`, `render/underwater_effect.rs`, plus system registration/ordering in `lib.rs` (lines ~1390–1560).

Key architecture facts confirmed from source:

- **Rapier is used query-only** (NoUserData). No dynamic rigid bodies; `RigidBody::Fixed` terrain trimeshes (one per 160×160 m block, `terrain.rs:290`), fixed character cuboids parented to skinned root bones (`character_model_add_collider_system.rs:132-134`).
- **All ground following is per-frame scene queries** (raycast + shape intersect) in `collision_player_system` (player) and `collision_height_only_system` (every server-spawned NPC/monster — `CollisionHeightOnly` inserted for every entity at `game_connection_system.rs:110`).
- **Position is server-authoritative (cm); Transform is render-space (m)**; conversions `x/100`, `z/100`, `-y/100` everywhere.
- System chain in `Update`: `facing_direction_system → update_position_system → monster_separation_system → collision_height_only_system → collision_player_system` (`lib.rs:1397-1406`), all mutating shared `Position`/`Transform`/`FacingDirection` components, so they serialize.
- Physics ordering: `GameStages::AfterUpdate` before `PhysicsSet::SyncBackend` (`lib.rs:1705-1718`).

---

## 2. Methodology

1. Read the four architecture docs and three pitfall entries listed above.
2. Read every file in the file list above end-to-end (line-numbered).
3. Traced spawn paths to count which components/entities feed each system (`CollisionHeightOnly` at `game_connection_system.rs:110`, `MonsterSeparation` at `game_connection_system.rs:717` and `zone_content/monsters.rs:296`).
4. Traced `get_terrain_height` call sites (15 matches) to quantify per-frame terrain sampling.
5. Cross-checked cost claims against bevy_rapier3d 0.33 scene-query semantics and Bevy 0.18.1 change-detection/transform-propagation behavior (source available at `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\` and `bevy_rapier3d-0.33.0\src\`; the repo's own `Physics.md` and `Transform.md` already document both).
6. No builds, no runs, no edits.

Cost notation used below: **per-frame per-entity** counts assume a populated zone (hundreds of NPCs/monsters) and the ocean zone (zone 200: 18 NPC boats, 10 sharks, many water planes). All estimates are relative ordering, not benchmarked numbers.

---

## 3. Findings

### F1 — Every NPC runs 2–3 Rapier scene queries every frame, unconditionally
**Files**: `src/systems/collision_system.rs:125-230`; marker applied at `src/systems/game_connection_system.rs:110`

`collision_height_only_system` iterates **all** NPCs/monsters each frame and, per entity, performs:
- a downward `cast_ray` with `max_fall_distance = 100.0` (lines 172-192),
- an `intersect_shape` feet sphere (inside `find_object_top_height`, line 208 → line 78),
- plus a heightmap sample (`get_terrain_height`, line 157).

**Why it matters**: A zone with ~300 NPCs/monsters costs ~900 Rapier scene queries/frame (broadphase + BVH traversal through 4,000+ triangle block trimeshes, plus every zone-object collider). None of this is gated on whether the entity moved. Rapier scene queries are not free: each `cast_ray` does an AABB-to-ray broadphase pass plus triangle/object tests. This is the single largest physics CPU sink in the game. The `Assets::get`/`CurrentZone` guards are fine (done once per system call, not per entity).

**Suggested fix**: Gate per-entity work on horizontal movement. Add a `GroundHeightCache` component (`last_x_cm: f32, last_y_cm: f32, cached_ground_y: f32`) on each `CollisionHeightOnly` entity. If `(position.x, position.y)` is unchanged since last frame, skip the raycast + intersect entirely and reuse `cached_ground_y`; only re-run gravity integration and `position.z` writeback:

```rust
if (pos.x - cache.last_x).abs() > 1.0 || (pos.y - cache.last_y).abs() > 1.0 {
    cache.last_x = pos.x; cache.last_y = pos.y;
    cache.ground_y = /* raycast + object-top (throttled, see F2) */;
}
let target_y = cache.ground_y;
// gravity integration + transform.y + position.z writeback as today
```

Idle NPCs (the vast majority) then cost one heightmap sample (which is also cacheable — see F8) and zero scene queries.

---

### F2 — `find_object_top_height`: up to 64 upward raycasts + heap Vec + O(k) predicate per call, run every frame
**File**: `src/systems/collision_system.rs:55-122` (called at :208 for every NPC and :577 for the player)

Each call:
- allocates `intersecting_objects: Vec` (line 77) and pushes every hit entity (lines 83-86),
- then loops up to **64 times** (line 101), each iteration building a fresh `QueryFilter` whose `predicate` closure does an O(k) `Vec::contains` (line 102), and casts an upward ray (line 110-111),
- worst case (entity inside a big building): 64 full scene queries with an allocation-heavy closure.

For the player this runs every frame even when standing still on flat ground (the feet sphere only intersects objects in the rare "spawned inside castle steps" case).

**Why it matters**: 2 scene queries per entity per frame just to discover "nothing to do" (the intersect finds nothing and returns early). For the player it adds 1 intersect + 1 Vec alloc per frame. For entities actually inside objects (rare but possible after teleports), the 64-iteration loop is a spike that can cost milliseconds.

**Suggested fix**:
1. Only run it when the entity is plausibly inside an object: skip when the downward ray's hit is at/below the feet level minus epsilon (i.e. standing in open air over terrain), or
2. Throttle to ~4 Hz using a `Local<HashMap<Entity, f32>>` of last-run timestamps (unsticking from an object after 250 ms is imperceptible), and
3. Replace the per-call `Vec` + closure predicate with a single reusable `Local<Vec<Entity>>`/buffer and an `exclude_collider` chain driven by the previous hit (the Vec is only needed because the filter excludes one collider at a time).

---

### F3 — Downward ground rays include the terrain trimesh, which is redundant with the heightmap
**Files**: `src/systems/collision_system.rs:178-186` (NPC ray filter), `:550-553` (player ray filter), `:536-559` (player ray); terrain collider groups at `src/zone_loader/spawning/terrain.rs:288-299`

Both downward rays are filtered with `COLLISION_FILTER_MOVEABLE` memberships / `!PHYSICS_TOY` filters — the terrain trimesh collider (`memberships = ZONE_TERRAIN`, `filters` includes `MOVEABLE`) **matches** these queries, so every ground ray sweeps the entire nearby terrain BVH (per block: 65×65 grid → ~8,192 triangles; several blocks loaded per zone). But the code **already computes the terrain ground from the heightmap** at `collision_system.rs:157` and `:562` (`get_terrain_height`), and the trimesh is built from the identical grid + the same noise (`terrain.rs:217-244`), so the ray's terrain hit is a strictly redundant duplicate of `terrain_height` — the code then takes `max()` of both.

The player variant additionally casts with `max_fall_distance = 10000.0` (line 542) — a 10 km ray while the player can be at most a few hundred meters above the map (NPC variant already uses `100.0`, line 170).

**Why it matters**: The most expensive colliders in the world (terrain trimeshes) are traversed by every ground ray every frame (see F1). Excluding terrain from these rays turns each query into a scan over only the sparse zone-object set (bridges, platforms, steps), which is the only thing the ray exists for.

**Suggested fix**: Add `& !COLLISION_GROUP_ZONE_TERRAIN` to the filters of both downward rays — exactly the pattern already used for the wall cast at `collision_system.rs:505`. Then shorten `max_fall_distance` to ~50 m (player) / ~20 m (NPC): zone objects never float higher than that above the ground, and the heightmap covers terrain. Behavior is unchanged because `target_y = max(ray_hit, terrain_height)` still uses the heightmap for ground.

```rust
QueryFilter::new().groups(CollisionGroups::new(
    COLLISION_FILTER_MOVEABLE,
    !COLLISION_GROUP_PHYSICS_TOY & !COLLISION_GROUP_ZONE_TERRAIN,
))
```

---

### F4 — `Collider::ball(...)` heap-allocated on every use, every frame
**Files**: `src/systems/collision_system.rs:76` (feet gate ball), `:354` (sailing wall cast), `:490` (player wall cast), `:597` (event/warp probe)

Each call constructs a fresh ball collider (`Collider::ball(r)` allocates an `Arc<dyn Shape>`). The player system allocates 3/frame, the NPC system 1–2/entity/frame, plus a `Vec` in `find_object_top_height` (F2).

**Why it matters**: minor per allocation, but it's pure garbage on hot paths that run hundreds of times per frame; removing it is a one-line-per-site change.

**Suggested fix**: Pre-create once per system invocation with `Local<Collider>` (or two `Local<Collider>` for the two radii in the player system) and pass `&*collider` via `From<&Collider> for &dyn Shape`... note `bevy_rapier3d`'s `ShapeCastOptions`/`intersect_shape` take `&dyn Shape`; a `Local<Collider>` reused across the loop is fine because Rapier copies the shape data into the query.

---

### F5 — Transform/Position write churn for every NPC every frame
**Files**: `src/systems/collision_system.rs:216-217` (NPC X/Z sync), `:228` (NPC `position.z` writeback), `:473-475` / `:583-584` (player sync), `src/systems/facing_direction_system.rs:40-43`

`collision_height_only_system` unconditionally writes `transform.translation.x/z` and `position.z` for **every** NPC each frame — even when `Position` did not change. In Bevy 0.18, any write marks the component changed, which:
- makes Bevy's change detection (`Changed<T>` filters in other systems, e.g. render extraction, skin systems) treat the entity as dirty, and
- forces transform propagation (`TransformSystems::Propagate` in PostUpdate) to recompute `GlobalTransform` for the whole skinned hierarchy, and the render extractor to re-extract the entity.

**Why it matters**: With hundreds of idle NPCs, this converts a no-op frame into hundreds of marked-dirty entities and their full hierarchy propagation — pure avoidable cost, and it compounds F1's savings because the entities that skip queries are also the ones whose writes can be skipped.

**Suggested fix**: Only write when the value actually changes:

```rust
let new_x = position.x / 100.0;
let new_z = -position.y / 100.0;
if transform.translation.x != new_x { transform.translation.x = new_x; }
if transform.translation.z != new_z { transform.translation.z = new_z; }
```

Same guard for `position.z` writeback (line 228) and for the player sync block (lines 583-584). Note the ground-ray gating from F1 makes the Y-write the only remaining per-frame mutation for idle NPCs; guard it with `if old_y != new_y`.

---

### F6 — `monster_separation_system` is O(n²) with a per-frame Vec allocation
**File**: `src/systems/monster_separation_system.rs:13-51` (collect at :13-20, nested scan at :33-51)

Every frame, the system collects all monsters into a `Vec<(Entity, Vec3, f32)>` (heap allocation), then for each monster loops over **all** others computing `(a-b).length()` (sqrt). With n monsters this is n² sqrt operations per frame (plus `.normalize()` per overlap).

**Why it matters**: The design doc (`system-architecture/monster-collision-system.md:295`) explicitly accepted O(n²) "for <20 monsters". Zones routinely exceed that; at n=150 that is 22,500 sqrt/length computations + a 150-entry Vec allocation per frame. The fish system hit exactly this wall (`pitfalls/water-system.md` — O(n²) fish separation pegged a core and cost "tens of ms/frame in debug"; the fix was an X-sorted sweep at `fish_system.rs:715-725`). The same fix is directly transferable.

**Suggested fix**: Copy the fish pattern: sort the collected positions once by X, then in the inner loop break as soon as `other_pos.x - my_pos.x > max_radius_cm` (use squared distance and `abs_diff` on X to early-out). O(n log n + n·k). Also skip the collect entirely when `query.iter().count()` is small, and reuse a `Local<Vec<_>>` instead of re-allocating:

```rust
let mut idx: Vec<usize> = (0..monsters.len()).collect();
idx.sort_by(|&a, &b| monsters[a].1.x.total_cmp(&monsters[b].1.x));
// per monster: scan idx until dx > (r_a + r_max) then break
```

---

### F7 — `get_terrain_height`: per-call block math + bilinear sample + RefCell TLS + up to 4-octave Perlin; called ~100+ times/frame in the ocean zone
**Files**: `src/zone_loader.rs:358-398` (height query), `src/terrain/noise_overlay.rs:98-133` (fBm noise), `:152-160` (TLS access); call sites: `collision_system.rs:157, 407, 562` (per entity/frame), `flight_movement_system.rs:46-62` (per frame), `zone_content/monsters.rs:507` (per shark/frame), `zone_content/boats.rs:218-226` (per navigation probe, up to 5/boat/frame — see F8), `systems/animation_sound_system.rs:129`, `boat_spawn_system.rs:86` (event only)

Each call does: 2 divisions + clamp + index math → heightmap block lookup → 4 `get_clamped` heightmap reads → bilinear blend → `TERRAIN_NOISE.with(|cell| ...)` TLS borrow → if noise is enabled, 4 octaves of `Perlin::get` (each a trig-heavy gradient sample). With the default `noise_enabled = false` the noise short-circuits, but the TLS borrow and block lookup still run.

**Why it matters**: The ocean-zone path alone calls this 90–110 times/frame (18 NPC boats × up to 5 probes + 10 sharks + player + NPCs), plus once per NPC per frame in F1. With noise enabled (a supported settings toggle), every call becomes ~4 Perlin octaves — measurable. And because terrain height is deterministic and static per zone, all these recomputations are of values that change only when the entity moves.

**Suggested fix** (pick one):
1. **Per-frame cache**: a `Local<HashMap<(i32, i32), f32>>` keyed on `(x / 100 as i32, y / 100 as i32)` (10 cm cells), cleared once per frame. NPC-boat probes within one step (~15 cm) collapse to ~1-2 misses/frame. Cheap and localized.
2. **Per-zone height grid**: precompute a `Vec<f32>` height LUT (e.g. 1 m cells) when the zone loads and make `get_terrain_height` a pure LUT lookup; noise baked in at build time. Removes the TLS + noise cost from the hot path entirely.
3. Keep the noise function but hoist the `noise_enabled` check and the octave loop into a single fused function that returns `(base, noise)` in one pass.

Also note: `get_thread_local_noise` and `GlobalTerrainNoise` are initialized once with `TerrainEnhancementSettings` (default `noise_enabled = false`); if settings change at runtime the thread-local and the terrain collider/noise can diverge — the collider was baked with the settings at zone-build time. Any caching fix must key on the settings version or rebuild the collider when noise changes.

---

### F8 — `npc_boat_movement_system`: up to 5 terrain samples + full dock scan per boat per frame
**File**: `src/zone_content/boats.rs:391-559` (`is_position_navigable` at :218-227 calls `get_terrain_height` + `is_clear_of_docks`; invoked at :484 candidate, :495 slide_x, :499 slide_y, :531/:532 both side probes; `boat_positions` Vec snapshot at :419-422; per-frame dock transform collection at :412-415)

18 boats × (1 candidate + up to 2 slides + 2 probes when blocked) height samples/frame, each also re-scanning every dock (`is_clear_of_docks` iterates all docks per call; `docks` query re-collected every frame, even though dock transforms are static).

**Why it matters**: Compounds F7; this is the dominant terrain-sampling consumer in the ocean zone. The dock set is static per zone but is re-snapped and re-tested every frame and every probe.

**Suggested fix**:
- Hoist `dock_spaces` collection into a `Local` that refreshes only on zone change (or on `ZoneEvent`), not every frame.
- Sort `dock_spaces` by X and early-out the dock loop on `local.x` delta (same sweep trick as F6/F7), or reject the boat with one distance check before iterating docks.
- Cache the previous frame's `is_position_navigable` result per boat (a step is ≤ ~0.15 m; the slide/probe logic only matters when the candidate was blocked — skip probes entirely when the candidate was clear last frame and is still clear this frame).
- The `boat_positions` snapshot Vec (line 419) is fine at n=18 (reuse a `Local<Vec>` anyway).

---

### F9 — Wake/spray particle counting is O(boats × alive particles) per frame
**File**: `src/systems/boat_wake_system.rs:114-121` (counts per boat), `:174-182` (re-count inside the spray burst loop)

For every active boat, the spawn system iterates **all** wake particles and **all** spray particles (`iter().filter(|s| s.boat_entity == boat).count()`). With max 100 wake + 30 spray per boat and 19 boats, that is 19 × 2 × ~2,500 entity iterations ≈ 95,000 filter iterations/frame, and the spray branch re-runs its count inside the burst loop (up to 5×).

**Why it matters**: pure O(n²)-style waste; scales badly if NPC-boat counts or particle caps grow. The information ("how many particles does boat X have") is cheaply maintainable.

**Suggested fix**: Track counts incrementally: add `wake_count: usize, spray_count: usize` to `WakeEmitter`, increment on spawn, decrement in `boat_wake_update_system` when a particle despawns (it already has `WakeSource.boat_entity` and `Commands` — or do the decrement in the same system that despawns). Remove both per-boat count queries. Keep the global caps.

---

### F10 — Every wake/spray particle swaps its material handle every frame
**File**: `src/systems/boat_wake_system.rs:264, 283` (+ `material_for_alpha` at :38-46)

Each frame every alive particle reassigns `material_handle.0` from one of the 8 pre-baked alpha materials. A changed `MeshMaterial3d` marks the entity for re-extraction in the render world (component change → extractor runs → render command regeneration) for every particle, every frame.

**Why it matters**: with ~500-2,500 wake/spray particles alive, that's hundreds of material handles rewritten and re-extracted per frame even when the alpha bucket doesn't change. The alpha bucket changes at most ~8 times over a particle's lifetime, not every frame.

**Suggested fix**: Store the current bucket index on the particle (`alpha_bucket: u8`), and only assign a new handle when the bucket changes. Fading can be quantized to the bucket cadence (already the visual design). Optionally halve buckets to 4 for a coarser but cheaper fade. This also removes the `Handle` clone churn (each `material_handle.0 = ...` clones an `Arc`).

---

### F11 — `update_position_system` re-processes and re-writes every mover every frame
**File**: `src/systems/update_position_system.rs:10-37`

Every entity with `Command::Move` + `MoveSpeed` is advanced every frame. When the destination is reached (`distance_squared == 0.0`, line 22) the system writes `position.position = destination` **every subsequent frame** (line 23) — a repeated identical write that keeps marking `Position` changed forever (an arriving-then-idle monster is never clean). There is no `Changed<Command>` gate and no per-entity "arrived" latch.

**Why it matters**: identical-write churn for every idle mover (F5's effect, but from a second system), and pointless `FacingDirection::set_desired_vector` calls for movers that are already aligned.

**Suggested fix**:
- Add an `arrived: bool` flag (or `if command.destination != last_dest` bookkeeping) so once `distance_squared == 0.0` the system stops writing until `Command` actually changes (e.g. gate on `Command` equality: skip when the same Move command was processed last frame — a `Local<HashMap<Entity, CommandMove>>` or a `Changed<Command>` filter, being careful that `Changed` still fires once per command insert).
- Skip `set_desired_vector` when `facing_direction.desired` already matches the computed angle (compare within epsilon).

---

### F12 — Flight: `NextCommand::with_stop()` inserted every frame + `MoveCollision` sent every frame while hovering
**File**: `src/systems/flight_movement_system.rs:120, 149, 164` (inserts), `:171-180` (hover reporting)

While flying, the system inserts `NextCommand::with_stop()` into the player every single frame (three code paths), even when the current `NextCommand` is already Stop — an `insert` still triggers change detection and allocates a new `NextCommand`. While hovering (not thrusting, speed 0), it also sends `ClientMessage::MoveCollision` to the server **every frame** (60 msgs/s) — the sailing system throttles the analogous report to 10 Hz (`SAIL_REPORT_INTERVAL = 0.1`, `sailing_movement_system.rs:11, 121-139`).

**Why it matters**: per-frame message traffic to the server and per-frame component writes during the entire flight duration; both are unnecessary by the project's own precedent.

**Suggested fix**:
- Insert `NextCommand::with_stop()` only when the current component is not already a stop (`query` gains `&mut NextCommand`; `if !matches!(*next_command, NextCommand::Stop(_) | NextCommand::default()) { ... }`).
- Reuse the sailing pattern: a `Local<f32>` accumulator and send `MoveCollision` at 10 Hz. The server accepts absolute positions, so 10 Hz is sufficient to stay under the 500 cm teleport limit (per pitfall `flying.md`, per-frame movement is already well under it).

---

### F13 — `remote_boat_sync_system` iterates and rewrites every client entity every frame
**File**: `src/systems/remote_boat_system.rs:13-133` (query at :19-31, loop body :34-133)

The query pulls `MoveMode + Position + FacingDirection + Option<CharacterModel> + Option<BoatState> + Option<RemoteBoatState>` for **every** non-player `ClientEntity` each frame. Non-sailing entities just `continue` (line 39-52) after the query already touched 6-7 components. Sailing entities get `RemoteBoatState` rewritten wholesale every frame (lines 66-85), including `update_age`/`update_interval`/`target_*` that the interpolation consumers then read.

**Why it matters**: O(all entities) fat query per frame with mostly no-ops. With a few hundred entities it is small but avoidable; it also forces serialization with anything else touching `Position` (F16).

**Suggested fix**:
- Filter to entities that can be sailing: query `With<BoatState>` instead of `Option<&mut BoatState>` for the rewrite path (only the `BoatState`-insert-on-first-sail branch needs the `Option` form, which can use a separate narrower query keyed on `MoveMode == Sail` via `QueryFilter`-style `With` + a `Changed<MoveMode>` check).
- Gate the per-frame `RemoteBoatState` rewrite on `Changed<Position>` (only rewrite when the server position actually moved).

---

### F14 — Character/NPC colliders are parented to animated root joints → Rapier sync churn every frame
**Files**: `src/systems/character_model_add_collider_system.rs:112-134` (spawn + `joints[0]` parenting), `src/systems/npc_model_add_collider_system.rs:90-111` (same)

Each character collider is a child of `skinned_mesh.joints[0]`. The root joint's transform changes whenever the character moves/animates, and `PhysicsSet::SyncBackend` then re-positions every such collider in the Rapier world every frame (broadphase update + AABB recompute) even though the colliders are used **only** for mouse-click raycasts (`COLLISION_FILTER_INSPECTABLE | CLICKABLE | PHYSICS_TOY` filters — they never participate in movement, ground, or wall queries, which exclude entity groups).

**Why it matters**: The sync cost is bounded by entity count, but it is pure overhead for queries that could instead use the entity's own `Transform`/AABB directly (a click raycast could test the skinned mesh AABB, which the code already computes for the collider at spawn). Also note the self-intersection foot-gun this creates (documented in `pitfalls/flying.md` — the entity's own collider matched ground queries until groups were fixed).

**Suggested fix** (medium-term): Replace per-entity Rapier character colliders with a manual click test: maintain the combined AABB (already computed at `character_model_add_collider_system.rs:104-109`) as a component and do a cheap ray-vs-AABB test in the click systems (`game_mouse_input_system.rs:105`, `character_select_system.rs:488`). Removes hundreds of colliders from the Rapier world entirely, shrinking every scene query's broadphase. If kept, at least stop parenting to the animated joint and snap the collider to the entity root transform instead.

---

### F15 — System ordering serializes 5+ movement systems
**File**: `src/lib.rs:1397-1406` (+ sailing chain :1441-1551)

`facing_direction_system` (writes `Transform.rotation`, `FacingDirection`), `update_position_system` (writes `Position`, `FacingDirection.desired`), `monster_separation_system` (writes `Position`), `collision_height_only_system` (writes `Position`, `Transform`), `collision_player_system` (writes `Transform`, sometimes `Position`), plus `flight_movement_system` (writes `Position`) and `sailing_movement_system` (writes `Position`, `FacingDirection`) all mutate overlapping components, so Bevy runs them strictly serially in a fixed chain on one thread.

**Why it matters**: No parallelism between the most expensive systems; the chain's total latency is the sum of per-frame costs (F1-F6, F7, F8). Rapier's `ReadRapierContext` is read-only, so the collision systems *could* parallelize if their Bevy-visible component accesses didn't conflict.

**Suggested fix** (structural, larger): split each system into a "pure compute into a per-entity scratch component (e.g. `PendingGroundY`) that writes nothing shared" phase + one writeback system. E.g. `collision_height_only_system` could write `GroundHeightCache` (its own component) and a single `apply_ground_height_system` writes `Position`/`Transform` afterwards — this lets the query-heavy work run in parallel across systems and is the cleanest long-term win. Even without that, moving `monster_separation_system`'s output into `Position` only (it already does) and giving `update_position_system` a `Changed<Command>` gate (F11) shortens the chain's per-frame work.

---

### F16 — Minor allocations and log spam on per-frame paths
**Files**:
- `src/systems/boat_spawn_system.rs:173-178` — `boat_toggle_system` builds two `HashSet<Entity>` from message readers **every frame** even when no events were sent (events are rare: board/disembark). Gate on `!board_events.is_empty() && !disembark_events.is_empty()` or build only one set.
- `src/systems/collision_system.rs:141, 150, 296, 303` — `log::warn!` fires **every frame** if `CurrentZone`/zone assets are briefly unavailable (e.g. during zone transitions), producing log spam (log formatting is not free, and it makes real warnings invisible). Use `log::warn_once!` or a `Local<bool>` latch per system.
- `src/systems/zone_content/monsters.rs:506-517` — per-shark-per-frame `get_terrain_height` sample (covered by F7's cache fix).

**Why it matters**: small but pure waste on per-frame paths; trivial to fix.

---

### F17 — Sailing water-surface lookup scans all water volumes per frame
**File**: `src/systems/sailing_movement_system.rs:13-51` (`sample_water_surface_height_cm` iterates `underwater_volumes.volumes` linearly per frame for the active boat)

The ocean zone builds water from many per-block planes (`pitfalls/water-system.md` notes "zones build water from many small per-block IFO planes"), so the `volumes` Vec can be in the hundreds; the boat scans all of them every frame. Volumes are static per zone.

**Why it matters**: per-frame linear scan + float math; small relative to F1-F8 but trivially avoidable and the same volume list is scanned by other systems.

**Suggested fix**: Cache the volume list's bounding structure — at minimum pre-sort by `center.x` and early-out on `dx > half_extents.x` (mirror of the sorted-sweep pattern), or build a coarse grid index at zone load and query only cells near the boat.

---

### F18 — `passive_recovery_system` and `facing_direction_system` — cheap, keep as-is
**Files**: `src/systems/passive_recovery_system.rs:9-19`, `src/systems/facing_direction_system.rs:10-44`

`passive_recovery_system` is a pure timer accumulator (fine). `facing_direction_system` early-outs correctly when the rotation is settled (line 16-18: `diff.abs() < 0.001 → continue`) and only writes `Transform.rotation` while actually rotating — this is the **model pattern** other systems should copy. No change needed.

---

## 4. Priority-Ranked Summary

| # | Finding | Impact | Effort | File:line |
|---|---|---|---|---|
| F1 | Per-frame raycast+intersect for every NPC, ungated | **High** | Low | `collision_system.rs:125-230` |
| F3 | Downward rays traverse terrain trimesh redundantly; 10 km ray for player | **High** | Low | `collision_system.rs:536-559`, `:178-186`; `terrain.rs:288-299` |
| F2 | `find_object_top_height` up to 64 raycasts + Vec + closure, per entity per frame | **High** | Low-Med | `collision_system.rs:55-122` |
| F5 | Unconditional Transform/Position writes mark every NPC dirty each frame | **High** | Low | `collision_system.rs:216-217, 228` |
| F6 | Monster separation O(n²) + Vec alloc/frame | **Med-High** | Low | `monster_separation_system.rs:13-51` |
| F7 | `get_terrain_height` per-frame spam (TLS + 4-octave noise), ~100 calls/frame in ocean zone | Med | Low-Med | `zone_loader.rs:358-398` |
| F8 | NPC boats: up to 5 terrain samples + full dock rescan per boat per frame | Med | Med | `boats.rs:218-227, 391-559` |
| F9 | Wake particle counting O(boats × particles) per frame | Med | Low | `boat_wake_system.rs:114-121, 174-182` |
| F10 | Per-particle material handle swap every frame | Med | Low | `boat_wake_system.rs:264, 283` |
| F11 | `update_position_system` rewrites Position forever at destination | Med | Low | `update_position_system.rs:22-23` |
| F12 | Flight: `NextCommand::with_stop()` + `MoveCollision` every frame | Med | Low | `flight_movement_system.rs:120, 149, 164, 171-180` |
| F14 | Character colliders on animated joints → rapier sync churn; colliders only used for clicks | Med | High (rework) | `character_model_add_collider_system.rs:112-134` |
| F15 | Movement systems serialized by shared Position/Transform writes | Med | High (restructure) | `lib.rs:1397-1406` |
| F4 | `Collider::ball` heap allocs on hot paths | Low | Low | `collision_system.rs:76, 354, 490, 597` |
| F13 | `remote_boat_sync_system` fat query over all entities every frame | Low | Low | `remote_boat_system.rs:19-31` |
| F16 | HashSet allocs/frame in boat toggle; per-frame warn! log spam | Low | Low | `boat_spawn_system.rs:173-178`; `collision_system.rs:141, 296` |
| F17 | Water-volume linear scan per frame | Low | Low | `sailing_movement_system.rs:13-51` |
| F18 | `facing_direction_system` / `passive_recovery_system` | — (already optimal) | — | `facing_direction_system.rs:10-44` |

---

## 5. Quick Wins (ordered by value-per-effort)

1. **Exclude `COLLISION_GROUP_ZONE_TERRAIN` from both downward ground rays** and cut `max_fall_distance` to ~50 m (player) / ~20 m (NPC). Ground already comes from the heightmap (`collision_system.rs:157, 562`). One-line filter change, same pattern already used for the wall cast at `collision_system.rs:505`.
2. **Gate per-NPC ground queries + transform writes on horizontal movement** (F1 + F5): a `GroundHeightCache` component; idle NPCs become a heightmap sample + nothing. This is the single biggest combined win.
3. **X-sorted sweep for monster separation** (F6) — copy `fish_system.rs:715-725`; also reuse a `Local<Vec>` instead of re-collecting.
4. **Throttle `find_object_top_height`** to 4 Hz with a `Local<HashMap<Entity, f32>>` (F2), and reuse one `Collider::ball` per radius via `Local` (F4).
5. **Flight hover: guard `NextCommand::with_stop` inserts** and throttle hover `MoveCollision` to 10 Hz using the `SAIL_REPORT_INTERVAL` pattern (F12).
6. **Wake particles: incremental counters on `WakeEmitter`** (F9) and **skip material assignment when the alpha bucket is unchanged** (F10).
7. **`update_position_system`: latch arrival** so an idle mover stops writing `Position` (F11).
8. **`log::warn! → warn_once`** for the zone-missing guards (F16).
9. **Terrain-height frame cache** keyed on quantized cell (F7) — combine with #2; biggest beneficiary is the ocean-zone NPC-boat + shark path.

---

## 6. Risks / Considerations

1. **Terrain ray exclusion correctness**: the heightmap + noise path must match the trimesh exactly for `target_y` to stay identical. It does today (same grid, same generator, same noise baked into both the mesh and the collider at spawn — `terrain.rs:225-231`), and the code already `max()`es ray hit against `terrain_height`, so the terrain hit is provably redundant. If `TerrainEnhancementSettings.noise_enabled` is ever toggled at runtime, the thread-local noise and the collider diverge (both initialized only at plugin/zone-build time, `noise_overlay.rs:144-176`) — any caching (F7) must be keyed on the settings version, and ideally terrain height should come from one authoritative source (recompute the LUT when noise changes).
2. **Change-detection consumers**: the write-guards in F5/F11 change which frames `Changed<Position>`/`Changed<Transform>` fire. Before landing, audit systems reading `Changed<Position>` (e.g. render extraction, `dirt_dash_system.rs:70` reads `Position`+`Transform`). The guards only suppress *identical* writes, so behavior should be equivalent — but verify with a brief in-game session.
3. **Server-authoritative position**: F1's cache must still apply gravity every frame and write `position.z` back so the server-visible height and falling visuals stay frame-accurate; only the *queries* are gated. The X/Z writes (F5) are pure render mirrors of server data and safe to gate.
4. **Flight throttling**: 10 Hz `MoveCollision` while hovering keeps the server lockstep (per `pitfalls/flying.md`, the teleport-rejection threshold is ~500 cm and per-frame flight motion is far below it). Keep the 60 Hz path while thrusting if momentum/position accuracy matters during high-speed flight — the sailing precedent (10 Hz `SailInput`) proves the server tolerates it.
5. **`find_object_top_height` cadence**: throttling to 4 Hz delays the "unstuck from object" fix by up to ~250 ms after a teleport-under-object event. Acceptable visually; keep the player path at a slightly higher cadence (e.g. 10 Hz) if jank is noticed.
6. **Monster separation sweep**: the sorted-sweep changes push order determinism slightly (ties in X). Monsters are pushed apart with averaged forces; the outcome is visually identical. Keep `distance > 0.001` guard and the cm conversion exactly as today.
7. **Out of scope / left as-is**: `facing_direction_system` and `passive_recovery_system` (already minimal); `vehicle.rs`/`vehicle_model.rs` components (no per-frame vehicle physics found in the analyzed set); `boat_buoyancy_system`/`sail_camera_system` (trivial math per frame, run only for boats).
8. **Structural work (F14/F15)** is worth doing but is a rework, not a tuning pass: they need behavioral tests (click-picking a character without Rapier colliders; parallel-safe ground writeback). Do the F1-F13 tuning first and re-measure; the serialization concern (F15) may largely evaporate once per-frame query cost drops by an order of magnitude.

---

*End of report. No game code was modified; no build was run.*

---

## 7. Verification Update (2026-08-04)

Independent sub-agent scrutiny of every finding (F1–F18) against the actual source, Bevy 0.18.1, parry/rapier, and the previous implementation attempt on `wip/local-changes-2026-08-04` (regressions in `12-validation.md` §3.1–3.3/§5.1). Verdicts per finding:

| Finding | Verdict | Scrutiny result / action |
|---|---|---|
| F1 | CONFIRM (claim) / fix PARTIAL | Claim accurate (2–3 Rapier queries per NPC per frame, ungated). The wip cache works mechanically (1 cm epsilon gate; lazy insert; gravity + z-writeback still run per frame). **But three regressions**: (1) 20/50 m ray caps — pass-through is real on the far-NPC path (`!in_range && cache.is_some()` → resolve never re-runs, even while falling) and via cap-poisoning (spawn >20 m above a platform resolves once with a miss); (2) the new submerged skip drops underwater bridges (wip-introduced, not in the review); (3) §3.1's scan rewrite (F2) can't climb. Correct fix: keep the cache, restore **300 m caps both systems** (terrain excluded ⇒ ray is cheap), make `falling` resolve **regardless of range**, add a time-throttled coarse far-NPC re-resolve (0.5 s when moved >25 cm), drop the submerged skip, and fix F2's scan. |
| F2 | CONFIRM | Claim accurate (heap Vec, 64×100 m upward casts, O(k) predicate, every frame). **12-validation §3.1's BROKEN rating confirmed line-by-line**: the wip's 0.25 m-first/0.1 m-window ascending scan cannot climb stairs or stacked objects (the "spawned under castle steps → place on top" purpose is dead; 100 m exit-point climbing was the load-bearing behavior, `ray_aabb` solid=false returns the exit point). Correct fix: restore the old fixed-origin long-ray algorithm (solid=false, 100 m reach) **plus** a provably-safe convergence break — the iteration is a 2-cycle (each cast excludes only the previous hit), so `hit_height <= top_height → break` caps worst case at 3 casts. Keep the wip's no-alloc `[Entity; 16]` buffer. Do NOT adopt §3.1 Option B (0.45 m first reach still can't climb). |
| F3 | PARTIAL | Redundancy claim + terrain exclusion correct — **keep the exclusion (the big win)**. The 20/50 m caps are a confirmed regression (§3.2): the ray must span *player altitude to object*, and player flight has no altitude ceiling. Correct fix: player `max_fall_distance = 300` **plus origin clamp to `terrain_height + 150`** (the validation's "clamp to camera-50" formulation is flawed — it lands the origin below the feet); NPC restore the pre-fix **100 m**. Revisit the submerged skip separately. Minor: with `noise_enabled=true` the trimesh (per-vertex baked noise) and heightmap (sampled at query point) diverge by up to ~half a step — few-cm clip artifact (§5.1). |
| F4 | PARTIAL | Claim confirmed (4 sites, one `SharedShape` heap alloc each; player 3/frame). The fix as written won't compile: Bevy 0.18.1 has no `Local::new` (a bare `Local<Collider>` is stuck at radius 0.5 vs needed 0.35/0.4/1.0 — silent behavior change), and `Collider` has no `Deref` (`&*collider` doesn't compile). Use `Local<Option<Collider>>` with first-frame init, or simply hoist the balls out of the per-entity loops. **Defer until F1/F2 land** (gating + convergence break eliminate ~95% of these call sites anyway). |
| F5 | CONFIRM | Claim accurate (unconditional X/Z + position.z writes per NPC per frame; propagation + mesh/skin re-extract chains verified in Bevy 0.18.1). The wip guards are correct at all 5 sites: exact `!=` (bit-stable — single writer per field verified), `position.z` guarded, and even the F1 cache's Y write is guarded. Zero `Changed<Position>`/`Changed<Transform>` game consumers exist. Implement as-is (wip). Accepted trade-off: far-falling NPCs snap on camera return. |
| F6 | CONFIRM | Claim accurate (O(n²) + per-frame Vec). The wip sorted sweep is correct: early-out `dx > max_dx` provably sound, crash-fix #2 bounds verified, behavior parity mock-verified (max diff 0 / 5.7e-14; ties can't matter — commutative accumulation). One cosmetic caveat: exact-X tie groups get one-sided pushing (rare, self-resolving). Implement as-is (wip). |
| F7 | PARTIAL | Mechanism confirmed; call counts overstated (~30–60/frame typical; **the biggest caller is missed**: `ui_sailing_hud_system.rs:265` shore search = 80 samples/frame — covered by 06-F7). Perlin is gradient-lattice, not "trig-heavy"; with noise disabled (current state) the cost is **2–6 µs/frame — noise floor**. The doc's zone premise is wrong: zones are **10,240 m × 10,240 m** → a 1 m LUT is **419 MB** (not 16 MB) — infeasible. Cell-cache win ≈ 1 µs/frame. **DEFER**. (Real separate bug found: the noise UI toggle is inert — generators are never re-initialized; re-init must rebuild the collider too.) |
| F8 | CONFIRM | Claim accurate (5 samples + full dock scan + per-frame dock collection; worst case 540–720 rotation tests/frame). The wip is mostly correct (10 Hz throttle, 51 m dock reject provably equivalent to the footprint, Local buffers) **but has one new regression**: the hoisted dock cache refreshes only on handle-id change — if the boat system runs before dock spawns land (no ordering between them), it captures an **empty list for the whole session** (boats sail over docks; pre-fix was immune). Add entity-count invalidation or a `ZoneEvent::Loaded` refresh. Waypoint-skip 6× slower is acceptable (blocking is rare). |
| F9 | CONFIRM | Arithmetic ~2× overstated (~47k filter iterations worst case, only for near/active boats); absolute cost modest (50–100 µs). Incremental counters are sound: the **only** particle despawn sites are two lines in `boat_wake_update_system` (verified across all 73 despawn sites); boat/zone despawns can't drift (generation-tagged entities). Doc inaccuracy: the update system does **not** already have `WakeSource.boat_entity` — both queries need it added. Use `saturating_sub`; batch with F10. |
| F10 | PARTIAL | Code does exactly what the doc says (unconditional handle write every frame), but the *mechanism* claim is misleading: render commands regenerate every frame anyway and mesh re-extraction is driven by the particles' own GlobalTransform changes. The real (modest) churn: `RenderMaterialInstances` inserts + specialization cache checks + Arc clones. The bucket fix works with zero visual delta; correct expectation: wake ≈6× savings, spray only 2–3× (buckets change every 2–3 frames over the short lifetime). Implement-with-changes (shared `alpha_bucket()` helper; don't halve buckets). |
| F11 | PARTIAL | Code claim true; **core premise wrong**: "arriving-then-idle monster never clean" doesn't hold — `command_system` converts completed moves to `Stop` within a frame, and the server never re-issues the same destination. Impact overstated (no `Changed<Position>`/`Changed<FacingDirection>` consumers; no transform propagation triggered). The wip write-guards are correct and safe — **keep them**; **reject** the `Changed<Command>` gate (it fires every frame anyway via `command_system.rs:852`'s identical rewrite — the larger real churn source — and would freeze mid-move movers after `AdjustPosition`). |
| F12 | CONFIRM | Claim accurate (3 insert sites; hover `MoveCollision` at 60 Hz; doc understates — momentum decel double-inserts in one frame). The wip's 10 Hz cadence-gated Stop insert + report is correct and matches the sailing precedent; server tolerates it (hover = zero distance). Client stays stopped between reports via the `MoveEntity` echo re-asserting Stop. Doc sketch bug: `matches!(NextCommand::Stop(_) | NextCommand::default())` is invalid Rust — the wip's cadence gating avoids the `&mut NextCommand` query conflict anyway. Implement as-is (wip). |
| F13 | PARTIAL | Mechanics confirmed; **impact REFUTED**: the wholesale-rewritten `RemoteBoatState` fields (previous/target positions, headings, speed, ages) have **no readers in the codebase** — dead data (only `last_authoritative_at` and `sail_trim` are read elsewhere). And `Changed<Position>` gating is **ineffective**: `boat_buoyancy_system` writes `position.z` every frame (sinusoidal heave) → gate always passes. Correct fix: narrow query to `With<BoatState>` (keep the non-sailing cleanup branch!) + delete the dead fields (keep the `target_position_cm` rebaseline). Fix the doc's "(F16)" cross-ref → F15. |
| F14 | CONFIRM | Claim accurate (colliders parented to `joints[0]`; `SyncBackend` repositions on `Changed<GlobalTransform>` — real per-frame churn for moving *and* idle-animating entities; only click raycasts consume them — verified no movement/ground/wall query can match). Caveats: the spawn AABB is static local-space, not stored/maintained (new code needed for a component AABB); the debug P-picker and physics-toy spawner *can* pick characters — a full removal changes debug tools. **Defer the full rework**; the minimal snap-to-root change (~20 lines) is a safe quick win when the next collision pass lands. |
| F15 | PARTIAL | Serialization confirmed (chain verified; declared-access serialization even for near-no-op systems). The split is heavier than the doc implies (new scratch component + writeback system; compute phase must still order after `monster_separation`; writeback remains serialized; z-writeback must stay frame-accurate). Doc's own risk #8 says do F1–F13 first — **DEFER**; the wip's F11 latch + F5 guards already shorten the chain's work. |
| F16 | PARTIAL | Mechanics real; "allocations" overstated (std HashSet allocates nothing on empty readers — it's minor per-frame work). The doc's monsters.rs note is wrong for main (F7 cache is wip-only). **12-validation's "demotions done" record is REFUTED**: every branch still has `warn=4, debug=0` in collision_system.rs — F16 is fully unaddressed. Correct fix: early-return in `boat_toggle_system` when both readers are empty (`MessageReader::is_empty` — NOT `PopulatedMessageReader`, which would drop one side's events), and `warn_once!` at the 4 sites. Implement-with-changes. |
| F17 | CONFIRM | Claim accurate (linear scan; **understated** — at least 6 systems scan the volume list, three with per-volume `sqrt`+`min_by`). Cost is sub-µs today, but **pre-condition found**: `UnderwaterVolumes` is never cleared — stale volumes from every zone transition accumulate forever (unbounded memory + breaks any sorted-sweep premise). Fix order: (1) clear/filter volumes on zone change (real bug, `WaterSpawnedEvent` already carries `zone_entity`); (2) shared spatial lookup (coarse grid preferred) for all 6 sites. Defer; batch with cheap wins. |
| F18 | CONFIRM | No-action verdict accurate: pure timer accumulator + settled early-out (convergence snaps `actual == desired`, so settled stays settled forever). No hidden cost. Keep as-is. |

**Cross-cutting note for the whole doc set:** the vendored source folder `bevy-collection\bevy-0.18.1` is actually **0.19.0-dev** (its `Cargo.toml` declares `version = "0.19.0-dev"`); the client builds against crates.io Bevy 0.18.1. API citations verified only against that folder must be re-checked against 0.18.1 before implementing.
