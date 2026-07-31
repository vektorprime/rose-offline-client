# Simplification Plan — `src/systems/` VISUAL/EFFECT & WORLD Half

**Module:** `src/systems/` (visual/effect/world subsystems)
**LOC analyzed:** ~15,000 (49 files: debug/zone diagnostics, name tags, effects/particles, blood/damage, ambient life, seasons, sailing)
**Status:** PLAN ONLY — no code changes were made. This is a research report.

Files analyzed: debug_rendering_system.rs (1727), zone_render_validation_system.rs (637), zone_memory_profiler_system.rs (424), zone_memory_protection_system.rs (158), zone_viewer_system.rs (115), zone_time_system.rs (626), world_time_system.rs (19), name_tag_system.rs (786), name_tag_visibility_system.rs (136), name_tag_update_color_system.rs (69), name_tag_update_healthbar_system.rs (27), animation_effect_system.rs (609), animation_sound_system.rs (565), particle_sequence_system.rs (647), spawn_effect_system.rs (168), spawn_projectile_system.rs (86), projectile_system.rs (129), effect_system.rs (61), blood_spatter_system.rs (575), blood_overlay_system.rs (414), gash_wound_system.rs (413), hit_event_system.rs (228), damage_digit_render_system.rs (209), pending_damage_system.rs (198), pending_skill_effect_system.rs (268), status_effect_system.rs (69), visible_status_effects_system.rs (69), passive_recovery_system.rs (20), wing_spawn_system.rs (564), boat_spawn_system.rs (806), boat_wake_system.rs (282), bird_system.rs (677), fish_system.rs (445), wind_system.rs (32), wind_effect_system.rs (310), model_viewer_system.rs (357), debug_inspector_system.rs (140), transform_propagation_diagnostics.rs (132), season/{mod,season_manager,spring,summer,fall,winter}.rs (1152), sail_animation_system.rs (121), sailing_movement_system.rs (112), sail_camera_system.rs (44), remote_boat_system.rs (123), systems/mod.rs (251).

Note: `sailing.rs` (141 LOC) lives at `src/sailing.rs` (referenced by the sailing systems), not under `src/systems/`.

---

## Finding 1 — `debug_rendering_system.rs` is 100% dead code (highest-priority win)

**Location:** `src/systems/debug_rendering_system.rs` (1727 lines), exports in `src/systems/mod.rs:149-158`.

**Evidence:** All 20 functions (`debug_entity_visibility`, `render_diagnostics_system`, `render_diagnostics_system_lightweight`, `frustum_culling_diagnostics`, `material_transparency_diagnostics`, `transform_validation_diagnostics`, `visibility_state_diagnostics`, `active_camera_diagnostics`, `camera_configuration_diagnostics`, `render_layer_diagnostics`, `aabb_validation_diagnostics`, `render_pipeline_diagnostics`, `render_stage_diagnostics`, `zone_entity_visibility_diagnostics`, `parent_child_visibility_diagnostics`, `zone_component_lifecycle_diagnostics`, `diagnose_render_world_extraction`, `diagnose_render_phase`, `diagnose_camera_entity_distances`, `verify_material_plugins`) are re-exported from `mod.rs` but **never registered in `lib.rs`** (grep across the crate finds no other usage). `camera_configuration_diagnostics` is not even exported. The module imports `Extract, ExtractSchedule, Render, RenderApp, RenderSystems` which are never used — the file has no RenderApp systems at all.

**Worse:** Several systems have every log statement commented out and only count entities (`frustum_culling_diagnostics:447-498`, `active_camera_diagnostics:793-868`, `camera_configuration_diagnostics:882-919`, `parent_child_visibility_diagnostics:1372-1448`, `diagnose_render_world_extraction:1562-1578`, `diagnose_camera_entity_distances:1642-1696`) — they perform a full world query every 60 frames and then do nothing with the data.

**Suggestion:** Delete the entire file and its `mod.rs` re-exports. If one lightweight status line is wanted, keep only `render_diagnostics_system_lightweight` (needs `RenderExtractionDiagnostics`, which is still used at `lib.rs:2508`).
**Savings:** ~1,720 LOC (≈11% of analyzed scope).

---

