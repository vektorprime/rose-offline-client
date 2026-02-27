# Event → Message Migration Analysis for Bevy 0.17

## Executive Summary

This document provides a comprehensive analysis of all event types in the ROSE Offline Client codebase that need to be migrated for Bevy 0.17's Event → Message refactoring.

**Total Events Found: 37**
- Events in [`src/events/`](src/events/mod.rs): 30
- Events in other locations: 7

**Recommendation:** ALL 37 events should be migrated to `Message` (buffered events). None appear to use the observer pattern.

---

## Migration Overview

In Bevy 0.17, "buffered events" have been renamed to "messages":

| Old (0.16) | New (0.17) |
|------------|------------|
| `Event` trait | `Message` trait |
| `EventWriter<E>` | `MessageWriter<M>` |
| `EventReader<E>` | `MessageReader<M>` |
| `Events<E>` | `Messages<M>` |
| `app.add_event::<E>()` | `app.add_message::<M>()` |
| `events.send()` | `messages.write()` |
| `world.send_event()` | `world.write_message()` |
| `commands.send_event()` | `commands.write_message()` |

**IMPORTANT:** The `Event` trait still exists in 0.17 but is ONLY for "observable events" used with observers.

---

## Part 1: Event Definition Changes

### 1.1 Events in [`src/events/mod.rs`](src/events/mod.rs:1)

All events in this directory need the `#[derive(Event)]` changed to `#[derive(Message)]`:

| File | Event Type | Line | Current Derive |
|------|------------|------|----------------|
| [`bank_event.rs`](src/events/bank_event.rs:5) | `BankEvent` | 5 | `#[derive(Event)]` |
| [`blood_effect_event.rs`](src/events/blood_effect_event.rs:14) | `BloodEffectEvent` | 14 | `#[derive(Event, Reflect, Clone, Debug)]` |
| [`character_select_event.rs`](src/events/character_select_event.rs:3) | `CharacterSelectEvent` | 3 | `#[derive(Event)]` |
| [`chat_bubble_event.rs`](src/events/chat_bubble_event.rs:4) | `ChatBubbleEvent` | 4 | `#[derive(Event, Reflect)]` |
| [`chatbox_event.rs`](src/events/chatbox_event.rs:3) | `ChatboxEvent` | 3 | `#[derive(Event)]` |
| [`clan_dialog_event.rs`](src/events/clan_dialog_event.rs:3) | `ClanDialogEvent` | 3 | `#[derive(Event)]` |
| [`client_entity_event.rs`](src/events/client_entity_event.rs:3) | `ClientEntityEvent` | 3 | `#[derive(Event, Copy, Clone, Debug)]` |
| [`conversation_dialog_event.rs`](src/events/conversation_dialog_event.rs:5) | `ConversationDialogEvent` | 5 | `#[derive(Event)]` |
| [`flight_event.rs`](src/events/flight_event.rs:4) | `FlightToggleEvent` | 4 | `#[derive(Event, Clone, Debug)]` |
| [`game_connection_event.rs`](src/events/game_connection_event.rs:5) | `GameConnectionEvent` | 5 | `#[derive(Event)]` |
| [`hit_event.rs`](src/events/hit_event.rs:5) | `HitEvent` | 5 | `#[derive(Event)]` |
| [`login_event.rs`](src/events/login_event.rs:3) | `LoginEvent` | 3 | `#[derive(Event)]` |
| [`message_box_event.rs`](src/events/message_box_event.rs:3) | `MessageBoxEvent` | 3 | `#[derive(Event)]` |
| [`move_destination_effect_event.rs`](src/events/move_destination_effect_event.rs:3) | `MoveDestinationEffectEvent` | 3 | `#[derive(Event)]` |
| [`move_speed_event.rs`](src/events/move_speed_event.rs:4) | `MoveSpeedSetEvent` | 4 | `#[derive(Event, Clone, Debug)]` |
| [`network_event.rs`](src/events/network_event.rs:3) | `NetworkEvent` | 3 | `#[derive(Event)]` |
| [`npc_store_event.rs`](src/events/npc_store_event.rs:5) | `NpcStoreEvent` | 5 | `#[derive(Event)]` |
| [`number_input_dialog_event.rs`](src/events/number_input_dialog_event.rs:3) | `NumberInputDialogEvent` | 3 | `#[derive(Event)]` |
| [`party_event.rs`](src/events/party_event.rs:3) | `PartyEvent` | 3 | `#[derive(Event)]` |
| [`personal_store_event.rs`](src/events/personal_store_event.rs:6) | `PersonalStoreEvent` | 6 | `#[derive(Event)]` |
| [`player_command_event.rs`](src/events/player_command_event.rs:8) | `PlayerCommandEvent` | 8 | `#[derive(Event, Clone)]` |
| [`quest_trigger_event.rs`](src/events/quest_trigger_event.rs:5) | `QuestTriggerEvent` | 5 | `#[derive(Event)]` |
| [`spawn_effect_event.rs`](src/events/spawn_effect_event.rs:37) | `SpawnEffectEvent` | 37 | `#[derive(Event)]` |
| [`spawn_projectile_event.rs`](src/events/spawn_projectile_event.rs:7) | `SpawnProjectileEvent` | 7 | `#[derive(Event)]` |
| [`system_func_event.rs`](src/events/system_func_event.rs:5) | `SystemFuncEvent` | 5 | `#[derive(Event, Clone)]` |
| [`use_item_event.rs`](src/events/use_item_event.rs:5) | `UseItemEvent` | 5 | `#[derive(Event)]` |
| [`world_connection_event.rs`](src/events/world_connection_event.rs:5) | `WorldConnectionEvent` | 5 | `#[derive(Event)]` |
| [`zone_event.rs`](src/events/zone_event.rs:8) | `LoadZoneEvent` | 8 | `#[derive(Event)]` |
| [`zone_event.rs`](src/events/zone_event.rs:23) | `ZoneEvent` | 23 | `#[derive(Event)]` |
| [`zone_event.rs`](src/events/zone_event.rs:30) | `ZoneLoadedFromVfsEvent` | 30 | `#[derive(Event, Clone)]` |

