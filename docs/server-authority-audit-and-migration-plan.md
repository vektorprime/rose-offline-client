# Client Authority Audit and Server-Authority Migration Plan

## Scope

This document captures client-side gameplay calculations/decisions that should be moved to server authority and synchronized back to the client via network messages.

Code was **not** modified for this audit.

---

## Executive Summary

### Highest-risk authority gaps
1. Cooldown timing/state is client-owned.
2. Collision resolution and some trigger detection are client-owned.
3. Skill/item target validity gating is client-heavy.

### Medium-risk authority gaps
4. Quest scroll flow still does client-side trigger pathing.
5. Interaction/range gating (pickup/NPC/store) is partly client-decided.

### Predictive/input areas to keep but harden
6. Keyboard/mouse destination shaping should remain input-side but be server-validated.
7. Flight movement currently mutates position locally and should become server-driven in multiplayer-authoritative mode.

---

## Detailed Findings and Per-Item Plan

## 1) Cooldown timing is client-authoritative

### Evidence
- Local cooldown start in `src/systems/player_command_system.rs`:
  - `set_global_cooldown(Duration::from_millis(250))`
  - `set_consumable_cooldown(...)`
- Local cooldown ticking in `src/systems/cooldown_system.rs`.
- Cooldown packet opcodes exist in `../rose-offline/rose-network-irose/src/game_server_packets.rs`:
  - `UpdateCooldown = 0x7ba`
  - `UpdateConsumableCooldown = 0x7bb`
- Current game client protocol handling does not parse these packets in `src/protocol/irose/game_client.rs`.

### Why this should be server-side
Cooldowns are anti-cheat sensitive. Client-local timers can drift or be manipulated.

### Plan
1. Implement server packet parsing for cooldown updates (`0x7ba`, `0x7bb`) and map into explicit server messages.
2. In client, replace local cooldown starts with optimistic UI-only state (optional), then reconcile on server message.
3. Make server the source of truth for remaining/total cooldown.
4. Keep local visual interpolation only; on mismatch, snap or blend to authoritative value.
5. Add telemetry logs for cooldown desync counts.

### Server-side modifications (where and what)
- `../rose-offline/rose-offline-server/src/game/systems/skill_effect_system.rs`
  - After skill cooldowns are written (`cooldowns.skill_global`, `cooldowns.skill`, `cooldowns.skill_group`), emit `ServerMessage::UpdateCooldown` to the caster.
  - For group cooldowns, map group index and duration consistently with client cooldown buckets.
- `../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs`
  - After `cooldowns.set_item_cooldown(...)`, emit `ServerMessage::UpdateConsumableCooldown` with authoritative duration.
- `../rose-offline/rose-offline-server/src/irose/protocol/game_server.rs`
  - Add protocol encoding branches for `ServerMessage::UpdateCooldown` and `ServerMessage::UpdateConsumableCooldown`.
- `../rose-offline/rose-network-irose/src/game_server_packets.rs`
  - Implement packet structs/serde for opcodes `0x7ba` / `0x7bb` (currently enum-only).
- `src/protocol/irose/game_client.rs`
  - Add receive handling for `ServerPackets::UpdateCooldown` / `ServerPackets::UpdateConsumableCooldown` and forward as `ServerMessage`.

### Validation
- Repeated skill/item use under lag cannot bypass cooldown.
- Cooldown UI remains smooth and converges to server values.

---

## 2) Collision/position correction is computed client-side

### Evidence
- In `src/systems/collision_system.rs`, client performs shape/ray tests and rewrites position (`position.x/y/z = ...`).
- Client reports collisions via `ClientMessage::MoveCollision`.
- Client-side overlap detection also dispatches trigger/warp actions.

### Why this should be server-side
Movement/collision are primary trust boundaries. Client correction can be exploited and causes divergence.

### Plan
1. Server owns collision resolution and final position.
2. Client sends movement intent only (destination/target/input vector).
3. Server emits periodic position snapshots + correction events (existing `AdjustPosition` path can be reused/expanded).
4. Move warp/event overlap authority to server; client becomes presentation-only.
5. Keep client prediction for responsiveness, but reconcile aggressively on authoritative corrections.

### Server-side modifications (where and what)
- `../rose-offline/rose-offline-server/src/game/systems/game_server_system.rs`
  - `ClientMessage::MoveCollision` currently trusts client position and inserts it directly; replace with reject/ignore or strict sanity + reconciliation request.
  - `ClientMessage::WarpGateRequest` currently teleports by id lookup only; add server-side proximity/eligibility checks before teleport.
- `../rose-offline/rose-offline-server/src/game/systems/update_position_system.rs`
  - Add server collision/navmesh/zone-bound validation to authoritative movement integration.
  - On invalid movement or divergence, emit `ServerMessage::AdjustPosition` to force reconciliation.
- `../rose-offline/rose-offline-server/src/game/systems/command_system.rs`
  - Keep pathing target resolution server-side, but ensure movement commands cannot bypass collision boundaries when retargeting entities.
