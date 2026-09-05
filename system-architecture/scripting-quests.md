# Scripting and Quests

Quest logic is data-driven Lua (Lua 4 VM vendored in-tree) plus Bevy glue systems. Plugin: `RoseScriptingPlugin` (`src/scripting/mod.rs`).

## Layout (`src/scripting/`)

- `lua4/{mod,vm,value,instruction,function}.rs`: Lua 4 virtual machine.
- `quest.rs`, `quest_condition_functions.rs`, `quest_reward_functions.rs`, `quest_function_context.rs`: quest evaluation.
- `lua_quest_functions.rs`, `lua_game_functions.rs`, `lua_game_constants.rs`: bindings exposed to scripts.
- `script_function_context.rs`, `script_function_resources.rs`: Bevy bridges.

## Runtime flow

- Systems: `src/systems/{quest_trigger_system,quest_scroll_event_system,systemfunc_event_system}.rs`.
- Events: `src/events/{quest_trigger_event,quest_scroll_event}.rs` (e.g. zone event objects with `quest_trigger_name` write `QuestTriggerEvent::DoTrigger` from `collision_system.rs` proximity queries).
- UI: `src/ui/{ui_quest_list_system,ui_quest_scroll_system}.rs`.
- Databases: quest/skill/item/string tables via `GameData` (`src/resources/game_data.rs`).

Start debugging from `quest_trigger_system.rs` and the `QuestTriggerEvent` writers, not from the VM.