### 1.2 Events in Other Locations

| File | Event Type | Line | Current Derive |
|------|------------|------|----------------|
| [`src/animation/animation_state.rs`](src/animation/animation_state.rs:11) | `AnimationFrameEvent` | 11 | `#[derive(Event)]` |
| [`src/ui/ui_sound_event_system.rs`](src/ui/ui_sound_event_system.rs:11) | `UiSoundEvent` | 11 | `#[derive(Event)]` |
| [`src/components/fish.rs`](src/components/fish.rs:91) | `WaterSpawnedEvent` | 91 | `#[derive(Event, Debug, Clone)]` |
| [`src/map_editor/systems/property_update_system.rs`](src/map_editor/systems/property_update_system.rs:15) | `PropertyChangeEvent` | 15 | `#[derive(Event, Debug, Clone)]` |
| [`src/map_editor/ui/mod.rs`](src/map_editor/ui/mod.rs:96) | `NewZoneEvent` | 96 | `#[derive(Event)]` |
| [`src/map_editor/resources.rs`](src/map_editor/resources.rs:12) | `DuplicateSelectedEvent` | 12 | `#[derive(Event, Debug, Clone)]` |
| [`src/map_editor/save/save_system.rs`](src/map_editor/save/save_system.rs:22) | `SaveZoneEvent` | 22 | `#[derive(Event, Debug, Clone)]` |

---

## Part 2: Import Statement Changes

### 2.1 Files Using `EventWriter`

Replace `EventWriter` with `MessageWriter` in the following files:

