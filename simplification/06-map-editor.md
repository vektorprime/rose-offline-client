# Simplification Analysis: `src/map_editor/` module

**PLAN ONLY — no code changes were made. This is a research report.**

- Analyzed: 25 files, ~11,444 LOC
- Map editor UI: 3,937 LOC | Systems: 4,122 LOC | Save: 2,500 LOC | Core (mod/resources/components): 885 LOC
- Notable: several large subsystems (undo/redo, gizmos, properties panel) are **partially built twice or contain abandoned scaffolding**

---

## 1. DUPLICATED: Two complete, simultaneously-registered undo/redo implementations (functional bug)

**Files:** `src/map_editor/systems/undo_system.rs` (entire file, 376 LOC) vs `src/map_editor/systems/property_update_system.rs:487-723` (`apply_undo_system`, `apply_undo_action`, `apply_redo_action`)

Both are registered and both listen for Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z:
- `UndoRedoPlugin` registered in `map_editor/mod.rs:106`
- `PropertyUpdatePlugin::build` registers `(property_update_system, apply_undo_system)` in `property_update_system.rs:732`

**Consequence:** one Ctrl+Z press pops **two** undo actions per frame (one per system). `apply_undo`/`apply_redo` (undo_system.rs:96-367) and `apply_undo_action`/`apply_redo_action` (property_update_system.rs:543-723) are near-verbatim copies of the same `match EditorAction` logic, with subtle inconsistencies (e.g. `ModifyComponent` redo pushes swapped values in undo_system.rs:243 but *unswapped* values in property_update_system.rs:639; `MAX_UNDO_STEPS=50` in undo_system.rs:16 vs `MAX_UNDO_HISTORY=100` in resources.rs:9, with redo manually truncating the stack at undo_system.rs:271 instead of using the helper).

**Suggestion:** Delete `undo_system.rs` entirely; keep ONE implementation (property_update_system.rs). Optionally move the keyboard handling into `keyboard_shortcuts_system.rs` and have property updates just process a message.

**LOC savings: ~376 (plus the dead code counted in finding 2)**

---

## 2. DUPLICATED + BROKEN: Undo machinery is over-engineered and half-fake

**File:** `src/map_editor/resources.rs:189-224` (`EditorAction`), `undo_system.rs`, `property_update_system.rs:543-723`

- `EditorAction` has 7 variants with **singular AND plural twins** (`TransformEntity`/`TransformEntities`, `AddEntity`/`AddEntities`, `DeleteEntity`/`DeleteEntities`). Only the plural variants and `TransformEntity` are ever produced; `AddEntity`/`DeleteEntity` singular branches duplicate the plural branches.
- `ModifyComponent` undo **does nothing**: undo_system.rs:226-249 and property_update_system.rs:625-645 only log and swap/push strings — the component is never actually restored. The whole variant could be dropped or re-implemented as old/new `Transform` snapshots.
- `DeleteEntity.serialized_data` is never populated (keyboard_shortcuts_system.rs:266 writes `String::new()`, type `"Unknown"`) and never used; undo "restores" a placeholder entity with only a Transform and Name (undo_system.rs:167-176) — not a real object.
- `EditorAction::AddEntities` **redo is a no-op** (undo_system.rs:317-322, property_update_system.rs:691-696 — "entity recreation needed" log only), so Ctrl+Y after a duplicate does nothing.

**Suggestion:** Replace the enum with a single `EditorSnapshot`-style action (entity, old_transform, new_transform, spawned: bool) — or simply record old/new transforms per entity and drop add/delete from undo entirely. ~200 LOC.

**LOC savings: ~200**

---

## 3. DEAD CODE: properties_panel.rs contains an entire abandoned system + duplicate function pairs

**File:** `src/map_editor/ui/properties_panel.rs` (1,673 LOC)