- `../rose-offline/rose-offline-server/src/game/systems/quest_system.rs`
  - For trigger-like world overlaps (event object proximity), prefer server-produced `QuestTriggerEvent` instead of client-triggered overlap events.

### Validation
- Wall clipping attempts fail.
- Position desync rate decreases in network diagnostics.
- Warp/event behavior is identical across clients.

---

## 3) Skill/item target validity is heavily client-side

### Evidence
- `src/systems/player_command_system.rs` performs large target-filter checks (team, guild, party, dead/alive, entity type) before sending cast/use messages.
- Similar filtering exists for targeted consumables.

### Why this should be server-side
Target eligibility must be enforced by authority to prevent invalid cast acceptance from modified clients.

### Plan
1. Keep client-side checks only as UX pre-checks (cursor feedback/early warnings).
2. Treat server as canonical validator for all cast/use requests.
3. Add/standardize explicit server reject reasons (invalid target, out of range, wrong state, etc.).
4. Route reject reasons to UI/chat consistently.
5. Remove any client logic that assumes validity solely because local filters passed.

### Server-side modifications (where and what)
- `../rose-offline/rose-offline-server/src/game/systems/command_system.rs`
  - Validation already exists (`check_skill_target_filter`, range checks, cooldown checks).
  - Add explicit negative responses when validation fails (e.g., `ServerMessage::CancelCastingSkill` with specific reason) instead of silent stop/continue.
- `../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs`
  - For targeted consumables/magic items, add canonical target/range/type/state checks (not just `target_entity.is_some()`).
  - Return deterministic reject feedback to client when item target rules fail.
- `../rose-offline/rose-offline-server/src/irose/protocol/game_server.rs`
  - Ensure all reject reasons used by command/use-item systems map to client-visible packet paths.

### Validation
- Invalid targets are always rejected by server even if client attempts request.
- User receives deterministic failure reason.

---

## 4) Quest scroll trigger path still includes client-side trigger dispatch

### Evidence
- In `src/systems/quest_scroll_event_system.rs`, after `UseItem`, client also emits `QuestTriggerEvent::DoTrigger` for local condition checking.
- `src/systems/quest_trigger_system.rs` forwards trigger requests to server.

### Why this should be server-side
Quest progression/rewards are progression-critical and must be authoritative.

### Plan
1. Remove local pre-validation trigger path for quest-scroll confirm flow.
2. On confirm, send only server request (`UseItem`/quest action).
3. Server evaluates conditions and returns explicit success/failure + reason.
4. Client UI displays pending state and resolves from server response only.
5. Maintain local dialog/UX, but not local quest decisioning.

### Server-side modifications (where and what)
- `../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs`
  - For quest-scroll consumables, trigger authoritative quest actions/events from server item-use path.
- `../rose-offline/rose-offline-server/src/game/systems/game_server_system.rs`
  - Tighten `ClientMessage::QuestTrigger` acceptance; avoid allowing arbitrary trigger execution from unrestricted client calls.
- `../rose-offline/rose-offline-server/src/game/systems/quest_system.rs`
  - Keep canonical trigger condition/reward execution; add richer failure categorization if UI needs exact reason.

### Validation
- Quest scroll can’t be forced by local event injection.
- Quest outcomes match server logs exactly.

---

## 5) Interaction/range gating (pickup/NPC/store) is partly client-decided

### Evidence
- In `src/systems/command_system.rs`, client computes “close enough” via local constants:
  - `NPC_MOVE_TO_DISTANCE`
  - `CHARACTER_MOVE_TO_DISTANCE`
  - `ITEM_DROP_MOVE_TO_DISTANCE`
- Client then opens dialogs/stores or sends pickup request based on local distance outcome.

### Why this should be server-side
Range gating affects gameplay fairness and consistency.

### Plan
1. Move canonical range checks to server for pickup/store/NPC interactions.
2. Client may still auto-approach target for UX, but server decides interaction success.
3. Add explicit server result messages for interaction accepted/denied (+ reason).
4. Keep local constants as soft steering hints only.
5. Align thresholds in one server-configurable source.

### Server-side modifications (where and what)
- `../rose-offline/rose-offline-server/src/game/systems/npc_store_system.rs`
  - Range checks already exist (`NPC_STORE_TRANSACTION_MAX_DISTANCE`); keep as canonical and expose clear error mapping.
- `../rose-offline/rose-offline-server/src/game/systems/personal_store_system.rs`
  - Add missing distance/zone validation before list/buy operations.
- `../rose-offline/rose-offline-server/src/game/systems/pickup_item_system.rs`
  - Ownership/party rules are canonical here; keep this as authority and standardize denial reasons.
- `../rose-offline/rose-offline-server/src/game/systems/command_system.rs`
  - Pickup distance constants are already server-side; ensure client constants do not drive acceptance.
