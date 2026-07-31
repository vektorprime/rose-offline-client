# Cleanup Report: `src/systems/` Gameplay & Networking half (report `02-systems-gameplay.md` implementation)

**Branch:** `code-simplification`
**Agent scope:** 23 gameplay/networking system files (see LOC table below).
**Status:** implemented; `cargo check` run at the end (see "Build check"). All errors in my files fixed; remaining errors are in other agents' files.

---

## Changes per finding

### Finding 6 — Dead code, commented-out blocks, debug logging (priority 1)

| File | Action |
|---|---|
| `game_connection_system.rs` | Removed commented-out `[DIAG_*]` logs (SPAWN_HEIGHT, DIAG_RUN_CONDITION, DIAG_MONSTER_SPAWN ×4, RESPAWN_MOVE_DIAG ×2, RESPAWN_DIAG ×3, TELEPORT commented logs, `// Nah bruv` comment). Removed 12 `log::info!` debug calls in hot paths: `[DIAG_JOIN_ZONE]` ×3 blocks (314-320, 336-340, 362-363), `[DIAG_NPC_SPAWN]` ×5 (474-483, 503-507), `DamageEntity` ×7 (814-815, 818-822, 825-828, 835, 841, 849, 856). Kept `error!`/`warn!` calls on genuine error paths (missing entity, missing PendingDamageList, connection errors). |
| `command_system.rs` | Deleted dead `QueryAttackTarget` struct (never used; verified by grep) and the commented `[RESPAWN_CMD_DIAG]` block. |
| `player_command_system.rs` | Removed commented-out `SkillBasicCommand` arms (AutoTarget/AddFriend/Trade/PrivateStore/SelfTarget/VehiclePassengerInvite) and commented `[RESPAWN_MOVE_DIAG]` logs in the `Move` handler. |
| `use_item_event_system.rs` | Removed the dead `if let Some(apply_status_effect) ... else if let Some(add_ability)` block (verified dead: first branch contained only a comment, second branch was a commented-out TODO). Removed the now-unused `&mut StatusEffects`/`&mut StatusEffectsRegen` query params + `StatusEffects`/`StatusEffectsRegen` imports + unused `Duration`/`Instant` imports. |
| `quest_trigger_system.rs` | Empty `ApplyRewards` stub arm reduced to `QuestTriggerEvent::ApplyRewards(_) => {}` (arm must remain — the variant is still emitted by `game_connection_system`; a full removal would be non-exhaustive). Removed unused `quest_apply_rewards`/`quest_check_conditions` imports (both were already unused). |
| `chat_command_system.rs` | Deleted `as_client_message()` (verified zero callers) + its stale doc comment. |
| `move_speed_command_system.rs` | Deleted the empty stub `move_speed_command_system` fn (never registered in `src/lib.rs`; only `move_speed_set_system` is at lib.rs:1725) + unused `bevy::prelude::*`/`PlayerCharacter`/`MoveSpeedSetEvent` imports. Kept `parse_move_speed_command` + tests. |
| `conversation_dialog_system.rs` | Deleted the 38-line commented-out `parse_message` (TODO "Fix parse_message for Bevy 0.13"). |
| `login_system.rs` | Removed 9 commented-out `[LOGIN SYSTEM]` logs. |
| `game_system.rs` | `game_state_enter_system`: removed all commented `[CAMERA]` logs, the dead `camera_count == 0` branch (only logged comments), and the pointless `let _ = entity;`. |
| `character_select_system.rs` | Removed 5 debug `log::info!` calls (enter, Entering-state transition, ray-cast ×3) + now-unused `use log::info;`. |
| `ping_command_system.rs` | Deleted `ping_measurement_system` — verified **never registered** (not in `mod.rs` re-exports, not in `lib.rs`; it raced `game_connection_system` on `server_message_rx.try_recv()`). `/ping` flow (`ping_command_system`/`ping_response_system`/`PingState`) untouched. |

### Finding 2 — Shared entity-spawn helper (priority 2)

`game_connection_system.rs`: added `type SpawnTransformBundle` alias + `spawn_client_entity(world, entity_id, entity_type, position, rotation, core_bundle, extra_components)` (nested `impl Bundle` tuples; verified against Bevy 0.18.1 source that tuple bundles nest beyond the 15-arity limit). All 4 `SpawnEntity*` handlers rewritten to call it:

- `SpawnEntityCharacter` — core 14-component bundle + extras (FacingDirection, PendingDamage*, VisibleStatusEffects, DirtDashEffect) + optional `PersonalStore`/`ClanMembership` inserts preserved.
- `SpawnEntityNpc` — rotation via `Quat::from_axis_angle(Vec3::Y, direction.to_radians())` moved into the helper as `rotation: Option<Quat>`.
- `SpawnEntityMonster` — `Equipment`/`MonsterSeparation` extras preserved.
- `SpawnEntityItemDrop` — name lookup preserved; spawn + insert merged (same deferred closure, no frame boundary between them — behavior identical).

