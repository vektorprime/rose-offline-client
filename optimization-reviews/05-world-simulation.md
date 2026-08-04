# Optimization Review 05 — World Simulation: Fish, Birds, Boats, Wind, Weather/Seasons, Zone Time / Lighting, World UI (Chat Bubbles + Name Tags), Monster Chatter, Login Flow & Cameras

**Repo**: `rose-offline-client` (single crate, Bevy 0.18.1, bevy_rapier3d 0.33.0)
**Date**: 2026-08-03
**Type**: Research + report only. No `.rs` files were modified, no build was run, no game was launched.

---

## 1. Scope Summary

Architecture docs that existed in `system-architecture/` before this review:

| Doc | Status | Relevance |
|---|---|---|
| `system-architecture/ECS.md` | existed, read | Bevy 0.18 ECS reference: change detection, schedules, query patterns (authoritative for the `is_changed` analysis below) |
| `system-architecture/Camera.md` | existed, read | Free/Orbit camera rigs, visibility layers, underwater effects |
| `system-architecture/README.md` | existed, read | Repo-wide system index |
| `system-architecture/weather-season-system.md` | existed, read | Season plugin, particle entities, hardcoded spawn radius 100 / height 15–25, `max_particles` cap, "no pooling, no LOD, no view culling" (confirmed) |
| `system-architecture/chat-bubble-and-name-tag-architecture.md` | existed, read | WorldUiRect pipeline, egui galley text, `WorldUiBatch` render path (confirmed) |
| `system-architecture/zone_lighting.md`, `sky_stars_architecture.md`, `Lighting.md`, `Render.md`, `UI.md`, `Physics.md`, `Animation.md`, `Transform.md`, `Window.md`, `Input.md`, `Assets.md`, `Audio.md` | existed, not read (out of scope or overlapping with report 04) | — |

Pitfalls read: `pitfalls/index.md`, `pitfalls/water-system.md` (the O(n²) → X-sorted-sweep fix already applied to fish), `pitfalls/zone-loading.md`, `pitfalls/performance-memory.md`, `pitfalls/login-sky-determinism.md` (midday world-time default; `ZoneTime` written every frame).

Files analyzed (all in `src/`): `systems/fish_system.rs`, `systems/bird_system.rs`, `systems/boat_spawn_system.rs`, `systems/boat_wake_system.rs`, `systems/remote_boat_system.rs`, `systems/wind_system.rs`, `systems/wind_effect_system.rs`, `systems/season/weather_system.rs`, `systems/season/season_manager.rs`, `systems/zone_time_system.rs`, `systems/world_time_system.rs`, `systems/directional_light_system.rs`, `systems/chat_bubble_spawn_system.rs`, `systems/chat_bubble_update_system.rs`, `systems/chat_bubble_cleanup_system.rs`, `systems/name_tag_system.rs`, `systems/name_tag_visibility_system.rs`, `systems/name_tag_update_color_system.rs`, `systems/name_tag_update_healthbar_system.rs`, `systems/monster_chatter_system.rs`, `systems/npc_idle_sound_system.rs`, `systems/login_system.rs`, `systems/login_connection_system.rs`, `systems/auto_login_system.rs`, `systems/character_select_system.rs`, `systems/free_camera_system.rs`, `systems/orbit_camera_system.rs`, `animation/camera_animation.rs`, `components/wind_effect.rs`, `components/fish.rs`, `components/bird.rs`, `components/season.rs`, `components/chat_bubble.rs`, `components/name_tag_entity.rs`, `components/night_time_effect.rs`, `resources/world_time.rs`, `resources/zone_time.rs`, `resources/wind_state.rs`, `resources/wind_settings.rs`, `resources/season_settings.rs`, `resources/season_materials.rs`, `resources/monster_chatter_phrases.rs`, `resources/name_tag_settings.rs`, `resources/name_tag_cache.rs`, `resources/login_state.rs`, `resources/character_select_state.rs`, `resources/current_zone.rs`, `resources/login_camera_animation.rs`, `render/world_ui.rs`, `render/zone_lighting.rs`, `render/terrain_material.rs`, `render/starry_sky_material.rs`, `zone_loader/spawning/objects.rs` (WindSway insertion), plus system registration/ordering in `lib.rs` (lines ~920–950, 1074–1150, 1410–1460, 1427–1550).

Key architecture facts confirmed from source:

