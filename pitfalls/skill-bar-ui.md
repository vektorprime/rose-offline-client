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