| File | Line(s) | Event Types Used |
|------|---------|------------------|
| [`src/zone_loader.rs`](src/zone_loader.rs:1173) | 1173, 1249-1250 | `WaterSpawnedEvent`, `ZoneEvent`, `ZoneLoadedFromVfsEvent` |
| [`src/ui/widgets/data_bindings.rs`](src/ui/widgets/data_bindings.rs:42) | 42 | `UiSoundEvent` |
| [`src/ui/ui_window_sound_system.rs`](src/ui/ui_window_sound_system.rs:11) | 11 | `UiSoundEvent` |
| [`src/ui/ui_skill_tree_system.rs`](src/ui/ui_skill_tree_system.rs:161) | 161 | `UiSoundEvent` |
| [`src/ui/ui_skill_list_system.rs`](src/ui/ui_skill_list_system.rs:68) | 68, 121-122 | `PlayerCommandEvent`, `UiSoundEvent` |
| [`src/ui/ui_server_select_system.rs`](src/ui/ui_server_select_system.rs:24) | 24, 29-30 | `UiSoundEvent`, `LoginEvent` |
| [`src/ui/ui_respawn_system.rs`](src/ui/ui_respawn_system.rs:22) | 22 | `UiSoundEvent` |
| [`src/ui/ui_quest_list_system.rs`](src/ui/ui_quest_list_system.rs:93) | 93 | `UiSoundEvent` |
| [`src/ui/ui_player_info_system.rs`](src/ui/ui_player_info_system.rs:94) | 94 | `UiSoundEvent` |
| [`src/ui/ui_personal_store_system.rs`](src/ui/ui_personal_store_system.rs:60) | 60, 126, 134-135 | `MessageBoxEvent`, `UiSoundEvent`, `NumberInputDialogEvent` |
| [`src/ui/ui_party_system.rs`](src/ui/ui_party_system.rs:78) | 78 | `UiSoundEvent` |
| [`src/ui/ui_party_option_system.rs`](src/ui/ui_party_option_system.rs:47) | 47 | `UiSoundEvent` |
| [`src/ui/ui_number_input_dialog_system.rs`](src/ui/ui_number_input_dialog_system.rs:54) | 54 | `UiSoundEvent` |
| [`src/ui/ui_npc_store_system.rs`](src/ui/ui_npc_store_system.rs:93) | 93, 390, 400-402 | `NumberInputDialogEvent`, `UiSoundEvent`, `MessageBoxEvent` |
| [`src/ui/ui_minimap_system.rs`](src/ui/ui_minimap_system.rs:92) | 92 | `UiSoundEvent` |
| [`src/ui/ui_message_box_system.rs`](src/ui/ui_message_box_system.rs:42) | 42 | `UiSoundEvent` |
| [`src/ui/ui_login_system.rs`](src/ui/ui_login_system.rs:33) | 33, 39-40 | `UiSoundEvent`, `AppExit`, `LoginEvent` |
| [`src/ui/ui_inventory_system.rs`](src/ui/ui_inventory_system.rs:227) | 227, 458, 464-465 | `PlayerCommandEvent`, `UiSoundEvent`, `NumberInputDialogEvent` |
| [`src/ui/ui_hotbar_system.rs`](src/ui/ui_hotbar_system.rs:76) | 76, 202, 205 | `PlayerCommandEvent`, `UiSoundEvent` |
| [`src/ui/ui_game_menu_system.rs`](src/ui/ui_game_menu_system.rs:37) | 37 | `UiSoundEvent`, `CharacterSelectEvent` |
| [`src/ui/ui_drag_and_drop_system.rs`](src/ui/ui_drag_and_drop_system.rs:20) | 20-21 | `PlayerCommandEvent`, `NpcStoreEvent` |
| [`src/ui/ui_debug_zone_list_system.rs`](src/ui/ui_debug_zone_list_system.rs:35) | 35 | `LoadZoneEvent` |
| [`src/ui/ui_debug_effect_list.rs`](src/ui/ui_debug_effect_list.rs:34) | 34 | `SpawnEffectEvent` |
| [`src/ui/ui_create_clan.rs`](src/ui/ui_create_clan.rs:52) | 52, 57 | `UiSoundEvent`, `MessageBoxEvent` |
| [`src/ui/ui_clan_system.rs`](src/ui/ui_clan_system.rs:66) | 66 | `UiSoundEvent` |
| [`src/ui/ui_chatbox_system.rs`](src/ui/ui_chatbox_system.rs:88) | 88-89 | `FlightToggleEvent`, `MoveSpeedSetEvent` |
| [`src/ui/ui_character_select_system.rs`](src/ui/ui_character_select_system.rs:46) | 46 | `UiSoundEvent`, `CharacterSelectEvent` |
| [`src/ui/ui_character_info_system.rs`](src/ui/ui_character_info_system.rs:74) | 74 | `UiSoundEvent` |
| [`src/ui/ui_character_create_system.rs`](src/ui/ui_character_create_system.rs:90) | 90 | `UiSoundEvent` |
| [`src/ui/ui_bank_system.rs`](src/ui/ui_bank_system.rs:57) | 57, 109, 119 | `PlayerCommandEvent`, `UiSoundEvent` |
| [`src/systems/character_select_system.rs`](src/systems/character_select_system.rs:167) | 167, 414 | `LoadZoneEvent`, `CharacterSelectEvent` |
| [`src/systems/client_entity_event_system.rs`](src/systems/client_entity_event_system.rs:22) | 22-23 | `ChatboxEvent`, `SpawnEffectEvent` |
| [`src/systems/collision_system.rs`](src/systems/collision_system.rs:145) | 145 | `QuestTriggerEvent` |
| [`src/systems/flight_command_system.rs`](src/systems/flight_command_system.rs:23) | 23 | `FlightToggleEvent` |
| [`src/systems/game_mouse_input_system.rs`](src/systems/game_mouse_input_system.rs:47) | 47 | `PlayerCommandEvent` |
| [`src/systems/gash_wound_system.rs`](src/systems/gash_wound_system.rs:26) | 26 | `BloodEffectEvent` |
| [`src/systems/login_system.rs`](src/systems/login_system.rs:21) | 21, 118-119 | `LoadZoneEvent`, `NetworkEvent` |
| [`src/systems/monster_chatter_system.rs`](src/systems/monster_chatter_system.rs:16) | 16 | `ChatBubbleEvent` |
| [`src/systems/move_speed_command_system.rs`](src/systems/move_speed_command_system.rs:36) | 36 | `MoveSpeedSetEvent` |
| [`src/systems/login_connection_system.rs`](src/systems/login_connection_system.rs:21) | 21 | `NetworkEvent` |
| [`src/systems/hit_event_system.rs`](src/systems/hit_event_system.rs:86) | 86 | `SpawnEffectEvent` |
| [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs:159) | 159-168 | `ChatboxEvent`, `ChatBubbleEvent`, `GameConnectionEvent`, `LoadZoneEvent`, `UseItemEvent`, `ClientEntityEvent`, `PartyEvent`, `PersonalStoreEvent`, `QuestTriggerEvent`, `MessageBoxEvent` |
| [`src/systems/command_system.rs`](src/systems/command_system.rs:345) | 345-347 | `ConversationDialogEvent`, `ClientEntityEvent`, `PersonalStoreEvent` |
| [`src/systems/pending_skill_effect_system.rs`](src/systems/pending_skill_effect_system.rs:148) | 148 | `HitEvent` |
| [`src/scripting/script_function_context.rs`](src/scripting/script_function_context.rs:46) | 46-50 | `BankEvent`, `ChatboxEvent`, `ClanDialogEvent`, `NpcStoreEvent`, `SystemFuncEvent` |
| [`src/systems/player_command_system.rs`](src/systems/player_command_system.rs:51) | 51 | `ChatboxEvent` |
| [`src/systems/projectile_system.rs`](src/systems/projectile_system.rs:16) | 16 | `HitEvent` |
| [`src/systems/world_connection_system.rs`](src/systems/world_connection_system.rs:17) | 17-18 | `NetworkEvent`, `WorldConnectionEvent` |
| [`src/systems/visible_status_effects_system.rs`](src/systems/visible_status_effects_system.rs:21) | 21 | `SpawnEffectEvent` |
| [`src/systems/use_item_event_system.rs`](src/systems/use_item_event_system.rs:21) | 21 | `SpawnEffectEvent` |
| [`src/systems/spawn_projectile_system.rs`](src/systems/spawn_projectile_system.rs:21) | 21 | `SpawnEffectEvent` |
| [`src/systems/systemfunc_event_system.rs`](src/systems/systemfunc_event_system.rs:8) | 8 | `ConversationDialogEvent` |
| [`src/systems/blood_spatter_system.rs`](src/systems/blood_spatter_system.rs:24) | 24 | `BloodEffectEvent` |
| [`src/systems/auto_login_system.rs`](src/systems/auto_login_system.rs:23) | 23-24 | `LoginEvent`, `CharacterSelectEvent` |
| [`src/systems/animation_effect_system.rs`](src/systems/animation_effect_system.rs:31) | 31-33 | `SpawnEffectEvent`, `SpawnProjectileEvent`, `HitEvent` |
| [`src/map_editor/ui/menu_bar.rs`](src/map_editor/ui/menu_bar.rs:26) | 26, 28 | `SaveZoneEvent`, `NewZoneEvent` |
| [`src/map_editor/ui/properties_panel.rs`](src/map_editor/ui/properties_panel.rs:122) | 122-123 | `PropertyChangeEvent`, `DuplicateSelectedEvent` |
| [`src/map_editor/ui/zone_list_panel.rs`](src/map_editor/ui/zone_list_panel.rs:60) | 60 | `LoadZoneEvent` |
| [`src/map_editor/ui/mod.rs`](src/map_editor/ui/mod.rs:119) | 119, 124-127 | `SaveZoneEvent`, `PropertyChangeEvent`, `DuplicateSelectedEvent`, `NewZoneEvent` |
| [`src/map_editor/systems/keyboard_shortcuts_system.rs`](src/map_editor/systems/keyboard_shortcuts_system.rs:26) | 26 | `DuplicateSelectedEvent` |
| [`src/animation/skeletal_animation.rs`](src/animation/skeletal_animation.rs:40) | 40 | `AnimationFrameEvent` |