- `editor_properties_panel_system` (lines 45-103) is **never registered** — ui/mod.rs registers the standalone `editor_properties_panel` (ui/mod.rs:66, 227). Its companion `GetEguiContext` trait (lines 105-115) always returns `None` — clear evidence of an abandoned approach.
- The following functions exist ONLY to serve that dead system and are ~95% verbatim copies of their working `_standalone` twins:
  - `single_object_properties` (176-269) vs `single_object_properties_standalone` (272-387)
  - `multi_object_properties` (390-438) vs `multi_object_properties_standalone` (441-491)
  - `transform_editor` (494-615) vs `transform_editor_standalone` (618-786)
  - `zone_object_editor` (789-799) + `zone_object_editor_inner` (815-926) vs `zone_object_editor_standalone` (802-812) + `zone_object_editor_inner_with_events` (929-1164)
  - `event_object_editor` (1167-1177) + `event_object_editor_inner` (1193-1211) vs `event_object_editor_standalone` (1180-1190) + `event_object_editor_inner_with_events` (1214-1249)
  - `warp_object_editor` (1252-1262) + `warp_object_editor_inner` (1278-1292) vs `warp_object_editor_standalone` (1265-1275) + `warp_object_editor_inner_with_events` (1295-1328)
  - `collision_editor` (1354-1364) + `collision_editor_inner` (1403-1463) vs `collision_editor_standalone` (1367-1377) + `collision_editor_inner_with_events` (1466-1602)
  - `get_collision_part_mut` (1391-1400) marked `#[allow(dead_code)]`
- The dead "inner" variants duplicate the 12-arm `match zone_object` ID-extraction (lines 819-864 == lines 939-984 verbatim).

**Suggestion:** Delete the dead system, the `GetEguiContext` trait, and every non-`_standalone` twin. Keep only the `_standalone`/`_inner_with_events` functions. The two 46-line ID-extraction matches can be replaced with one helper on `ZoneObject`.

**LOC savings: ~750**

---

## 4. DEAD CODE: menu_bar.rs — ~15 buttons are log-only stubs

**File:** `src/map_editor/ui/menu_bar.rs` (633 LOC)

Nearly every menu item logs and closes the menu without doing anything: Cut, Copy, Paste, Delete, Duplicate, Select All, Deselect All (lines 338-386), Reset Camera, Frame Selection, Toggle Colliders, Toggle Gizmos (441-461), Zone Info, Validate Zone (476-484), Add Object, Add Effect, Add Sound, Delete Selected, Group Selected, Ungroup Selected (491-529). The Edit menu also implies Undo/Redo are wired (314-334) but they only log — actual undo is keyboard-only.

**Suggestion:** Remove all no-op items (or wire them to the real handlers: Undo/Redo/Delete/Duplicate can send the existing messages). Replace the `file_menu`'s duplicated save-status block (lines 177-186, duplicated again in status_bar.rs:71-89) with a call to a shared helper.

**LOC savings: ~150**

---

## 5. DEAD CODE: `model_browser_panel_system` duplicate + unused components/enums/resources

- `src/map_editor/ui/model_browser_panel.rs:215-229` — a second `model_browser_panel_system` that is never imported (ui/mod.rs:243 defines its own; only `editor_model_browser_panel` is imported from this file). Same for `toggle_model_browser` (232-234) whose only caller is the dead system.
- `src/map_editor/components.rs`: `EditorGizmo` (13-28), `EditorPreview` (81-82), `EditorModified` (85-89), `EditorHandle` (96-102), `HandleType` (105-118) — defined and re-exported (mod.rs:54-55) but **never spawned or queried anywhere**. Gizmos are drawn ad-hoc with Bevy `Gizmos` API instead.
- `src/map_editor/resources.rs`: `SelectionMode` (271-277) unused; `HierarchyFilter` (280-309) unused (hierarchy_panel.rs imports it at line 11 but uses hardcoded labels; the filter dropdown at hierarchy_panel.rs:164-179 even shows `editor_mode.display_name()` — a bug); `AvailableModels::get_models_mut` (415-423) has an `All => &mut deco_models` fallback that is wrong-but-unused.
- `src/map_editor/systems/keyboard_shortcuts_system.rs`: `keyboard_shortcuts_help_system` (379-426, `#[allow(dead_code)]`), `is_alt_pressed` (180-183, `#[allow(dead_code)]`), `handle_select_all` (354-361) and `handle_focus_selected` (364-376) are log-only stubs.
- `src/map_editor/systems/load_models_system.rs:254-281`: `try_load_zsc_from_vfs` (`#[allow(dead_code)]`).
- `src/map_editor/systems/grid_system.rs`: `grid_spawn_system` (129-144, `#[allow(dead_code)]`) and `grid_visibility_system` (147-157) are never registered; `EditorGrid` component + `EditorOnly` are only used by those dead systems.
- `src/map_editor/systems/selection_system.rs:122-127`: computes `is_selectable` then discards it with `let _ = is_selectable;` — remove the query and comment.
- `src/map_editor/systems/model_placement_system.rs:485-599`: `model_preview_system` is registered but shows a **green wireframe cube** placeholder ("Model preview" per mod.rs docs); `EditorPreview` component unused. Either implement a real preview or remove.