- **`WindSway` is attached per object *part*** for any grass/leaf/foliage/bush/tree-top mesh (`zone_loader/spawning/objects.rs:273-327`) — not per object. `wind_sway_system` (`components/wind_effect.rs:127-185`) then rewrites `Transform.rotation` for **every** such part **every frame** with 5–6 `sin()` calls + 2–3 quaternion constructions. Vegetation parts in populated Rose zones number in the hundreds to low thousands.
- **Zone time chain**: `world_time_system` (trivial tick accumulator) → `zone_time_system` writes `ZoneTime` **and** `ZoneLighting` (fog/ambient colors) every frame → `update_sun_position_system` (`zone_lighting.rs:336-350`) gates on `zone_time.is_changed()` (always true) → `update_terrain_lighting_system` (`terrain_material.rs:73-81`) and `update_volumetric_fog_system` (`zone_lighting.rs:215-222`) gate on `is_changed::<ZoneLighting>` (always true). The "only write if different" pattern exists and works at `starry_sky_material.rs:430-433` — it is simply not used in the zone-time chain.
- **Weather particles are ordinary `Mesh3d` entities** (unlit `StandardMaterial`, alpha blend), per-frame CPU billboarding (`Quat::from_mat3` + normalize), capped at `max_particles: 2000` with `spawn_rate: 100/s` → `max(10, …)` spawns per frame (`weather_system.rs:55-57`). No pooling, no LOD, no frustum culling (arch doc, confirmed).
- **World UI** is a fully custom render path: every frame the extract system copies all visible `WorldUiRect`s, the queue system sorts them with a matrix multiply each, spawns a **`WorldUiBatch` entity per rect**, recreates the view bind group, and re-uploads the vertex buffer (`world_ui.rs:139-177, 542-552, 584-593, 685-690, 741-743`).
- **Login / camera flow is already event-driven and minimal**: `auto_login_system` (Local state machine), `login_connection_system` (`try_recv`), `character_select_system` (event-driven), `camera_animation.rs` gates on `completed()`, free/orbit cameras are single-entity dolly rigs. Nothing to fix there.
- Boat wake/remote-boat/buoyancy per-frame costs were already reviewed in report 04 (F9/F10/F13/F8/F16/F17); this report only cross-references them.

Cost notation used below: **per-frame per-entity** counts assume a populated zone and default settings; worst-case caps come from settings resources. All estimates are relative ordering, not benchmarked numbers.

---

## 2. Methodology

1. Read the architecture docs and pitfall entries listed above (weather-season-system.md and chat-bubble-and-name-tag-architecture.md in full).
2. Read every file in the file list above end-to-end (line-numbered).
3. Traced spawn paths to count which entities feed each system: `WindSway` insertion sites (`objects.rs:273-327`), bird count formula (`bird.rs:68-71`, spawn loop `bird_system.rs:179`), fish count per water plane (`fish.rs:84-86`, `WaterSpawnedEvent` flow), weather particle cap (`season_settings.rs`, `weather_system.rs:55-57`).
4. Traced the change-detection chain from `zone_time_system` writes through `is_changed()` consumers (`terrain_material.rs`, `zone_lighting.rs`, `starry_sky_material.rs`) to quantify how much per-frame work the write-every-frame pattern keeps alive.
5. Cross-checked cost claims against Bevy 0.18.1 change-detection semantics (a `ResMut`/`&mut Transform` write marks the resource/component changed even when the value is identical — per `system-architecture/ECS.md` and bevy source at `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1`).
6. No builds, no runs, no edits.

---

## 3. Findings

### F1 — `wind_sway_system` rewrites the rotation of every vegetation part in the zone every frame
**Files**: `src/components/wind_effect.rs:127-185`; `WindSway` inserted at `src/zone_loader/spawning/objects.rs:273-327`; registered `Update` at `wind_effect.rs:197`, plugin added `lib.rs:941`

Per entity per frame the loop:
- calls **5–6 `sin()`** (primary `:155`, secondary `:157`, slow `:160`, grass `sway_z` uses the same value, flutter `:178`),
- constructs **2–3 `Quat::from_axis_angle`** (`:167`, `:174`/`:181`),
- writes `transform.rotation` unconditionally (`:175`/`:182`) — every write marks the entity changed → `GlobalTransform` propagation + render-world extraction + (for shadow-casting parts) dirty flags.

There is **no camera-distance gate, no change check, and no frequency reduction**. `WindSway` is added per *part*, so a tree = trunk (no sway) + top (sway); a zone with hundreds of trees/grass/bushes yields **hundreds–thousands** of parts, each paying ~6 trig ops + 2–3 quaternion builds + a dirty Transform every frame, in a single serial system. Compare: fish already implement distance culling (`fish_system.rs:693-695, 742-750`) — vegetation has none.