### 2.2 Files Using `EventReader`

Replace `EventReader` with `MessageReader` in the following files:

| File | Line(s) | Event Types Used |
|------|---------|------------------|
| [`src/zone_loader.rs`](src/zone_loader.rs:1248) | 1248, 1746 | `LoadZoneEvent`, `ZoneLoadedFromVfsEvent` |
| [`src/ui/ui_sound_event_system.rs`](src/ui/ui_sound_event_system.rs:25) | 25 | `UiSoundEvent` |
| [`src/ui/ui_personal_store_system.rs`](src/ui/ui_personal_store_system.rs:127) | 127 | `PersonalStoreEvent` |
| [`src/ui/ui_party_system.rs`](src/ui/ui_party_system.rs:83) | 83 | `PartyEvent` |
| [`src/ui/ui_npc_store_system.rs`](src/ui/ui_npc_store_system.rs:391) | 391 | `NpcStoreEvent` |
| [`src/ui/ui_create_clan.rs`](src/ui/ui_create_clan.rs:56) | 56 | `ClanDialogEvent` |
| [`src/ui/ui_chatbox_system.rs`](src/ui/ui_chatbox_system.rs:83) | 83 | `ChatboxEvent` |
| [`src/ui/ui_bank_system.rs`](src/ui/ui_bank_system.rs:112) | 112 | `BankEvent` |
| [`src/ui/dialog_loader.rs`](src/ui/dialog_loader.rs:107) | 107 | `AssetEvent<Dialog>` |
| [`src/systems/character_select_system.rs`](src/systems/character_select_system.rs:165) | 165-166 | `GameConnectionEvent`, `WorldConnectionEvent` |
| [`src/systems/character_select_system.rs`](src/systems/character_select_system.rs:326) | 326 | `CharacterSelectEvent` |
| [`src/systems/chat_bubble_spawn_system.rs`](src/systems/chat_bubble_spawn_system.rs:40) | 40 | `ChatBubbleEvent` |
| [`src/systems/client_entity_event_system.rs`](src/systems/client_entity_event_system.rs:21) | 21 | `ClientEntityEvent` |
| [`src/systems/conversation_dialog_system.rs`](src/systems/conversation_dialog_system.rs:370) | 370 | `ConversationDialogEvent` |
| [`src/systems/fish_system.rs`](src/systems/fish_system.rs:21) | 21 | `WaterSpawnedEvent` |
| [`src/systems/flight_toggle_system.rs`](src/systems/flight_toggle_system.rs:15) | 15 | `FlightToggleEvent` |
| [`src/systems/free_camera_system.rs`](src/systems/free_camera_system.rs:52) | 52-53 | `MouseMotion`, `MouseWheel` |
| [`src/systems/game_system.rs`](src/systems/game_system.rs:58) | 58 | `ZoneEvent` |
| [`src/systems/hit_event_system.rs`](src/systems/hit_event_system.rs:85) | 85 | `HitEvent` |
| [`src/systems/login_system.rs`](src/systems/login_system.rs:116) | 116 | `LoginEvent` |
| [`src/systems/name_tag_system.rs`](src/systems/name_tag_system.rs:400) | 400 | `LoadZoneEvent` |
| [`src/systems/network_thread_system.rs`](src/systems/network_thread_system.rs:19) | 19 | `NetworkEvent` |
| [`src/systems/orbit_camera_system.rs`](src/systems/orbit_camera_system.rs:78) | 78-79 | `MouseMotion`, `MouseWheel` |
| [`src/systems/quest_trigger_system.rs`](src/systems/quest_trigger_system.rs:12) | 12 | `QuestTriggerEvent` |
| [`src/systems/spawn_effect_system.rs`](src/systems/spawn_effect_system.rs:37) | 37 | `SpawnEffectEvent` |
| [`src/systems/spawn_projectile_system.rs`](src/systems/spawn_projectile_system.rs:17) | 17 | `SpawnProjectileEvent` |
| [`src/systems/systemfunc_event_system.rs`](src/systems/systemfunc_event_system.rs:7) | 7 | `SystemFuncEvent` |
| [`src/systems/wing_spawn_system.rs`](src/systems/wing_spawn_system.rs:51) | 51 | `FlightToggleEvent` |
| [`src/systems/use_item_event_system.rs`](src/systems/use_item_event_system.rs:20) | 20 | `UseItemEvent` |
| [`src/systems/player_command_system.rs`](src/systems/player_command_system.rs:28) | 28 | `PlayerCommandEvent` |
| [`src/systems/pending_skill_effect_system.rs`](src/systems/pending_skill_effect_system.rs:147) | 147 | `AnimationFrameEvent` |
| [`src/systems/move_speed_set_system.rs`](src/systems/move_speed_set_system.rs:12) | 12 | `MoveSpeedSetEvent` |
| [`src/systems/move_destination_effect_system.rs`](src/systems/move_destination_effect_system.rs:28) | 28 | `MoveDestinationEffectEvent` |
| [`src/systems/gash_wound_system.rs`](src/systems/gash_wound_system.rs:77) | 77 | `BloodEffectEvent` |
| [`src/systems/blood_spatter_system.rs`](src/systems/blood_spatter_system.rs:50) | 50 | `BloodEffectEvent` |
| [`src/systems/bird_system.rs`](src/systems/bird_system.rs:45) | 45 | `ZoneEvent` |
| [`src/systems/animation_sound_system.rs`](src/systems/animation_sound_system.rs:81) | 81 | `AnimationFrameEvent` |
| [`src/systems/animation_effect_system.rs`](src/systems/animation_effect_system.rs:30) | 30 | `AnimationFrameEvent` |
| [`src/map_editor/systems/duplicate_system.rs`](src/map_editor/systems/duplicate_system.rs:44) | 44 | `DuplicateSelectedEvent` |
| [`src/map_editor/systems/property_update_system.rs`](src/map_editor/systems/property_update_system.rs:98) | 98 | `PropertyChangeEvent` |
| [`src/map_editor/systems/load_models_system.rs`](src/map_editor/systems/load_models_system.rs:293) | 293 | `ZoneEvent` |
| [`src/map_editor/ui/mod.rs`](src/map_editor/ui/mod.rs:198) | 198 | `NewZoneEvent` |
| [`src/map_editor/save/save_system.rs`](src/map_editor/save/save_system.rs:151) | 151 | `SaveZoneEvent` |