**Suggestion:** Delete all of the above. Also remove the Escape-deselect duplicate: selection_system.rs:184-191 does the same thing as keyboard_shortcuts_system.rs:68-71 (`handle_deselect_all`).

**LOC savings: ~400**

---

## 6. DUPLICATED: `world_to_block_coords` + HIM/TIL writers copied verbatim in 3 places

- `src/map_editor/systems/keyboard_shortcuts_system.rs:24-38` and `src/map_editor/save/save_system.rs:21-37` — **byte-identical** functions including the same `ZONE_CENTER_X=5200.0`, `ZONE_CENTER_Z=-5200.0`, `BLOCK_SIZE_METERS=160.0`, `ZONE_BLOCK_COUNT=64` constants. The same constants also appear inline in `model_placement_system.rs:178-181, 643` and `ui/mod.rs:443-449` and `map_editor/mod.rs:145`.
- `src/map_editor/ui/mod.rs:563-597` (`write_default_him`/`write_default_til`) vs `src/map_editor/save/save_system.rs:39-76` (`write_him_file`/`write_til_file`) — same 4-byte header + le bytes loops.

**Suggestion:** Centralize in one place — e.g. `save/ifo_export.rs` or a new `map_editor::coords` helper: `world_to_block_coords`, `ZONE_CENTER`, `BLOCK_SIZE`, and one `write_him`/`write_til` pair. Check whether `rose_data` already defines these constants (zone blocks are 160m/64x64 in `rose_file_readers`).

**LOC savings: ~80**

---

## 7. DUPLICATED: map_editor re-implements zone-object spawning/material creation from the main game

**Main game:** `src/zone_loader/spawning/objects.rs:3-260+` (`spawn_object` — canonical part spawn: IFO->Bevy coord transform, mesh cache, `ExtendedMaterial<StandardMaterial, RoseObjectExtension>` construction, collision-filter flags from `ZscCollisionFlags`).

**Editor copies:**
- `src/map_editor/systems/model_placement_system.rs:209-470` (`place_model_at_position`) — same part loop, same coord transforms (lines 293-311 == objects.rs:65-83), same material construction (358-384 == objects.rs:157-189, minus lightmap/specular), same collision-filter logic (387-401 == objects.rs:191-218).
- `src/map_editor/systems/duplicate_system.rs:296-414` (`duplicate_child_parts`) and 417-494 (`load_material_for_part`) — a *third* copy of the same material creation (460-486) and part-spawn component bundles (316-391).
- The `AlphaMode::Mask/Blend` + `SPECULAR_SPHEREMAP` fallback pattern also exists in `effect_loader.rs:333-342` and `model_loader.rs:1384-1397`.

**Suggestion:** Extract a shared spawn helper (e.g. `zone_loader::spawning::spawn_object_with_override` or a new `spawn_rose_object(commands, zsc, object_instance, ifo_id, options)` in a shared module) and have all three call sites use it. The editor variants need the `ifo_object_id=0` + `EditorPlacedObject` extras — make those options.

**LOC savings: ~350**

---

## 8. DUPLICATED: ifo_export.rs implements every write method twice

**File:** `src/map_editor/save/ifo_export.rs` (728 LOC)

- Every `write_*` method exists in two forms: writing into `self.buffer` (`write_object` 67-106, `write_event_object` 109-113, `write_warp_object` 116-118, `write_sound_object` 121-128, `write_effect_object` 131-134, `write_npc` 137-143, `write_monster_spawn` 163-201, `write_water_plane` 204-213) and writing into a passed `Vec` (`write_*_to_vec` 446-565). Only the `_to_vec` variants are used by `write_block`; the `self.buffer` versions are dead (used only by tests).
- `BlockType` enum (ifo_export.rs:14-30) duplicates `IfoBlockType` (ifo_types.rs:305-360), which is itself dead (never referenced). Pick one.
- `export_zone_ifo_files` (607-632) is dead — save_system.rs:780-821 inlines the identical loop.
- Extreme logging: `write_block` logs 15+ lines per block (221-247, 405-441) and `write_object_to_vec` logs 4 lines per object (457-490) — remove or downgrade to trace.

