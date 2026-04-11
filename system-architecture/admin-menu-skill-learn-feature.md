# Admin Menu Skill Learn Feature - Implementation Plan

## Overview
Add a menu option to the admin menu (F10) that allows players to learn skills on-demand, similar to the existing item spawner popup.

## Server-Side Command Analysis

The server already supports skill learning via the `/skill` command:
- **Command format**: `/skill add <skill_id>` or `/skill remove <skill_id>`
- **Location**: [`../rose-offline/rose-offline-server/src/game/systems/chat_commands_system.rs`](../rose-offline/rose-offline-server/src/game/systems/chat_commands_system.rs:949)
- **Implementation**: The command directly adds/removes skills from the player's skill list without checking requirements (admin bypass)

## Client-Side Implementation Plan

### 1. Update `UiStateAdminMenu` Resource
**File**: [`src/ui/ui_admin_menu_system.rs`](src/ui/ui_admin_menu_system.rs:18)

Add new fields to track skill popup state:
```rust
pub struct UiStateAdminMenu {
    // ... existing fields ...
    
    // Skill popup state
    pub show_skill_popup: bool,
    pub skill_search_filter: String,
    pub filtered_skills: Vec<SkillId>,
}
```

Update `Default` implementation accordingly.

### 2. Add Skill Popup Button
**Location**: In the admin menu UI, add a button similar to the item spawner:

```rust
// In the "Spawning" or new "Skills" section
if ui.button("📜 Learn Skill (Popup)").clicked() {
    ui_state_admin_menu.show_skill_popup = true;
}
```

### 3. Create Skill Popup Renderer
**Function**: `render_skill_learn_popup()`

Similar to `render_item_spawner_popup()`, this will:
- Display a scrollable list of all skills from `game_data.skills.iter()`
- Show skill icon, ID, name, and type
- Include a search filter for skill names
- "Learn" button that sends `/skill add <id>` command
- "Remove" button that sends `/skill remove <id>` command

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

The skill popup should mirror the item spawner popup structure:
1. Search filter text field with "Clear" button
2. Scrollable table with columns:
   - Icon (with tooltip on hover)
   - ID
   - Name
   - Type
   - Action (Learn/Remove buttons)

## Testing Checklist

- [ ] Popup opens when clicking "Learn Skill (Popup)" button
- [ ] Search filter correctly filters skills by name
- [ ] Skill icons display correctly
- [ ] "Learn" button sends correct `/skill add <id>` command
- [ ] "Remove" button sends correct `/skill remove <id>` command
- [ ] Popup can be closed
- [ ] No performance issues with large skill lists

## Files to Modify

1. [`src/ui/ui_admin_menu_system.rs`](src/ui/ui_admin_menu_system.rs) - Main implementation

## Estimated Complexity
**Low-Medium** - This is a straightforward addition that mirrors the existing item spawner popup functionality. The server-side command already exists and works.