---

## Part 3: Resource Usage Changes

### 3.1 `Events<T>` → `Messages<T>`

Replace `Events<T>` with `Messages<T>` in the following files:

| File | Line(s) | Usage |
|------|---------|-------|
| [`src/ui/ui_personal_store_system.rs`](src/ui/ui_personal_store_system.rs:100) | 100 | `world.get_resource_mut::<Events<PersonalStoreEvent>>()` |
| [`src/ui/ui_number_input_dialog_system.rs`](src/ui/ui_number_input_dialog_system.rs:56) | 56 | `ResMut<Events<NumberInputDialogEvent>>` |
| [`src/ui/ui_npc_store_system.rs`](src/ui/ui_npc_store_system.rs:159) | 159 | `world.resource_mut::<Events<NpcStoreEvent>>()` |
| [`src/ui/ui_message_box_system.rs`](src/ui/ui_message_box_system.rs:44) | 44 | `ResMut<Events<MessageBoxEvent>>` |
| [`src/ui/ui_inventory_system.rs`](src/ui/ui_inventory_system.rs:662) | 662 | `world.get_resource_mut::<Events<PlayerCommandEvent>>()` |
| [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs:725) | 725, 807-808, 1249, 1794, 1821, 1838, 1864, 1883, 1931, 1967, 1996, 2022, 2083, 2100, 2263, 2275, 2294 | Multiple `world.resource_mut::<Events<...>>()` calls |

