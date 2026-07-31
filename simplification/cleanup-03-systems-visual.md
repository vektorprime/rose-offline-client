# Cleanup Report — `03-systems-visual` (VISUAL/EFFECT & WORLD half)

**Branch:** `code-simplification`
**Date:** 2026-07-31
**Scope:** src/systems/ visual/effect/world subsystems per `simplification/03-systems-visual.md`
**Result:** **~5,450 net LOC removed** (deleted 4,565 + trimmed 1,350, minus 467 LOC of new shared modules).

---

## Finding 1 — `debug_rendering_system.rs` deleted (DONE, −1,727 LOC)

- Verified: all 20 functions referenced only from own file + `mod.rs` re-exports; never registered in `lib.rs`.
- Deleted `src/systems/debug_rendering_system.rs` (1,727 lines) and the `mod.rs` `pub use debug_rendering_system::{...}` block + `mod` declaration.
- `RenderExtractionDiagnostics` untouched: it lives in `src/resources/debug_render.rs` (not this file); `lib.rs:997/2508` references intact.

## Finding 2 — three zone-diagnostic plugins deleted (DONE, −1,219 LOC)

- Verified: `ZoneRenderValidationPlugin`, `ZoneMemoryProfilerPlugin`, `ZoneMemoryProtectionPlugin` appear only in their own files; never added in `lib.rs`.
- Deleted `zone_render_validation_system.rs` (637), `zone_memory_profiler_system.rs` (424), `zone_memory_protection_system.rs` (158) + the four `pub mod` lines in `mod.rs`.
- Also removed the unsafe `static mut FRAME_COUNTER` instances with them (5+ systems).
- `src/resources/zone_debug_diagnostics.rs` (430 LOC) deleted — its only consumers were the two deleted zone systems. Removed the dangling `pub mod zone_debug_diagnostics;` from `src/resources/mod.rs`.

## Finding 3 — remaining unregistered diagnostics (DONE, −260 LOC)

- Deleted `transform_propagation_diagnostics.rs` (132) + `mod.rs` exports.
- `zone_viewer_system.rs`: deleted `debug_camera_render_state_system` + `calculate_look_direction` and ~20 lines of commented-out `[CAMERA FIX]`/`[CAMERA]` logs; file 115 → 35 lines.
- `zone_time_system.rs`: deleted `color_grading_time_of_day_system` (97 lines) + 14 color-grading consts (already disabled in `lib.rs:269-270/1327-1330` — no lib.rs edit made, per instructions). Also dropped the now-unused `ColorGrading*` import.
- `wind_effect_system.rs`: deleted unexported `cleanup_wind_particles_on_flight_end` (16 lines).

## Finding 4 — wing-spawn dead implementation removed (DONE, −526 LOC)

- `WingSpawnPlugin` kept + still registered (`lib.rs:1110`); `WingMaterialPlugin` still added.
- Deleted `spawn_wings`, `create_angel_wing_mesh` (280-line feather mesh), `wing_animation_system`, the trivial `test_wing_side_mirror` test, and the commented re-enable block.
- `wing_spawn_system` reduced to event → minimal log line. File 564 → 38 lines.

## Finding 5 — season weather systems merged (DONE, ~−770 LOC net)

- New `season/weather_system.rs`: one parameterized `weather_particle_system` replacing `fall_particle_system`, `winter_snow_system`, `spring_rain_system`.
  - Shared structural code: player-relative circle spawn, spawn-rate loop, despawn-on-lifetime/ground, billboard look-at block (identical across all three originals).
  - Per-season differences preserved exactly via `particle_spawn()` (per-particle random size/velocity/lifetime/rotation/wobble, mesh/material per season) and `update_particle_movement()` (Straight/Swirl/Wobble math copied verbatim from each original).
  - The spring flower-despawn block was removed: `SpringFlower` entities could only be created by the deleted `spawn_flower_system`, so the query is always empty.
- Deleted `fall_system.rs` (219), `winter_system.rs` (146), `spring_system.rs` (184), `summer_system.rs` (508).
- Deleted dead/deprecated items: `fall_particle_spawn_system`, `spawn_flower_system`, `summer_vegetation_system` + `vegetation_sway_system` (deprecated CPU grass), ~160 lines of commented `bevy_procedural_grass` code, `spawn_season_particles` stub, unused `SeasonSystemSet`.
- `SeasonPlugin` registration unchanged in effect: `season_cleanup_system` + weather system.

## Finding 6 — `pending_damage`/`hit_event` shared helpers (DONE, ~−155 LOC net)