## Finding 2 — Three "zone diagnostic" plugin files are defined but never registered

**Locations:**
- `zone_render_validation_system.rs` (637 lines) — `ZoneRenderValidationPlugin` (line 618)
- `zone_memory_profiler_system.rs` (424 lines) — `ZoneMemoryProfilerPlugin` (line 412)
- `zone_memory_protection_system.rs` (158 lines) — `ZoneMemoryProtectionPlugin` (line 151)

**Evidence:** None of the three plugins is added in `lib.rs` (grep finds only the file-internal definitions and `mod.rs` `pub mod` declarations). They exist solely as leftover black-screen debugging. They also misuse `static mut FRAME_COUNTER` with `unsafe` in 5+ systems (`zone_render_validation_system.rs:102-106, 257-263, 312-318, 356-362, 452-458, 539-545`) instead of `Local<u32>` — unsafe code that never runs. `RenderValidationFailure`, `EntityFrameTracer`, `LeakAlert`, etc. are all unused.

**Suggestion:** Delete the three files and the `pub mod` lines in `mod.rs:242-248`.
**Savings:** ~1,220 LOC.

---

## Finding 3 — Remaining unregistered diagnostic systems

- `transform_propagation_diagnostics.rs` (132 LOC): both functions exported (`mod.rs:222-224`) but never registered. `post_update_systems_diagnostics` prints a static text block once and does nothing. Delete file + exports.
- `zone_viewer_system.rs:78-115` `debug_camera_render_state_system`: never exported/registered; all logs commented out. `calculate_look_direction` (lines 66-75) is unused. Delete both.
- `zone_time_system.rs:530-626` `color_grading_time_of_day_system`: DISABLED in `lib.rs:269-270, 1327-1330` ("conflicts with Bevy 0.16 Atmosphere"). It is 78 lines of dead code plus 14 const declarations — but note `zone_time_system` itself uses the lerp/temperature values nowhere else.
- `wind_effect_system.rs:295-310` `cleanup_wind_particles_on_flight_end`: never exported from `mod.rs` (only `wind_emitter_spawn_system`, `wind_particle_spawn_system`, `wind_particle_update_system`, `WindEffectPlugin` are). Dead.

**Savings:** ~260 LOC.

---

## Finding 4 — `wing_spawn_system.rs`: feature disabled, ~450 lines of dead implementation

**Location:** `src/systems/wing_spawn_system.rs` (564 lines).

**Evidence:** `wing_spawn_system` (lines 47-93) explicitly states "Wing spawning is currently DISABLED" and only logs. The full implementation remains:
- `spawn_wings()` lines 108-210 (~100 lines) — never called
- `create_angel_wing_mesh()` lines 220-499 (~280 lines of procedural feather mesh) — never called
- `wing_animation_system` (505-544) animates `AngelicWings` entities that are never spawned (query always empty)
- Test `test_wing_side_mirror` (546-563) tests a trivial match that asserts nothing of value
- `WingSpawnPlugin` IS registered (`lib.rs:1110`), so this runs every frame and logs on every `FlightToggleEvent`.

**Suggestion:** Either delete the disabled spawn code (keep the plugin for the material setup) or re-enable it. Do not keep both.
**Savings:** ~450-550 LOC.

---

## Finding 5 — Season systems are near-copies of each other

**Locations:** `season/fall_system.rs` (219), `season/winter_system.rs` (146), `season/spring_system.rs` (184), `season/summer_system.rs` (508).

**Evidence:** `fall_particle_system`, `winter_snow_system`, and `spring_rain_system` share ~90% of their body:
- Identical player-relative spawn block: `spawn_radius = 100.0`, `spawn_y = player_pos.y + 15.0 + rand*10.0`, `particles_this_frame = ((settings.spawn_rate * dt) as usize).max(10)`, circle offset math (`spring:40-52`, `winter:34-47`, `fall:105-119`)
- Identical billboard look-at block (`Quat::from_mat3(&Mat3::from_cols(...))`) copied verbatim 3× (`spring:107-120`, `winter:124-141`, `fall:197-214`)
- Identical despawn-on-lifetime / below-ground checks
- Differences are only: mesh/material handle, particle velocity fields, rotation/rotation_speed, wobble params

