# Validation Report 12 — Cross-Check of 11 Optimization Fixes + 3 Crash Fixes

**Repo:** `rose-offline-client` (Bevy 0.18.1, bevy_rapier3d 0.33, bevy_egui 0.39.1)
**Date:** 2026-08-03
**Role:** review/validation only — no `.rs` files modified, no build run, no game run.
**Method:** full `git diff HEAD` read file-by-file (104 files, +3187/−1542), cross-checked against the fix lists in `optimization-reviews/01…11`, Bevy 0.18.1 sources at `bevy-collection\bevy-0.18.1\crates\`, and `pitfalls/` entries (`terrain-physics.md`, `combat-sync.md`, `performance-memory.md`, `water-system.md`, `flying.md`, `zone-loading.md`).

---

## 1. Change-Set Summary

| Scope | Files | What changed |
|---|---|---|
| Animation | `animation/*`, `model_loader.rs`, `components/blink_clip.rs` | ViewVisibility+distance culling on 3 animation systems, epsilon write guards, precomputed ZMO flags, per-frame material-mutation early-out, NPC skeleton cache, dead-code removal |
| Physics/collision | `systems/collision_system.rs`, `monster_separation_system.rs`, `components/collision.rs` | Ground-height cache + distance gating for NPCs, terrain-group exclusion from ground rays, 10 km→50 m player ray, rewritten object-top scan, write guards, O(n log n) monster separation |
| Zone loading | `zone_loader.rs`, `loading.rs`, `systems.rs`, `spawning.rs`, `spawning/terrain.rs`, `spawning/objects.rs`, `vfs_asset_io.rs` | Byte cache with zone tags + LRU-ish eviction, parsed-zone eviction on zone change, cache-first reads, shared terrain material, log demotion |
| Rendering | `render/world_ui.rs`, `underwater_effect.rs`, `terrain_material.rs`, `zone_lighting.rs`, `systems/zone_time_system.rs`, `graphics/apply_systems.rs`, `graphics_settings.rs`, `lib.rs` | Bind-group cache, underwater-pass early-out, lighting write guards, DoF/SSAO/fog insert-remove, new motion-blur/FXAA/SMAA apply systems, water-settings diff guard |
| Gameplay | `pending_damage_system.rs` (+new `resources/projectile_index.rs`), `projectile_system.rs`, `spawn_projectile_system.rs`, `blood_overlay_system.rs`, `gash_wound_system.rs`, `dirt_dash_system.rs`, `update_position_system.rs`, `sail_animation_system.rs`, `spawn_effect_system.rs`, `move_speed_set_system.rs`, `flight_movement_system.rs`, `fish_system.rs`, `bird_system.rs`, `wind_effect.rs`, `weather_system.rs` | Projectile index, in-place transforms, Changed/epsilon guards, log demotion, 15 Hz sail updates, 10 Hz hover reports, distance culls, weather cap |
| Audio | `audio/mod.rs`, `spatial_sound.rs`, `streaming_sound.rs`, `monster_sound_cap.rs`, `background_music_system.rs`, `animation_sound_system.rs` | 100 m cull, monster-sound concurrency cap, true crossfade, streaming refill budget, footstep distance gate |
| UI | `drag_and_drop_slot.rs` (+8 call sites), `ui_minimap_system.rs`, `ui_admin_menu_system.rs`, `ui_chatbox_system.rs`, `ui_settings_system.rs`, `editbox.rs`, `name_tag_system.rs`, `name_tag_update_healthbar_system.rs`, `ui_resources.rs`, misc | `SlotAccept` enum (replaces Box closures), icon caches, filter-key cache, deleted dead Style clone, window-open gate |
| Scripting | `lua4/value.rs`, `function.rs`, `vm.rs`, `conversation_dialog_system.rs` | Arc<str> strings, call-depth guard, single-stack VM, con-script cache |
| Network | `protocol/mod.rs`, `game_connection_system.rs`, `login/world_connection_system.rs` | Malformed-packet skip (cap 8), reconnect entity despawn, per-frame message caps |
| Misc | `memory_diagnostics.rs`, `logging/mod.rs`, `map_editor/*`, `systems/directional_light_system.rs` (deleted), `resources/name_tag_cache.rs` (deleted), `components/mod.rs`, `systems/mod.rs`, `Cargo.toml` | 30 s run condition, state-gated editor systems, dead-code deletion |

**3 crash fixes (uncommitted, all verified):**
1. `apply_post_processing_settings` / `apply_depth_of_field_settings` in `src/lib.rs` now filter `(With<Camera>, Without<WaterReflectionCamera>)` — **OK**. The reflection camera (LDR target) can no longer receive post-process components. Verified the main camera is the only post-process target.
2. `monster_separation_system.rs` — bounds check placed **after** the increment in both scan directions (`k += 1; if k >= n { break; }` and `if k == 0 { break; } k -= 1;`) — **OK** (see §3.1 for the full algorithm audit).
3. `world_ui.rs` — pipeline target format `Bgra8UnormSrgb` → `TextureFormat::bevy_default()`. Verified in Bevy 0.18.1 source: non-HDR main texture format **is** `TextureFormat::bevy_default()` (= `Rgba8UnormSrgb`, `bevy_image/src/image.rs:35-38`, `bevy_render/src/view/mod.rs:1081-1085`) — **OK**.

---

## 2. Per-Module Validation Table

| Module | Claimed fix (from reviews) | Actual diff | Verdict |
|---|---|---|---|
| 01 Animation F1 — visibility culling | `should_animate_entity` in all 3 animation systems | Implemented; culls when `!ViewVisibility && >200 m` | **REGRESSION RISK** — see §3.4 (frozen poses, delayed hit frames) |
| 01 F2 — bone write guards | epsilon-guard bone writes | `abs_diff_eq` 1e-5/1e-4 | OK |
| 01 F3 — `update_active_motion` guard | speed write guard + skip default-handle insert | Implemented; `is_strong()` guard added | OK (verified `Handle::is_strong` exists; motion handles are strong) |
| 01 F4 — material mutation early-out | early-out + precomputed flags | Implemented; `zmo_asset.flags` precomputed in both loaders, matching old per-frame computation exactly | OK |
| 01 F5 — sail 15 Hz | quantized CPU updates | Implemented (`SAIL_MESH_UPDATE_INTERVAL = 1/15`) | OK |
| 01 F6 — blink Changed filter | `Changed<BlinkClip>` | Implemented; dead twin deleted | OK |
| 01 F7 — IBP cache | cache inverse-bind-pose handles | **NOT implemented** | Partial (not a regression) |
| 01 F8 — material cache | shared character materials | **NOT implemented** | Partial (not a regression) |
| 01 F9 — NPC skeleton cache | cache by skeleton_index | Implemented but keyed by **npc_id** (per-NPC-id, correct but less effective); `spawn_npc_model` now `&mut self` | OK (suboptimal key) |
| 01 F10/F12/F15/F17 — misc | — | F15 `advance` change-guards implemented; debug scaffolding removed | OK |
| 02 F1/F2 — storage-buffer reuse | reuse buffers, grow-only recreate | Implemented in both systems; `buffer_description.size` is the correct capacity field (`bevy_render/src/storage.rs:34`); shaders index by `vertex_index` so capacity > data is safe; old handles `remove`d only on grow (no leak) | OK (partial win — material still `get_mut` every frame ⇒ bind-group re-prepare per frame remains) |
| 02 F3/F4 — terrain/water diff guards | value-diff before material mutation | Implemented | OK |
| 02 F6 — underwater early-out | skip when `!is_underwater` | Implemented before `post_process_write` | OK |
| 02 F9 — FX apply systems | wire motion blur/FXAA/SMAA | Implemented; `MotionBlurNode` verified to early-out on `shutter_angle <= 0.0` (`bevy_post_process/src/motion_blur/mod.rs:193`); defaults changed (`motion_blur_enabled: true`, `smaa_quality: High`) to match the hardcoded spawn | OK; SMAA/MotionBlur still always-on at boot until settings change (incomplete F9), SSR/CAS still uncontrolled |
| 02 F10 — remove disabled FX | remove components instead of min quality | Implemented (DoF/SSAO/fog) | OK |
| 02 F11 — spawn FX from settings | frame-0 parity | Implemented (DoF/SSAO/fog gated at spawn) | OK |
| 03 CM-01 — projectile index | per-attacker projectile index | Implemented + world-verification of each entry; despawn paths in `projectile_system` remove from index | OK — but index entries are never pruned on zone-change despawns (slow leak) and one despawn path (`effect_system`?) unverified — see §3.6 |
| 03 PR-01 — in-place transform | mutate `&mut Transform` | Implemented | OK |
| 03 DD-01 — digit buffers | see 02 F2 | Implemented | OK |
| 03 BL-02 — overlay clean frames | avoid subtree walk on clean frames | Implemented via cached `material_part_entities` — **REGRESSION** — see §3.7 |
| 03 BL-03 — Changed<HealthPoints> | Implemented | OK (wound visibility recomputed on HP change) |
| 03 BL-04 — cap check before UV projection | Implemented | OK |
| 03 EF-01/GC-04/PC-04 — log demotion | Implemented across ~10 sites | OK |
| 04 F1+F5 — ground cache + write guards (NPCs) | movement/distance-gated queries | Implemented with `GroundHeightCache` | **PARTIAL/REGRESSION RISK** — see §3.2/§3.3 (falling/cache edge cases, far-NPC ground staleness) |
| 04 F2 — object-top scan rewrite | 64×100 m rays → ascending 0.1 m scan | Implemented | **BROKEN** — see §3.1 (primary suspected collision regression) |
| 04 F3 — terrain exclusion + ray caps | exclude ZONE_TERRAIN, cap rays | Implemented (20 m NPC / 50 m player) | **REGRESSION RISK** — see §3.1 (50 m cap pass-through) |
| 04 F6 — monster separation sweep | sorted-sweep | Implemented; audited fully — correct | OK (crash fix #2 verified) |
| 04 F8 — boats nav throttle | 10 Hz blocked-path nav, dock radius reject, Local buffers | Implemented; dock rejection radius (51 m) correct vs deck footprint (~50.4 m); **`blocked_frames` now counts 10 Hz ticks ⇒ waypoint-skip ~6× slower** | OK (minor behavior change) |
| 04 F12 — flight hover throttle | 10 Hz MoveCollision + guarded Stop insert | Implemented | OK |
| 04 F16 — warn-once / HashSet gates | — | warn→debug demotions done; boat-toggle HashSet gate **not** found in diff | Partial |
| 05 F1 — wind culling | 120 m cull | Implemented (no hysteresis) | OK |
| 05 F2 — bird culling | 150 m cull | Implemented | OK |
| 05 F3 — weather cap | cap particles | Hard cap 1000 (UI allows 20 000) — **silently ignores settings > 1000**; billboard culling skips >150 m / behind camera | OK/Partial — see §3.8 |
| 05 F4 — zone_time write guards | epsilon writes | Implemented; state writes moved inside `state != X` guards | OK (1e-3 quantization of `state_percent_complete` is imperceptible) |
| 05 F5/F6/F7 — sun/ambient/shadow guards | value-diff + latch | Implemented; shadow latch keyed on `(state, settings)` is correct (body depends only on those) | OK |
| 05 F9 — world-UI bind group cache | cache keyed on buffer id | Implemented; `RawBufferVec::buffer().id()` correct (`buffer_vec.rs:66`); content updates in place, so cache is sound | OK |
| 05 F10 — healthbar write guard | `last_health_percent` | Implemented; single construction site sets `-1.0` | OK |
| 06 F2 — DnD Box-closure → enum | port all 8 sites | **All 8 call sites verified byte-for-byte against the old closures** (inventory, hotbar, bank, npc store ×3, personal store, skill tree/list, quest, player info) | OK |
| 06 F3/F5/F8/F12/F18/F21 — chatbox/admin/editbox/minimap/name-tag | Implemented | OK |
| 06 F19 — always-on systems | state gates | `ui_settings_system` gated on `settings_window_open` + Game; map-editor plugin fully state-gated | OK (verified `ui_settings_system` only draws the window) |
| 07 F1/F4 — parsed-zone + byte-cache eviction | evict on zone change | Implemented: `zone_loader_assets.remove` for all cached zones, `evict_zone_tagged_files` keep {incoming,current} | **PARTIAL/REGRESSION RISK** — see §3.9 (revisit re-parse, cache-first staleness) |
| 07 F3 — zone reads through byte cache | Implemented (cache-first) | **REGRESSION RISK** — real-FS priority inverted, stale-cache shadowing (map editor) — §3.9 |
| 07 F7 — shared terrain material | one TerrainMaterial per zone | Implemented | OK |
| 07 F8 — mesh_cache reuse | Implemented (clear+resize, no stale handles) | OK |
| 07 F13/F14/F17/F18 — key normalize, merge probe+read, log demote | Implemented | OK |
| 07 F20 — delete dead NameTagCache | Deleted file + module | OK |
| 08 F2 — distance cull | `AUDIBLE_CUTOFF = 100` in queue + cull in `spatial_sound_system` | Implemented | **REGRESSION RISK** — one-shot hard-despawn mid-play (click), loop restart phase jump — §3.10 |
| 08 F3 — monster sound concurrency cap | cap = 64 active | Implemented (`MonsterSound` marker + count) — `.drain(..).take(3)` still discards the rest (matches old `.clear()`) | OK |
| 08 F4 — smaller ring | 500 → 150 | Implemented (consistent with 100 m cull) | OK |
| 08 F7 — real crossfade | overlapping tracks + gain ramp | Implemented; state machine audited (no re-trigger during FadingOut) | OK |
| 08 F9 — streaming refill | packet borrow + budget | Implemented | **REGRESSION RISK** — refill budget ≈ 1 frame of audio at 60 fps; underrun/stutter risk — §3.11 |
| 09 F2 — message caps | 4096/64/64 | Implemented | OK |
| 09 F6 — malformed-packet skip | skip + resync, cap 8 | Implemented; only `DecryptBodyFailed` is skipped, other errors still fatal | OK (conservative) |
| 09 F9 — reconnect despawn | despawn before `clear()` | Implemented via deferred commands | OK/MEDIUM — entity-id-reuse panic risk — §3.12 |
| 10 F1/F2/F3 — Lua Arc strings + call guard | Implemented; upvalue `Arc` shared but never mutated (audited) | OK; 512 depth cap converts stack overflow into a VM error (intended) |
| 10 F6 — con-script cache | Implemented per VfsPath | OK |
| 11 XC-02 — remove first ApplyDeferred | Removed only the standalone PostUpdate `ApplyDeferred`, kept the `DebugRenderPreFlush` one (exactly the recommended minimal change) | OK |
| 11 XC-04 — delete directional_light_system | Deleted | OK |
| 11 XC-05 — diagnostics run condition | Implemented | OK |
| 11 XC-08/11 — zone-list borrow, grid hoist | Implemented | OK |
| 11 XC-14 — log level default | Default was **already** `"info"` in HEAD; only the comment changed | OK (stale review) |

---

## 3. Detailed Findings

### 3.1 [HIGH] Collision: `find_object_top_height` ascending scan cannot climb stacked objects (suspected "can't climb" bug — CONFIRMED)

**File:** `src/systems/collision_system.rs:111-177` (new scan), constants at `:45-54`.

**Problem:** The original algorithm cast a **100 m upward ray** from `feet + 0.1` up to 64 times, each iteration excluding the previous hit collider, and took the max hit height. That climbed entire staircases/stacked objects (each iteration found the next surface above). The new algorithm:
- first window: `probe_y = feet+0.1`, reach `0.25 m` (ends at feet+0.35);
- subsequent windows: 0.1 m tall, each excluding the previous collider;
- **the loop breaks at the first empty window** (usually after 1–2 hits).

Consequences:
1. **Under a staircase / inside a building** the sphere intersects the lowest step face; the scan hits that face and then **stops** (the excluded collider is the whole stairs object) — the entity is placed at the *bottom face*, not the top. The old code kept climbing (64 iterations × 100 m) and placed the entity on top. The "spawned under castle steps → place on top" behavior (review 04 F2's whole purpose) is effectively dead.
2. A surface the sphere touches between 0.35 m and 0.45 m above the feet (sphere center at feet+0.1, radius 0.35 ⇒ max touch height feet+0.45) is **missed** by the first ray (0.35 reach) ⇒ `top_height = None`.
3. A wall/step top more than 0.35 m above the feet (player standing against a low step face, feet at mid-step height) is never reached — the old 100 m ray placed the player on top.

**Impact:** Exactly the reported symptom — "can't climb others". Also breaks the under-stairs unstuck behavior.

**Suggested fix:** Restore a fixed-origin long ray as the first pass (e.g., one `cast_ray` of 10–20 m from `feet+0.1` with the predicate list, excluding hits iteratively, max 64), or increase `first_reach` to `GATE_BALL_RADIUS_M + STEP` and keep climbing windows until an empty window at the SAME height as the previous hit (climb through any number of stacked colliders, not just 10). Simplest correct version:
```rust
let mut probe_y = feet.y + 0.1;
let mut reach = 10.0;              // long first reach: climb the whole object stack
loop {
    // exclude previous collider; cast from (x, probe_y, z) up by `reach`
    // hit → top = max(top, probe_y + dist); probe_y = hit + 0.05; reach = 0.1
    // miss → break
}
```

### 3.2 [HIGH] Collision: player/NPC ground rays capped at 50 m / 20 m — falls from height pass through platforms

**Files:** `src/systems/collision_system.rs:45-48` (`NPC_GROUND_RAY_DISTANCE_M = 20.0`, `PLAYER_GROUND_RAY_DISTANCE_M = 50.0`), player ray `:715-729`, NPC ray `:286-304`.

**Problem:** The old player ray had `max_fall_distance = 10000.0`. The new 50 m cap: if the player is more than ~50 m above a platform/bridge (falling off a cliff, teleport drop, end of flight), the ray returns `None` → `target_y = terrain_height` → **the player falls straight through the bridge** and lands on terrain. Same for NPCs spawning >20 m above objects. "Pass through some steps" is also consistent with this for elevated structures.

**Suggested fix:** Keep the terrain exclusion (good) but restore a generous cap (e.g., 200–300 m) — the expensive part (terrain trimesh) is already excluded, so the ray is cheap; or clamp the ray origin to `max(camera_y, feet) - 50` so the reach is relative to the ground, not the player.

### 3.3 [HIGH] Collision: NPC ground cache + distance gating has stale-ground edge cases

**File:** `src/systems/collision_system.rs:227-347`.

**Problem:** `use_cached = cache_valid || (!in_range && ground_cache.is_some())` means a far NPC (>150 m) that *moves* keeps its last cached ground forever (no re-resolve, `falling` re-check only when `in_range`). If such an NPC walks onto/off a bridge far from the camera, its `transform.y` freezes at the old height until the camera approaches. On camera approach there is no hysteresis: the 150 m boundary flips `in_range` per frame, but behavior converges. Also the `falling` heuristic (`transform.y - cached_ground_y > dt*9.81`) is false for an NPC standing still whose cached ground is *above* its current y (e.g., a platform was cached, then the NPC walked off the platform edge — horizontal movement > 1 cm invalidates the cache, so this resolves — OK). Net: far-NPCs use stale ground; acceptable for visuals, but a spawned-while-far NPC resolves once at spawn and never re-resolves while the camera stays away — if the resolve happened mid-air (initial y), the NPC may float. Recommend resolving at spawn unconditionally and adding a small hysteresis band.

**Also:** `transform.translation.y` write and `position.z` writeback are skipped when equal (fine), but the cached `target_y` is used verbatim — for an entity whose ground was cached while a zone object was later *despawned* (zone reload), the cache stays stale until movement — acceptable.

### 3.4 [HIGH] Animation culling freezes off-screen poses → delayed `AnimationFrameEvent` / delayed kills (combat-sync interaction)

**Files:** `src/animation/mod.rs:32-56` (`should_animate_entity`), `skeletal_animation.rs:36-70`, `mesh_animation.rs:44-70`, `transform_animation.rs:27-46`.

**Problem:** Entities with `ViewVisibility == Hidden/false` and >200 m from the main camera are skipped **before** `AnimationState::advance()` runs. Consequences:
1. Off-screen attackers' `SkeletalAnimation` never advances → `completed()` stays false → `pending_damage_system::hit_frame_expected` (attacker mid `Attack`/`CastSkill` with uncompleted animation ⇒ kill waits) **delays kills by up to the 1.5 s `KILL_MAX_DAMAGE_AGE` cap** whenever the attacker is off-screen — a behavior change against `pitfalls/combat-sync.md` (which bounds but expects hit-frame sync). Bounded, but a real delay.
2. `AnimationFrameEvent` (attack hit frames, footsteps, skill-effect frames) is never emitted for off-screen entities — `animation_sound_system`/`animation_effect_system` miss events that were previously emitted (mostly invisible, but sound events for off-screen NPCs are also culled by the new audio gate, consistent).
3. When the entity re-enters the frustum, the pose **jumps** to the current animation time (visible pop for monsters walking behind the camera or NPCs at the 200 m margin). No hysteresis.
4. Death animations off-screen never complete (no despawn-triggering path found that depends on it — LOW).

**Suggested fix:** For entities with a `Command::Attack/CastSkill`/pending kill (i.e., combat-relevant), always advance; cull only *pose writes*. At minimum, keep advancing the `AnimationState` (cheap) and skip only the bone `Transform` writes:
```rust
if !should_animate_entity(...) { skeletal_animation.advance(...); continue; }
```
and add hysteresis (cull at >R+50, resume at <R).

### 3.5 [HIGH] Zone byte cache: cache-first read inverts real-FS priority and can serve stale data

**File:** `src/zone_loader/loading.rs:31-38` (`read_bytes_with_priority`), `vfs_asset_io.rs:44-60`.

**Problem:** The function now checks `get_cached_bytes` **before** the real filesystem. Any file the zone loader reads that is also writable by the user/tools (extracted `base_path` terrain blocks, `--new-terrain` mesh exports, map-editor saved blocks) will be shadowed by the session's first read **forever** (the byte cache is process-global, `static`, and never invalidated). The map editor writes `write_him_file`/`write_til_file` to disk and reloads zones — saved terrain edits will not appear on reload. The comment/priority contract ("real filesystem first") is now a lie.

**Suggested fix:** On cache hit, stat the real file (cheap `metadata` mtime/size) and bypass the cache if the real file is newer or has a different size; or keep the cache only for VFS-sourced reads (tag entries with their source and skip cached entries sourced from VFS when a real file exists).

### 3.6 [MED] ProjectileIndex: stale entries never pruned on zone change/despawn paths

**Files:** `src/resources/projectile_index.rs` (new), `src/systems/projectile_system.rs`, `spawn_projectile_system.rs`.

**Problem:** The index removes entries only in `projectile_system`'s two despawn sites. Zone-change cleanup and any other despawn path (e.g., `commands.entity(...).despawn()` during reconnect — see §3.12) leave stale `(attacker → projectile)` entries. The `query_projectiles.get()` world-verification makes them *harmless for correctness* (stale entries are skipped), but the map grows monotonically per session (one small Vec entry per projectile ever spawned, keyed by attacker entity ids that also go stale). Low-memory leak; consider pruning via `RemovedComponents<Projectile>` or clearing on `ZoneEvent::Loaded`.

### 3.7 [MED] Blood overlay: clean-frame sync no longer picks up *new* model-part entities (equip change regression)

**File:** `src/systems/blood_overlay_system.rs:103-130`.

**Problem:** On clean frames the new code iterates the cached `blood_overlay.material_part_entities` (entity ids captured on the last dirty frame) and re-binds their *current* material handles. The old code re-walked the subtree every frame, so model parts **spawned after** the blood was painted (equipment change, model respawn) were re-bound. With the cache, a freshly spawned part entity is not in the list → its material never receives `blood_overlay_texture` → **blood disappears on equip change and only returns on the next hit**. The code comment claims the cache still handles "respawned parts", but it only handles *material-handle* changes on the same entities.

**Suggested fix:** Keep the dirty-frame walk, but also re-run a cheap query (`Query<(Entity, &MeshMaterial3d<RoseObjectExtension>), (With<CharacterModelPart> or ChildOf>)`) on clean frames keyed on *part-count/entity-set hash*; or store the part *mesh-path keys* and re-resolve to entities each frame with one query pass (still cheaper than the recursive walk).

### 3.8 [MED] Weather: silent hard cap of 1000 particles vs UI (20 000) + billboard freeze

**File:** `src/systems/season/weather_system.rs:48-53, 132-142`.

**Problem:** (a) `settings.max_particles.clamp(0, 1000)` silently overrides the UI's advertised range (settings > 1000 are ignored — a UX mismatch, though the intent (review 05 F3 tier 1) was to lower the default, not to hard-cap the setting). (b) Particles that leave the front 150 m cone keep moving but stop being billboarded; they keep their last orientation (edge-on quad = invisible, or a visibly frozen sprite when the camera turns and the particle is still in view at the boundary). No hysteresis. Flag the cap as a settings-decision; make the UI range reflect it or clamp in the UI layer.

### 3.9 [MED] Zone switching now re-parses revisited zones; eviction keep-sets inconsistent

**Files:** `src/zone_loader/systems.rs:516-555` (`zone_loaded_from_vfs_system`), `:333-364` (`zone_loader_system` Spawned branch), `vfs_asset_io.rs:100-128`.

**Problem:**
1. Every zone change now `take()`s **all** cached zones (including the outgoing current zone) and `zone_loader_assets.remove`s their parsed data. A return visit to the previous zone re-reads, re-decompresses (unless the byte cache still holds it), re-parses, and re-spawns everything — **zone switching is slower for revisits** than before (previously the parsed data stayed cached; that was the memory leak the review targeted, but the fix swings fully to the other side). The byte cache keeps {incoming, current} so a direct A→B→A bounce is cheap; a longer cycle A→B→C→A is not.
2. `zone_loader_system`'s Spawned branch evicts with `evict_zone_tagged_files(&[zone_id])` (keep = incoming **only**), while `zone_loaded_from_vfs_system` keeps {incoming, current} — inconsistent keep-sets for the same logical operation (the Spawned branch is nearly dead for async loads, but if it ever runs, the current zone's block bytes are dropped while the player is still in it).
3. **First-login / temporary-zone suspicion:** no definitive logic break was found in the async load/spawn flow (the incoming zone is never evicted from the cache by `take()` because it isn't cached yet; the byte cache keeps the incoming zone's files). The most plausible contributor to "terrain not loading on first login" is §3.5's stale-cache shadowing if any block file was previously cached under a case-variant key, or the `get_terrain_height` block-clamp at exactly `block_x/y == 64.0` (pre-existing). Recommend a targeted session log check (`[ZONE LOADER SYSTEM]` + `[VFS CACHE]` lines) on first login before further code changes.

**Suggested fix:** Keep the *incoming* zone and the *current* zone's parsed data in the cache (evict only older zones), or evict-but-keep-byte-cache as today but re-insert the outgoing zone's `CachedZone` (data_handle kept alive) so the A→B→A bounce stays parse-free.

### 3.10 [MED] Audio: one-shots hard-despawned beyond 100 m (mid-playback pop); loops restart from the beginning

**File:** `src/audio/spatial_sound.rs:140-165`.

**Problem:** A one-shot playing at 90 m that drifts to 101 m is **despawned instantly** — an audible hard cut (the review's own risk note: "Pausing a Stop control mid-playback clicks"). Loops are `stop()`-ed, and on re-entry (<80 m) the control/stream handles are dropped and the sound re-initialized from the **start of the file** — a phase jump in the loop (noticeable for short repeating SFX like waterfalls/ambient loops with a beat). No gain ramp.

**Suggested fix:** For one-shots, let them finish once started (only cull *new* spawns beyond the cutoff — already done in `queue_monster_sound`); for loops, ramp `SoundGain` to 0 before `stop()` and remember the playback offset (`StreamingSound` position) to resume near it.

### 3.11 [MED] Streaming refill budget ≈ one 60 fps frame of audio — underrun/stutter risk

**File:** `src/audio/streaming_sound.rs:76-118`.

**Problem:** `refill_budget = source.sample_rate() / 20` limits each fill to 1/20 s of *source* samples (e.g., 2205 @ 44.1 kHz). The oddio ring drains at the *device* rate (e.g., 2400/frame @ 48 kHz/60 fps) and the source rate may differ. At exactly 60 fps the budget matches consumption with **zero slack**; any dropped/slow frame or rate mismatch (source 44.1 k vs device 48 k ⇒ ~0.4% deficit) progressively drains the ring ⇒ stutter in BGM/streaming SFX. The old code refilled the ring to capacity per call.

**Suggested fix:** Size the budget to ~2–3 frames of device-rate audio (`2.5 * sample_rate / 60`) or, better, refill until the ring rejects a full packet (the old behavior) and rely on the new `start`/swap mechanics only for the memcpy removal.

### 3.12 [MED] Reconnect despawn uses deferred commands — entity-id reuse between sessions could panic

**File:** `src/systems/game_connection_system.rs:333-349`.

**Problem:** `ConnectionRequestSuccess` now despawns all client entities via deferred `Commands`, then `client_entity_list.clear()`, then subsequent messages in the same drain spawn the new session's entities. If the server reuses the previous session's entity ids for the new spawns, the *same frame's* command queue would apply `despawn(old_id)` followed by `insert(...)` on the same id — the insert then targets an entity despawned earlier in the same queue (Bevy 0.18: `Commands::entity(id).insert` on a non-existent entity panics when applied). If ids are unique per session this is fine (order is correct: despawn before spawn). Verify the server's id allocation; if reuse is possible, defer the despawns to the *next* frame (or skip the old ids when spawning).

### 3.13 [LOW] Miscellaneous

- `update_shadows_for_time_of_day_system` latch (`zone_lighting.rs:518-534`): correct — body depends only on `(state, settings)`; ordering (`.after(apply_shadow_quality_system)`) verified.
- `ui_settings_system` gating on `settings_window_open` (`lib.rs:1617-1621`): the system only draws the settings window; settings are applied by the resource-driven apply systems — safe.
- `ui_admin_menu` popup ScrollArea id change (`grid_id` param removed): egui state for the scroll position is now anonymous — cosmetic.
- `fish_system` separation indexing (`fish_system.rs:770-779`): `near_idx` increments in query order of the apply query; the collect query requires `Entity + &GlobalTransform + &Fish`, the apply query `&mut Transform + &GlobalTransform + &mut Fish` — orders match only while every fish has `Transform` (all spawns do). Fragile invariant; a fish without `Transform` would panic on `buffers.pushes[near_idx]`. Add a `With<Transform>` filter to the collect query for symmetry.
- `model_loader.rs` NPC skeleton cache keyed by `npc_id` instead of `skeleton_index` (review asked for skeleton_index): correct but keeps N duplicate `Arc<ZmdFile>` for N NPC ids sharing a skeleton.
- `background_music_system`: `sound_cache.clear()` on zone change is safe (playing entities hold handle clones); the crossfade state machine was audited — no re-trigger during `FadingOut`.
- `zone_loader/systems.rs:541-547`: `cache.take()` + `assets.remove` runs even for the incoming zone's cache entry when the same zone is reloaded — the "already loaded" branch (line 136-146) already clears it first, so no double-remove panic (remove of a missing asset is a no-op).
- `zone_lighting` sun-color/rotation guards use exact `!=` equality — safe (skip only identical values).
- `write_f32_if_changed` epsilon 1e-3 on `state_percent_complete`: quantizes sky-transition steps to ~1.6 s real time at typical day length — imperceptible.
- Map-editor state gating (XC-17): complete and consistent; `load_available_models_system` is now MapEditor-only — verify the model viewer doesn't consume `AvailableModels` (reviewer could not rule it out statically).
- `Cargo.toml` +1 line — presumably a new dependency for the audio/UI changes (not verified against the lockfile).

---

## 4. Prioritized Issue List

### High
1. **§3.1** `find_object_top_height` ascending scan can't climb stacked objects (can't-climb / under-stairs unstuck broken). `collision_system.rs:111-177`.
2. **§3.2** 50 m / 20 m ground-ray caps cause pass-through on high falls onto platforms/bridges. `collision_system.rs:45-48, 715-729`.
3. **§3.4** Animation culling freezes poses + suppresses `AnimationFrameEvent` → up to 1.5 s delayed kills, pose pops at frustum/radius boundaries. `animation/mod.rs:32-56`, `skeletal_animation.rs`.
4. **§3.5** Zone byte cache serves stale data (real-FS priority inverted; map-editor saves shadowed). `loading.rs:31-38`.

### Medium
5. **§3.9** Zone-switch eviction re-parses revisited zones (slower switching); inconsistent keep-sets; candidates for first-login terrain issue.
6. **§3.7** Blood overlay lost on model-part respawn (equip change) until next hit. `blood_overlay_system.rs:103-130`.
7. **§3.10** Audio one-shot hard-cut at 100 m; loop restart phase jump. `spatial_sound.rs:140-165`.
8. **§3.11** Streaming refill budget underrun → BGM stutter at 60 fps / rate mismatch. `streaming_sound.rs:76-118`.
9. **§3.12** Reconnect despawn vs entity-id reuse (potential panic). `game_connection_system.rs:333-349`.
10. **§3.3** Far-NPC ground cache staleness (frozen heights beyond 150 m, no hysteresis).
11. **§3.6** `ProjectileIndex` monotonic growth from zone-change despawns.
12. **§3.8** Weather hard cap ignores UI settings >1000; billboard freeze at cone edge.

### Low
13. Fish separation fragile `near_idx` invariant. `fish_system.rs:770-779`.
14. NPC skeleton cache keyed by npc_id (memory) — `model_loader.rs:255-268`.
15. SMAA/MotionBlur still always-on at boot until settings change; SSR/CAS never controllable (incomplete 02-F9).
16. `blocked_frames` in boats now ticks at 10 Hz — waypoint skip ~6× slower. `boats.rs:534-560`.
17. Admin popup scroll-id change; particle/digit material still re-prepared per frame (partial 02-F1/F2).
18. `apply_bloom_system` removal only fires on settings change — fine (component path is unified now).

---

## 5. Explicit Notes on the Three Suspected Problem Areas

### 5.1 Collision (world-object collision broken: "pass through some, can't climb others")
**CONFIRMED — multiple regressions, all in `src/systems/collision_system.rs`:**
- **"Can't climb"** = §3.1 (ascending scan; the 64×100 m ray was load-bearing for stairs/stacked objects; the new scan terminates after the first surface and cannot climb a staircase or reach a step top >0.35 m above the feet).
- **"Pass through"** = §3.2 (50 m ray cap for high falls; objects beyond the cap are invisible to the ray; the player falls to the heightmap through them). Also, with `COLLISION_GROUP_ZONE_TERRAIN` excluded from both ground rays, terrain steps are only as good as the heightmap's bilinear sample — at non-planar step quads the bilinear height differs from the triangulated collider by up to ~half a step, so players can clip into vertical step faces by centimeters (minor but visible).
- **Submerged skip (§3.3)** additionally drops ray/object-top for underwater entities (bridges under water are passed through).
- The write guards and `GroundHeightCache` logic are correct in isolation; the regressions are the ray filter/length and the scan rewrite.

### 5.2 Zone loading (terrain on first login, slower switching, temporary-zone teleport)
**PARTIAL — no single smoking gun found in the async load/spawn flow, but three credible contributors:**
- §3.5 cache-first read shadows freshly written/modified real-FS files (first-login terrain missing if the data dir is being extracted/written concurrently; also map-editor reload).
- §3.9 every switch now drops all parsed zone data → revisits re-parse + re-spawn (slower switching); byte-cache keep-sets differ between the two eviction sites.
- The "temporary zone" teleport path: the byte cache keeps {incoming, current} so a rapid temp→real→temp dance keeps the two zones' bytes but re-parses on each hop. Terrain "not loading" on first login could also be the pre-existing `get_terrain_height` clamp at block 64.0 or noise divergence (only when `noise_enabled`). Recommend capturing a `logs/<session>` trace around the first `LoadZoneEvent` before changing code.
- Verified NOT broken: incoming-zone self-eviction, double-remove panics, handle lifetime (the incoming asset is re-inserted immediately after spawn).

### 5.3 Animation visual glitches (ViewVisibility culling, epsilon guards)
**CONFIRMED RISK, localized:**
- §3.4: culling freezes off-screen poses (pop on re-entry, no hysteresis) and **suppresses `AnimationFrameEvent`/kill-hit sync** (up to 1.5 s delayed kills via `hit_frame_expected`). This is the most likely source of "animation glitches".
- The epsilon guards (1e-5/1e-4) and `advance()` change-guards are value-preserving and safe; the ZMO `flags` precompute matches the old per-frame computation exactly; the mesh-animation early-out covers `alpha` (derived from the frame index already packed into `current_next_frame`).
- The 200 m margin only covers distance; frustum-culled-but-close entities still freeze — widen the margin or always advance `AnimationState` (see §3.4 fix).

---

*End of report. No game code was modified; no build was run; no game was launched.*

---

## 6. Verification Update (2026-08-04)

Independent sub-agent re-scrutiny of every detailed finding (§3.1–§3.13) against the actual code on both branches, reconciled with the per-finding verdicts now recorded in the §7 verification sections of `optimization-reviews/01…11`. **Status note: everything this report validated lives on `wip/local-changes-2026-08-04`; the current main working tree has the validated fixes REVERTED (both ApplyDeferreds, directional_light_system, the diagnostics gate, the cache-first reads, the animation culling — all present only on wip).** Verdicts per section:

| Section | Verdict | Scrutiny result / action |
|---|---|---|
| §3.1 | CONFIRM (diagnosis + impact) | The wip ascending scan is broken exactly as described (0.25 m first window / 0.1 m windows / break on first empty / 10-step cap; parry `ray_aabb` solid=false exit-point climbing was the load-bearing behavior — confirmed in parry source). **§3.1's own Option A (10 m first reach) is insufficient** (the post-first-hit 0.1 m windows still can't climb stairs) and Option B (0.45 m) is unsafe. The correct fix is the 04-F2 row's: restore the fixed-origin 100 m ray verbatim + convergence break `hit_height <= top_height → break` (use `<=`, not `<`) + keep the `[Entity;16]` buffer. Typical 2–3 casts; worst case N+1 for staircases (bounded by the old 64 cap). |
| §3.2 | PARTIAL (with corrections) | Caps confirmed (20 m NPC / 50 m player on wip; pre-fix 10000 m player / 100 m NPC). **The mechanism is imprecise**: in-range per-frame-resolving entities re-cast during descent and land on the platform — the genuine pass-through is the far-NPC cache path + the submerged skip + spawn-time cap poisoning. **§3.2's option (b) (clamp origin to camera-50) is FLAWED — reject** (lands the origin ~45 m below the feet for a typical camera; the entity falls through the object it stands on). 04-F3 supersedes: player ray 300 m + origin clamp to `terrain_height + 150`; NPC restore 100 m; keep ZONE_TERRAIN exclusion; make `falling` re-resolve regardless of range (F1); drop/gate the submerged skip. |
| §3.3 | PARTIAL (superseded by 04-F1) | Mechanism real and line-accurate (far-NPC `use_cached = cache_valid \|\| (!in_range && cache.is_some())` → resolve never re-runs even while falling; far NPCs sink through bridges or hover). **"Acceptable for visuals" is wrong**; **"resolve at spawn" is ALREADY the behavior** (the cache is lazily inserted — first frame always resolves; that spawn-frame resolve is itself what poisons via the 20 m cap) — the suggested fix is a no-op; the hysteresis band is cosmetic. Required: 04-F1's fix (300 m caps, `falling` resolves regardless of range, 0.5 s / >25 cm throttled far re-resolve, drop the submerged skip). §3.3's (a)/(c)/(d) mechanism descriptions stand; the fix column should be rewritten. |
| §3.4 | PARTIAL (severity refuted for the current wip — informational/LOW) | The mechanics are accurate, but **the freeze is unreachable**: combat roots have no `Aabb` (no Mesh3d → Bevy's `check_visibility` sets them VISIBLE unconditionally) → `should_animate_entity` always returns true → the 200 m distance check never runs. **The wip culling is a NO-OP for the entire character/NPC population** (only EffectMesh entities can be culled — cosmetic). Delayed kills/event suppression/pose pops are hypothetical unless someone makes culling effective. Keep §3.4 as the regression checklist for implementing 01-F1's culling correctly: gate on distance only (root ViewVisibility is unusable), always advance `AnimationState`, cull only pose writes (keeps AnimationFrameEvent emission), hysteresis (skip >260 m / resume <200 m), never cull combat-relevant entities, debug counter. Note: `advance` must move before the gate and needs the ZmoAsset in scope (restructure, not a one-liner). |
| §3.5 | CONFIRM (with minor imprecisions) | Cache-first on wip verified end-to-end (map-editor saves shadowed forever once read; the real-FS priority contract is violated). "Shadowed forever" applies only to files read before the write (cold-cache first reads still hit real-FS); the **first-login-terrain connection is implausible** (process-global static, empty at first login; the case-variant sub-suspicion is moot on wip — keys are normalized). Fix per 07-F3/F4: **real-FS check BEFORE cache** using 07-F14's merged probe form (single `std::fs::read` + `NotFound` fallback); do NOT adopt the stat-on-hit/mtime design (same syscall cost as the reorder + more machinery; a size-only check misses HIM height edits — mtime is load-bearing); optional `SaveZoneEvent`-completion invalidation for zero-stat editor reloads. |
| §3.6 | **REFUTE** (leak does not manifest) | The index **self-prunes** (remove() drops empty source keys); zone-change cleanup never touches projectiles (top-level, not in the zone cache); reconnect/RemoveEntities/die-command only touch client_entity_list entities; the bullet effect is a **child** of the projectile (the "effect_system? unverified" flag is resolved — not a projectile path); growth is bounded by concurrent in-flight projectiles. A `RemovedComponents` sweep is infeasible (the component carrying `.source` is gone by fire time) and clearing on `ZoneEvent::Loaded` would be **actively wrong** (projectiles survive zone changes — it would unindex live projectiles and delay kills). Only defensible hardening: `clear()` on reconnect, and that's belt-and-suspenders. Fix the doc's wording. |
| §3.7 | PARTIAL (mechanism real; **regression framing wrong**) | The cache mechanism is as described — but **the blood disappearance pre-exists on main**: all per-part data (stains, overlay textures, dirty flags) is keyed by the OLD entity id; an equip change spawns new ids with new material handles, and the main-branch per-frame walk binds them to `None` too. The two branches are visually identical for the equip-change case — it's a pre-existing data-keying limitation, not a cache-introduced regression. The suggested fix **doesn't compile** (CharacterModelPart is an enum, not a Component; wrong material type; invalid filter syntax) and an unscoped query is more expensive than the walk. Correct fix per BL-02: read `CharacterModel.model_parts` / `NpcModel.model_parts` (authoritative, always current; same pattern on VehicleModel/ItemDropModel/PersonalStore) on clean frames; keep the dirty-frame walk as the fallback; skip `get_mut` when the part has no overlay entry. Re-scope the entry from "regression" to "pre-existing limitation". |
| §3.8 | CONFIRM | Both items verified — with an addition: the resource **default (2000) is itself silently clamped to 1000**, so even a fresh session's settings display lies (the alignment fix must cover slider range AND default). The billboard-freeze mechanism is fully confirmed (rotation is written exclusively by the billboard block; culled particles keep moving with frozen orientation; edge-on quads are invisible; no hysteresis; no far-plane despawn). Fixes per 05-F3: clamp in the UI layer (slider + default to the cap — the cap VALUE is a settings decision, the defect is the silent divergence; do not raise the cap back); add hysteresis (skip >150 m / resume <100 m, plus an angle buffer for the behind-camera test). |
| §3.9 | PARTIAL (items 1–2 stale — resolved by the final wip commit) | Both eviction sites in the final wip commit keep **{incoming, current}** (item 2's "keep = incoming only" matches no reachable commit); A↔B bounces are now parse-free AND decompress-free (strictly better than pre-wip); longer cycles re-parse by design (accepted per 07-F1 — no LRU needed). Item 3 correctly defers to §3.5. The suggested fix is already implemented. The real remaining work is the four 07-F1/F4 follow-ups, all still present on wip: (1) the stale-drop branch leaks one parsed zone (`remove(&event.zone_handle)`); (2) the §3.5 real-FS inversion; (3) the all-or-nothing budget `retain` on loading threads (replace with real LRU); (4) zone-tag refresh on hit. |
| §3.10 | PARTIAL (both regressions real; fix per 08-F2, not as written) | One-shot hard cut confirmed (termination is abrupt mid-waveform → pop, even with the ring/delay tail draining first); loop `stop()`+restart confirmed (stop permanently removes the signal from oddio's set — un-resumable; phase jump + ring re-alloc per crossing). **§3.10's "remember the playback offset" is unnecessary and unimplementable cleanly** (StreamingSound has no seek; stop is for good) — `pause()` achieves phase-exact resume for free (signal stays in the set, ring/position retained). Required rework: (1) delete the one-shot cull branch (the spawn gate + F3's 64-cap already bound them); (2) loops: gain-ramp → `pause()`, keep handles, restore+ramp gain on resume (the component-gain-at-0 trap — the resume path builds oddio gain from the component); (3) **radius-aware cutoff (`distance > cutoff + radius`)** — the piece §3.10 missed entirely (IFO sound objects carry ranges up to 25.5 m; a large-radius waterfall is cut ~75 m inside its audible sphere). |
| §3.11 | **REFUTE** (arithmetic wrong; 08-F1's 3× headroom stands) | "2400 frames/frame" is a units error (2400 = 48000/20 = the budget at device rate, mislabeled as per-frame consumption). oddio drains the ring in **source-rate** frames (735/frame @ 44.1 kHz) and resamples internally — the "0.4% deficit" is fabricated; headroom is 3.0× (2205/735). **The suggested formula (`2.5 × SR/60` = SR/24 = 1838) is SMALLER than the current budget** — a strict regression (underrun floor 20 → 24 fps). Keep `refill_budget = sample_rate/20` unchanged. Residual real risks (already covered by 08-F1): the spawn window (ring starts ~50–70 ms; a zone-load hitch at BGM start could clip the first instant — benign) and sustained <20 fps. Reclassify with the corrected math. |
| §3.12 | PARTIAL (panic **unreachable**; the mitigation is counterproductive) | The server DOES reuse wire ids (first-fit free slots) — but the panic can't happen: Bevy never reuses (index, generation) tuples for fresh spawns (generation increments per free); all list-keyed message arms go inert after `clear()`; the only stale pointers (player_entity/player_entity_id) would need a player-keyed message between ConnectionRequestSuccess and CharacterData — the server sends them back-to-back with nothing between (verified). **"Defer despawns to the next frame" is counterproductive** (creates a duplicate-player window for the 76 With<PlayerCharacter> systems). Keep the wip fix; add the free 2-line hardening (reset player_entity/player_entity_id in the arm). One correction: the "prune ProjectileIndex" note is N/A not because the structure doesn't exist (it does, on wip — see §3.6) but because pruning on reconnect is unnecessary belt-and-suspenders. |
| §3.13 | Per-bullet (see below) | #1 **REFUTE** — the latch key can't see `apply_shadow_quality_system`'s cross-write (any GraphicsSettings change during Evening/Night leaves moon shadows on + sun shadow map at night; self-heals at the next transition) — replace the latch with per-field value-diff writes (05-F7). #2 CONFIRM (settings gate safe; line ref 1628). #3 PARTIAL — the change is a full Grid→virtualized Table rewrite, not just id removal; the "cosmetic" conclusion holds (per-window auto-ids — no scroll-state collision). #4 **REFUTE** — the described dual-query doesn't exist on either branch (one shared query; identical entity sets; the `With<Transform>` filter is a no-op — Transform is already required); no hazard. #5 CONFIRM — re-key the NPC skeleton cache by `skeleton_index` (~2 lines). #6 CONFIRM — `clear()` is `fill(None)` (no `Assets::remove` — the 08-F11 hazard doesn't apply); note it's a partial F11 only. #7 PARTIAL — mechanism wrong (both eviction loops explicitly SKIP the incoming entry; correct refs are :568-579/:356), no-panic conclusion right. #8 CONFIRM. #9 CONFIRM — align the ~1.6 s figure with 05-F4's ~0.5–2.6 s (day_cycle-dependent). #10 CONFIRM — the AvailableModels question is **resolved**: zero consumers outside src/map_editor. #11 **REFUTE** — the Cargo.toml +1 line is a stray blank line; Cargo.lock identical; nothing to verify. |

**Cross-cutting note for the whole doc set:** the vendored source folder `bevy-collection\bevy-0.18.1` is actually **0.19.0-dev** (its `Cargo.toml` declares `version = "0.19.0-dev"`); the client builds against crates.io Bevy 0.18.1 — API citations verified only against that folder must be re-checked against 0.18.1 (this invalidated the F2/XC-10 "set_data reuses the GPU buffer" claim, and F1's storage-buffer resize APIs).
