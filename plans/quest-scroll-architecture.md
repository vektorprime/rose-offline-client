# QuestScroll Usage Implementation Architecture

## Executive Summary

QuestScroll items ([`ItemClass::QuestScroll`](../rose-offline/rose-data/src/item_database.rs:267)) are consumable items that trigger quests when used. The current implementation logs a TODO message and skips processing (see [`player_command_system.rs` line 450](src/systems/player_command_system.rs:450)). This document details the architecture for implementing full QuestScroll functionality.

---

## Current Item Usage Pipeline

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         CLIENT SIDE                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Player clicks item                                            │
│       │                                                        │
│       ▼                                                        │
│  ┌──────────────────────────────────────┐                     │
│  │  player_command_system               │                     │
│  │  (src/systems/player_command_system.rs)                    │
│  └──────────────┬───────────────────────┘                     │
│                 │                                              │
│    ┌────────────┼────────────┐                                 │
│    │            │            │                                 │
│    ▼            ▼            ▼                                 │
│  Check:      Check:       Check:                              │
│  RepairTool   QuestScroll   MagicItem                         │
│               (TODO!)                                │
│                                    │                                              │
│                           If not skipped:                    │
│                               │                              │
│                               ▼                              │
│                   ┌─────────────────────┐                   │
│                   │ ClientMessage::     │                   │
│                   │ UseItem {{          │                   │
│                   │   item_slot,        │                   │
│                   │   target_entity_id  │                  │
│                   │ }}                    │                   │
│                   └──────────┬────────────┘                   │
│                              │                                │
│                        [NETWORK]                              │
│                              │                                │
└──────────────────────────────┼───────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────┴─────────────────────────────────┐
│                         SERVER SIDE                             │
│  (rose-offline-server - processes quest triggers, item use)   │
└──────────────────────────────┬─────────────────────────────────┘
                               │
                               ▼
┌──────────────────────────────┴─────────────────────────────────┐
│                         CLIENT SIDE (RESPONSE)                 │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Server sends back:                                             │
│   - ServerMessage::UseItem                                     │
│   - ServerMessage::QuestTriggerResult (if applicable)           │
│   - ServerMessage::RewardItems/RewardMoney (rewards)            │
│                 │                                              │
│                 ▼                                              │
│          ┌───────────────────────┐                           │
│          │ game_connection_system  │                           │
│          │ (src/systems/           │                           │
│          │ game_connection_system.rs)                         │
│          └──────────┬──────────────┘                          │
│                     │                                         │
│         ┌───────────┼──────────┐                              │
│         │                   │                                 │
│         ▼                   ▼                                │
│  UseItemEvent       QuestTriggerEvent                       │
│  (for visual)      ApplyRewards                             │
│                                                                 │
│                     │                                          │
│                     ▼                                         │
│              quest_trigger_system                            │
│                 │                                            │
│                 ├──────────┬───────────                      │
│                 ▼          ▼                                 │
│           Check      Apply Rewards                          │
│           Conditions                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## QuestScroll Data Structure Analysis

### Key Finding: Confile Index

The quest trigger information for QuestScroll items is NOT stored directly in [`ConsumableItemData`](../rose-offline/rose-data/src/item_database.rs:421). Instead, it references a **configuration file** via the `confile_index` field:

```rust
pub struct ConsumableItemData {
    pub item_data: BaseItemData,
    pub store_skin: i32,
    pub confile_index: usize,      // <-- Points to config file!
    pub add_fuel: i32,
    // ... other fields (no quest_trigger_name!)
}
```

### Data Sources

| Field | Source | Description |
 240|
|-------|--------|-------------|
| `item_data.class` | STB column ~15 | [`ItemClass::QuestScroll`](../rose-offline/rose-data/src/item_database.rs:267) (value 316) |
| `confile_index` | STB column 22 | Index into confile database containing quest trigger info |

---

## Required Implementation Steps

### Step 1: Add QuestTrigger Event Variant for Item Use

**File:** [`src/events/quest_trigger_event.rs`](src/events/quest_trigger_event.rs)

Currently:
```rust
pub enum QuestTriggerEvent {
    ApplyRewards(QuestTriggerHash),
    DoTrigger(QuestTriggerHash),
}
```

Add new variant for item-triggered quests that may need to open a dialog first:
```rust
pub enum QuestTriggerEvent {
    ApplyRewards(QuestTriggerHash),
    DoTrigger(QuestTriggerHash),
    UseQuestScroll(ItemReference, QuestTriggerHash),  // NEW: Shows dialog before trigger
}
```

---

### Step 2: Modify Player Command System to Handle QuestScroll

**File:** [`src/systems/player_command_system.rs`](src/systems/player_command_system.rs)

Current TODO at line 445-451:
```rust
if matches!(consumable_item_data.item_data.class, ItemClass::QuestScroll) {
    // TODO: This should open a dialog
    log::info!("TODO: Implement using ItemClass::QuestScroll");
    continue;
}
```

Replace with implementation that:
1. Reads quest trigger hash from confile (via `confile_index`)
2. Opens quest scroll dialog UI
3. On confirm, dispatches `QuestTriggerEvent::UseQuestScroll`
4. Sends `ClientMessage::UseItem` to server

---

### Step 3: Create Quest Scroll Dialog UI Component

**New File:** `src/ui/quest_scroll_dialog.rs`