**Suggested fix (in increasing order of effort, all compatible)**:
1. **Distance cull** — copy the fish pattern: query `&GlobalTransform` (or `GlobalTransform` via `transform_query`) and skip parts farther than a `simulation_distance` (e.g. 120 m, Rose fog/clip range) from the main camera. With hysteresis (skip when `> far`, resume when `< near`) to avoid threshold flicker. Zero visual change for anything the player can see at a meaningful scale.
2. **Write-only-when-different** — the sway angle changes every frame by design, so this alone doesn't help; instead compare *against a per-entity cheap proxy* (e.g. skip when `|Δangle| < ε` at low wind). Marginal alone; combine with 1.
3. **Time-slice** — update only `N` parts per frame (e.g. a `Local<usize>` rotating offset), so each part sways at ~15 Hz instead of 60 Hz. Visible stutter at low FPS; prefer 1.
4. **GPU sway (best long-term)** — move the 3-sine function into the vertex shader of the object material (the parts use `ExtendedMaterial<StandardMaterial, RoseEffectExtension>`; a per-part shader-time offset replaces `phase_offset`). This removes the CPU loop entirely and is the standard approach for thousands of swaying plants. Larger change: new material flag + shader edit + `WindSway`→`WindSwayData` (shader-time phase) migration.

**Impact estimate**: hundreds–thousands of `sin` + quaternion ops and transform dirty-writes per frame; at 2,000 parts ≈ 12k trig ops + 2–3k quaternion constructions per frame in one serial system — comparable to the entire physics budget of report 04's F1–F3. Highest-priority CPU win in this review.

---

### F2 — `update_bird_movement_system` updates every bird every frame, no distance culling
**File**: `src/systems/bird_system.rs:552-627`

Settings: `birds_per_1000_units: 50`, `min_birds_per_zone: 20`, `max_birds_per_zone: 300` (`bird.rs:68-71`) — a standard 2000×2000 zone spawns ~200–300 birds at zone load (`spawn_birds_on_zone_system` has a `Local<bool>` guard against re-spawn, good).

Per bird per frame the loop (`:572-627`) does: target-distance check, `atan2` + `slerp` for facing (`:594-595`), 2 phase accumulators, 2 `sin()` (`:612, :615`), and — most importantly — **3 extra query lookups per bird**: `children_query.get(bird_entity)` (`:618`) + `left_wing_query.get_mut(child)` (`:621`) + `right_wing_query.get_mut(child)` (`:626`), writing the wing rotations. At 300 birds that is ~900 component lookups + ~6 `sin()` + ~300 transform writes (bird body) + 600 wing writes per frame. **No camera-distance check exists.**

**Suggested fix**: query `&GlobalTransform` on the bird query and skip birds beyond ~250 m of the main camera (birds are ambient sky decor; with a 1000 m+ view they are invisible specks past a few hundred meters). Use the hysteresis pattern from F1. Additionally cache the wing `Entity` on the `Bird` component at spawn to eliminate the per-frame `children_query.get` + two `get_mut` lookups — or make wings children of a single `BirdWing` component query that stores both wing entities.

**Impact estimate**: ~200–300 entities × ~5–8 float/trig ops + 3 lookups + 3 transform writes per frame; culling removes ~90% of it for free.

---

### F3 — Weather particles: up to 2,000 separate draw calls + per-frame CPU billboard for every particle
**Files**: `src/systems/season/weather_system.rs:55-57, 100-112`; caps/defaults at `src/resources/season_settings.rs`; arch doc `weather-season-system.md` (no pooling/LOD/culling — confirmed)

`weather_particle_system` runs every frame in `Update`:
- spawn gate: `current_count < settings.max_particles` then spawns `max(10, spawn_rate * dt)` particles/frame around the player (radius 100, 15–25 above player) (`:55-57`);
- update loop: per particle, velocity/wind/wobble math + **billboard matrix `Quat::from_mat3` + `normalize`** + `transform` write, per frame;
- despawn when `age >= lifetime` or `y < 0.5` (`:112`).

Each particle is a distinct `Mesh3d` + `MeshMaterial3d<StandardMaterial>` (unlit, `AlphaMode::Blend`) entity. Defaults: `max_particles: 2000`, `spawn_rate: 100/s` → the pool saturates at 2,000 in ~3 s and stays there. Steady state = **2,000 entities, 2,000 CPU billboard matrix builds, 2,000 transform writes, and 2,000 alpha-blended draw calls in the Transparent3d phase every frame**. The settings UI lets the user raise `max_particles` up to **20,000** (`ui_settings_system.rs` slider `1000..=20000`), making it 10× worse.

**Suggested fix (tiered)**:
1. **Immediate, zero-risk**: lower the default cap (e.g. 500–800) and spawn only when the spawn point is inside the camera frustum; spawn relative to the *camera* forward cone instead of a full 360° around the player (half the particles are behind the camera and never visible).
2. **Cheap GPU batching**: particles share meshes/materials per season — replace per-particle entities with a **single instanced mesh** (or a handful, one per material variant) and store per-instance data (position, size, rotation, color, uv-frame) in a storage buffer updated once per frame. The codebase already has the exact template for this: `src/render/particle_material.rs` / `particle_render_data.rs` (storage-buffer particle pipeline used by rose effect files). This turns 2,000 draws into ~1–4 instanced draws.
3. Keep CPU billboarding only if the pool is small; otherwise move billboard into the vertex shader (same instancing pass).