Additional dead code inside the season module:
- `season/summer_system.rs`: `summer_vegetation_system` (22-115, `#[deprecated]`), `vegetation_sway_system` (259-347, `#[deprecated]`), and ~160 lines of commented-out procedural-grass code (349-508) — none registered (`season/mod.rs:31-38` comment them out)
- `season/spring_system.rs:133-184` `spawn_flower_system` — `#[allow(dead_code)]`
- `season/fall_system.rs:7-76` `fall_particle_spawn_system` — `#[allow(dead_code)]`, duplicates `fall_particle_system`
- `season/season_manager.rs:21-38` `spawn_season_particles` — empty `#[allow(dead_code)]` stub (half the file)
- `season/mod.rs` declares `SeasonSystemSet` (52-56) that is never used

**Suggestion:** Merge `fall_particle_system`/`winter_snow_system`/`spring_rain_system` into one parameterized `weather_particle_system` driven by a per-season profile resource (mesh, material, velocity/size/lifetime ranges, rotation behavior). Delete all `#[allow(dead_code)]`/deprecated functions and commented blocks.
**Savings:** ~300 LOC from the merge + ~400 LOC of dead/deprecated removal.

---

## Finding 6 — `pending_damage_system.rs` and `hit_event_system.rs` duplicate each other

**Locations:** `pending_damage_system.rs` (198 lines), `hit_event_system.rs` (228 lines). Both registered (`lib.rs:1304-1306`).

**Evidence — verbatim duplicated code:**
- `normalize_or` — identical copy (`pending_damage_system.rs:13-20`, `hit_event_system.rs:22-29`)
- `random_local_wound_pose` — identical copy (`pending_damage_system.rs:22-30`, `hit_event_system.rs:31-39`)
- `apply_damage` — same logic (damage digits spawn + `Dead`/`DeathBloodHandled`/`NextCommand::with_die` insert + `ClientEntity` removal), `hit_event_system.rs:64-96` is a superset of `pending_damage_system.rs:35-71` (which also has the bug-prone `let _ = pending_damage_list;` and dead `query_transform` parameter)
- The blood/wound event block (spatter with profile + 2-3 `show_wound` events with `wound_events = if is_killed {3} else {2}`) is duplicated at `hit_event_system.rs:157-194` and `pending_damage_system.rs:150-190`

Both systems may apply the same pending damage entry (one on hit-event arrival, one on expiry/attacker-death) — duplicated side effects risk double blood/wound/digit spawning.

**Suggestion:** Extract shared helpers (`apply_damage_effects`, `emit_blood_and_wounds`) into a small `damage_effects.rs` module (or `uv_projection`-style shared util) and have both systems call them.
**Savings:** ~120 LOC + removes duplicate side-effect code paths.

---

## Finding 7 — `animation_effect_system.rs` / `animation_sound_system.rs`: duplicated resolution chains + log spam

**Locations:** `animation_effect_system.rs` (609), `animation_sound_system.rs` (565).

**Evidence:**
- The "resolve weapon → ammo class → ammo item → bullet_effect_id (with weapon fallback)" chain is duplicated verbatim: `animation_effect_system.rs:144-177` vs `animation_sound_system.rs:371-405`.
- The "vehicle arms bullet effect" chain is duplicated: `animation_effect_system.rs:123-139` vs `animation_sound_system.rs:359-369`.
- Both files independently re-implement `EventEntity`-style queries and the same weapon-item lookup (`get_weapon_item(...).map_or(0,...)` repeated ~6× in `animation_effect_system.rs` alone).
- `animation_effect_system.rs` logs with `log::info!` for every event/spawn (`:247-258, 267-295, 305-333`) — hot-path per-frame log spam.

**Suggestion:** Extract `resolve_bullet_effect_id(equipment, game_data) -> Option<EffectId>` and `resolve_vehicle_arms_effect_id(...)` into a shared helper used by both systems; downgrade per-event `info!` to `debug!` or remove.
**Savings:** ~80 LOC + quieter logs.

---

## Finding 8 — Procedural mesh/entity-spawn duplication across ambient-life systems