### 3.2 `.send()` → `.write()`

The `.send()` method on `Events<T>` (which becomes `Messages<T>`) needs to change to `.write()`:

| File | Line(s) | Current Code Pattern |
|------|---------|---------------------|
| [`src/ui/ui_personal_store_system.rs`](src/ui/ui_personal_store_system.rs:102) | 102 | `personal_store_events.send(...)` |
| [`src/ui/ui_npc_store_system.rs`](src/ui/ui_npc_store_system.rs:160) | 160 | `npc_store_events.send(...)` |
| [`src/ui/ui_inventory_system.rs`](src/ui/ui_inventory_system.rs:664) | 664 | `player_command_events.send(...)` |
| [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs:725) | 725, 807, 812, 1249, 1794, 1821, 1838, 1864, 1883, 1932, 1967, 1996, 2022, 2083, 2100, 2101, 2109, 2264, 2276, 2295 | Multiple `.send(...)` calls |

---

## Part 4: App Registration Changes

### 4.1 Main Registration in [`src/lib.rs`](src/lib.rs:955)

Replace all `.add_event::<E>()` with `.add_message::<M>()`:

```rust
// Current (lines 955-984)
app.add_event::<BankEvent>()
    .add_event::<ChatBubbleEvent>()
    .add_event::<ChatboxEvent>()
    .add_event::<CharacterSelectEvent>()
    .add_event::<ClanDialogEvent>()
    .add_event::<ClientEntityEvent>()
    .add_event::<ConversationDialogEvent>()
    .add_event::<FlightToggleEvent>()
    .add_event::<GameConnectionEvent>()
    .add_event::<HitEvent>()
    .add_event::<LoginEvent>()
    .add_event::<LoadZoneEvent>()
    .add_event::<MessageBoxEvent>()
    .add_event::<MoveDestinationEffectEvent>()
    .add_event::<MoveSpeedSetEvent>()
    .add_event::<NetworkEvent>()
    .add_event::<NumberInputDialogEvent>()
    .add_event::<NpcStoreEvent>()
    .add_event::<PartyEvent>()
    .add_event::<PersonalStoreEvent>()
    .add_event::<PlayerCommandEvent>()
    .add_event::<QuestTriggerEvent>()
    .add_event::<SystemFuncEvent>()
    .add_event::<SpawnEffectEvent>()
    .add_event::<SpawnProjectileEvent>()
    .add_event::<UseItemEvent>()
    .add_event::<WorldConnectionEvent>()
    .add_event::<ZoneEvent>()
    .add_event::<ZoneLoadedFromVfsEvent>()
    .add_event::<UiSoundEvent>();
```

### 4.2 Plugin Registration Locations