**Suggestion:** Keep only the `_to_vec` writers (rename to `write_object` etc. against a `&mut Vec<u8>`), delete `export_zone_ifo_files` and the `self.buffer` twins, delete `IfoBlockType`.

**LOC savings: ~300**

---

## 9. DEAD CODE: ifo_types.rs unused members

**File:** `src/map_editor/save/ifo_types.rs` (673 LOC)

- `IFO_MAGIC` (10), `IFO_VERSION` (13) — unused.
- `IfoObject::to_bevy_transform` (84-98) — unused.
- `IfoBlockType` enum + `from_u32`/`to_u32` (305-360) — unused (see finding 8).
- `IfoFileData::file_path` field (427) — set but never read (`file_name()` recomputes it).
- `ZoneExportData::populated_block_count` (530-532) — unused.

**Suggestion:** Delete the above (~90 LOC). Note `IfoWarpObject` wraps a single field — could be replaced by `IfoObject` + warp_id, but that's a judgment call.

**LOC savings: ~90**

---

## 10. OVER-ENGINEERED: gizmo system with no gizmo interaction; mode-switch handling duplicated/conflicting

**Files:** `src/map_editor/systems/transform_gizmo_system.rs` (505 LOC), `keyboard_shortcuts_system.rs:185-217`

- `GizmoDragState.active_axis` (transform_gizmo_system.rs:19) is **never set** — nothing ever picks gizmo handles. So every drag falls into the `Free` axis path and the entire `GizmoAxis` axis-snapping machinery (31-47, 210-358) is dead weight. The gizmos drawn by `draw_gizmo_visuals` (370-495) are purely cosmetic.
- `EditorGizmo`/`GizmoType` components (components.rs) exist but gizmos are drawn with the immediate-mode `Gizmos` API — the component-based design was abandoned.
- Mode switching exists TWICE: `handle_mode_switches` in transform_gizmo_system.rs:183-207 (W=Translate, E=Rotate, R=Scale, Q=Select) AND `handle_mode_switches` in keyboard_shortcuts_system.rs:187-217 (E/R/Q + V=Add, X=Delete). Both run on the same frames; W is contradictory (gizmo says Translate; keyboard file comment says W is reserved for FreeCamera at lines 11, 186). The menu_bar docs (menu_bar.rs:571-575) even show yet another mapping (Q/E/R/V/X).
- `snap_to_grid` G-toggle also duplicated: transform_gizmo_system.rs:82-85 (Ctrl+G!) vs keyboard_shortcuts_system.rs:117-124 (G without Ctrl) — inconsistent behavior for the same key.

**Suggestion:** Delete the gizmo-visual + axis machinery down to a simple "drag = apply delta on Free axis" (keeping `apply_translation/rotation/scale` but removing the axis match arms), delete `GizmoAxis` or collapse to a fixed enum of 3, keep ONE `handle_mode_switches` (the keyboard_shortcuts one, V/X included) and ONE snap toggle. `draw_gizmo_visuals` can stay as visual feedback.

**LOC savings: ~200**

---

## 11. OVER-LOGGED: pervasive `log::info!` in hot paths and per-save dumps

`save_system.rs` logs ~40 lines per save (232-306, 326-464, 759-770), `ifo_export.rs` per-block/per-object (221-247, 405-441, 457-490), `keyboard_shortcuts_system.rs`, `menu_bar.rs`, `properties_panel.rs` log on every click. Comments in menu_bar.rs even state intent to keep logs (e.g. 148-150). Most should be `debug!` or removed.

**LOC savings: ~120 (mostly noise, not LOC)**

---

## 12. Long functions (refactor candidates)