**Locations:** `bird_system.rs`, `fish_system.rs`, `wing_spawn_system.rs`, `boat_spawn_system.rs`, `boat_wake_system.rs`, `wind_effect_system.rs`.

**Evidence:**
- `create_bird_wing_left_mesh` (`bird_system.rs:436-502`) and `create_bird_wing_right_mesh` (`bird_system.rs:505-570`) are 65-line mirror copies (only X signs flip). The right wing could be generated by mirroring the left mesh.
- The `(Mesh3d, MeshMaterial3d, Transform, GlobalTransform, Visibility, InheritedVisibility, ViewVisibility)` spawn tuple is written out ~20+ times across these files (bird body/wings/fish/wake/spray/wind/boat parts). A small `visual_part_bundle(mesh, material, transform)` helper (like `spawn_visual_part` already in `boat_spawn_system.rs:495-512`) would cut repeated boilerplate.
- `boat_wake_system.rs:246-281`: the wake and bow-spray update loops are near-identical (tick → despawn → move → damp → alpha bucket → scale), differing only in gravity and material list.
- `wing_spawn_system.rs:144-209`: left/right wing spawn blocks are near-identical.

**Suggestion:** Mirror left→right bird wing; add shared spawn helper; merge wake/spray update into one generic `boat_particle_update` using a trait or enum.
**Savings:** ~150 LOC.

---

## Finding 9 — Over-engineering in the blood-effect trio

**Locations:** `blood_spatter_system.rs`, `blood_overlay_system.rs`, `gash_wound_system.rs` (~1,400 LOC combined).

**Evidence:**
- `blood_overlay_system.rs:239-247` `blood_overlay_update_system` iterates all overlays and does nothing ("Blood overlay intensity is managed via the texture alpha") — registered in `BloodOverlayPlugin` (394-403).
- `blood_overlay_system.rs:253-278` `blood_overlay_force_enable_system` checks the `DEBUG_FORCE_BLOOD` env var every frame in `PostUpdate` — constant env-string scan for a debug switch; could be a startup-only check or removed.
- `blood_overlay_system.rs:231-236` legacy `BloodOverlayTexture` component ("Legacy component for backward compatibility") — unused.
- `blood_spatter_system.rs:363-368`: `enable_layered_effects` only increments placeholder diagnostic counters ("Placeholder layered counters... until dedicated mist/droplet render entities are added").
- `blood_spatter_system.rs:297-357`: the pool-reuse branch and the fresh-spawn branch build the identical component tuple twice — only the material-mutation differs; the branches can be unified.
- `blood_spatter_system.rs` + `gash_wound_system.rs` + `hit_event_system.rs` + `pending_damage_system.rs` each re-define `normalize_or` (4 copies) — see Finding 6.
- `BloodEffectDiagnostics` counters (`blood_spatter_system.rs:184, 243, 320, 360-368, 409-453`) accumulate forever; only printed when `config.enable_diagnostics`.

**Suggestion:** Delete `blood_overlay_update_system` and the legacy component; gate `blood_overlay_force_enable_system` behind startup config; unify the pool/spawn branches; share `normalize_or`.
**Savings:** ~100 LOC.

---

## Finding 10 — GPU buffer churn pattern duplicated between particle and damage-digit systems

**Locations:** `particle_sequence_system.rs:513-647`, `damage_digit_render_system.rs:17-209`.

**Evidence:** Both systems every frame: create 3-4 new `ShaderStorageBuffer` assets, assign to material, then `storage_buffers.remove(&old_*)` (particle: `:573-593`, digits: `:175-199`). This is a single alloc/GC cycle per frame per entity — identical pattern, two implementations. Also:
- `particle_sequence_system.rs:567-593`: `should_recreate_buffers = true; // For now, always update...` — a dead optimization branch.
- `particle_sequence_system.rs:531-561`: three `error!` validation checks run every frame for every particle entity (could be `debug_assert!`).
- `damage_digit_render_system.rs` has ~20 commented-out `log::info!` lines in the hot path.

**Suggestion:** Extract a shared `update_storage_buffer_slots(material, data, storage_buffers)` helper used by both; downgrade per-frame validation to debug assertions.
**Savings:** ~60 LOC.

---

## Finding 11 — `zone_time_system.rs` long function and table-driven opportunity