**Impact estimate**: at 2,000 particles this is the largest *draw-call* and second-largest *CPU* sink in this review (Transparent3d sort + 2,000 draw submission per frame on top of the CPU billboards). Tier 1 alone reduces both proportionally; tier 2 removes the draw-call problem entirely.

---

### F4 — `zone_time_system` writes `ZoneLighting`/`ZoneTime` every frame, permanently defeating every `is_changed()` guard downstream
**Files**: `src/systems/zone_time_system.rs:281-306, 380-395, 498`; consumers gated on change detection: `src/render/terrain_material.rs:73-81`, `src/render/zone_lighting.rs:215-222`; the correct pattern that already exists: `src/render/starry_sky_material.rs:430-433`

`zone_time_system` runs every frame and unconditionally writes `zone_lighting.volumetric_fog_color`, `volumetric_density_factor`, `map_ambient_color`, `character_ambient_color/diffuse_color`, `fog_color`, `fog_density` (`:281-306, :380-395`) and `zone_time.time` (`:498`). In Bevy 0.18, any `ResMut` write marks the resource changed — **even when the written value is identical to the previous one**.

Consequences (all of these would be no-ops if the values stopped being rewritten):
- `update_terrain_lighting_system` (`terrain_material.rs:73-81`) — guarded with `is_changed::<ZoneLighting>` — now re-runs **every frame**, recomputing and rewriting the terrain material's uniform buffer (the guard is correct in intent, permanently defeated by the writer).
- `update_volumetric_fog_system` (`zone_lighting.rs:215-222`) re-writes the `FogVolume` every frame (one entity, cheap, but still dead work).
- `update_sun_position_system` (see F5) and `sync_zone_lighting_to_bevy_lights_system` (see F6) and `update_shadows_for_time_of_day_system` (see F7) all trip on this every frame.

**Suggested fix**: apply the exact "only write if different" pattern already proven at `starry_sky_material.rs:431` to every `zone_time_system` write:

```rust
if zone_lighting.volumetric_fog_color != new_color {
    zone_lighting.volumetric_fog_color = new_color;
}
```

Better still: early-return when nothing moved — `if !world_time.is_changed() { return; }` at the top of `zone_time_system` (the computed values only depend on ticks, which only change when `world_time_system` ticks). That single guard deactivates F5, F6, and F7 below, and the two `is_changed` consumers in `terrain_material.rs` and `zone_lighting.rs` start doing their intended job.

**Impact estimate**: eliminates a chain of per-frame material/uniform/light writes whose cost grows with terrain material complexity and light count — medium CPU + medium render churn per frame, for zero visual change.

---

### F5 — `update_sun_position_system` rewrites the directional-light rotation every frame → shadow pipeline churn
**File**: `src/render/zone_lighting.rs:336-350 (guard), 384-429 (rotation write)`

The guard is `zone_time.is_changed() || sky_settings.is_changed()` — with F4 in place, `zone_time` is changed *every frame*, so the system always proceeds and calls `transform.rotation = Quat::from_euler(...)` (`:423-428`). Writing an identical rotation still marks the light's `Transform` changed, which in Bevy 0.18 re-validates the light's shadow map / cascade state each frame. The sun angle actually moves at ~7.5°/game-hour of real time (`day_cycle: 160` s per 24 h ≈ 15 s/hour) — far below frame-to-frame visible motion, and identical for long stretches at dawn/dusk plateaus.

**Suggested fix**: compare and skip the write, exactly like the `light_direction` comparison already used at `zone_lighting.rs:318`:

```rust
let new_rotation = Quat::from_euler(EulerRot::ZYX, earth_tilt_rad, 0.0, -day_fract * TAU);
if transform.rotation != new_rotation { transform.rotation = new_rotation; }
```

**Impact estimate**: removes per-frame dirty-Transform on the shadow-casting sun light; avoids re-running cascade/shadow-map validation every frame when the sun is effectively static. Low effort, dead-certain win once F4 lands (it also helps if F4 is skipped, because the write is skipped only when identical).

---

### F6 — `sync_zone_lighting_to_bevy_lights_system` writes `GlobalAmbientLight` and the sun color every frame
**File**: `src/render/zone_lighting.rs:262-322`

The system is registered unconditionally in `Update` (`zone_lighting.rs:117`) and writes:
- `ambient_light.color` + `ambient_light.brightness` every frame (`:302-303`),
- `light.color` on the directional light every frame (`:308-313`),
- `zone_lighting.light_direction` guarded by `!=` (`:318-320` — the only good part).