- New `src/systems/damage_effects.rs` with shared `normalize_or`, `random_local_wound_pose`, `spawn_damage_digits`, `emit_blood_and_wounds`.
- `pending_damage_system.rs` 198 → 105: inlined `apply_damage` (dropped dead `pending_damage_list` param and `let _ =` suppression, dropped `total_entities_processed`/`total_damage_applied` counters and ~15 commented logs, removed dead `query_transform` alias).
- `hit_event_system.rs` 228 → 163: shared digit/blood code; removed dead `HitAttackerQuery` (never used anywhere in crate).
- Behavior preserved: the kill-path difference (hit_event only removes `ClientEntity` when `entity_type != Character`; pending always removes) kept inline in each system.

## Finding 7 — bullet-effect resolution dedupe (DONE, −169 LOC net)

- New `src/systems/effect_resolution.rs`: `resolve_vehicle_arms_bullet_effect_id`, `resolve_weapon_bullet_effect_id` (weapon→ammo class→ammo item→bullet_effect_id with weapon fallback), `resolve_weapon_hit_effect_id` (weapon effect_id + NPC hand fallback), `weapon_blood_profile` + `weapon_to_blood_profile`.
- `animation_effect_system.rs` 609 → 468: uses the helpers (duplicate chains at 123-139/144-177 and 411-427 removed); merged two identical `EFFECT_SKILL_ACTION` match arms (BasicAction/CreateWindow/Immediate ∪ SelfBound*); per-event `log::info!` spawn spam downgraded to `log::debug!` (kept `warn!` for genuinely missing data).
- `animation_sound_system.rs` 565 → 537: uses the helpers for the `SOUND_WEAPON_FIRE_BULLET` chain; dropped now-unused `AmmoIndex`/`ItemClass` imports (re-added `EquipmentIndex` — still used by two other chains).
- Behavior preserved exactly: call sites keep the `Option<&Equipment>` `.and_then(...)` wrapping (no unwraps added).

## Finding 8 — bird wing mesh mirror (DONE, −47 LOC)

- `create_bird_wing_left_mesh`/`create_bird_wing_right_mesh` merged into `create_bird_wing_mesh(meshes, right_side)`.
- Right wing generated by exact X mirror of the left: negate position/normal X, flip UV X (1−u), reverse per-triangle winding (verified index-by-index against the original right-wing table).

## Finding 9 — blood trio cleanup (DONE, ~−52 LOC)

- Deleted no-op `blood_overlay_update_system` + its `PostUpdate` registration + `mod.rs` export.
- Deleted unused legacy `BloodOverlayTexture` component.
- `blood_overlay_force_enable_system`: env-var check cached in `Local<bool>` on first run instead of per-frame `std::env::var` scan (DEBUG_FORCE_BLOOD still works when set before launch).
- Shared `normalize_or` now used by `blood_spatter_system.rs` and `gash_wound_system.rs` (removed 2 more local copies; crate-wide total was 4 copies, now 1).
- Left the pool/fresh-spawn branch unification in `blood_spatter_spawn_system` untouched (higher risk, moderate gain).

## Finding 12 — name tag hot-path log reduction (DONE, −82 LOC)

- `name_tag_system.rs` 786 → 704: removed per-name-tag `[NAME_TAG_DIAG]` info! logs and the atlas/row alpha scan loops that existed only to feed them (`atlas_nonzero`, `row_nonzero_alpha`, `total_glyphs_copied`), removed ~10 commented `[NAME_TAG_DEBUG]` lines, removed now-unused `debug_entity` parameter from `create_nametag_data`, downgraded the per-rect spawn log to `log::debug!`.
- `total_nonzero_alpha == 0 → return None` not-ready logic preserved (still computed).

## Finding 10 — partial (DONE, −29 LOC in `damage_digit_render_system.rs`)

- Removed ~20 commented-out `log::info!` lines in the hot path and the dead `entity_count` counters.
- SKIPPED the shared `update_storage_buffer_slots` helper and the `error!`→`debug_assert` downgrade in `particle_sequence_system.rs`: both touch per-frame GPU buffer paths with no way to test at runtime; risk outweighs ~60 LOC gain.

## Finding 11 — partial

