# Optimization Review 11 — Cross-Cutting Concerns

## 1. Title & Scope

**Scope:** Everything that cuts across the gameplay/systems/render layers: ECS schedule construction and system ordering (`src/lib.rs`, `src/systems/mod.rs`), per-frame allocation hotspots in shared systems (name tags, chat bubbles, world-UI, diagnostics, map editor), the structured-logging infrastructure (`src/logging/`), debug/diagnostics systems (memory diagnostics, debug inspector, debug UI, debug render scaffolding), the map editor subsystem wiring, and the CLI/mode dispatch (`src/main.rs`).

**Files reviewed (all verified against source):**

| Area | Files |
|---|---|
| Schedule / registration | `src/lib.rs` (2297 lines, full), `src/systems/mod.rs` (233), `src/main.rs` (315) |
| Logging | `src/logging/mod.rs` (232), `src/logging/json_format.rs` (full) |
| Diagnostics / debug | `src/systems/memory_diagnostics.rs` (141), `src/systems/debug_inspector_system.rs` (140), `src/resources/debug_render.rs`, `src/resources/debug_inspector.rs`, `src/ui/ui_debug_render_system.rs`, `src/ui/ui_debug_client_entity_list_system.rs`, `src/systems/directional_light_system.rs` (49) |
| Shared per-frame systems | `src/systems/name_tag_system.rs` (704), `src/systems/name_tag_visibility_system.rs`, `src/systems/chat_bubble_update_system.rs`, `src/systems/zone_time_system.rs` (499), `src/ui/ui_minimap_system.rs` (949, partial), `src/resources/ui_resources.rs` (`update_ui_resources` at 317), `src/render/world_ui.rs` (grep) |
| Map editor | `src/map_editor/mod.rs` (166), `components.rs`, `coords.rs`, `resources.rs` (528), `systems/` (selection, transform_gizmo, grid, property_update, load_models, keyboard_shortcuts, selection_highlight, model_placement, duplicate), `ui/` (hierarchy, model_browser, properties, menu_bar, zone_list_panel), `save/save_system.rs` |
| Network glue | `src/systems/network_thread_system.rs` (136) |

**Architecture docs that existed** (checked in `system-architecture/`): `ECS.md`, `map-editor-architecture.md`, `chat-bubble-and-name-tag-architecture.md`, `README.md`, plus feature docs (Assets, Animation, Audio, Camera, Lighting, Input, Physics, Render, Transform, UI, Window, weather-season-system, flying-system-architecture, blood-effect-system, planar-water-reflection, sky_stars_architecture, SUN_DOCUMENTATION, zone_lighting, admin-menu-skill-learn-feature).

**Gap:** there is **no** architecture doc describing `src/lib.rs`'s schedule: the `GameStages` / `GameSystemSets` / `UiSystemSets` / `ModelSystemSets` / `EffectSystemSets` / `UiSystemOrdering` sets (lib.rs:661–714), the PostUpdate barrier chain (lib.rs:1705–1743), or the debug/diagnostics systems. `ECS.md` documents Bevy 0.18.1 APIs but not this app's actual set graph. A `schedule.md` doc (or comments at each `configure_sets` call site) would help future contributors understand why ordering edges exist before they add new ones.

## 2. Methodology

- Read every source file in scope (lib.rs, main.rs, logging, systems/mod.rs fully; hot subsystems at least 100+ lines deep).
- Read `system-architecture/ECS.md` (Bevy 0.18.1 ECS conventions), `system-architecture/map-editor-architecture.md`, `pitfalls/index.md`, `pitfalls/performance-memory.md`.
- Verified registration sites and ordering edges against `src/lib.rs` (all `add_systems`/`configure_sets` calls), and the CLI dispatch in `src/main.rs`.
- Grepped for `Vec::new`/`Vec::with_capacity`, `format!`, `log::*!`, `.chain()`, `run_if`, `in_state` across `src/` to triage per-frame allocation and logging hotspots.
- Perf analysis is static: hot loops, per-frame allocations, per-frame resource churn, redundant sync points, and log spam identified from source. No profiling data was available; severity estimates are relative and assume a ~1000-entity game world at 60 fps.
- Bevy 0.18.1 source at `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1` used to confirm `ApplyDeferred`/`chain()`/`run_if` semantics where relevant.

## 3. Findings

### A. ECS schedule construction (`src/lib.rs`)

**XC-01 — Monolithic 2297-line `run_client` with ad-hoc per-system ordering instead of sets.**
- **Location:** `src/lib.rs:716-1763` (whole function); e.g. the sailing block `:1430-1551` is 11 separate `add_systems(Update, system.run_if(in_state(AppState::Game)).after(...))` calls, and "Game systems - part 1" `:1391-1428` mixes set membership with ~15 individual `.after()/.before()` edges.
- **Description:** Ordering knowledge is scattered across dozens of inline chains; system sets exist (`GameStages`, `GameSystemSets`, `ModelSystemSets`, `EffectSystemSets`, `UiSystemOrdering`, `UiSystemSets`) but cover only a fraction of the ~150 registered systems.
- **Why it matters:** (a) Maintainability — every new ordering edge must be re-derived from comments; the `GameStages` enum has 5 stages but `DebugRender` is empty (XC-03) and `ZoneChange`/`ZoneChangeFlush`/`AfterUpdate` are only partly populated; (b) scheduler — each inline `.after()`/`.before()` is an additional schedule graph edge; dozens of them measurably increase `Schedule::run` bookkeeping per frame (Bevy evaluates the dependency graph each schedule run).
- **Fix sketch:** Group systems into `configure_sets(Update, (GameSets::Simulation, GameSets::Sail, GameSets::Combat, ...).chain())` and register tuples into those sets once; keep individual `.after()` only for genuinely system-specific edges (e.g. `collision_player_system_join_zone.before(collision_player_system)` lib.rs:1405).

