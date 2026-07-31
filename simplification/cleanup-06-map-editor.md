# Cleanup Report: `src/map_editor/` (branch `code-simplification`)

Implements the simplifications from `simplification/06-map-editor.md`. All changes are confined to `src/map_editor/`. No changes to `src/lib.rs`, `src/zone_loader/`, or anything outside the module.

**Total: ~2,056 lines deleted (gross), ~1,991 net (after new 65-line `coords.rs` helper).**

## Changes per finding

### Finding 1 — duplicate undo/redo system (double-undo bug fix) — 376 LOC
- Deleted `systems/undo_system.rs` (entire file, 376 LOC).
- Removed `UndoRedoPlugin` registration (`map_editor/mod.rs`) and `pub mod undo_system` / re-export (`systems/mod.rs`).
- Verified by grep that `apply_undo_system` (`property_update_system.rs:488`) handles Ctrl+Z / Ctrl+Y / Ctrl+Shift+Z itself, so undo/redo still works with only the one implementation.
- **Behavior fix (approved):** one Ctrl+Z/Y now pops exactly one undo step instead of two.

### Finding 3 — dead properties-panel system + duplicate twins — 623 LOC
`ui/properties_panel.rs` 1673 → 1050:
- Deleted `editor_properties_panel_system` (never registered; verified via grep — only `editor_properties_panel` is used at `ui/mod.rs`).
- Deleted `GetEguiContext` trait (always returned `None`).
- Deleted non-`_standalone` twins: `single_object_properties`, `multi_object_properties`, `transform_editor`, `zone_object_editor` (+`_inner`), `event_object_editor` (+`_inner`), `warp_object_editor` (+`_inner`), `collision_editor` (+`_inner`), `get_collision_part_mut`.
- Kept `get_collision_part`, `has_collision`, `get_entity_type_string`, `list_components` (shared with the standalone chain).

### Finding 5 — dead components/enums/systems — ~345 LOC
- `components.rs` 119 → 13: deleted `EditorGizmo`, `GizmoType`, `EditorGrid`, `EditorPreview`, `EditorModified`, `EditorHandle`, `HandleType`, `EditorOnly` (all never spawned/queried; verified via grep). Kept `SelectedInEditor`, `EditorSelectable` (used by `zone_loader.rs:363`).
- `resources.rs`: deleted `SelectionMode`, `HierarchyFilter` (+impl), `AvailableModels::get_models_mut` (all unreferenced).
- `grid_system.rs`: deleted `grid_spawn_system`, `grid_visibility_system` (never registered; re-export in `systems/mod.rs` removed).
- `keyboard_shortcuts_system.rs`: deleted `is_alt_pressed`, `handle_select_all`, `handle_focus_selected`, `keyboard_shortcuts_help_system` (all `#[allow(dead_code)]` or log-only stubs) plus their call sites (Ctrl+A, F).
- `selection_system.rs`: removed dead `query_selectable` / `is_selectable` computation and the duplicate Escape-deselect block (now handled only by `keyboard_shortcuts_system::handle_deselect_all`).
- `selection_highlight_system.rs`: removed dead `query_selectable` query + unused vars.
- `model_browser_panel.rs`: deleted dead `model_browser_panel_system` (ui/mod.rs:243 defines and registers its own). **Note:** `toggle_model_browser` was NOT deleted — the report claimed it was dead, but grep verified it is called by the registered `model_browser_keyboard_shortcuts`.
- `load_models_system.rs`: deleted `try_load_zsc_from_vfs` (`#[allow(dead_code)]`).
- Kept `model_preview_system` (registered and renders the green preview cube — removing it would change behavior).

### Finding 9 — ifo_types unused members — 92 LOC
`save/ifo_types.rs` 673 → 581:
- Deleted `IFO_MAGIC`, `IFO_VERSION`, `IfoObject::to_bevy_transform`, `IfoBlockType` (+`from_u32`/`to_u32`), `IfoFileData::file_path` field (set but never read; `file_name()` recomputes), `ZoneExportData::populated_block_count`. All verified unreferenced.

### Finding 8 — ifo_export double writers — 201 LOC
`save/ifo_export.rs` 728 → 527:
- Deleted the dead `self.buffer` writer twins (`write_object`, `write_event_object`, `write_warp_object`, `write_sound_object`, `write_effect_object`, `write_npc`, `write_monster_spawn`, `write_water_plane`, instance `write_u8_string`).
- Renamed the `_to_vec` statics to the plain names (against `&mut Vec<u8>`), updated `write_block` call sites.
- Deleted `export_zone_ifo_files` (save_system.rs inlines the identical loop). Kept `ExportStats` (used by save_system.rs).
- Kept `BlockType` enum (used by `write_block`); `IfoBlockType` (its duplicate) removed in Finding 9.
- Trimmed per-block/per-object `log::info!` noise in `write_block`/`write_object` (Finding 11, partial).
- Updated unit tests to exercise the renamed static writers.