- `../rose-offline/rose-offline-server/src/game/systems/game_server_system.rs`
  - Add warp-gate interaction range checks (same authority pattern as NPC/store interactions).

### Validation
- Out-of-range interaction cannot be forced client-side.
- Behavior remains stable under latency and desync.

---

## 6) Movement destination shaping is client-computed (acceptable with strict server validation)

### Evidence
- `src/systems/game_keyboard_input_system.rs` computes lead-distance destination.
- `src/systems/game_mouse_input_system.rs` converts click ray hit into destination.

### Why this needs a plan
This is normal for responsive controls, but must remain request-only.

### Plan
1. Keep these systems as input/prediction producers.
2. On server, clamp movement by speed, timestep, map bounds, and collision.
3. Return authoritative movement stream (`MoveEntity`, `StopMoveEntity`, `AdjustPosition`).
4. Implement stronger client reconciliation thresholds and smoothing strategy.

### Server-side modifications (where and what)
- `../rose-offline/rose-offline-server/src/game/systems/game_server_system.rs`
  - In `ClientMessage::Move`, validate destination intent (zone, max radius from current state, no invalid coordinates).
- `../rose-offline/rose-offline-server/src/game/systems/update_position_system.rs`
  - Enforce speed/timestep bounds and environment constraints during integration.
  - Emit `ServerMessage::AdjustPosition` when authoritative state diverges from expected client path.
- `../rose-offline/rose-offline-server/src/game/systems/command_system.rs`
  - Retain server-owned move/stop transitions and avoid accepting impossible command transitions from client spam.

### Validation
- Client remains responsive.
- Server consistently overrides impossible movement.

---

## 7) Flight movement currently mutates local position directly

### Evidence
- `src/systems/flight_movement_system.rs` directly updates `Position` each frame while flying.

### Why this should be server-side (for authoritative multiplayer)
Flight introduces 3D freedom and larger exploit surface than ground movement.

### Plan
1. Convert flight controls into intent messages (thrust, direction, mode).
2. Simulate flight motion/constraints server-side (speed, accel/decel, min altitude, zone restrictions).
3. Stream authoritative transforms/positions back to clients.
4. Keep optional local prediction, but reconcile with server snapshots.

### Server-side modifications (where and what)
- `../rose-offline/rose-game-common/src/messages/client.rs`
  - Add explicit client->server flight intent messages.
- `../rose-offline/rose-offline-server/src/game/systems/game_server_system.rs`
  - Parse and route flight intents into server command/state.
- `../rose-offline/rose-offline-server/src/game/components/`
  - Add authoritative flight state component(s) (mode, velocity, constraints).
- `../rose-offline/rose-offline-server/src/game/systems/`
  - Add a dedicated server flight simulation system and integrate with existing movement authority/reconciliation.
- `../rose-offline/rose-offline-server/src/irose/protocol/game_server.rs` and `../rose-offline/rose-network-irose/src/game_server_packets.rs`
  - Add packet mapping if flight needs protocol-level messages beyond existing movement/correction packets.

### Validation
- No client-only altitude/speed hacks.
- Other clients see consistent flight trajectories.

---

## Suggested Rollout Order

1. Cooldowns (high impact, low-to-medium complexity).
2. Target validity enforcement (high impact, medium complexity).
3. Interaction/range authority (medium-high impact, medium complexity).
4. Collision/warp trigger authority (high impact, medium-high complexity).
5. Quest scroll trigger hardening (medium impact, low complexity).
6. Movement reconciliation hardening (medium impact, medium complexity).
7. Flight authority migration (high impact, high complexity).

---

## Server Change Matrix (Quick Reference)

1. **Cooldown authority + sync packets**
   - Server gameplay: `skill_effect_system.rs`, `use_item_system.rs`
   - Shared message model: `rose-game-common/src/messages/server.rs` (already contains variants)
   - Protocol encode/decode: `irose/protocol/game_server.rs`, `rose-network-irose/src/game_server_packets.rs`, `src/protocol/irose/game_client.rs`

2. **Movement/collision authority**
   - Input handling: `game_server_system.rs`
   - Authoritative integration/reconciliation: `update_position_system.rs`
   - Command gating: `command_system.rs`

3. **Interaction/range authority**
   - NPC store: `npc_store_system.rs`
   - Personal store: `personal_store_system.rs`
   - Pickup: `pickup_item_system.rs`
   - Warp gate checks: `game_server_system.rs`

4. **Quest trigger authority hardening**
   - Trigger ingestion: `game_server_system.rs`
   - Canonical execution: `quest_system.rs`
   - Quest-scroll item coupling: `use_item_system.rs`

5. **Flight authority (new server feature area)**
   - Messages/components/systems/protocol in server + shared message crates as listed above.

---

## Notes on Existing Good Direction

- Passive regen already notes server authority in `src/systems/passive_recovery_system.rs`.
- Item-use gameplay effects already indicate authority migration in `src/systems/use_item_event_system.rs`.

These should be used as pattern references for the rest of the migration.