- Removed the hardcoded-`false` `should_log` dead blocks and the dead `FRAME_COUNTER` static in `zone_time_system.rs` (part of the −139).
- SKIPPED the full table-driven `TimeOfDayProfile` refactor of the 445-line `zone_time_system`: it is the lighting backbone; a behavioral-mismatch risk is not worth ~180 LOC. Also note: moving the one-time zone-change log block into a `Changed<CurrentZone>` system would require registering a new system in `lib.rs`, which is off-limits — so it stays inline.
- `zone_time_system.rs` 626 → 487 (also includes Finding 3's color-grading removal).

## Finding 13 — checks

- `passive_recovery_system.rs`: verified REGISTERED at `lib.rs:1824` — kept as instructed (no changes).
- `spawn_effect_system.rs` closure dedupe: SKIPPED — file owned by another agent.
- Boat/sailing files: NOT touched (boat_wake merge, spawn_boat_visual, wake/spray loops, visual-part-bundle all skipped per scope rules).

## Audio coordination

- `spawn_spatial_sound` does not exist yet in `src/audio/mod.rs` — the optional migration of `animation_sound_system.rs:48-80` was skipped (audio files untouched).

---

## LOC accounting (net −5,448)

| File | Before | After | Δ |
|---|---|---|---|
| debug_rendering_system.rs | 1,727 | deleted | −1,727 |
| zone_render_validation_system.rs | 637 | deleted | −637 |
| zone_memory_profiler_system.rs | 424 | deleted | −424 |
| zone_memory_protection_system.rs | 158 | deleted | −158 |
| transform_propagation_diagnostics.rs | 132 | deleted | −132 |
| resources/zone_debug_diagnostics.rs | 430 | deleted | −430 |
| season/fall_system.rs | 219 | deleted | −219 |
| season/spring_system.rs | 184 | deleted | −184 |
| season/summer_system.rs | 508 | deleted | −508 |
| season/winter_system.rs | 146 | deleted | −146 |
| wing_spawn_system.rs | 564 | 38 | −526 |
| animation_effect_system.rs | 609 | 468 | −141 |
| zone_time_system.rs | 626 | 487 | −139 |
| pending_damage_system.rs | 198 | 105 | −93 |
| name_tag_system.rs | 786 | 704 | −82 |
| zone_viewer_system.rs | 115 | 35 | −80 |
| hit_event_system.rs | 228 | 163 | −65 |
| bird_system.rs | 677 | 630 | −47 |
| animation_sound_system.rs | 565 | 537 | −28 |
| damage_digit_render_system.rs | 209 | 180 | −29 |
| systems/mod.rs | 251 | 230 | −21 |
| wind_effect_system.rs | 310 | 292 | −18 |
| blood_overlay_system.rs | 414 | 400 | −14 |
| blood_spatter_system.rs | 575 | 567 | −8 |
| gash_wound_system.rs | 413 | 407 | −6 |
| resources/mod.rs | 88 | 85 | −3 |
| season/mod.rs | 57 | 27 | −30 |
| season/season_manager.rs | 38 | 18 | −20 |
| season/weather_system.rs (new) | — | 290 | +290 |
| damage_effects.rs (new) | — | 88 | +88 |
| effect_resolution.rs (new) | — | 89 | +89 |
| **Total** | | | **−5,448** |

## cargo check result

Ran `cargo check` after all changes. All errors introduced by my edits were fixed:
- `animation_sound_system.rs` `EquipmentIndex` (2×) — re-added import (still used by two other chains).
- `systems/mod.rs` stale `pub use move_speed_command_system::move_speed_command_system` (system no longer exists in that file — pre-existing breakage, now fixed).

**Current state:** 41 remaining errors, ALL in files owned by other agents / out of scope: `lib.rs` (missing `diagnostics` module at lib.rs:73 — pre-existing, dangling before my work; render import drift at lib.rs:117), `scripting/` (lua macro errors), `ui/` (ui_admin_menu, ui_minimap, widgets), `zone_loader/` (systems.rs, spawning.rs), `audio/streaming_sound.rs`, `bundles/ability_values.rs`, `systems/game_connection_system.rs`. None are in the files listed in my scope. Warnings in my files are pre-existing (e.g., unused `mut` on `damage_digit_render_system` queries, `bird_system` `base_y`).

## Skipped items (with reasons)

| Item | Reason |
|---|---|
| Shared `update_storage_buffer_slots` helper (Finding 10) | Per-frame GPU buffer path, no test vehicle; risk > ~60 LOC |
| `particle_sequence_system.rs` per-frame `error!` → `debug_assert` | Runtime validation is the only guard for that code; would change release behavior |
| Table-driven `zone_time` refactor (Finding 11) | Lighting backbone; refactor risk not worth ~180 LOC |
| `Changed<CurrentZone>` logging system (Finding 11) | Requires lib.rs registration (off-limits) |
| Glyph-copy shared helper (Finding 12) | chat_bubble_spawn_system.rs owned by another agent |
| Boat: wake/spray merge, `spawn_boat_visual` data-ification, visual-part-bundle helper (Finding 8) | Explicitly out of scope — boat/sailing files must stay 100% unchanged |
| `spawn_effect_system.rs` closure (Finding 13) | Owned by another agent |
| Audio `spawn_spatial_sound` migration | Function not yet created by the audio agent |
| Blood spatter pool/fresh-spawn branch unification (Finding 9) | Moderate risk in active combat rendering path; marginal gain |