### Finding 4 — menu_bar no-op buttons — 185 LOC
`ui/menu_bar.rs` 633 → 448:
- Removed entire Edit menu (Undo/Redo/Cut/Copy/Paste/Delete/Duplicate/Select All/Deselect All were log-only).
- View menu: kept Model Browser toggle; removed Toggle Grid, Snap to Grid, Reset Camera, Frame Selection, Toggle Colliders, Toggle Gizmos (log-only).
- Zone menu: kept Open Zone; removed Zone Info, Validate Zone.
- Object menu: kept Add Water Plane; removed Add Object/Effect/Sound, Delete Selected, Group/Ungroup Selected.
- Removed the now-dead `map_editor_state` param from `editor_menu_bar`/`file_menu` (and its call site in `ui/mod.rs`); removed dead `new_zone_events` param from `file_menu`.
- Updated the Keyboard Shortcuts help window: removed "Ctrl+A - Select all" and "F - Focus" lines (handlers deleted in Finding 5). Mode mapping shown (Q/E/R/V/X) matches the kept `keyboard_shortcuts_system::handle_mode_switches`.

### Finding 6 — world_to_block_coords + HIM/TIL writers dedup — ~65 new / ~50 deleted
- New `map_editor/coords.rs`: `ZONE_CENTER_X/Z`, `BLOCK_SIZE_METERS`, `ZONE_BLOCK_COUNT`, `world_to_block_coords`, `write_him_file`, `write_til_file`.
- `keyboard_shortcuts_system.rs` and `save_system.rs` now import `world_to_block_coords` (byte-identical copies deleted).
- `save_system.rs`'s `write_him_file`/`write_til_file` replaced by the shared ones; `ui/mod.rs`'s `write_default_him`/`write_default_til` wrappers deleted, call site now uses the shared helpers with zero-filled data.
- Remaining inline constants in `model_placement_system.rs` / `map_editor/mod.rs` / `ui/mod.rs` (camera positions, etc.) left as-is — noted as future work.

### Finding 10 — gizmo simplification — 130 LOC
`transform_gizmo_system.rs` 505 → 375:
- Deleted `GizmoAxis` enum + `GizmoDragState.active_axis` (verified never set — drag always took the default path).
- Collapsed `apply_translation` (Free: XZ-plane delta), `apply_rotation` (Y-axis default), `apply_scale` (uniform) — behavior identical to the pre-existing default paths.
- Removed the duplicated `handle_mode_switches` (W=Translate conflict) — the `keyboard_shortcuts_system.rs` one (E/R/Q/V/X) remains.
- Removed the duplicated Ctrl+G snap toggle — the `keyboard_shortcuts_system.rs` G toggle remains.
- Kept `draw_gizmo_visuals` and all gizmo drawing (visual feedback only).
- Removed unused params (`commands`, `keyboard`, `cameras`, `camera_transforms`).

### Finding 13 — misc (partial) — 17 LOC
- Removed dead `MapEditorState::new()` (never called; `Default` used everywhere).
- Removed dead `ZoneListPanelState::new()` (== `Default`).
- `ui/mod.rs` unused imports cleaned (`egui`, `EventObject`, `WarpObject`, `EditorMode`, `ZoneLoaderAsset`).

## Skipped / deferred items

| Item | Reason |
|---|---|
| Finding 2 (EditorAction enum overhaul) | Not in priority list; touches undo semantics beyond the approved double-undo fix; deferred. |
| Finding 7 (shared spawn/material helper with zone_loader) | **Explicitly deferred** by task instructions — no shared helper created in zone_loader; editor-internal dedup of `model_placement_system` vs `duplicate_system` material creation left for a follow-up. |
| Finding 11 (log trimming in save_system.rs, keyboard/menu/properties) | Partial: done in ifo_export.rs. Per-save `log::info!` dumps in save_system.rs left (noise only, zero functional value change). |
| Finding 12 (long function refactors) | Not requested; behavior-neutral refactors only. |
| Finding 13 remaining (hierarchy search/filter stubs, status_bar `get_zone_name`, properties Name/Tag dead fields, `model_browser_panel` wrapper duplication in ui/mod.rs) | All are visible UI elements or require cross-file refactors; removing changes user-facing behavior. Left as future work. |
| Finding 6 leftover inline constants (model_placement_system.rs, map_editor/mod.rs) | Different usage contexts (camera placement, block math); consolidating risks behavior change; noted as future work. |

## Behavior changes (all approved or intentionally preserved)

1. Ctrl+Z/Y now pops exactly one undo step (was two) — Finding 1.
2. W no longer switches to Translate mode (gizmo's handler removed; W remains FreeCamera-forward only) — Finding 10.
3. Snap-to-grid toggle is now `G` only (Ctrl+G duplicate removed) — Finding 10.
4. Escape-deselect handled solely by keyboard_shortcuts_system (selection_system duplicate removed; edge case: Escape while egui captures keyboard no longer deselects) — Finding 5.
5. Ctrl+A and F no longer produce no-op log lines — Finding 5.
6. Removed menu items that only logged (Edit menu, several View/Zone/Object items) — Finding 4.
7. IFO export byte output unchanged — writers were verbatim copies; only logging removed — Finding 8.

## cargo check result

`cargo check` run twice (initial + verification after fixes):
- **Zero errors within `src/map_editor/`** after the fix (the only errors my changes caused were 2 missing `ZONE_CENTER_X/Z` references in `save_system.rs:536`, fixed by importing the constants from `coords.rs`).
- All newly-introduced warnings were fixed (unused imports in grid_system, selection_highlight_system, properties_panel, ui/mod.rs, menu_bar).
- 57 pre-existing errors remain **outside** `src/map_editor/` (e.g. `src/scripting/` Lua4Value, `src/systems/` MessageBoxEvent/MessageWriter, `src/render/` imports, missing `diagnostics` module in `src/lib.rs`, `zone_loader` asset-io traits, `bevy_procedural_grass`). These were present before this cleanup and were not touched per scope rules.
