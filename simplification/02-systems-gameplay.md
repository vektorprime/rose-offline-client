# Simplification Plan — `src/systems/` (Gameplay & Networking half)

**PLAN ONLY — no code changes were made during this analysis.**

## Module and LOC analyzed

| File | LOC |
|---|---|
| game_connection_system.rs | 2916 |
| command_system.rs | 1151 |
| player_command_system.rs | 959 |
| conversation_dialog_system.rs | 730 |
| character_select_system.rs | 523 |
| chat_command_system.rs | 410 |
| game_mouse_input_system.rs | 205 |
| login_system.rs | 180 |
| game_keyboard_input_system.rs | 140 |
| login_connection_system.rs | 131 |
| client_entity_event_system.rs | 125 |
| auto_login_system.rs | 120 |
| network_thread_system.rs | 110 |
| world_connection_system.rs | 110 |
| use_item_event_system.rs | 109 |
| game_system.rs | 103 |
| ping_command_system.rs | 100 |
| move_speed_command_system.rs | 72 |
| systemfunc_event_system.rs | 62 |
| quest_trigger_system.rs | 57 |
| quest_scroll_event_system.rs | 44 |
| clan_system.rs | 44 |
| move_speed_set_system.rs | 30 |
| mod.rs (registration only) | 251 |
| **Total** | **~8,630** |

Estimated removable/simplifiable: **~1,400–1,700 LOC (~18%)** without behavior change.

---

## Finding 1 — `game_connection_system.rs`: one giant 2,900-line message dispatcher mixing ~70 concerns

`src/systems/game_connection_system.rs:182-2904` — a single `loop { try_recv() → match }` handles every `ServerMessage` domain in one function: character/entity spawning, movement/teleport, combat damage, stats/leveling, inventory/equipment, skills/cooldowns, party (13 variants), clan (8 variants), bank, personal store, quests, chat, emotes, sit, reconnect logic.

Concrete problems:
- 14 system parameters (`game_connection_system.rs:158-175`) that every handler must thread through.
- 64 `commands.queue(move |world: &mut World| { ... })` closures (`rg -c "queue(move"` = 64), most following the identical shape: *"if entity exists → queue closure → entity_mut → get_mut::<X> → mutate"*.
- 48 `log::info!`/`warn!`/`error!` calls, the majority `[DIAG_*]` debug noise (see Finding 6).
- Entity lookup idioms (`client_entity_list.get(id)` + `commands.entity(entity).insert(...)`) repeated inline dozens of times.

**Suggestion:** split into per-domain free functions with a shared params struct, e.g.:
```
GameConnectionSystemParams<'w,'s> { commands, game_data, client_entity_list, chatbox, chat_bubble, ... }  // one struct instead of 14 params
handle_character_spawn(params, msg)   // CharacterData + SpawnEntity*
handle_movement(params, msg)          // MoveEntity, AdjustPosition, Teleport, MoveToggle, StopMoveEntity
handle_combat(params, msg)            // DamageEntity, ApplySkillEffect, UpdateHealth/ManaPoints
handle_player_stats(params, msg)      // UpdateAbility*, UpdateBasicStat, UpdateLevel, UpdateSpeed
handle_inventory(params, msg)         // UpdateInventory/Money/Equipment/Ammo/VehiclePart/ItemLife, Pickup*, Reward*, Bank*
handle_party(params, msg)             // 13 party arms
handle_clan(params, msg)              // 8 clan arms
handle_skills(params, msg)            // LearnSkill*, CastSkill*, cooldowns
```
Each fn is then `Ok(msg) => handle_x(&mut params, msg)` — one line in the dispatcher.

**Estimated savings:** −300 lines of structural boilerplate; rest is reorganization. Biggest maintainability win; the actual *line* savings come from Findings 2–4 inside it.

---

## Finding 2 — Entity spawn duplication: 4 handlers share ~90% identical code

`game_connection_system.rs:368-462` (`SpawnEntityCharacter`), `463-548` (`SpawnEntityNpc`), `549-658` (`SpawnEntityMonster`), `659-704` (`SpawnEntityItemDrop`).

Every handler does, near-verbatim:
1. `StatusEffects { active, ..Default::default() }` wrapper construction (3x).
2. `get_spawn_height_from_world(world, x, y)` + `commands.queue(move |world| ...)` deferral.
3. `world.spawn((...)).id()` split into **two** inserts with the comment *"avoid tuple size limit"* (3x).
4. The identical visual boilerplate bundle: `Transform::from_xyz(x/100.0, spawn_y, -y/100.0), GlobalTransform::default(), Visibility::default(), InheritedVisibility::default(), ViewVisibility::default()` (4x).
5. `world.resource_mut::<ClientEntityList>().add(entity_id, entity)` (4x).