Requirements:
- Modal dialog showing quest information (title, description)
- Accept/Decline buttons
- Passes result back via event or closure

Example structure:
```rust
pub struct QuestScrollDialog {
    pub trigger_hash: QuestTriggerHash,
    pub item_reference: ItemReference,
    // UI state handled by egui
}
```

---

### Step 4: Update UseItemEvent System for Visual Effects

**File:** [`src/systems/use_item_event_system.rs`](src/systems/use_item_event_system.rs)

Currently only handles `ItemType::Consumable`. Add handling for visual effects specific to QuestScroll:
```rust
// After checking item_type == Consumable
if let Some(consumable) = game_data.items.get_consumable_item(item.item_number) {
    if matches!(consumable.item_data.class, ItemClass::QuestScroll) {
        // Spawn scroll usage animation/effect
        // Play appropriate sound
    }
}
```

---

### Step 5: Add Confile Parsing for Quest Triggers

**File:** Depends on where confiles are parsed (likely in `rose-data-irose` or `rose-file-readers`)

The confile contains quest trigger information. Need to:
1. Parse the confile format (likely text-based config file)
2. Extract quest trigger name/hash from confile at `confile_index`
3. Cache mapping: `confile_index -> QuestTriggerHash`

---

## Message Types Involved

### Client → Server Messages

| Message | Location | Purpose |
---------|----------|----------|
| [`ClientMessage::UseItem`](src/protocol/irose/game_client.rs:1276) | game_client.rs:1276 | Use a consumable item (including QuestScroll) |
| [`ClientMessage::QuestTrigger`](src/protocol/irose/game_client.rs:1217) | game_client.rs:1217 | Trigger quest (sent after conditions checked client-side) |

### Server → Client Messages

| Message | Location | Purpose |
---------|----------|----------|
| [`ServerMessage::UseItem`](src/protocol/irose/game_client.rs:647) | game_client.rs:647 | Notify clients of item usage (visual) |
| [`ServerMessage::QuestTriggerResult`](src/protocol/irose/game_client.rs:542) | game_client.rs:542 | Result of quest trigger attempt |

---

## Files Requiring Modification

| File | Change Type | Description |
------|-------------|-------------|
| [`src/events/quest_trigger_event.rs`](src/events/quest_trigger_event.rs) | Modify | Add `UseQuestScroll` variant |
| [`src/systems/player_command_system.rs`](src/systems/player_command_system.rs) | Modify | Replace TODO with QuestScroll handling |
| [`src/systems/use_item_event_system.rs`](src/systems/use_item_event_system.rs) | Modify | Add visual effects for QuestScroll |
| `src/ui/quest_scroll_dialog.rs` | New | Quest scroll modal dialog UI |
| `src/ui/mod.rs` | Modify | Export new dialog module |
| [`src/lib.rs`](src/lib.rs) | Modify | Register quest scroll systems if needed |

---

## Dependencies Analysis

### Item System
- ✅ Inventory management already handles consumable items
- ✅ [`ItemReference::consumable()`](../rose-offline/rose-data/src/item_database.rs:88) creates references
- ⚠️ Need confile → quest trigger mapping (not implemented)

### Quest System  
- ✅ [`QuestTriggerHash`](src/events/quest_trigger_event.rs) type exists for identifying triggers
- ✅ [`quest_check_conditions()`](src/scripting/quest.rs:13) validates quest requirements
- ✅ [`quest_apply_rewards()`](src/scripting/quest.rs:65) grants quest rewards
- ⚠️ Quest scroll UI dialog missing (needs implementation)

### Network System
- ✅ `ClientMessage::UseItem` sends item usage to server  
- ✅ `ClientMessage::QuestTrigger` triggers quests on server
- ✅ Response handling for quest results exists

---

## C++ Client Reference Implementation

For the complete reference implementation, consult the C++ client at:
```
C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client
```

Key files to examine:
1. `src/game/item_use.cpp` - Item usage handling
2. `src/ui/quest_scroll_dialog.cpp` (if exists) - Quest scroll UI
3. Server-side: How quest scrolls are processed

---

## Implementation Recommendations

### Recommended Order of Operations

1. **Priority 1:** Add confile parsing to map `confile_index → QuestTriggerHash`
2. **Priority 2:** Modify [`player_command_system.rs`](src/systems/player_command_system.rs) to dispatch quest scroll event instead of TODO
3. **Priority 3:** Create quest scroll dialog UI component
4. **Priority 4:** Add visual effects in [`use_item_event_system.rs`](src/systems/use_item_event_system.rs)

### Testing Strategy

1. Find a QuestScroll item ID from game data (class = ItemClass::QuestScroll)
2. Test that clicking the item opens dialog instead of logging TODO
3. Verify quest trigger hash is correctly read from confile  
4. Confirm server receives UseItem message and processes quest trigger
5. Check rewards are granted on successful trigger

---

## Summary

QuestScroll implementation requires:
1. **Data layer:** Parse config files (confiles) to extract quest trigger info
2. **UI layer:** Modal dialog for accepting/declining quest
3. **Event layer:** New event variant or use existing `DoTrigger`  
4. **Visual layer:** Scroll usage animations in existing [`use_item_event_system`](src/systems/use_item_event_system.rs)

The architecture leverages existing quest infrastructure - the main gaps are confile parsing and UI dialog.