`GlobalAmbientLight` is a global resource; every write marks it changed, forcing per-frame re-evaluation of ambient terms across the whole render (view/light binding refresh for every PBR material). The values only actually change when `zone_lighting` or `graphics_settings` change (i.e. at most a few times per second during time transitions).

**Suggested fix**: gate on `zone_lighting.is_changed() || graphics_settings.is_changed()` (both are change-detected resources), and inside, write `ambient_light`/`light.color` only when the computed values differ.

**Impact estimate**: medium — a global-resource write every frame that Bevy treats as a real change each time; trivial to gate.

---

### F7 — `update_shadows_for_time_of_day_system` writes identical light settings every frame
**File**: `src/render/zone_lighting.rs:499-533`

Runs every frame (after `zone_time_system`, `:119`) and unconditionally writes `light.shadows_enabled` + `light.illuminance` to the sun and `light.shadows_enabled` to the moon (`:524-532`) with values that depend only on `zone_time.state` (four states; changes at most twice per game-day). No `Local` latch exists — every frame marks both lights changed.

**Suggested fix**: hold `Local<Option<(ZoneTimeState, bool)>>` of the last-applied state; write only when it differs (or simply gate on `zone_time.is_changed()`). One-liner, zero behavior change.

---

### F8 — Fish separation pass runs over **all** fish (before camera culling) with 3 heap allocations per frame
**File**: `src/systems/fish_system.rs:697-750`

`update_fish_movement_system` correctly implements distance culling for *movement* (`:693-695`, `simulation_distance: 80.0`, skip at `:745-750`) — but the O(n·k) separation pass **precedes** the cull:
- collects a `Vec<Vec3>` of **every** fish in the zone (`:701-704`),
- builds an index `Vec<usize>` and sorts it by X (`:714-715`),
- allocates a `Vec<Vec3> pushes` (`:717`) and runs the sweep over all pairs (`:718-740`),
- only *then* applies movement + culling per fish (`:742-750`).

With `max_fish_per_water: 150` per plane and many planes per lake zone, the full-zone fish population can be ~1,000+; the sort + sweep + 3 allocations run even when the camera is far from every fish. (The X-sorted sweep with early-out is already the right pattern per `pitfalls/water-system.md` — the issue is doing it for fish that are then culled anyway, and the per-frame allocations.)

**Suggested fix**: collect positions only for fish within the cull radius (do the cull check *before* pushing into the position list), and reuse `Local<Vec>` buffers across frames instead of allocating fresh `Vec`s. Separation only matters visually for fish near the camera; distant schools can re-merge silently (bump the per-frame separation to also run a tiny cheap pass when the camera approaches — the sweep is cheap by design).

**Impact estimate**: medium — an O(n log n) sort + O(n·k) sweep + 3 allocs per frame over the entire zone population when only a fraction is visible.

---

### F9 — World UI queue: view bind group recreated every frame + one `WorldUiBatch` entity spawned per rect per frame
**File**: `src/render/world_ui.rs:542-552, 584-593, 606-738, 741-743`

`queue_world_ui_meshes` (Render, `PrepareBindGroups`) every frame:
- **recreates the view bind group** via `render_device.create_bind_group` (`:542-552`) — the comment explains it must be recreated *when the view-uniform buffer is resized* (shadow-cascade-count changes), but the code recreates it unconditionally every frame;
- sorts all extracted rects with a `view_proj.project_point3` matrix multiply per rect (`:584-593`);
- for each visible rect spawns a **new `WorldUiBatch` entity** in the render world (`:685-690`) and pushes a `Transparent3d` phase item (`:729-737`);
- re-uploads the vertex buffer every frame (`:741-743`).