**XC-02 — Two `ApplyDeferred` sync points in `PostUpdate`, one at the very end of the visibility chain.**
- **Location:** `src/lib.rs:1002` (`app.add_systems(PostUpdate, ApplyDeferred);`) and `:1004-1007` (`(ApplyDeferred,).in_set(GameStages::DebugRenderPreFlush)`).
- **Description:** PostUpdate is split into two batches by the two apply points; the second batch (character_model_blink_system:1204, vehicle_model_system:1208, force_zone_visibility_system:1230, model_viewer_system:1266, graphics::apply_*:1377-1388, network_thread_system:1682) additionally only applies after `VisibilitySystems::CheckVisibility` (lib.rs:1742).
- **Why it matters:** Each apply point is a hard barrier: every PostUpdate system in the first batch must finish before any system in the second batch starts, even if they touch disjoint data. The second barrier sits after the visibility pass — i.e. on the render-prep critical path — and applies commands that mostly belong to the earlier batch anyway (the `DebugRenderPreFlush` set contains only `ApplyDeferred`, nothing else).
- **Fix sketch:** Merge into a single apply point: keep `PostUpdate`'s default deferred-application (Bevy applies deferred commands at the end of the schedule automatically) and remove the explicit `ApplyDeferred` at :1002 **and** :1006 unless a command-apply is genuinely required before `CheckVisibility`. If the second apply must exist, put it in `PreUpdate` instead (systems there already run before Update's read of those entities).

**XC-03 — Empty `GameStages::DebugRender` set + `chain()` that only links an empty set.**
- **Location:** `src/lib.rs:666-667` (enum members), `:1720-1723` (`(GameStages::DebugRenderPreFlush, GameStages::DebugRender).chain()`).
- **Description:** No system is ever registered into `GameStages::DebugRender`; the `.chain()` therefore only orders `ApplyDeferred` (via `DebugRenderPreFlush`) before nothing. The `DebugRenderConfig` resource (src/resources/debug_render.rs:28-47) defaults **all four** toggles (`colliders`, `skeleton`, `bone_up`, `directional_light_frustum`) to `true`, but the only consumer is `ui_debug_render_system`'s egui checkboxes — the actual rapier debug-render plugin is commented out (lib.rs:825).
- **Why it matters:** The scaffolding is misleading (default-true toggles suggest debug rendering is active when it isn't; the `DebugRender` stage suggests systems exist when they don't). Cost is ~zero per frame, but any future contributor will either trust the toggles (and be confused when nothing shows) or register systems into an empty stage and silently re-introduce the double-apply pattern from XC-02.
- **Fix sketch:** Delete `GameStages::DebugRender` and the `.chain()`; keep `DebugRenderPreFlush` only if the second apply point is kept (then name it `PostVisibilityCommands`). Change `DebugRenderConfig::default()` to `false` toggles, or remove the resource until a real consumer exists.

**XC-04 — `directional_light_system` is dead code that still runs every frame and computes matrix inverses.**
- **Location:** `src/systems/directional_light_system.rs:11-48` (whole system), registered at `src/lib.rs:1143` (no run condition).
- **Description:** The system computes `Mat4::look_at_rh`, `Mat4::orthographic_rh`, `light_transform.to_matrix()`, and a `Mat4` inverse (line 43) every frame, then discards everything: `let _ = (shadow_map, view_transform, view_projection, views);` (line 47). The comment says Bevy 0.13+ builds shadow cascades automatically and manual cascade management is no longer supported.
- **Why it matters:** Pure wasted work per frame — two `Mat4` constructions, one full matrix inverse, one matrix multiply — for zero output. It also serializes via `Query::single()` on the light (a cheap but pointless lookup).
- **Fix sketch:** Delete the system and its registration. If the shadow follow-camera math is ever needed again, restore it then; git history preserves it.

**XC-05 — `memory_diagnostics_system` runs every frame but only logs every 30 s.**
- **Location:** `src/systems/memory_diagnostics.rs:80-140`, registered unconditionally at `src/lib.rs:1082`.
- **Description:** The system's time gate (30 s `elapsed` check) only guards the `log::info!` call — the per-frame body first runs **seven+ queries** over every zone, bird, client entity, zone object, fish, name tag, and chat bubble, plus counts of ~13 asset collections, plus VFS cache sizes, every frame.
- **Why it matters:** O(entities) iterator cost per frame (each `Query::iter().count()` is a full archetype walk) purely to decide whether to log. The system is explicitly a temporary leak-detection tool ("remove once leak confirmed fixed" — see pitfalls/performance-memory.md, GPU storage buffer leak fixed 2026-03-03).
- **Fix sketch:** Give the system a `Local<Timer>` run condition (e.g. `run_if(|mut t: Local<Timer>| t.tick(delta).just_finished())` with `Timer::from_seconds(30.0, repeating)`), or move it into a dedicated `FixedUpdate`-style schedule that runs at 1/30 Hz. Better: remove it entirely now that the leak it was built to detect is fixed.

