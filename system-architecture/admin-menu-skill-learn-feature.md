# Admin Menu Skill Learn Feature - Implementation Plan

## Overview
Add a menu option to the admin menu (F10) that allows players to learn skills on-demand, similar to the existing item spawner popup.

**Status: Implemented** — this plan describes the completed implementation in `src/ui/ui_admin_menu_system.rs`.

## Server-Side Command Analysis

The server already supports skill learning via the `/skill` command:
- **Command format**: `/skill add <skill_id>` or `/skill remove <skill_id>`
- **Location**: [`../rose-offline/rose-offline-server/src/game/systems/chat_commands_system.rs`](../rose-offline/rose-offline-server/src/game/systems/chat_commands_system.rs:957)
- **Implementation**: The command directly adds/removes skills from the player's skill list without checking requirements (admin bypass)

## Client-Side Implementation Plan

### 1. Update `UiStateAdminMenu` Resource
**File**: [`src/ui/ui_admin_menu_system.rs`](src/ui/ui_admin_menu_system.rs:19)

New fields track skill popup state:
```rust
pub struct UiStateAdminMenu {
    // ... existing fields ...
    
    // Skill popup state
    pub show_skill_popup: bool,
    pub skill_search_filter: String,
    filtered_skills: Vec<SkillId>,
}
```

The `Default` implementation was updated accordingly.

### 2. Add Skill Popup Button
**Location**: In the admin menu UI, add a button similar to the item spawner:

```rust
// In the "Spawning" or new "Skills" section
if ui.button("📜 Learn Skill (Popup)").clicked() {
    ui_state_admin_menu.show_skill_popup = true;
}
```

### 3. Create Skill Popup Renderer
**Functions**: `render_searchable_popup()` (shared popup renderer, dispatched via `PopupList::Skills`) and `render_skill_popup_rows()` (skill rows)

`render_skill_popup_rows()`:
- Displays a scrollable list of filtered skills from `game_data.skills.iter()`
- Shows skill icon, ID, name, and a "Learn" button
- Includes a search filter for skill names
- "Learn" button sends `/skill add <id>` command

### 4. Create Skill Filter Function
**Function**: `update_filtered_skills()`

Similar to `update_filtered_items()`, filters skills based on search text.

## Dependencies

### Required Imports
```rust
use rose_data::SkillId;
```

### Existing Resources Used
- `GameData::skills` - Provides `iter()` and `get_skill()` methods
- `UiResources` - Provides skill sprite icons via `get_sprite_by_index(UiSpriteSheetType::Skill, icon_number)`
- `GameConnection` - Sends client messages to server

## UI Layout Reference

The skill popup mirrors the item spawner popup structure:
1. Search filter text field with "Clear" button
2. Scrollable table with columns:
   - Icon
   - ID
   - Name
   - Action (Learn button)

## Testing Checklist

- [ ] Popup opens when clicking "Learn Skill (Popup)" button
- [ ] Search filter correctly filters skills by name
- [ ] Skill icons display correctly
- [ ] "Learn" button sends correct `/skill add <id>` command
- [ ] (Not implemented) "Remove" button for `/skill remove <id>` — the server supports removal, but no UI button exists
- [ ] Popup can be closed
- [ ] No performance issues with large skill lists

## Files to Modify

1. [`src/ui/ui_admin_menu_system.rs`](src/ui/ui_admin_menu_system.rs) - Main implementation

## Estimated Complexity
**Low-Medium** - This is a straightforward addition that mirrors the existing item spawner popup functionality. The server-side command already exists and works.