At ~20 visible characters × 4–6 rects each (name rows, healthbar bg/fg, target marks, chat bubbles) ≈ 100–200 rects, this is ~1 GPU bind-group creation + ~150 entity spawns + one vertex upload per frame in the render app. Entity churn in the render world is cheap-ish (they're despawned by the ECS between frames) but completely avoidable; the bind-group creation is a real GPU-side cost per frame.

**Suggested fix**:
1. Cache `world_ui_meta.view_bind_group` keyed on the **identity of the uniform buffer** — e.g. store the `BufferId`/generation of `view_uniforms.uniforms.buffer()` (or a `WeakHandle` to it) alongside the bind group, and only recreate when the buffer changes (which is exactly the resize case the comment worries about):
```rust
let buffer_id = view_uniforms.uniforms.buffer().map(|b| b.id());
if world_ui_meta.view_bind_group_buffer_id != buffer_id {
    world_ui_meta.view_bind_group = Some(render_device.create_bind_group(...));
    world_ui_meta.view_bind_group_buffer_id = buffer_id;
}
```
2. Leave the per-rect `WorldUiBatch` entity + vertex-buffer design as-is (it is simple and correct); if profiling shows churn, pool the batch entities.

**Impact estimate**: low-medium — removes 1 GPU object creation + revalidation per frame; the per-rect sort/matrix math is fine at current scale.

---

### F10 — `name_tag_update_healthbar_system` rewrites every healthbar rect every frame (no `Changed<HealthPoints>` filter)
**File**: `src/systems/name_tag_update_healthbar_system.rs:7-27`

The system iterates **all** `NameTagHealthbarForeground` + `WorldUiRect` pairs each frame and recomputes width/uv/color from `HealthPoints` even when HP hasn't changed (idle characters regenerate nothing; monsters at full HP sit unchanged). The sibling system `name_tag_update_color_system` already uses the correct pattern — `Changed<Level>` / `Changed<Team>` filters on the player query — so the inconsistency is established.

**Suggested fix**: apply `Changed<HealthPoints>` to the character query (or store `last_hp_percent` on `NameTagHealthbarForeground` and skip when unchanged). Trivial; matches an existing in-repo pattern.

**Impact estimate**: low-medium — per-frame writes to ~2 rects × N entities that are re-extracted by the world-UI path anyway (F9); the writes are the only part that can be removed cleanly.

---

### F11 — Chat bubble update/cleanup do small per-frame work on all bubbles, including entity lookups
**Files**: `src/systems/chat_bubble_update_system.rs:9-60`, `src/systems/chat_bubble_cleanup_system.rs:7-36`

- `chat_bubble_update_system` iterates all `ChatBubble` entities every frame and rewrites `WorldUiRect.color`/offsets for every child rect even when the bubble is not in its fade window (fade is only the last 20% of life, `fade_start_fraction: 0.2` per arch doc).
- `chat_bubble_orphan_cleanup_system` (`cleanup_system.rs:25-36`) iterates all bubbles every frame and performs a `query_targets.get(bubble.target_entity)` lookup per bubble — O(bubbles) entity lookups per frame even when nothing changed.

Both are cheap at current scale (bubbles are transient, counts are single digits to tens), but they are pure per-frame waste and trivially gated.

**Suggested fix**: in `chat_bubble_update_system`, only touch `WorldUiRect.color` while `remaining_time/total_time < fade_start_fraction` (positions still need per-frame updates to follow the target); in the orphan system, early-return when no bubble entities exist (`if query.is_empty() { return; }`) and consider a `Changed<RemovedComponents<...>>` trigger instead of a per-frame sweep.

**Impact estimate**: low — good hygiene, negligible at current populations; matters only if bubble counts grow (e.g. mass chatter).

---

### F12 — `monster_chatter_system`: per-frame iteration over all NPCs; allocates a phrase Vec on each trigger
**Files**: `src/systems/monster_chatter_system.rs:12-73`, `src/resources/monster_chatter_phrases.rs:359-380`

The system decrements `time_until_next_chat` for every `MonsterChatter` entity each frame (`:33`) — cheap per entity, but runs across every NPC/monster in the zone every frame. On trigger it calls `get_random_phrase` → `get_all_phrases()` which **builds a fresh `Vec<&String>`** by extending 6 category vecs (`monster_chatter_phrases.rs:359-368`), then picks via `(rand::random::<f32>() * len) as usize` (`:66` — float-index truncation, slightly biased but irrelevant).

**Suggested fix**: pre-flatten the phrase lists once at resource construction into `Vec<String>` fields; on trigger use `rng.gen_range(0..len)` (or a deterministic hash of `(entity, chat_index)` to avoid `rand` entirely). Also add a `Local<f32>` time-gate or run the timer decrement only on a coarse cadence (e.g. 4 Hz) — a chat timer has no need for 60 Hz resolution.

**Impact estimate**: low — small allocs only on trigger (rare), but the per-frame full-zone iteration is avoidable at trivial cost.

---

### F13 — `npc_idle_sound_system` — already optimal, no change needed
**File**: `src/systems/npc_idle_sound_system.rs:25-95`

Despite iterating all NPCs per frame, this system is already written the right way: `rand::thread_rng()` is created **once per system run** (`:42`, not per NPC), per-NPC early-outs on `command.is_stop()` (`:60`) and animation-loop-count comparisons (`:66-73`), and the ~20% sound roll only runs once per animation loop. Keep as-is (mirrors report 04's F18 "already optimal" conclusion).

---

### F14 — `wind_update_system` / `sync_vegetation_wind_system` rewrite their resources every frame
**File**: `src/systems/wind_system.rs:6-38 (wind_update_system), 40-45 (sync_vegetation_wind_system)`

`wind_update_system` writes `WindState` fields every frame (`:17-20` / `:34-37`), and `sync_vegetation_wind_system` writes `WindSwaySettings.global_intensity` every frame (`:44`). The writes are `ResMut` writes → resources marked changed every frame. Today this is harmless (the only consumer, `wind_sway_system`, reads the settings unconditionally), but it blocks any future change-detection-based gating of the sway system (F1 tier 3) and churns `WindState` extraction.

**Suggested fix**: write only when the value changed beyond epsilon (`if (wind.speed - prev).abs() > ε …`), or leave as-is — flagging only as a precondition for F1's tier-3 gating.

**Impact estimate**: low in isolation; prerequisite hygiene for F1.

---

### F15 — Zone-time visibility, starry-sky night factor, camera & login flow — already optimal (no change)
**Files**: `src/systems/zone_time_system.rs:375-378` (night-effect hiding only on state transition), `src/render/starry_sky_material.rs:430-433` (only-write-if-different), `src/animation/camera_animation.rs:37` (`completed()` gate), `src/systems/auto_login_system.rs`, `src/systems/login_connection_system.rs:17`, `src/systems/character_select_system.rs` (event-driven), `src/systems/free_camera_system.rs:49`, `src/systems/orbit_camera_system.rs`

These are the model patterns and already-correct pieces observed during this review:
- `zone_time_system` only iterates/hides `NightTimeEffect` entities on state transitions (not per frame).
- `update_starry_sky_night_factor` writes the material setting only when the value changed — the pattern F4/F5/F6/F7 should copy.
- `camera_animation.rs` skips advancing when the ZMO is `completed()`.
- Login/server-select/character-select flows are event-driven (`try_recv`, message readers, state machines); `orbit_camera_system`'s single per-frame Rapier raycast is negligible; `free_camera_system` is a plain dolly update.

---

## 4. Priority-Ranked Summary

| # | Finding | Impact | Effort | File:line |
|---|---|---|---|---|
| F1 | Wind sway rewrites rotation of every vegetation part every frame (5–6 sin + 2–3 quats each), no culling | **High** | Low–Med (cull) / High (GPU) | `wind_effect.rs:127-185`; `objects.rs:273-327` |
| F4 | `zone_time_system` writes `ZoneLighting`/`ZoneTime` every frame → `is_changed()` guards always true → terrain material uniform + fog rewritten every frame | **High** | Low | `zone_time_system.rs:281-306, 380-395, 498`; `terrain_material.rs:73-81` |
| F5 | Sun rotation rewritten every frame (guard always true) → directional-light/shadow churn | **High** | Low | `zone_lighting.rs:336-350, 384-429` |
| F3 | Weather: 2,000 mesh-entity particles, per-frame CPU billboards, 2,000 alpha draw calls; UI allows 20,000 | **High** | Low (cap/cull) / Med-High (instancing) | `weather_system.rs:55-57, 100-112`; `season_settings.rs` |
| F6 | `GlobalAmbientLight` + sun color rewritten every frame | Med-High | Low | `zone_lighting.rs:262-322` |
| F2 | All birds updated every frame (300 max), 3 query lookups + 2 sin + 3 writes each, no culling | Med-High | Low | `bird_system.rs:552-627` |
| F7 | Shadow on/off + illuminance rewritten every frame with identical values | Med | Low | `zone_lighting.rs:499-533` |
| F8 | Fish separation pass + 3 Vec allocs run over the whole zone before culling | Med | Low | `fish_system.rs:697-750` |
| F9 | World UI: view bind group recreated + `WorldUiBatch` entity per rect every frame | Med | Low | `world_ui.rs:542-552, 685-690` |
| F10 | Healthbar widths/uvs rewritten for all entities every frame, no `Changed<HealthPoints>` | Med-Low | Low | `name_tag_update_healthbar_system.rs:7-27` |
| F11 | Chat bubble per-frame alpha writes + per-bubble entity lookups in orphan sweep | Low | Low | `chat_bubble_update_system.rs:9-60`; `chat_bubble_cleanup_system.rs:25-36` |
| F12 | Chatter timer iterates all NPCs per frame; Vec alloc on each trigger | Low | Low | `monster_chatter_system.rs:12-73`; `monster_chatter_phrases.rs:359-380` |
| F14 | Wind resources rewritten every frame (blocks future gating) | Low | Low | `wind_system.rs:6-45` |
| F13/F15 | `npc_idle_sound_system`, cameras, login flow, starry-sky/zone-time patterns | — (already optimal) | — | see F13/F15 |

---

## 5. Quick Wins (ordered by value-per-effort)

1. **`zone_time_system`: early-return on `!world_time.is_changed()` + write-only-if-different on `ZoneLighting`** (F4). This one change deactivates F5, F6, and F7 (sun rotation, ambient light, shadow toggles) because their always-true change-detection gates stop firing. Copy the existing `starry_sky_material.rs:431` pattern; zero visual difference (values are identical when suppressed).
2. **Distance-cull `wind_sway_system`** (F1 tier 1): reuse the fish culling pattern (`fish_system.rs:693-695`) with hysteresis (~120 m). Biggest single CPU win in this review; no visual change at gameplay distances.
3. **Distance-cull birds** (F2) with the same hysteresis pattern (~250 m), and store wing entities on `Bird` to drop the 3 per-bird query lookups.
4. **Weather: default `max_particles` → ~600, frustum/cone-relative spawning** (F3 tier 1). Settings-only change; leaves the architecture intact until the instancing pass (tier 2) is scheduled.
5. **Sun rotation + shadow latch + ambient-light gates** (F5/F7/F6): three one-line guards that only write on change — mostly redundant once #1 lands, but correct independently.
6. **`Changed<HealthPoints>` on `name_tag_update_healthbar_system`** (F10) — mirrors the existing `name_tag_update_color_system` pattern.
7. **Cache the World-UI view bind group keyed on the uniform-buffer id** (F9) — the resize concern the comment mentions is exactly what the buffer-id key handles.
8. **Fish separation: cull before the sweep + `Local<Vec>` buffers** (F8).
9. **Pre-flatten chatter phrases + coarse timer cadence** (F12); chat-bubble fade-only writes + empty-sweep early-out (F11).

---

## 6. Risks / Considerations

1. **Change-detection consumers**: F4–F7 suppress only *identical* writes, so `Changed<ZoneLighting>` / `Changed<Transform>` / `Changed<GlobalAmbientLight>` still fire on real changes. Before landing, audit systems reading `is_changed::<ZoneLighting>` or `Changed<ZoneTime>` (grep found `terrain_material.rs:73`, `zone_lighting.rs:220`; `update_sun_position_system`'s `zone_time.is_changed()` is the one being silenced). Behavior is equivalent because only equal-value writes are suppressed — but verify with an in-game day/night cycle pass.
2. **Distance-culling hysteresis**: naive `distance > R → skip` causes visible popping when the camera crosses the threshold. Use different enter/exit radii (e.g. skip beyond 140 m, resume inside 100 m). Fish already accept one-frame-stale positions for this (`fish_system.rs:743-744`); the same tradeoff applies to birds/vegetation and is invisible for sway.
3. **Bird culling radii**: birds are ambient; a 250 m cull keeps them across the playable vista at the game's camera distances. Confirm against the camera far plane (report 04 noted ~1 km views) — beyond ~300 m birds are sub-pixel anyway.
4. **Weather cap changes**: lowering `max_particles` changes density of rain/snow/leaves. It's a settings default (`season_settings.rs`) and UI-exposed, so players can tune it; the instancing path (F3 tier 2) must preserve per-particle alpha ordering or be drawn in a dedicated additive/blend pass to avoid the classic transparent-sorting artifacts of instanced blends.
5. **World-UI bind-group cache**: keying on `BufferId` is safe (a new buffer = new id) and handles the exact resize case documented in the code comment (`world_ui.rs:538-541`). Do not key on content.
6. **GPU sway (F1 tier 4)**: object parts use `ExtendedMaterial<StandardMaterial, RoseEffectExtension>` — adding vertex sway means touching that material's shader and re-baking per-part `phase_offset`. Do the CPU culling first (tier 1); re-evaluate tier 4 only if sway shows up in profiling as the remaining hotspot.
7. **Zone-time early-return**: `world_time_system` ticks every frame (accumulator), so the guard must check `world_time.is_changed()` on the *tick resource* (`ZoneTime`), not the accumulator — verify the resource's `Changed` flag semantics: if `world_time_system` itself writes the tick value only when it advances, gating on that resource's change is correct.
8. **Out of scope / left as-is**: boat wake particles, remote-boat sync, wake counting (report 04 F9/F10/F13); `npc_idle_sound_system` (F13 — already optimal); cameras/login flow (F15 — already minimal); `starry_sky` per-frame `time` write (required for twinkle); `name_tag_system`'s egui galley + texture cache (cache is string-keyed and correct; per-frame rect positioning is inherent to billboarding).
9. **Structural reworks**: F1 tier 4 (GPU sway) and F3 tier 2 (instanced particles) are render-pipeline changes, not tuning; schedule them separately with visual validation. Everything else in this report is write-suppression or culling and should land as a batch with a single in-game verification pass (day/night cycle, seasons, sailing, combat with many NPCs).

---

*End of report. No game code was modified; no build was run.*