Eliminated the redundant pre-closure `clone()`s of `character_info`/`equipment`/`personal_store_info`/`clan_membership` where Rust-2021 precise captures allowed field moves (note: `SpawnEntityCharacter` keeps the original local-capture pattern — moving `message` whole after partially moving `status_effects` out fails with E0382).

### Finding 3 — Level-up recalculation helper + max_mana bug fix (priority 3)

`game_connection_system.rs`: extracted `recalculate_ability_values_and_refill(world, entity)` (single `resource_scope` + 5-component destructure + HP/MP refill). Both `UpdateLevel` and `LevelUpEntity` now call it.

**Bug fix applied:** `mana_points.mp = ability_values.get_max_health()` → `get_max_mana()` (was copied twice at the old lines 1327 and 1380).

Behavior notes: `UpdateLevel` inserts the message's `Level` component before the queued closure runs, so the helper reads the same value the old code passed as `&level`. `LevelUpEntity` still increments the component inside the closure before calling the helper.

### Finding 5 — SkillTargetFilter dedupe (priority 4)

`player_command_system.rs`: extracted `is_valid_skill_target(filter, target, player_entity, player_team_id, player_party, player_clan)` — the 11-arm match was byte-identical at both call sites (skill targeting 310-395, consumable magic-item targeting 529-620). Both sites now call it. Note: `Team.id` is `u32` (verified in `rose-game-common/components/team.rs`), so the helper takes `player_team_id: u32` (the report's draft signature said `u16`).

### Finding 7 — Network bootstrap + connection-lost dedupe (priority 5)

`network_thread_system.rs` (shared network helpers live here; sibling modules can use them via `crate::systems::network_thread_system::…` without touching `mod.rs`):

- `start_protocol_client<T: ProtocolClient + Send + Sync + 'static>(network_thread, server_address, construct_client)` — creates both channels, spawns the protocol thread, returns the sender/receiver pair. `ConnectLogin` passes `irose::LoginClient::new` directly; `ConnectWorld`/`ConnectGame` pass closures for their extra `packet_codec_seed`.
- `handle_connection_lost<R: Resource>(commands, message_box_events, server_name, error)` — the warn + `MessageBoxEvent::Show` + `remove_resource::<R>()` pattern was duplicated 3× (`login_connection_system.rs:121-130`, `world_connection_system.rs:100-109`, `game_connection_system.rs:2906-2915`); all three call sites now use it with their own resource type. The `try_recv` loop skeletons themselves were NOT unified (the per-protocol match bodies differ completely; a shared loop would be a forced abstraction).

### Finding 4 — `command_system.rs` (partial, as time permitted)

- **Done:** added `update_stop_motion(...)` and `update_move_motion(...)` helpers (the report's "set_motion_pair" idea) and replaced the 4 duplicated driver+vehicle animation blocks: the `Command::Stop` arm (618-644) and idle branch (575-598) previously duplicated each other verbatim; the vehicle-motion update was duplicated again at 630-640/902-912. `update_move_motion` preserves the exact original gating (vehicle motion only updated when a character move animation exists — verified block-by-block).
- **Skipped:** full per-command handler extraction (`handle_move`, `handle_attack`, `handle_cast_skill`, …) — the system is one loop over a 14-param query with ~20 borrow interactions; the report itself estimated −250..−350 only for the full split, which is a high-risk pure refactor. Not attempted to keep behavior guarantees.

### Finding 8 — Details (partial)

- **8.1 done** (`chat_command_system.rs`): fixed duplicated chars in prefix lists (`PARTY_PREFIXES ['#','#'] → ['#','＃']`, `TRADE_PREFIXES ['$','$'] → ['$','＄']`, `SPACE_CHARS` deduped), deleted the pointless `char_byte_width` wrapper (`prefix.len_utf8()`), replaced 5 near-identical match arms with the `PREFIX_TABLE` lookup (whisper/help arms kept separate — different shapes). Guard-set overlap verified nil, so arm order change is behavior-neutral.
- **8.2 skipped** (`conversation_dialog_system` `Arc<dyn Any>` handle): low value, touches `scripting` types (another agent's module, currently mid-refactor).
- **8.3 skipped** (spawn_spatial_sound): **the shared helper does not exist in `src/audio/mod.rs` yet** (checked at implementation time), so no migration performed per instructions.
- **8.4 skipped** (character-select state machine → Bevy substate): risky, low gain, per report.
- **8.6**: `StartCastingSkill` kept as an explicit empty arm (folding it into the catch-all would spam a warn for a routinely-sent protocol-drift message; the original behavior was silent). The 6 warn-only unimplemented arms + the 15-variant "unexpected login/world packet" arm were folded into one `Ok(other_message) => log::warn!(...)` catch-all.

---

## LOC accounting (report's before → `rg -c ""` after)

| File | Before | After | Delta |
|---|---|---|---|
| game_connection_system.rs | 2916 | 2765 | −151 |
| command_system.rs | 1151 | 1169 | **+18** (dead code −12; motion helpers +66/−48 of duplicated blocks) |
| player_command_system.rs | 959 | 852 | −107 |
| conversation_dialog_system.rs | 730 | 690 | −40 |
| character_select_system.rs | 523 | 505 | −18 |
| chat_command_system.rs | 410 | 343 | −67 |
| login_system.rs | 180 | 168 | −12 |
| login_connection_system.rs | 131 | 130 | −1 |
| network_thread_system.rs | 110 | 136 | **+26** (shared helpers; 3 arms shed ~45 lines) |
| world_connection_system.rs | 110 | 109 | −1 |
| use_item_event_system.rs | 109 | 71 | −38 |
| game_system.rs | 103 | 86 | −17 |
| ping_command_system.rs | 100 | 74 | −26 |
| move_speed_command_system.rs | 72 | 49 | −23 |
| quest_trigger_system.rs | 57 | 53 | −4 |
| **Total (15 changed files)** | **7661** | **7200** | **−461** |

Unchanged (verified no dead code flagged in report for these): `game_mouse_input_system.rs`, `game_keyboard_input_system.rs`, `client_entity_event_system.rs`, `auto_login_system.rs`, `systemfunc_event_system.rs`, `quest_scroll_event_system.rs`, `clan_system.rs`, `move_speed_set_system.rs`.

Notes on the two positive deltas: `command_system.rs` — the two shared motion helpers are factored-out code (4 duplicated blocks → 2 helpers + 4 calls); the duplication is gone even though the factored helpers are more verbose than the inlined blocks. `network_thread_system.rs` — the bootstrap helper centralizes channel creation + thread spawn (~45 lines removed across the 3 arms, +71 for the 2 helpers/imports). Full Finding-1 domain-split of `game_connection_system` (−300 est.) was intentionally not attempted — pure reorganization, high churn, zero behavior delta.

---

## Skipped items (with reasons)

1. **Finding 1** — split `game_connection_system` into per-domain handlers behind a params struct: pure reorganization (report: "rest is reorganization"); deferred by the report itself until Findings 2-4 land. High churn for no behavior change.
2. **Finding 4 (full)** — per-command handler extraction in `command_system.rs`: high-risk borrow restructuring in a single 1,100-line system; only the verified-safe motion-pair dedupe was done.
3. **Finding 8.2** — `event_object_handle: Arc<dyn Any>` → `LuaUserValueEntity`: touches `src/scripting` types (another agent's module, mid-refactor); low LOC gain.
4. **Finding 8.3** — `spawn_spatial_sound` migration: helper does not exist in `src/audio/mod.rs` yet; skipped per instructions (do not edit audio files).
5. **Finding 8.4** — character-select state machine → Bevy substate: risky, low gain.
6. **Finding 8.6 server-side** — `StartCastingSkill`/`LevelUpEntity` protocol drift: needs a `rose-offline` (server) change, out of scope.
7. Pre-existing unused-import/unused-var warnings in my files were left alone when not introduced by my edits (`character_select_system` `World`, `conversation_dialog_system` `QuestState`/event imports, `player_command_system` `cooldown_duration`, `quest_trigger_system` `mut script_context`, `game_mouse_input_system` `Local`/`MoveDestinationEffectEvent`).

---

## Build check

Ran `cargo check` (final output: 39 errors, all outside my scope):

- **My files: 0 errors.** Errors I introduced and fixed during the check: `start_protocol_client` missing `'static` bound (E0310) and `MessageWriter` import; `is_valid_skill_target` `u16` vs `u32` team id (E0277/E0308); `quest_trigger` match non-exhaustive (E0004 — arm restored as `{}`); `SpawnEntityCharacter` partial-move of `message` (E0382 — restored local-capture pattern).
- `src/systems/conversation_dialog_system.rs:92,96` E0282 are **cascade errors** from the scripting agent's `Lua4Value` breakage (`src/scripting/mod.rs:16` `lua_closures` macro) — code at those lines is untouched by me.
- Remaining errors are in other agents' files: `src/scripting/*` (lua_closures macro, Lua4Value, E0658), `src/render/*` + `src/lib.rs:117` (render agent's deletions), `src/ui/*` (widgets match arms, drag-and-drop closures), `src/zone_loader/*` + `src/lib.rs:872`, `src/bundles/ability_values.rs`, `src/audio/streaming_sound.rs`, `src/lib.rs` `diagnostics` module. Not touched.
- The `src/systems/mod.rs:184` re-export of the deleted `move_speed_command_system` fn was resolved by the mod.rs agent mid-session (final check shows it re-exports only `parse_move_speed_command`) — no action needed from me.