**Location:** `zone_time_system.rs:84-528` — `zone_time_system` is a single 445-line function.

**Evidence:** The four state branches (Night/Evening/Day/Morning) repeat the same shape: state detection → `set_visible_recursive` on change → volumetric fog color/density (with two sub-branches lerping `t<0.5`/`t>=0.5` for Evening and Morning) → skybox ambient color assignments (4 fields × 2 sub-branches). The lerp chains for `map_ambient_color`/`character_ambient_color`/`character_diffuse_color`/`fog_color`/`fog_density` are structurally identical across branches. A small `TimeOfDayProfile { start_color, end_color, ... }` + one interpolation helper would collapse ~150 duplicated lines. Also:
- 60+ lines of one-time zone-change logging (`:119-183`) using `static AtomicU32` — belongs in a `Changed<CurrentZone>` system.
- `should_log` is hardcoded `false` with a dead logging block (`:205-218, 237-256`).
- `set_visible_recursive` (53-72) + `SingleLerp` trait (74-82) are reasonable, keep.

**Savings:** ~180 LOC (plus `color_grading_time_of_day_system` already counted in Finding 3).

---

## Finding 12 — Name tag systems: separation is good, but hot-path logging and cross-file duplication

**Locations:** `name_tag_system.rs` (786), `name_tag_visibility_system.rs` (136), `name_tag_update_color_system.rs` (69), `name_tag_update_healthbar_system.rs` (27).

**Evidence:** The 4-way split (spawn / visibility / color / healthbar) is sensible and the three small systems are clean. Problems are inside `name_tag_system.rs`:
- Per-name-tag `info!` logging in the spawn hot path: `:227-234`, `:335-338`, `:395-409`, `:707-717` (logs every rect of every name tag with `[NAME_TAG_DIAG]`/`[NAME_TAG_DEBUG]` tags). ~30 lines of logging that should be `debug!`/removed.
- The egui-atlas glyph-copy block (`:274-321`) deliberately mirrors `chat_bubble_spawn_system.rs` behavior ("matching chat_bubble behavior", `:304`) — a shared font-atlas-to-image helper would serve both (chat_bubble_system not in this analysis scope, but the duplication exists).
- `create_nametag_data` is 236 lines (`:195-430`) mixing texture generation, alpha diagnostics, and rect building.

**Suggestion:** Remove/reduce the `info!` diagnostics; consider extracting the glyph-copy into a shared `texture_util` used by both name tags and chat bubbles.
**Savings:** ~40 LOC here (more if chat_bubble shares the util).

---

## Finding 13 — Misc small items

- `passive_recovery_system.rs` (20 lines): only ticks a timer; comment says server applies regen. Verify it is actually registered in `lib.rs`; if not, delete (likely dead — worth checking before removal).
- `debug_inspector_system.rs` (140): fine, but ~60 `register_type` calls are inspector support; keep.
- `sail_camera_system.rs` (44), `sail_animation_system.rs` (121), `sailing_movement_system.rs` (112), `remote_boat_system.rs` (123), `boat_wake_system.rs` (282): reasonably sized; the boat spawn visual (`spawn_boat_visual`, `boat_spawn_system.rs:514-806`, 292 lines) is long but mostly literal part data — could be data-driven but low priority.
- `model_viewer_system.rs` (357): `model_viewer_system` is a 228-line egui function (130-357); dev-only tool, low priority.
- `spawn_effect_system.rs` (168): four `SpawnEffectEvent` match arms repeat the 10-argument `spawn_effect(...)` call — a small closure would trim ~30 lines.

---

## Long functions (file:line ranges)

| Function | Range | Lines |
|---|---|---|
| `zone_time_system` | zone_time_system.rs:84-528 | 445 |
| `animation_effect_system` | animation_effect_system.rs:42-590 | 549 |
| `animation_sound_system` | animation_sound_system.rs:82-565 | 484 |
| `spawn_boat_visual` | boat_spawn_system.rs:514-806 | 292 |
| `create_angel_wing_mesh` | wing_spawn_system.rs:220-499 | 280 |
| `render_diagnostics_system` | debug_rendering_system.rs:106-356 | 251 |
| `create_nametag_data` | name_tag_system.rs:195-430 | 236 |
| `blood_spatter_spawn_system` | blood_spatter_system.rs:142-374 | 233 |
| `model_viewer_system` | model_viewer_system.rs:130-357 | 228 |
| `spawn_birds` | bird_system.rs:116-289 | 174 |
| `particle_sequence_system` | particle_sequence_system.rs:326-502 | 177 |
| `boat_toggle_system` | boat_spawn_system.rs:141-315 | 175 |
| `zone_render_validation_system` | zone_render_validation_system.rs:83-246 | 164 |