| File | Line | Current Code |
|------|------|--------------|
| [`src/animation/mod.rs`](src/animation/mod.rs:42) | 42 | `app.add_event::<AnimationFrameEvent>()` |
| [`src/blood_effect_plugin.rs`](src/blood_effect_plugin.rs:64) | 64 | `app.add_event::<BloodEffectEvent>()` |
| [`src/systems/fish_system.rs`](src/systems/fish_system.rs:447) | 447 | `.add_event::<WaterSpawnedEvent>()` |
| [`src/map_editor/systems/property_update_system.rs`](src/map_editor/systems/property_update_system.rs:609) | 609 | `.add_event::<PropertyChangeEvent>()` |
| [`src/map_editor/ui/mod.rs`](src/map_editor/ui/mod.rs:68) | 68-69 | `.add_event::<PropertyChangeEvent>()` and `.add_event::<NewZoneEvent>()` |
| [`src/map_editor/systems/duplicate_system.rs`](src/map_editor/systems/duplicate_system.rs:32) | 32 | `app.add_event::<DuplicateSelectedEvent>()` |
| [`src/map_editor/save/save_system.rs`](src/map_editor/save/save_system.rs:142) | 142 | `app.add_event::<SaveZoneEvent>()` |

---

## Part 5: Special Cases

### 5.1 Bevy Built-in Events

The following Bevy built-in events also need migration:

| Event | Location | Notes |
|-------|----------|-------|
| `AppExit` | [`src/ui/ui_login_system.rs:40`](src/ui/ui_login_system.rs:40) | Built-in, may have different migration |
| `AssetEvent<Dialog>` | [`src/ui/dialog_loader.rs:107`](src/ui/dialog_loader.rs:107) | Built-in asset event |
| `MouseMotion` | [`src/systems/free_camera_system.rs:52`](src/systems/free_camera_system.rs:52), [`src/systems/orbit_camera_system.rs:78`](src/systems/orbit_camera_system.rs:78) | Built-in input event |
| `MouseWheel` | [`src/systems/free_camera_system.rs:53`](src/systems/free_camera_system.rs:53), [`src/systems/orbit_camera_system.rs:79`](src/systems/orbit_camera_system.rs:79) | Built-in input event |

**Note:** Bevy's built-in events like `AppExit`, `AssetEvent`, `MouseMotion`, and `MouseWheel` may have their own migration path. Check Bevy 0.17 documentation for these specific types.

### 5.2 Events with Reflect Derive

The following events also derive `Reflect` and may need additional consideration:

| Event | File | Line |
|-------|------|------|
| `BloodEffectEvent` | [`src/events/blood_effect_event.rs:14`](src/events/blood_effect_event.rs:14) | `#[derive(Event, Reflect, Clone, Debug)]` |
| `ChatBubbleEvent` | [`src/events/chat_bubble_event.rs:4`](src/events/chat_bubble_event.rs:4) | `#[derive(Event, Reflect)]` |

These should become `#[derive(Message, Reflect, ...)]` if reflection is still needed.

---

## Part 6: Migration Checklist

### Phase 1: Event Definitions (37 files)

