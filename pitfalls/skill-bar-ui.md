# Skill Bar UI Pitfalls

This document records UI-related issues encountered with the skill bar and hotbar systems.

---

## Skill Bar Drag and Drop Not Working (Fixed 2026-03-15)

### Problem
Skills could be dragged from the skill menu and an outline showed up on the hotbar, but dropping never worked - the skill wouldn't be placed in the hotbar slot.

### Root Cause
System ordering race condition between `ui_drag_and_drop_system` and `ui_hotbar_system`. Both ran in `Update` stage after `EguiPreUpdateSet::InitContexts` with no defined ordering:

1. `ui_drag_and_drop_system` takes `dragged_item` when pointer is released (lines 59-65)
2. `ui_hotbar_system` needs `dragged_item` to be `Some` to detect if a drop is valid (via `accepts_dragged_item`)
3. If `ui_drag_and_drop_system` ran first, it took `dragged_item` before hotbar could process the drop
4. Result: `accepts_dragged_item` was false, drop was never detected

### Solution
In `src/lib.rs`, added `.after()` constraints to ensure `ui_drag_and_drop_system` runs after all UI systems that handle drop targets:
```rust
app.add_systems(
    Update,
    ui_drag_and_drop_system
        .after(bevy_egui::EguiPreUpdateSet::InitContexts)
        .after(ui_hotbar_system)
        .after(ui_inventory_system)
        .after(ui_npc_store_system)
        .after(ui_personal_store_system)
        .after(ui_bank_system)
        .after(ui_skill_list_system)
        .after(ui_skill_tree_system),
);
```

### Files Modified
- `src/lib.rs` - Added system ordering constraints for `ui_drag_and_drop_system`

### Lesson Learned
When multiple systems share mutable resource state (`UiStateDragAndDrop`), ensure proper system ordering so that "consumer" systems (drop targets) run before "cleanup" systems. The cleanup system should always run last to avoid race conditions.

---

## Skill List Window Drag/Misalignment Regression (Fixed 2026-04-09)

### Problem
The skill list dialog (`DLGSKILL`) had multiple regressions:
- skill icons/text detached from the dialog when moving the window,
- icons overflowing beyond the bottom of the window,
- the skill window could not be dragged downward.

### Root Cause
Two issues combined:
1. A global change to [`add_at`](src/ui/widgets/draw.rs:20) altered positioning semantics for all windows, causing unrelated UI drift.
2. In [`ui_skill_list_system`](src/ui/ui_skill_list_system.rs:146), the rendered row count used `SKILL_PAGE_SIZE` instead of the actual dialog listbox height, so content exceeded dialog bounds and influenced drag interaction.

### Solution
- Reverted global [`add_at`](src/ui/widgets/draw.rs:20) behavior to the stable baseline (relative-to-`min_rect` semantics).
- Scoped the fix to [`ui_skill_list_system`](src/ui/ui_skill_list_system.rs:146):
  - compute visible rows from `ZLISTBOX.height / 44.0`,
  - anchor skill content using a stable `window_min`,
  - enforce dialog size with [`fixed_size`](src/ui/ui_skill_list_system.rs:162).

### Files Modified
- `src/ui/widgets/draw.rs`
- `src/ui/ui_skill_list_system.rs`

### Lesson Learned
For ROSE XML dialogs, avoid global coordinate helper changes when only one dialog is broken. Derive visible row count from widget geometry (height/row height), not database/page constants, to keep drag/input bounds aligned with actual window size.
