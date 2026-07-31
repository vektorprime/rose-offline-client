# Cleanup Report — `src/ui/` module (branch: code-simplification)

Implements the findings from `simplification/01-ui.md`. Scope: only files under `src/ui/`.
No files outside `src/ui/` were modified. No boats/sailing UI was touched (`ui_sailing_hud_system.rs` untouched).
`PostProcessingSettings` struct preserved unchanged (used by `src/lib.rs:1590,2554`).
`ui_debug_item_list_system.rs` behavior untouched.

## What was changed per finding

### Finding 1 — tooltips.rs skill-type arms ✅ (biggest win)
- Replaced the 14 near-identical `match skill_data.skill_type` arms (each ~30 lines) with a
  table-driven design: a `match` returning `&[AddSkillSection]` (fn-pointer slice of the differing
  middle `add_skill_*` sections), plus one shared driver that handles name / type-and-target /
  sections / requirements / description / next-level recursion. All 14 arms' call order preserved
  exactly (verified arm-by-arm while writing the table).
- Added `AddSkillSection` type alias (fn pointer with the uniform 4-arg signature); updated
  `add_skill_power`, `add_skill_cast_range`, `add_skill_aoe_range`, `add_skill_recover_xp`,
  `add_skill_steal_ability_value` to the uniform signature with `_player` param.
- Added `get_ability_value(ability_type, player)` helper; the 3 duplicated 14-arg
  `ability_values_get_value(...)` call sites now use it.
- Merged `add_equipment_item_name` + `add_stackable_item_name` into one `add_item_name`.
- Collapsed the 5 identical `ItemClass` arms (EngineFuel/SkillBook/MagicItem/RepairTool/`_`) into a
  plain `if let` (all bodies were identical).
- Extracted `skill_name_text(game_data, skill_data)` shared by `add_skill_name` and
  `add_skill_next_level`.
- Removed now-unused `StackableItem` import.

### Finding 2 — ui_settings_system.rs helpers + page split ✅ (largest absolute win)
- Added `settings_slider(ui, label, value, range, suffix)`, `settings_checkbox(...)`,
  `settings_combo(...)` helpers (single-line per grid row).