Only differences: entity-type tag, extra components (NPC rotation via `Quat::from_axis_angle`, monster `Equipment`/`MonsterSeparation`, personal store/clan membership), and the item-drop name lookup.

**Suggestion:** one `fn spawn_client_entity(world: &mut World, config: SpawnEntityConfig) -> Entity` where `SpawnEntityConfig { entity_id, entity_type, position, rotation, core_bundle, extra_components }`, plus a named tuple alias for the visual bundle (e.g. `type SpawnTransformBundle = (Transform, GlobalTransform, Visibility, InheritedVisibility, ViewVisibility)`).

**Estimated savings:** −150 to −200 lines.

---

## Finding 3 — Level-up / ability-recalc duplication (includes a duplicated bug)

`game_connection_system.rs:1276-1333` (`UpdateLevel`) and `1334-1386` (`LevelUpEntity`) are near-identical:
- same `world.resource_scope` + 5-component destructure (`BasicStats, CharacterInfo, Equipment, SkillList, StatusEffects`),
- same `ability_value_calculator.calculate(...)` call,
- same HP/MP refill:
```
health_points.hp = ability_values.get_max_health();
mana_points.mp = ability_values.get_max_health();   // BUG: should be get_max_mana() — copied twice
```

**Suggestion:** extract `fn recalculate_ability_values_and_refill(world, entity, game_data)`; fix the `max_mana` bug in the single shared function. Note `UpdateLevel` also already emits `ClientEntityEvent::LevelUp` (line 1284), so `LevelUpEntity` could arguably be reduced to the event + level increment.

**Estimated savings:** −60 lines + one bug fixed.

---

## Finding 4 — `command_system.rs`: giant 835-line system with 9 near-identical motion helpers

`command_system.rs:316-1151` is one function. Inside it:
- `get_attack_animation` / `get_die_animation` / `get_move_animation` / `get_sitting_animation` / `get_sit_animation` / `get_standing_animation` / `get_stop_animation` / `get_pickup_animation` / `get_vehicle_action_animation` (`33-252`) all reduce to the same idiom: `if model.action_motions[action].is_strong() { Some(handle.clone()) } else { None }`. Could be one generic helper or a small macro.
- `update_active_motion` (`273-296`) is invoked ~15 times with identical `(&mut commands.entity(X), &mut active_motion, motion, speed, repeat)` argument shapes.
- The `Command::Stop` branch (`618-644`) duplicates the idle-animation block already executed at `575-598`; vehicle-motion updates duplicated verbatim at `630-640` and `902-912`.
- `CastSkill` casting state machine (`476-561` + `1013-1148`) — 4-state hand-rolled transition (Casting → CastingRepeat → Action) — most complex section.
- `QueryAttackTarget` struct (`310-314`) is **dead code** — declared, never used.
- Dead commented block at `412-418` (disabled DIAGNOSTIC logging).

**Suggestion:** extract per-command handlers (`handle_move`, `handle_attack`, `handle_cast_skill`, `handle_emote`, `handle_sit`) taking a context struct (entity, models, motion refs, game_data, asset_server, ...); add `fn set_motion_pair(...)` that updates driver-model + vehicle-model animations together; delete `QueryAttackTarget`.

**Estimated savings:** −250 to −350 lines.

---

## Finding 5 — `player_command_system.rs`: the 90-line `SkillTargetFilter` match duplicated twice

`player_command_system.rs:310-395` (skill targeting) is copied verbatim at `529-620` (consumable magic-item targeting). Both build `target_is_alive / target_is_caster` and the same 14-arm `SkillTargetFilter` match with nested clan/party lookups.

**Suggestion:** extract
```
fn is_valid_skill_target(
    filter: SkillTargetFilter,
    target: (Entity, Option<&CharacterInfo>, &ClientEntity, &Command, &Team),
    player_entity: Entity, player_team_id: u16,
    player_party: Option<&PartyInfo>, player_clan: Option<&Clan>,
) -> bool
```
and call it from both sites.

Also minor: `EquipAmmo`/`EquipEquipment`/`EquipVehicle` (`658-763`) share the same *item → index → `Change*` message* shape; `DropItem` and `DropItemWithQuantity` (`797-821`) are duplicates.