- [ ] [`src/events/bank_event.rs`](src/events/bank_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/blood_effect_event.rs`](src/events/blood_effect_event.rs:14) - Change `Event` to `Message`
- [ ] [`src/events/character_select_event.rs`](src/events/character_select_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/chat_bubble_event.rs`](src/events/chat_bubble_event.rs:4) - Change `Event` to `Message`
- [ ] [`src/events/chatbox_event.rs`](src/events/chatbox_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/clan_dialog_event.rs`](src/events/clan_dialog_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/client_entity_event.rs`](src/events/client_entity_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/conversation_dialog_event.rs`](src/events/conversation_dialog_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/flight_event.rs`](src/events/flight_event.rs:4) - Change `Event` to `Message`
- [ ] [`src/events/game_connection_event.rs`](src/events/game_connection_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/hit_event.rs`](src/events/hit_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/login_event.rs`](src/events/login_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/message_box_event.rs`](src/events/message_box_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/move_destination_effect_event.rs`](src/events/move_destination_effect_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/move_speed_event.rs`](src/events/move_speed_event.rs:4) - Change `Event` to `Message`
- [ ] [`src/events/network_event.rs`](src/events/network_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/npc_store_event.rs`](src/events/npc_store_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/number_input_dialog_event.rs`](src/events/number_input_dialog_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/party_event.rs`](src/events/party_event.rs:3) - Change `Event` to `Message`
- [ ] [`src/events/personal_store_event.rs`](src/events/personal_store_event.rs:6) - Change `Event` to `Message`
- [ ] [`src/events/player_command_event.rs`](src/events/player_command_event.rs:8) - Change `Event` to `Message`
- [ ] [`src/events/quest_trigger_event.rs`](src/events/quest_trigger_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/spawn_effect_event.rs`](src/events/spawn_effect_event.rs:37) - Change `Event` to `Message`
- [ ] [`src/events/spawn_projectile_event.rs`](src/events/spawn_projectile_event.rs:7) - Change `Event` to `Message`
- [ ] [`src/events/system_func_event.rs`](src/events/system_func_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/use_item_event.rs`](src/events/use_item_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/world_connection_event.rs`](src/events/world_connection_event.rs:5) - Change `Event` to `Message`
- [ ] [`src/events/zone_event.rs`](src/events/zone_event.rs:8) - Change `Event` to `Message` (3 types)
- [ ] [`src/animation/animation_state.rs`](src/animation/animation_state.rs:11) - Change `Event` to `Message`
- [ ] [`src/ui/ui_sound_event_system.rs`](src/ui/ui_sound_event_system.rs:11) - Change `Event` to `Message`
- [ ] [`src/components/fish.rs`](src/components/fish.rs:91) - Change `Event` to `Message`
- [ ] [`src/map_editor/systems/property_update_system.rs`](src/map_editor/systems/property_update_system.rs:15) - Change `Event` to `Message`
- [ ] [`src/map_editor/ui/mod.rs`](src/map_editor/ui/mod.rs:96) - Change `Event` to `Message`
- [ ] [`src/map_editor/resources.rs`](src/map_editor/resources.rs:12) - Change `Event` to `Message`
- [ ] [`src/map_editor/save/save_system.rs`](src/map_editor/save/save_system.rs:22) - Change `Event` to `Message`

### Phase 2: Import Statements (~60+ files)

- [ ] Replace all `EventWriter` with `MessageWriter`
- [ ] Replace all `EventReader` with `MessageReader`
- [ ] Replace all `Events<T>` with `Messages<T>`
- [ ] Update `use bevy::prelude::Event` to `use bevy::prelude::Message`

### Phase 3: Method Calls

- [ ] Replace all `.send()` calls with `.write()`
- [ ] Replace all `world.send_event()` with `world.write_message()`
- [ ] Replace all `commands.send_event()` with `commands.write_message()`

### Phase 4: App Registration

- [ ] [`src/lib.rs`](src/lib.rs:955) - Replace all 30 `.add_event::<E>()` with `.add_message::<M>()`
- [ ] [`src/animation/mod.rs`](src/animation/mod.rs:42) - Replace `.add_event::<AnimationFrameEvent>()`
- [ ] [`src/blood_effect_plugin.rs`](src/blood_effect_plugin.rs:64) - Replace `.add_event::<BloodEffectEvent>()`
- [ ] [`src/systems/fish_system.rs`](src/systems/fish_system.rs:447) - Replace `.add_event::<WaterSpawnedEvent>()`
- [ ] [`src/map_editor/systems/property_update_system.rs`](src/map_editor/systems/property_update_system.rs:609) - Replace `.add_event::<PropertyChangeEvent>()`
- [ ] [`src/map_editor/ui/mod.rs`](src/map_editor/ui/mod.rs:68) - Replace 2 `.add_event()` calls
- [ ] [`src/map_editor/systems/duplicate_system.rs`](src/map_editor/systems/duplicate_system.rs:32) - Replace `.add_event::<DuplicateSelectedEvent>()`
- [ ] [`src/map_editor/save/save_system.rs`](src/map_editor/save/save_system.rs:142) - Replace `.add_event::<SaveZoneEvent>()`

---

## Edge Cases and Complications

### 1. Box<dyn FnOnce> in Events

[`MessageBoxEvent`](src/events/message_box_event.rs:3) and [`NumberInputDialogEvent`](src/events/number_input_dialog_event.rs:3) contain `Box<dyn FnOnce(&mut Commands) + Send + Sync>`. These should still work as Messages but verify this compiles correctly.

### 2. World Resource Access Pattern

Several files access events via `world.resource_mut::<Events<T>>()` and then call `.send()`. This pattern becomes:
```rust
// Before
world.resource_mut::<Events<MyEvent>>().send(MyEvent::...);

// After  
world.resource_mut::<Messages<MyEvent>>().write(MyEvent::...);
```

### 3. Conditional Event Sending

In [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs:725), there are many instances of:
```rust
let _ = world.resource_mut::<Events<ChatboxEvent>>().send(...);
```

These should become:
```rust
let _ = world.resource_mut::<Messages<ChatboxEvent>>().write(...);
```

---

## Summary Statistics

| Category | Count |
|----------|-------|
| Total Events | 37 |
| Event Definition Files | 37 |
| Files with EventWriter | ~55 |
| Files with EventReader | ~45 |
| Files with Events<T> resource | 7 |
| App Registration Sites | 8 |
| Total Files Needing Changes | ~70 |

---

## Next Steps

1. **Backup the codebase** before making changes
2. **Run tests** to establish baseline
3. **Migrate in phases** as outlined in the checklist
4. **Compile after each phase** to catch errors early
5. **Run tests** after all phases complete