- Split the 17-arm `SettingsPage` match into 17 `render_*_page` fns; `ui_settings_system` is now a
  thin dispatcher. `SettingsSystemParams` kept (system param count still exceeds Bevy's limit).
- All rows, ranges, suffixes, tips, clamp blocks, and the GHOSTING DEBUG pages preserved verbatim.
- ComboBoxes that used `from_label("")` now use unique `from_id_salt` ids (behavior equivalent —
  only the internal widget id changed).
- The StarrySkyRender/PostProcessing debug pages were left as-is (they are gated at runtime by the
  page selector; gating behind `cfg(debug_assertions)` was judged a behavior change for debug builds).
- Note: an initial rewrite kept the multi-line call format and came out *longer* (2209 lines); the
  calls were then collapsed to single lines, final 1282 lines.

### Finding 3 — ui_minimap_system.rs icon helper ✅
- Added `draw_minimap_icon(ui, minimap_rect, zoom, pos, icon, tint, hover)`; the players/NPCs/
  monsters blocks now call it (monsters keep the red tint + 4px hover, NPCs keep 6px hover + name,
  players no hover — identical to before).
- Player-arrow rotated quad block simplified with a `rotate` closure (same corner order/UVs).
- Removed the 2 commented-out constants (Finding 12 item).

### Finding 4 — ui_chatbox_system.rs ✅
- 10 event arms reduced to `match -> (String, Color32)` + one `append` call.
- Deleted the verbatim duplicate `/ping` block (dead `else if`).
- Channel-button handlers collapsed into one loop over `[(response, Option<prefix>)]`.

### Finding 11 — ui_admin_menu_system.rs popup/filter merge ✅
- `render_item_spawner_popup` + `render_skill_learn_popup` merged into one
  `render_searchable_popup(ctx, title, grid_id, ui_state, ..., PopupList)`; per-list differences
  handled via a `PopupList::{Items, Skills}` enum (tabs vs separator, filter field, filtered vec,
  update fn, row renderer).
- `update_filtered_items`/`update_filtered_skills` now share `apply_name_filter(filter_text, rows,
  name_fn)`; the two update fns became thin wrappers.
- An earlier closure-based variant was abandoned because simultaneous `&mut` captures of
  `UiStateAdminMenu` conflicted at the call site; the enum dispatch avoids the borrow issue.

### Finding 5 — ui_number_input_dialog_system.rs ✅
- 10 digit-button blocks collapsed into one `[(&mut response, digit); 10]` loop.
- Modal blocker extracted into `draw_modal_blocker(ctx, id)` in `ui_message_box_system.rs`; both
  message box and number input systems use it (identical 16-line block removed from both).

### Finding 6 — widgets field boilerplate (PARTIAL / mostly skipped)
- Tried making `LoadWidget::load_widget` a default no-op to delete the 6 empty impls — **failed**:
  trait method resolution still requires the impl to exist for the `Widget` dispatch match
  (`&mut Caption::load_widget` etc. → E0599). Reverted; the empty impls were restored and the trait
  kept its original signature.
- **Skipped** the `WidgetCommon` + `#[serde(flatten)]` refactor: the XML deserializer is
  quick-xml's serde layer and flatten support cannot be verified safely; changing 18 structs would
  ripple through every field access site (dialog.rs, drag_and_drop_slot.rs, message box, etc.) —
  too risky for this pass. The `widget_to_rect!` macro is retained.
- Removed the dead `GetWidget` impls (always-`None`) from `widgets/draw_text.rs` and
  `widgets/draw_widget.rs` (verified zero call sites).

### Finding 7 — widgets/mod.rs dispatch (PARTIAL)
- `get_widget`/`get_widget_mut` on `Vec<Widget>` collapsed: the 16-variant `continue` lists became
  `_ => continue` (the `Unknown => panic!` arm was unreachable — `id()` panics first). Container
  recursion (Pane/TabbedPane/Skill) unchanged.
- **Skipped** the `widget_match!` macro for `id()`/`draw_widget`/`load_widget`: verified with
  `rustc` that modern Rust rejects macros expanding to match arms ("macros cannot expand to match
  arms"), so the one-list goal is impossible with `macro_rules!`. Kept the 3 explicit matches.
- Cleaned the dead commented-out diagnostics in the `Vec<Widget>` draw/load impls (unused
  `index`/`discriminant`/`widget_count` variables).

### Finding 8 — slot grids (tooltip part only)
- Added `tooltip_on_hover(response, game_data, player, item)` in `tooltips.rs` (re-exported from
  `ui::mod.rs`); used at ui_inventory (was `if item.is_some() { ... }`), ui_bank, ui_quest_list.
- **Skipped** the shared `ui_grid_slots` helper: the 6 grids are not actually the same pattern —
  inventory has a `Grid`+`end_row()` nested loop, bank/hotbar use flat loops with `slot % ROW`
  indexing (no grid), quest_list uses a fixed position array, npc_store/personal_store use nested
  loops *without* `end_row`. Only 2 sites share the exact idiom, and the slot functions have
  10+ parameter heterogeneous signatures — a shared helper would not save meaningful lines.
- npc_store/personal_store/hotbar tooltip sites kept inline (they append extra price/status labels
  inside the same `on_hover_ui` closure, so they are not pure duplicates).

### Finding 9 — drag-drop accept predicates ✅
- 8 micro-fns replaced by one `drag_accepts(page, allow_bank, drag_source)`; the call site computes
  `(page, allow_bank)` from the slot and passes a `move` closure.
- Required changing `DragAndDropSlot.accepts` from `fn(&DragAndDropId) -> bool` to
  `Box<dyn Fn(&DragAndDropId) -> bool>` (with `impl Fn + 'static` constructor params). All other
  call sites (hotbar, bank, npc_store, quest_list, personal_store) still pass plain fn pointers —
  no changes needed there.

### Finding 10 — window boilerplate ✅
- Added `Dialog::window(title)` builder (`frame(none) + title_bar(false) + resizable(false) +
  default_width/height` from the dialog). 11 call sites updated: inventory, npc_store ×2, bank,
  hotbar, skill_list, skill_tree, personal_store, login, server_select, game_menu.
- Skipped: chatbox (uses `.frame(fill)` + anchor), minimap (fully custom), character_select
  (screen-size based, not dialog sized).

### Finding 12 — dead code and diagnostics ✅ (partial)
- `dialog_loader.rs`: removed `static mut DIALOG_LOAD_COUNT`/`DIALOG_LOAD_BYTES` + all `unsafe`
  blocks; removed the every-100-loads `log::warn!`; simplified the `AssetEvent` match; removed the
  dead widget-type logging loop and unused counters in `load_dialog_sprites_system`.
- `ui_skill_tree_system.rs`: removed the empty `if response.double_clicked() { /* no-op */ }`.
- `ui_minimap_system.rs`: removed 2 commented-out constants (see Finding 3).
- **Skipped** `ui_party_system.rs` gauge `&0.5, "50%"` and `format!("Party Level: {}", 1)`:
  removing them removes visible UI content, and no real party-XP data exists in the protocol
  (`PartyMemberInfoOnline` has HP/stamina/concentration but no XP) — a behavior change, not a
  simplification. Left as-is.
- **Skipped** chatbox 13 × `visible: false` bindings: whether the XML widgets default to visible
  is not verifiable from source; removing could change rendering.
- Caption/RadioBox empty `draw_widget` bodies: cannot be removed from the `Widget` enum (they
  appear in the XML schema; removing would deserialize them as `Unknown` and panic in `id()`).
  Left as-is.

### Finding 13 — dialog OK-callback plumbing
Skipped. Making `NumberInputDialogEvent`/`MessageBoxEvent` carry `MessageWriter`-style payloads
changes the event API and every producer site; estimated gain (~40-50 lines) did not justify the
cross-cutting risk in this pass.

### Finding 14 — data_bindings linear scans
Nothing to do — the report itself recommends keeping the binding model; a getter macro was deemed
low priority. Not attempted.

## LOC removed per file (before → after)

| File | Before | After | Delta |
|---|---|---|---|
| ui_settings_system.rs | 2140 | 1282 | -858 |
| tooltips.rs | 1603 | 1270 | -333 |
| ui_chatbox_system.rs | 599 | 500 | -99 |
| widgets/mod.rs | 362 | 297 | -65 |
| dialog_loader.rs | 164 | 109 | -55 |
| ui_number_input_dialog_system.rs | 295 | 250 | -45 |
| ui_inventory_system.rs | 736 | 701 | -35 |
| ui_minimap_system.rs | 966 | 946 | -20 |
| ui_admin_menu_system.rs | 691 | 677 | -14 |
| widgets/draw_text.rs | 72 | 62 | -10 |
| widgets/draw_widget.rs | 66 | 56 | -10 |
| ui_skill_tree_system.rs | 294 | 287 | -7 |
| ui_npc_store_system.rs | 719 | 713 | -6 |
| ui_hotbar_system.rs | 377 | 373 | -4 |
| ui_personal_store_system.rs | 352 | 348 | -4 |
| ui_login_system.rs | 176 | 172 | -4 |
| ui_server_select_system.rs | 144 | 140 | -4 |
| ui_game_menu_system.rs | 202 | 198 | -4 |
| ui_bank_system.rs | 251 | 249 | -2 |
| ui_skill_list_system.rs | 387 | 385 | -2 |
| ui_quest_list_system.rs | 312 | 310 | -2 |
| widgets/dialog.rs | 115 | 124 | +9 (new `Dialog::window`) |
| ui_message_box_system.rs | 314 | 318 | +4 (shared `draw_modal_blocker`) |
| ui/mod.rs | 138 | 140 | +2 (re-export) |
| **Total** | | | **-1568** |

Files with a 0 delta (drag_and_drop_slot.rs, button.rs, editbox.rs, listbox.rs, table.rs,
zlistbox.rs, caption.rs, radio_box.rs) were touched but ended at the same line count.

Module total: ~18,215 → ~16,647 LOC (-8.6%).

## cargo check result

Final `cargo check` (dev profile): **Finished — 0 errors** in the whole crate.
`src/ui/` contributes no errors or new warnings; the remaining warnings in `src/ui/` are
pre-existing (deprecated egui API usage, unused imports/params that predate this change).

Notes:
- `cargo check` was run several times during the work (each edit batch followed by a check); all
  errors found were in `src/ui/` files I own and were fixed. Mid-flight runs showed errors in
  `src/scripting/` (`lua4/vm.rs`, `lua_game_constants.rs`); those did NOT reproduce in the final
  check (final run: Finished, 0 errors) — they appeared to be cascade artifacts of the broken
  intermediate state.
- `cargo build` was not run (per rules). No git commands were run; nothing was committed.

## Items skipped (with reasons)

1. **WidgetCommon serde-flatten** (Finding 6): unverifiable against the quick-xml XML deserializer;
   ripples across every widget field access site.
2. **LoadWidget default body / empty-impl deletion** (Finding 6): trait dispatch requires impls.
3. **widget_match! macro** (Finding 7): impossible — `macro_rules!` cannot expand to match arms.
4. **ui_grid_slots helper** (Finding 8): grids are not actually uniform (see above).
5. **Party fake data** (Finding 12): removing visible UI is a behavior change; no real data exists.
6. **Chatbox 13 hidden widgets** (Finding 12): XML default visibility not verifiable.
7. **Caption/RadioBox no-op draws** (Finding 12): cannot remove from enum (XML schema + id() panic).
8. **Finding 13 dialog plumbing**: cross-cutting API change, low value/risk ratio.
9. **StarrySkyRender/PostProcessing debug pages cfg-gated** (Finding 2 suggestion): left as-is to
   avoid changing debug-build behavior.