---

## Dead code / unused summary (by file)

| Item | Location | Status |
|---|---|---|
| Entire `debug_rendering_system.rs` | systems/debug_rendering_system.rs | Never registered |
| 3 zone diagnostic plugins | zone_render_validation/zone_memory_profiler/zone_memory_protection | Never registered |
| `transform_propagation_diagnostics` + `post_update_systems_diagnostics` | transform_propagation_diagnostics.rs | Never registered |
| `debug_camera_render_state_system`, `calculate_look_direction` | zone_viewer_system.rs:66-115 | Not exported |
| `color_grading_time_of_day_system` + its consts | zone_time_system.rs:530-626 | Disabled in lib.rs |
| `cleanup_wind_particles_on_flight_end` | wind_effect_system.rs:295-310 | Not exported |
| `spawn_wings`, `create_angel_wing_mesh`, `wing_animation_system` | wing_spawn_system.rs:108-544 | Never called (feature disabled) |
| `summer_vegetation_system`, `vegetation_sway_system` (deprecated) | season/summer_system.rs | Not registered |
| ~160 lines commented grass code | season/summer_system.rs:349-508 | Comments |
| `spawn_flower_system`, `fall_particle_spawn_system` | season/spring_system.rs:133, season/fall_system.rs:7 | `#[allow(dead_code)]` |
| `spawn_season_particles` stub | season/season_manager.rs:21-38 | `#[allow(dead_code)]` |
| `blood_overlay_update_system` (no-op) | blood_overlay_system.rs:239-247 | Registered, does nothing |
| Legacy `BloodOverlayTexture` | blood_overlay_system.rs:231-236 | Unused |
| `should_recreate_buffers = true` branch | particle_sequence_system.rs:567-593 | Dead branch |
| Unsafe `static mut FRAME_COUNTER` ×6 | zone_render_validation_system.rs | Unsafe for no reason |
| Unused imports (`Face`, `RenderApp`, `Extract`, etc.) | wing_spawn_system.rs, debug_rendering_system.rs, bird_system.rs, fish_system.rs | Various |

---

## Prioritized summary — top 5 quick wins first

1. **Delete `debug_rendering_system.rs`** (never registered) — **~1,720 LOC**. Also remove `mod.rs:149-158` exports. No functional risk.
2. **Delete the three zone diagnostic files** (plugins never registered) — **~1,220 LOC**. Removes unsafe `static mut` counters too.
3. **Delete remaining unregistered diagnostics** (`transform_propagation_diagnostics.rs`, `debug_camera_render_state_system`, `color_grading_time_of_day_system`, `cleanup_wind_particles_on_flight_end`) — **~260 LOC**.
4. **Remove disabled wing-spawn implementation** (or re-enable it — pick one) — **~450-550 LOC**.
5. **Merge the three season weather systems** into one parameterized system + delete deprecated/dead season code — **~300 LOC (merge) + ~400 LOC (dead removal)**.

Next tier: dedupe `pending_damage`/`hit_event` shared helpers (~120), shared bullet-effect resolution between animation_effect/animation_sound (~80), bird wing mesh mirror (~65), `zone_time` table-driven refactor (~180), buffer-update helper for particle/damage-digit systems (~60), blood system no-op/diagnostic cleanup (~100).

**Total estimated savings: ~4,500-5,500 LOC** out of ~15,000 analyzed (30-37%).

---

**NOTE:** This document is a PLAN ONLY. No code changes were made during this research pass. All "dead code" claims were verified by crate-wide grep: the flagged systems appear only in their own file and in `systems/mod.rs` re-exports, never in `lib.rs` registration, except where noted.