**XC-06 — ~10 debug UI systems registered unconditionally in `EguiPrimaryContextPass`.**
- **Location:** `src/lib.rs:1169-1199` (ui_debug_menu, camera_info, client_entity_list, command_viewer, dialog_list, effect_list, entity_inspector, item_list, npc_list, render, skill_list, zone_lighting, zone_list, zone_time).
- **Description:** Each runs every frame during the egui pass; most early-return after checking `UiStateDebugWindows.debug_ui_open` / `debug_inspector_state.enable_picking` (e.g. src/ui/ui_debug_render_system.rs:28-31).
- **Why it matters:** Cost is low (early-return + a few resource reads each), but it keeps `EguiContexts` and large query parameters (entity inspector, npc/item lists hold 8+-param queries) alive in the schedule in *all* modes including production Game sessions, and the egui pass is already the single-threaded hot spot.
- **Fix sketch:** Register the debug group with `.run_if(resource_exists_and_equals::<UiStateDebugWindows>(UiStateDebugWindows { debug_ui_open: true, .. }))` — or simpler: a `Local<bool>` toggled by `ui_debug_menu_system` once per frame that the others read. Also consider `debug_inspector_system`'s picking (src/systems/debug_inspector_system.rs:93-139) staying gated on `KeyP` (it is — fine).

**XC-07 — `apply_*` settings systems: good `is_changed()` gating in gameplay ones, unverified/absent in the graphics ones.**
- **Location:** gated: `apply_depth_of_field_settings` (src/lib.rs:2087 `dof_settings.is_changed()`), `apply_post_processing_settings` (:2121), `apply_water_settings` (:2174); ungated-but-cheap: the `graphics::apply_*` block (lib.rs:1377-1388, 7 systems, each iterating cameras/materials).
- **Description:** The three settings systems early-return when their resource is unchanged — the right pattern. The `graphics::apply_*` systems run every frame in PostUpdate; whether they gate internally on `GraphicsSettings::is_changed()` was not verified from source in this pass (they live in `src/graphics/`).
- **Why it matters:** If any of them unconditionally iterates all cameras/materials, that's per-frame work that should be gated; the `is_changed()` pattern already established in the same file should be applied uniformly.
- **Fix sketch:** Verify `src/graphics/*` apply systems use `DetectChanges::is_changed()`; if not, add the same early-return. This is a cheap audit with potentially medium payoff on the PostUpdate path.

### B. Per-frame allocations