**Estimated savings:** −90 to −130 lines.

---

## Finding 6 — Dead code, commented-out blocks, and debug logging (quick wins)

1. **game_connection_system.rs**:
   - Commented-out `[DIAG_MONSTER_SPAWN]` blocks: `560-563`, `616`, `714-719`, `926-943`, `971`.
   - `StartCastingSkill` handler body is literally `// Nah bruv` (`1969-1971`) — either implement or fold into a catch-all.
   - Six "unimplemented" arms that only `log::warn!` (`2863-2880`: `CraftInsertGem`, `CraftInsertGemError`, `RepairedItemUsingNpc`, `LogoutSuccess`, `LogoutFailed`, `ReturnToCharacterSelect`) — collapse into one `Ok(other_unimplemented) => warn` catch-all.
   - Heavy diagnostic `log::info!` in hot paths: `314-320`, `336-340`, `362-363`, `474-483`, `814-871` (7 `info!` calls inside the `DamageEntity` handler alone, including inside the queued closure per hit).
   - `148`, `177`: commented-out logs.
2. **command_system.rs `412-418`**: commented-out DIAGNOSTIC block. `QueryAttackTarget` dead struct (`310-314`).
3. **player_command_system.rs `240-247`**: commented-out `SkillBasicCommand` arms.
4. **use_item_event_system.rs `85-107`**: entire `if let Some(apply_status_effect) ... else if let Some(add_ability)` is dead — first branch has only a comment ("Authority migrated to server"), second branch is a commented-out TODO block. The system reduces to effects + sound; remove ~25 lines and the now-unused `StatusEffects`/`StatusEffectsRegen` query params.
5. **quest_trigger_system.rs `18-20`**: `ApplyRewards` arm is an empty stub (server-authoritative) — delete arm (and check `quest_apply_rewards` import at line 7 becomes unused).
6. **chat_command_system.rs `240-245`**: `as_client_message()` returns `""` placeholder, zero callers — delete.
7. **move_speed_command_system.rs `35-42`**: the system function is an empty stub (`let _ = (move_speed_events, player_query);`) and is **not registered** in `src/lib.rs` (only `move_speed_set_system` is, line 1725). Delete the fn; keep `parse_move_speed_command` + tests.
8. **conversation_dialog_system.rs `113-151`**: 38-line commented-out `parse_message` (TODO "Fix parse_message for Bevy 0.13") — delete.
9. **login_system.rs**: commented-out logs at `30`, `60`, `63`, `81`, `102`, `108`, `115`, `125`, `133`.
10. **game_system.rs `36-70`**: `game_state_enter_system` is half comments + `let _ = entity;` suppression; camera-count branch dead (only logs).
11. **character_select_system.rs**: `log::info!` at `59`, `290-294`, `487-491`, `502`, `521` are debug noise.
12. **ping_command_system.rs `51-73`**: `ping_measurement_system` races with `game_connection_system` — both consume from `server_message_rx` (`try_recv` in both), so the "any server message = ping response" heuristic can silently eat or mis-attribute messages. Either remove it or move ping detection into the dispatcher.

**Estimated savings:** −200 to −250 lines.

---

## Finding 7 — Network bootstrap duplication across 3 connection systems

- `network_thread_system.rs:23-107`: three near-identical arms (`ConnectLogin`/`ConnectWorld`/`ConnectGame`) each creating 2 channels, parsing the address, sending `RunProtocolClient`, inserting a resource — only the protocol class and resource type differ. Extract `fn start_protocol_client<T>(...)`.
- The *connection-lost* pattern is duplicated 3x:
  - `login_connection_system.rs:121-130`
  - `world_connection_system.rs:100-109`
  - `game_connection_system.rs:2906-2915`
  all doing: `warn → MessageBoxEvent::Show → remove_resource`. Extract `fn handle_connection_lost(commands, message_box_events, title, error)`.
- The `try_recv` loop skeleton (`match ... { Ok(..) ..., Err(Disconnected) => break, Err(Empty) => break }`) is also duplicated 3x (login/world/game connection systems).

**Estimated savings:** −40 to −60 lines.

---

## Finding 8 — Over-engineering details