| Function | Location | Length |
|---|---|---|
| `save_zone_system` | save/save_system.rs:210-860 | ~650 lines (steps 1-3 could be split into helpers; the 7-arm deletion match at 337-432 and 12-arm add match at 677-755 are repetitive) |
| `zone_object_editor_inner_with_events` | ui/properties_panel.rs:929-1164 | ~235 |
| `editor_menu_bar` + `file_menu` | ui/menu_bar.rs:51-195 | ~145 |
| `write_block` | save/ifo_export.rs:220-443 | ~220 |
| `place_model_at_position` | systems/model_placement_system.rs:209-470 | ~260 |
| `handle_duplicate_event` | systems/duplicate_system.rs:42-190 | ~150 |
| `update_existing_object` | save/save_system.rs:872-1031 | ~160 (7 near-identical arms — use a closure over the list) |
| `from_existing_blocks` | save/ifo_types.rs:537-672 | ~135 (10 repetitive conversion loops) |
| `handle_delete_selected` | systems/keyboard_shortcuts_system.rs:241-351 | ~110 |

---

## 13. Misc smaller issues

- `ui/mod.rs:243-262` `model_browser_panel_system` duplicates the check in `editor_model_browser_panel` (both test `enabled`); the 2nd `model_browser_panel_system` in model_browser_panel.rs is dead (finding 5). One layer of wrapper could go.
- `resources.rs` `MapEditorState::new()` (80-95) duplicates `Default` (39-76) with a different `enabled` value — either derive Default or keep one.
- `properties_panel.rs:1045-1049` — "Tag" text field edits a local that is discarded (dead UI).
- `properties_panel.rs:915-919` — "Name" field edits a local, discarded (dead UI).
- `hierarchy_panel.rs:150-157` — search box edits a local copy, no effect (dead UI); filter dropdown at 164-179 uses `editor_mode.display_name()` (bug) and non-functional `selectable_label(false, ...)`.
- `hierarchy_panel.rs:321-359` `object_list_item` + `format_zone_object_label` (`#[allow(dead_code)]`).
- `status_bar.rs:11-48` `get_zone_name` hardcodes 40 zone names that `GameData.zone_list` already knows; camera/FPS/object-count labels (159-168) are hardcoded placeholders.
- `undo_system.rs:16` `MAX_UNDO_STEPS` vs `resources.rs:9` `MAX_UNDO_HISTORY` — two limits (see finding 1).
- `menu_bar.rs` Undo/Redo buttons (317-334) don't trigger anything — wire to `KeyboardShortcutsPlugin` or remove.
- `zone_list_panel.rs` is decent; minor: `ZoneListPanelState::new()` == `Default`.
- `selection_highlight_system.rs:49, 81, 92` — acknowledged-unused vars; `query_selectable` query does nothing.
- `load_models_system.rs` loads ZSC data twice (real FS then VFS fallback) with hand-rolled file reading, duplicating `GameData`'s existing ZSC parsing — could reuse `GameData` helpers.
- `model_placement_system.rs` and `duplicate_system.rs` both register `.add_message::<DuplicateSelectedEvent>()`? (duplicate_system.rs:32) — registered twice is harmless but check.

---

## Prioritized summary (quick wins first)

1. **Delete the duplicate undo/redo system** (`undo_system.rs`, ~376 LOC) — also fixes the double-undo-per-keypress bug. (Finding 1)
2. **Delete the abandoned properties-panel system + all non-standalone twins** (~750 LOC) — pure dead code with zero behavior change. (Finding 3)
3. **Delete log-only menu items and dead UI panels** (menu_bar no-op buttons, hierarchy search/filter stubs, dead status-bar placeholders, ~250 LOC). (Findings 4, 13)
4. **Delete dead components/enums/functions** — `EditorGizmo`, `EditorPreview`, `EditorModified`, `EditorHandle`, `HandleType`, `SelectionMode`, `HierarchyFilter`, `grid_spawn_system`, `grid_visibility_system`, `try_load_zsc_from_vfs`, `keyboard_shortcuts_help_system`, dead `model_browser_panel_system`, ifo_types unused members (~600 LOC). (Findings 5, 9)
5. **Collapse ifo_export's double writer methods + delete `export_zone_ifo_files` + `IfoBlockType`** (~300 LOC). (Finding 8)
6. **Consolidate `world_to_block_coords` + HIM/TIL writers** into one shared helper (~80 LOC). (Finding 6)
7. **Extract shared spawn/material helper** used by zone_loader + model_placement + duplicate systems (~350 LOC). (Finding 7)
8. **Simplify gizmo machinery** — drop unused `GizmoAxis` picking paths, one `handle_mode_switches`, one snap-toggle (~200 LOC). (Finding 10)

Total estimated savings: **~2,900 LOC out of ~11,444 (~25%)**, most of it pure dead code with no behavior risk.