**XC-08 — `zone_list_panel` clones the entire filtered zone list every frame.**
- **Location:** `src/map_editor/ui/zone_list_panel.rs:112` (`let filtered_zones = state.filtered_zones.clone();`).
- **Description:** The zone list can hold hundreds of `ZoneId`s; while the "Open Zone" panel is open (it is opened by default on entering the map editor — src/map_editor/mod.rs:140), the whole `Vec<ZoneId>` is cloned every frame solely so the table builder can borrow it (the closure borrows it; the clone exists only to dodge the `state` borrow).
- **Why it matters:** One alloc + memcpy of N×4 bytes per frame, N = number of zones (100s). Trivial individually, but it is the *poster child* of the per-frame egui clone pattern in this codebase (also `format!` at :103, :135, :182 run per frame for the same panel).
- **Fix sketch:** `let filtered_zones = &state.filtered_zones;` inside the closure (egui's `TableBuilder::body` closure can borrow from the outer scope; avoid `state` mutation inside the body) or move the `filtered_zones` Vec into a `Local` that is only rebuilt on `filter_dirty` (which `update_filtered_zones` already does).

**XC-09 — World-UI render extraction pre-allocates a 1024-slot rect buffer every frame.**
- **Location:** `src/render/world_ui.rs:134` (`rects: Vec::with_capacity(1024)`), plus per-frame bind-group recreation noted at `world_ui.rs:538`.
- **Description:** Every frame the extractor allocates a fresh 1024-capacity `Vec<WorldUiRect>` even when the scene has a handful of name tags/chat bubbles/damage digits; the same holds for the per-view bind-group rebuild.
- **Why it matters:** ~1024 × ~48 bytes ≈ 48 KB zero-initialized allocation per frame (allocation + page fault churn), even in an empty scene. This is the single largest fixed per-frame heap allocation in the render path.
- **Fix sketch:** Use a reusable `Local<Vec<WorldUiRect>>` (`clear()` + `extend()` instead of new allocation), or size the capacity from `query.iter().len()` after a cheap first pass; cache the bind group and recreate only when the texture set/handle set changes (the extraction already writes `rects` incrementally — the Vec is the only fresh-allocation part).

**XC-10 — Damage-digit / particle systems still create 3–4 new `ShaderStorageBuffer`s per frame.**
- **Location:** `src/systems/damage_digit_render_system.rs:27-30` (3 buffers per frame), `src/systems/particle_sequence_system.rs` (4 buffers per particle per frame).
- **Description:** The *leak* (old handles never removed) was fixed on 2026-03-03 (pitfalls/performance-memory.md), but the churn remains: `Assets<ShaderStorageBuffer>::add()` every frame for every particle/digit batch.
- **Why it matters:** Each `add()` allocates a new GPU buffer + new handle entry; with many particles/digits this is a per-frame allocator hot path (buffer creation is deferred to the render thread, so the CPU cost is the asset-store churn + growable buffer re-alloc each frame).
- **Fix sketch:** Reuse two ping-pong buffer handles per effect type, `resize()`/`write_buffer` in place and only re-`add()` when capacity grows; or keep the per-frame `add` but reuse handles from a free-list. (Covered partly in review 03; listed here because it is cross-cutting infrastructure shared by multiple systems.)

**XC-11 — Map editor grid: gizmo lines re-emitted and colors re-converted every frame.**
- **Location:** `src/map_editor/systems/grid_system.rs` (`grid_render_system`, `num_lines = extent / cell_size + 1`, ~101 lines at defaults 100.0/1.0; `grid_color.to_srgba()` called ~3× per line inside the loop).
- **Description:** Every frame (while editor enabled) the system rebuilds the full grid: `to_srgba()` converts the same `Color` to `Srgba` up to ~300 times, and pushes ~200 gizmo line segments.
- **Why it matters:** The gizmo push is inherent (Bevy gizmos are per-frame), but the color conversion and loop overhead are pure waste; grid settings rarely change.
- **Fix sketch:** Hoist `let color = grid_color.to_srgba();` above the loop; early-return when `EditorGridSettings` is unchanged (gizmo buffers are cleared per frame anyway, so the visual result is identical — the fix only saves the conversion + re-emission bookkeeping when settings are dirty). Also guard with `show_grid`/`visible` (already partially done).

**XC-12 — Per-frame `format!`/`to_string()` in egui panels.**
- **Location:** `src/map_editor/ui/model_browser_panel.rs` (category tabs `format!("▶ {}", label)` and `label.to_string()` per frame), `src/map_editor/ui/zone_list_panel.rs:103/135/182`, `src/ui/ui_debug_client_entity_list_system.rs:76-194`, `src/ui/ui_minimap_system.rs` (zone-name galley — this one is *cached* correctly at :294-304).
- **Description:** Several panels rebuild small strings every frame inside the egui pass (labels, counts, ids). egui itself allocates per frame, so these are marginal on top of that — but they are trivially avoidable.
- **Why it matters:** Low severity; each is a small alloc per frame per panel. They add up on the single-threaded egui pass when several panels are open (map editor default state).
- **Fix sketch:** For static labels, use `ui.label("▶ Deco")` constants or `Label::new(static_str)`; for counts, `format!` only when the value changed (compare against a `Local` cache). Follow the minimap's galley-caching pattern (ui_minimap_system.rs:294-304) for anything expensive.

**XC-13 — `NameTagCache` is keyed only by name — cache efficiency is good, but per-new-name texture alloc is unbounded.**
- **Location:** `src/systems/name_tag_system.rs:55` (cache `HashMap<String, NameTagData>`), texture creation `:242-321` (`vec![0; w*h*4]` + `images.add`), cache flush on zone change/pixel-ratio change `:383-397`.
- **Description:** Positive pattern: galleys are laid out once per unique name and the CPU-side font-atlas copy happens only on cache miss; `pixels_per_point` and zone-change flush prevent stale textures. The cost is bounded by unique names alive (monsters with distinct names create one texture each; texture sizes are next-power-of-two of the galley bounds).
- **Why it matters:** This is a *good* pattern worth keeping (contrast with XC-09/X-10). The residual risk is texture memory growth in a zone with hundreds of unique NPC names; the cache has no eviction.
- **Fix sketch:** No change needed now; if memory ever becomes an issue, evict least-recently-used entries (e.g. cap the cache at 256 entries) since name tags are regenerated on demand.

### C. Logging infrastructure

**XC-14 — Default log level is `debug`, so every debug event is JSON-serialized to disk and formatted to console.**
- **Location:** `src/logging/mod.rs:98-99` (`LoggingConfig.level`, default `"debug"`), subscriber construction `:169-193` (JSON layer via `non_blocking` writer + console layer, both with the same `EnvFilter`).
- **Description:** In normal play (no `RUST_LOG` set) every `log::debug!`/`log::info!`/`log::warn!` is: (1) formatted by the tracing JSON layer with tag extraction (`TagExtractingJsonFormat`) and serde field building, written to `logs/<session>/structured.jsonl` through the non-blocking appender; (2) formatted a *second* time by the console layer. The codebase has 100+ `log::*!` call sites, several in per-message paths (see also review 03 GC-04).
- **Why it matters:** Double formatting per event plus file I/O at debug level, all the time, in release builds. The `EnvFilter` from Bevy's `LogPlugin` (lib.rs:792-796, `level: INFO` for the bevy side) does **not** gate the tracing layers — they have their own filter from `config.level` (default `debug`). So the structured.jsonl captures debug events even when Bevy's own logs are INFO.
- **Fix sketch:** Default `level` to `"info"` (or `"warn"` for release builds); gate debug events behind `RUST_LOG` explicitly. Optionally drop the console layer when not a debug build (console formatting is pure overhead in release).

**XC-15 — `TagExtractingJsonFormat` performs per-event tag extraction and JSON value construction.**
- **Location:** `src/logging/json_format.rs` (`extract_tag`, `remove_tag_prefix`, JSON Lines `{"ts","level","tag","msg","kvs"}`), applied to every event at `src/logging/mod.rs:173-178`.
- **Description:** Each event pays: prefix scan + tag string alloc + serde `Value` building + JSON serialization to a string, then hand-off to the non-blocking writer (which itself allocates per message).
- **Why it matters:** Structured logging is deliberately expensive (it's a feature for LLM queryability), but combined with XC-14 the steady-state cost per frame is nonzero even when nothing is logged — the layers are always installed.
- **Fix sketch:** Keep the design (it's the session-analysis feature), but enforce XC-14's level gate; if frame-time is ever an issue, `tracing_subscriber`'s `enabled()` callback can short-circuit the format entirely below the configured level (it already does when the filter excludes an event — the fix is purely the level default).

**XC-16 — Positive: hot-path logging is already mostly event-gated.**
- **Location:** `src/systems/zone_time_system.rs:119-123` (zone-change `AtomicU32` gate — logs once per zone), `src/systems/bird_system.rs` / `fish_system.rs` (spawn-time only), `src/systems/collision_system.rs:141-150,296-303` (warns only while `CurrentZone` missing — i.e. transient states).
- **Description:** The per-frame systems audited do **not** spam logs in steady state; the remaining offenders are the per-message info logs in `game_connection_system.rs` (already flagged as GC-04 in review 03) and `memory_diagnostics`'s 30-s summary (fine).
- **Why it matters:** Positive confirmation that the logging cost is dominated by infrastructure (XC-14/XC-15), not call sites.

### D. Map editor subsystem wiring

**XC-17 — `MapEditorPlugin` (and all its systems) is registered in every mode, including normal Game sessions.**
- **Location:** `src/lib.rs:957` (`map_editor::MapEditorPlugin` added unconditionally), map editor systems gated only by `map_editor_state.enabled` (e.g. selection_system, transform_gizmo_system.rs:245, selection_highlight_system, grid_system, keyboard_shortcuts_system, zone_list_panel_system.rs:232).
- **Description:** In a normal `run_game` session, ~10+ editor systems still execute every frame: each reads `MapEditorState`, `EditorGridSettings`, `EguiContexts`, holds wide queries (transforms, zone objects, cameras), and early-returns. `load_available_models_system` + `update_models_on_zone_load_system` (src/map_editor/mod.rs:109-115) run unconditionally in Update in all modes (they early-return on resource-exists, but the check runs per frame).
- **Why it matters:** Dead per-frame schedule entries in production play: each is a system call + parameter fetch + early-return; ~10 of them plus their egui-context access in the single-threaded egui pass (hierarchy/model_browser/properties panels are registered in `EguiPrimaryContextPass`).
- **Fix sketch:** Gate the whole plugin's Update/egui systems with `.run_if(in_state(AppState::MapEditor))` (the plugin's build fn receives `App`, so a `run_if(in_state(...))` on each registered group is straightforward), or `add_plugins` conditionally based on the `AppState` passed to `run_client`. Keep `OnEnter/OnExit(AppState::MapEditor)` handlers as-is.

**XC-18 — Editor picking raycasts + keyboard shortcuts scan on every frame while enabled.**
- **Location:** `src/map_editor/systems/selection_system.rs` (`editor_picking_system` — Rapier `cast_ray` on pointer activity), `src/map_editor/systems/keyboard_shortcuts_system.rs` (queries all `Transform`s/zone objects to find selected entities on key press).
- **Description:** While editing, selection raycasts against the full collider set and the shortcuts system iterates transforms; both are event-driven in effect (click / key press) but run every frame.
- **Why it matters:** Low severity (editor-only), but in large zones (thousands of colliders) an accidental raycast-per-frame cost can appear if the click gate is ever loosened; the current click-gating is correct.
- **Fix sketch:** No change required; note for future: gate `editor_picking_system` on `Window::cursor_position().is_some() && just_pressed(PrimaryButton)` to keep it strictly event-driven, and filter shortcuts queries with `Changed<Transform>` where possible.

**XC-19 — Undo history `Vec::remove(0)` on overflow.**
- **Location:** `src/map_editor/resources.rs:123-124` and `:144-145` (`if len > MAX_UNDO_HISTORY { undo_stack.remove(0); }`).
- **Description:** Pushing the 101st action shifts all 100 entries down once (O(100) memmove) — negligible at this scale.
- **Why it matters:** Non-issue today; flagged only because `VecDeque` is the natural structure and the fix is one line.
- **Fix sketch:** `VecDeque` or simply accept the shift (100 × `EditorAction` move is ~µs).

### E. CLI / mode dispatch (`src/main.rs`)

**XC-20 — Clean mode dispatch; game systems correctly state-gated per mode.**
- **Location:** `src/main.rs:274-314` (mode selection + `run_model_viewer`/`run_zone_viewer`/`run_map_editor`/`run_game` dispatch), `src/lib.rs:964` (`app.insert_state(app_state)`), `src/lib.rs:1427/1433-1551/1596/1615` (`run_if(in_state(AppState::Game))` on all gameplay systems).
- **Description:** Viewer/editor modes reuse the same `run_client` but gameplay systems are state-gated, so they don't pay for simulation. Session logging is initialized once with the mode name (main.rs:284-290) and the `LoggingGuard` is kept alive for the process (main.rs:286).
- **Why it matters:** Positive — the one thing to preserve when refactoring XC-01/XC-17: do **not** add gameplay systems to sets that run outside `AppState::Game`.
- **Fix sketch:** None.

## 4. Priority-Ranked Summary Table

| # | Finding | Impact | Effort | Priority |
|---|---|---|---|---|
| XC-14 | Default log level `debug` → JSON+console double formatting always on | Medium (CPU + disk, all sessions) | Trivial (one default value) | **High** |
| XC-05 | `memory_diagnostics_system` scans all entities every frame, logs every 30 s | Medium (O(entities)/frame) | Trivial (timer run condition / removal) | **High** |
| XC-04 | `directional_light_system` dead code with per-frame matrix inverse | Low–Medium (wasted math/frame) | Trivial (delete 38 lines) | **High** |
| XC-09 | World-UI extractor allocates 1024-slot Vec every frame | Low–Medium (fixed 48 KB alloc/frame) | Low (Local buffer reuse) | **Medium** |
| XC-17 | Map editor plugin + ~10 systems run in all modes | Low (dead schedule entries/frame) | Low (state-gate the plugin) | **Medium** |
| XC-02/03 | Double `ApplyDeferred` in PostUpdate + empty DebugRender set chain | Low–Medium (PostUpdate parallelism + clarity) | Medium (schedule refactor) | **Medium** |
| XC-07 | Graphics `apply_*` systems ungated (unverified) | Low–Medium (if ungated, per-frame camera/material loops) | Low (audit + `is_changed()`) | **Medium** |
| XC-08 | `zone_list_panel` clones filtered zone list per frame | Low (N×4 B alloc/frame) | Trivial (borrow instead of clone) | **Low** |
| XC-11 | Grid re-emits lines + converts color 3×/line every frame | Low (editor-only) | Trivial (hoist color, dirty-gate) | **Low** |
| XC-10 | Damage-digit/particle ShaderStorageBuffer churn per frame | Low (leak already fixed; churn remains) | Medium (handle reuse) | **Low** |
| XC-15 | JSON layer per-event tag extraction/serde | Low (below level gate once XC-14 done) | Low (level gate suffices) | **Low** |
| XC-06 | ~10 debug UI systems always in egui pass | Low (early-return gates) | Low (state-gate) | **Low** |
| XC-01 | Monolithic run_client + scattered ordering edges | Maintainability | High (large refactor) | **Low** |
| XC-12 | Per-frame `format!` in egui panels | Low | Trivial | **Low** |
| XC-13 | NameTagCache good pattern (no action) | n/a (positive) | n/a | n/a |
| XC-16 | Hot-path logging already event-gated | n/a (positive) | n/a | n/a |
| XC-18/19 | Editor picking/shortcuts scan, undo Vec::remove(0) | Negligible (editor-only) | n/a | n/a |
| XC-20 | Mode dispatch + state gating correct | n/a (positive) | n/a | n/a |

## 5. Quick Wins (in order of effort)

1. **XC-04** — Delete `directional_light_system` + its registration (lib.rs:1143): removes a per-frame matrix inverse. 10-minute change.
2. **XC-05** — Add a 30 s `Local<Timer>` run condition to `memory_diagnostics_system` (or remove it — the leak it tracked is fixed per pitfalls/performance-memory.md).
3. **XC-14** — Change `LoggingConfig` default `level` from `"debug"` to `"info"`; debug events still available via `RUST_LOG=debug`. Immediate disk/CPU savings in every session.
4. **XC-08** — Borrow `filtered_zones` instead of cloning in `zone_list_panel_system`.
5. **XC-11** — Hoist `to_srgba()` out of the grid loop and skip re-emission when `EditorGridSettings` is unchanged.
6. **XC-07** — Audit `src/graphics/apply_*` for missing `is_changed()` gates (the pattern already exists in lib.rs).
7. **XC-09** — Reuse a `Local<Vec<WorldUiRect>>` in the world-UI extractor instead of `Vec::with_capacity(1024)` per frame.

## 6. Risks and Considerations

- **Do not remove both `ApplyDeferred`s (XC-02) blindly.** Bevy 0.18.1 applies deferred commands automatically at the end of `PostUpdate`, but the *manual* apply at `DebugRenderPreFlush` exists so that systems in the second PostUpdate batch (vehicle model, zone visibility, model viewer, graphics applies) see freshly applied components. Verify each batch's dependencies before merging; the safest minimal change is removing only the *first* explicit `ApplyDeferred` (lib.rs:1002) if nothing between it and :1006 requires applied commands mid-schedule.
- **XC-17 touches scheduling shared with the zone viewer and model viewer.** Map editor systems must keep running when `AppState::MapEditor` is active; a naive `run_if(in_state(AppState::MapEditor))` is correct only if the plugin is registered for all modes (it is — lib.rs:957). Test zone viewer + model viewer after the change.
- **XC-14 changes log output behavior.** The structured.jsonl is the LLM-analysis feature; the level default is a product decision (user may *want* debug-by-default). Recommend: default `"info"`, with the `--config` file (`LoggingConfig.level`) preserving per-session overrides, and document `RUST_LOG=debug` for deep sessions.
- **XC-09's bind-group caching**: world_ui.rs:538's per-frame bind-group recreation may be intentional (texture set changes every frame when name tags update). Only reuse buffers for the `rects` Vec; leave the bind group unless profiling shows it hot.
- **Review scope limits:** `graphics::apply_*` internals (XC-07), `ui_minimap_system`'s full body (949 lines, partial read), and `world_ui.rs` extraction details were not read line-by-line; their findings are flagged as *unverified* and should be confirmed before acting.
- **No code was modified in this pass.** All findings are static-analysis based; validate frame-time impact with `--disable-vsync` (main.rs:70-74) + the frame-time diagnostic before/after each change.

---

## 7. Verification Update (2026-08-04)

Independent sub-agent scrutiny of every finding (XC-01–XC-20) against the actual source, Bevy 0.18.1, and the previous implementation attempt on `wip/local-changes-2026-08-04` (validated in `12-validation.md`). **CRITICAL META-FINDING FIRST: the current working tree does NOT match the validated wip state — the XC-02/03/04/05 fixes (all present and validated on wip) have been reverted in the working tree; the 11 implemented 11-review items (12-validation rows 92–96) exist only on the wip branch.** Verdicts per finding:

| Finding | Verdict | Scrutiny result / action |
|---|---|---|
| XC-01 | PARTIAL (defer refactor; do the docs-only half) | Structure confirmed — worse than stated: 93 inline edges (84 after + 9 before), only **4 `in_set()` registrations in the whole codebase**, and the ModelSystemSets (8 sets) and EffectSystemSets (7 sets) chains are **completely inert** (zero member systems — the declared ordering does nothing). **The scheduler-cost claim is FALSE**: Bevy 0.18 rebuilds the graph only when changed; steady state uses the precomputed executable; 93 edges ≈ 0.1–0.3 µs/frame (0.002%). The proposed `chain()` refactor would be **actively harmful** (chain() total-orders ~150 currently-parallel systems → serialization barriers, likely slower), and several edges are genuinely system-specific. Do: write the missing `system-architecture/schedule.md` (the doc's own gap note — the high-value outcome) + optionally delete the inert set declarations. Do NOT refactor until the wip state lands and the §3.1–3.4 regressions are resolved. |
| XC-02 | CONFIRM (fix on wip, correct — merge) | Accurate (two explicit ApplyDeferreds; second batch after CheckVisibility; DebugRenderPreFlush contains only ApplyDeferred). The "hard barrier" framing is overstated — both sync points are isolated graph nodes (no ordering edges through them), so the executor does not enforce batch ordering; the parallelism cost is much smaller than implied. **Remove only the first ApplyDeferred** (lib.rs:1003) — safe (isolated; covered by the kept AD2 + the mandatory end-of-schedule final apply before render extract and rapier's SyncBackend); **do NOT move AD2 to PreUpdate** (would delay PostUpdate-issued commands a full frame; `force_zone_visibility_system` takes no Commands, so the CheckVisibility path needs no apply). Wip implements exactly this (12-validation:92 OK) — the working tree does not. |
| XC-03 | CONFIRM (implement-with-changes) | All four sub-claims verified (empty stage; chain orders ApplyDeferred before nothing; config defaults all true; no reader besides the egui checkboxes — the rapier plugin is commented out). Fix safe: delete the variant + chain **atomically** (they reference each other), flip defaults to false, keep `DebugRenderPreFlush` (it IS the second apply point — rename cosmetic, pending XC-02's decision), keep the config resource (it's the checkbox storage); optional: delete dead `color_for_entity`. Wip removed the stage+chain but did **not** flip the defaults; XC-03 is absent from 12-validation. The working tree has neither half. |
| XC-04 | CONFIRM (delete — already done on wip) | Claim accurate (one inverse per frame, read-only system, no run condition; the Bevy 0.13+ comment is correct — cascades are computed automatically by the pipeline). Deletion is safe: zero other references; shadows are fully independent (apply_shadow_quality_system + the sun's explicit CascadeShadowConfig at zone_lighting.rs:147). **Already deleted on wip** (12-validation:93 OK) — port to main; optionally update the stale SUN_DOCUMENTATION.md mention. |
| XC-05 | PARTIAL (premise stale — both branches fixed) | **The "queries run every frame" premise is stale**: HEAD's gate is at the top with an early return (queries + asset counts run only when the 30 s log fires — residual cost is one clock read/frame); the wip achieves the same via `run_if(memory_diagnostics_run_condition)` (12-validation:94 OK). Keep-until-F1/F2-validated-then-delete (its counters are the before/after harness — align with 02-F19). Correct the stale wording in this doc (XC-05 text + table rows). |
| XC-06 | CONFIRM (implement-with-changes; fix sketch needs correction) | 14 systems (not ~10); 11 of 13 panels early-return on debug_ui_open (camera_info and entity_inspector gate on their own flags); param-fetch cost is real but tiny (~µs total in a ms-scale egui pass). **The doc's fix sketch won't compile** (`resource_exists_and_equals` requires `PartialEq`, which UiStateDebugWindows lacks) **and is a footgun** (all-fields-vs-Default compare flips false when the model viewer sets npc_list_open); `Local<bool>` can't be shared across systems. Correct form: `.run_if(|s: Res<UiStateDebugWindows>| s.debug_ui_open)` on the 13 panel systems only; `ui_debug_menu_system` must stay ungated (it processes the Ctrl+D toggle before its early return). ~5 lines; not implemented anywhere. |
| XC-07 | **REFUTE** (already-gated — audit resolved) | All 7 graphics apply systems carry `is_changed()` gates in every commit of the file's history (7/7 on main; 10/10 on wip with the new F9/F10 systems). No code change; doc-only correction (drop "ungated", mark verified-clean). The first-frame behavior is correct (init_resource marks changed; gates fire on the first run and on user edits only). |
| XC-08 | CONFIRM (implement the one-line borrow) | Clone-per-frame real (correction: `ZoneId` is NonZeroU16 → N×2 bytes, even cheaper); the dirty-flag cache already exists (only the clone is per-frame). The doc's borrow fix **compiles** (empirically verified with rustc 1.93.1 — field-precise closure capture; TableBuilder::body takes a FnOnce with a fixed-lifetime &Vec). Wip fixed it via `mem::take` (heavier than needed). Apply the one-line `clone()` → `&` on main; neither variant has been built yet. |
| XC-09 | **REFUTE** (no per-frame allocation exists) | `Vec::with_capacity(1024)` lives in `ExtractedWorldUi::default()`, invoked **once** by `init_resource` at plugin build; the extractor (a system) does `clear()` + `push` — the 1024-slot buffer is **reused across frames**. The fix is architecturally incompatible (a `Local` is invisible cross-schedule; the resource IS the reuse). Reject as a code change; the real outstanding world_ui items belong to 05-F9 (BufferId bind-group cache on wip, the WorldUiBatch entity leak, and the missing `bevy_default()` format fix). |
| XC-10 | PARTIAL (fold into 02-F1/F2 + DD-01 — corrected implementation) | Claim accurate (cite fix: `:27-30` is the once-per-spawn create system; the per-frame block is `:145-171`; "per particle" → per sequence entity) and **understated** (the per-frame material get_mut also forces bind-group re-prepare + mesh re-extract per frame). The wip is a functional NO-OP (size-0 capacity bug — verified in crates.io 0.18.1: `From<T>` leaves `buffer_description.size = 0`, so the grow branch fires every frame). **Two corrections to the previously-validated story**: (1) `set_data` does NOT reuse the GPU buffer on 0.18.1 (`prepare_asset` always `create_buffer_with_data` — the write_buffer-reuse path is 0.19-dev behavior); (2) the "guard material mutation" half is WRONG on 0.18.1 — the per-frame material re-prepare is **load-bearing** (storage-buffer bindings resolve at prepare time; skipping re-prepare freezes digits/particles). Corrected implementation: `with_size`-created buffers (capacity ≥ 10 digits) + **unconditional** per-frame material get_mut + capacity-check `set_data` + grow path `mem::replace` + `remove(&old)` — removes handle/asset-store churn only; GPU realloc/bind-group re-prepare/mesh re-extract remain. True reuse needs the persistent `Buffer` + `write_buffer` pattern as a separate pipeline milestone. Severity should match DD-01 (High). |
| XC-11 | PARTIAL (hoist-only — reject the dirty-gate half) | Mechanics confirmed (~66 to_srgba calls for major lines, not ~300). **The early-return justification is WRONG**: Bevy gizmos are immediate-mode — storage is cleared every frame in `Last` via `mem::take`, so a skipped frame renders NO grid; and `EditorGridSettings` is never mutated anywhere (is_changed fires once → the grid would render one frame total). The doc contradicts its own "gizmo push is inherent". Fix: color hoist only — already on wip (with `run_if(MapEditor)`, 12-validation:95 OK). Do not port the early-return to main. |
| XC-12 | PARTIAL (implement the model-browser sites only; defer the rest) | Claims confirmed (model_browser tabs + unlisted siblings; zone_list :103/:135/:182; debug entity list; minimap correctly cached). Gating reality: zone_list is closed by default; the debug list is gated by debug_ui_open (nil in production) — only the model-browser tabs are always-on (in the editor). Implement: category-tab labels with a count-keyed `Local` cache preserving the `▶` marker + live counts (~8–12 allocs/frame saved). Defer zone_list (adjacent to the pending XC-08 edit); reject the debug list and minimap sites. Not implemented anywhere. |
| XC-13 | CONFIRM (no action — positive) | Verified (galley + texture only on cache miss; zone/ppp flush correct; bounded by unique names per zone; KB-scale textures). An LRU cap would be safe (tag entities hold their own image-handle clones) but is unnecessary now. Confirm no-change; leave LRU as a documented fallback. |
| XC-14 | PARTIAL (premise stale — default is already "info") | **The core premise is refuted**: the default has been `"info"` since 2026-03-02 (git blame) — the doc cited the stale comment at mod.rs:98; the wip carries the comment fix. Side-findings: `LogPlugin` never installs its subscriber (init_session_logging runs first → `set_global_default` fails → a **spurious `error!` in structured.jsonl every session** + dead LogPlugin config); 59 debug sites include 3 per world-UI draw per frame (short-circuited at info). Fix: comment-only (align main with wip); **reject the release console-layer drop** (unmeasured, blinds release warn/error output); optional: `.disable::<LogPlugin>()` to kill the startup error. |
| XC-15 | CONFIRM (resolved-by-XC-14 — no independent work) | Per-event cost structure accurate (prefix scans, tag allocs, serde Value, JSON serialize, non-blocking hand-off + a `chrono::Local::now().to_rfc3339()` bonus). The enabled() short-circuit is verified (the Filtered layer means below-level events never reach format_event). Since the default is already "info", the finding's own fix sketch is satisfied. No code change. |
| XC-16 | CONFIRM (positive — no independent action) | All three legs verified (zone-time AtomicU32 gate; bird/fish spawn-time only; collision warns are per-frame *during the transition window* only). Nuance: the collision warns are resolved by 04-F16's `warn_once` (still unimplemented on main — the 12-validation "demoted" record for 04-F16 is wrong); GC-04's demotions live on wip only. Add the 04-F16 dependency note; no change to XC-16 itself. |
| XC-17 | CONFIRM (fix on wip — merge; understated) | 21 registrations (not ~10) run in Game sessions on main, plus a real one-time cost: `load_available_models_system`'s full ZSC catalog scan executes **once in Game mode** (the Option early-return doesn't fire on first run — the resource doesn't exist yet). The wip gates **all 21** (verified) + the two mod.rs systems with `run_if(in_state(AppState::MapEditor))`; OnEnter/OnExit handlers correctly untouched; `AvailableModels` has zero consumers outside map_editor (resolves 12-validation §3.9's open question). Merge + test zone viewer/model viewer/map editor after. |
| XC-18 | CONFIRM (no action — framing correction) | The raycast is strictly `just_pressed`-gated (idle cost is O(1) early-returns); the shortcuts system declares queries but only iterates on Delete/Ctrl+Backspace. The proposed run_if would duplicate the existing in-body checks; `Changed<Transform>` on the shortcuts queries is misguided (they never iterate per frame). Confirm no-change; reword the finding to not imply per-frame raycasts/scans. |
| XC-19 | CONFIRM (no action) | Accurate (100-entry shift ≈ 13 KB memmove, µs-scale; editor-only). All access flows through 7 wrapper methods (the pub fields are never touched outside resources.rs) — a VecDeque switch would be ~5 lines with zero caller changes. Non-issue; opportunistic only. |
| XC-20 | CONFIRM (positive — no action) | Verified point-by-point (mode dispatch; state gating; no simulation-cost leakage into viewers; the only ungated gameplay-adjacent systems internally early-return on state). XC-17 is the acknowledged counter-example (leakage into Game mode, opposite direction). Re-run the leak audit when the wip gating merges. |

**Cross-cutting note for the whole doc set:** the vendored source folder `bevy-collection\bevy-0.18.1` is actually **0.19.0-dev** (its `Cargo.toml` declares `version = "0.19.0-dev"`); the client builds against crates.io Bevy 0.18.1. API citations verified only against that folder must be re-checked against 0.18.1 before implementing (this invalidated the F2-set_data "write_buffer reuse" claim in 02-F2/XC-10).