1. **chat_command_system.rs**:
   - `PARTY_PREFIXES = ['#', '#']` (`27`) and `TRADE_PREFIXES = ['$', '$']` (`30`) list the same char twice; `SPACE_CHARS` (`34`) contains `' '` twice. Either deliberate-but-sloppy Unicode duplication or a copy-paste bug.
   - `char_byte_width` (`37-39`) is a pointless wrapper around `c.len_utf8()`.
   - 7 near-identical match arms (`92-228`) differing only in prefix list + `ChatType` → replace with a `&[(ChatType, &[char])]` table lookup.
2. **conversation_dialog_system.rs**: `event_object_handle: Arc<dyn std::any::Any + Send + Sync>` (`45`, constructed at `107` as `Arc::new(LuaUserValueEntity { owner_entity })`) — always the same concrete type; could be `LuaUserValueEntity` directly, removing the `Any` indirection. The 4-lifetime `LuaVMContext` (`48-54`) is justified by the Lua bridge but the `Any` isn't.
3. **client_entity_event_system.rs `95-101` / use_item_event_system.rs `76-82`**: identical spatial-sound spawn bundle (`SoundCategory, gain, SpatialSound::new(...), Transform, GlobalTransform`) — extract `spawn_spatial_sound(commands, ...)` helper (a third instance is hidden inside `queue_monster_sound` call sites).
4. **character_select_system.rs `163-346`**: `character_select_system` mixes world-connection events, egui rendering, a 5-state hand-rolled camera state machine (`Entering → CharacterSelect → ConnectingGameServer → Leaving → Loading`) using `Local<Option<ZoneId>>`. Could be a Bevy substate; low priority (moderate risk, low gain).
5. **command_system.rs `CooldownType`** (`game_connection_system.rs:16-20`) — fine as-is; noted only to show the "extract from reference before closure" pattern repeats ~10x (each one is a comment explaining a borrow issue).
6. **game_connection_system.rs `StartCastingSkill` / `LevelUpEntity`**: two server messages exist solely because the server sends redundant messages; the client workaround comments (`1336`, `1969`) document protocol drift — worth a server-side fix note in `rose-offline` instead.

---

## Prioritized summary — top 5 quick wins first

| # | Change | Files:lines | Est. LOC |
|---|---|---|---|
| 1 | **Delete dead code & debug logs**: commented DIAG blocks, `// Nah bruv`, 6 warn-only unimplemented arms, empty stubs (`move_speed_command_system` fn, `quest_trigger` `ApplyRewards`, `chat_command` `as_client_message`, `use_item_event` dead branches, `QueryAttackTarget`), commented-out `parse_message`, excess `log::info!` in hot paths | game_connection_system.rs (148, 177, 560-563, 616, 714-719, 814-871, 1969-1971, 2863-2880); command_system.rs (310-314, 412-418); use_item_event_system.rs (85-107); move_speed_command_system.rs (35-42); chat_command_system.rs (240-245); conversation_dialog_system.rs (113-151); quest_trigger_system.rs (18-20); game_system.rs (36-70); login_system.rs | −200 to −250 |
| 2 | **Dedupe `SkillTargetFilter` validation** in player_command_system.rs (copy at 310-395 and 529-620) | player_command_system.rs | −90 |
| 3 | **Extract shared entity-spawn helper** for the 4 `SpawnEntity*` handlers + `type SpawnTransformBundle` | game_connection_system.rs:368-704 | −150 to −200 |
| 4 | **Extract level-up recalculation helper** (and fix duplicated `max_mana` bug: `mana_points.mp = ...get_max_health()` at lines 1327, 1380) | game_connection_system.rs:1276-1386 | −60 + bugfix |
| 5 | **Split `game_connection_system` into per-domain handlers** behind a params struct (Finding 1) — do *after* wins 3-4 so the split is smaller | game_connection_system.rs:182-2904 | −300 structural |

Follow-ups (next tier):
6. Extract `command_system` per-command handlers + generic motion lookup (Finding 4) — −250 to −350.
7. Network bootstrap + connection-lost dedupe (Finding 7) — −40 to −60.
8. `chat_command_system` prefix-table refactor + prefix-list fixes (Finding 8.1) — −60.
9. `spawn_spatial_sound` helper (Finding 8.3) — −15.
10. Character-select state machine to Bevy substate (Finding 8.4) — riskier, low LOC gain.

**Bottom line:** ~8,630 LOC analyzed; ~1,400–1,700 lines (~18%) removable without behavior change, the largest single win being a domain-split + dead-code purge of `game_connection_system.rs` (~2,900 lines → ~1,800-2,000), followed by `command_system.rs` (~1,151 → ~800-900).

---

*This document is a research/planning artifact only. No source files were modified during this analysis.*
