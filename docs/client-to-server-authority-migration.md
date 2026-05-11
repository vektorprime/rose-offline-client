# Client-to-Server Authority Migration Analysis

## Executive Summary

This document identifies client-side functionality that should be migrated to server authority to prevent cheating and abuse in the ROSE Offline game. The analysis covers both the game client (`rose-offline-client`) and game server (`rose-offline`) codebases.

**Key Finding:** There is already an existing audit document at `docs/server-authority-audit-and-migration-plan.md` that identifies many of the same issues. This document expands on that analysis with additional findings from examining both codebases.

---

## Critical Priority (High Exploit Risk)

### 1. Movement and Position Authority

**Current State:**
- Client computes collision response in [`src/systems/collision_system.rs`](src/systems/collision_system.rs:1)
- Client modifies `Position` component directly after ray casts
- Client sends `ClientMessage::MoveCollision` with self-reported position
- Flight movement in [`src/systems/flight_movement_system.rs`](src/systems/flight_movement_system.rs:23) directly modifies position

**Exploit Vectors:**
- Speed hacking by modifying movement calculations
- Wall clipping by ignoring collision results
- Flying through terrain by modifying flight logic
- Teleportation by sending false position updates

**Server Authority Required:**
- Server should validate all movement against collision data
- Server should be the source of truth for entity positions
- Client should only send movement *intent* (destination, input vectors)
- Server should send periodic position snapshots for reconciliation

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/update_position_system.rs`](../rose-offline/rose-offline-server/src/game/systems/update_position_system.rs:1)
- Server: [`../rose-offline/rose-offline-server/src/game/systems/command_system.rs`](../rose-offline/rose-offline-server/src/game/systems/command_system.rs:1)
- Client: Remove position mutation from collision system

---

### 2. Cooldown Management

**Current State:**
- Client sets cooldowns locally in [`src/systems/player_command_system.rs`](src/systems/player_command_system.rs:91-100)
- Client ticks cooldowns in [`src/systems/cooldown_system.rs`](src/systems/cooldown_system.rs:5)
- Server has cooldown logic but doesn't synchronize to client

**Exploit Vectors:**
- Cooldown bypass by modifying client timer
- Rapid skill/item spam by resetting cooldowns
- Desync exploitation during lag

**Server Authority Required:**
- Server must be the authoritative source for cooldown state
- Client should only display optimistic UI, reconcile on server message
- Server must send `UpdateCooldown` (0x7ba) and `UpdateConsumableCooldown` (0x7bb) packets

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/skill_effect_system.rs`](../rose-offline/rose-offline-server/src/game/systems/skill_effect_system.rs:1)
- Server: [`../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs`](../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs:1)
- Client: Add packet handlers for cooldown updates
- Client: Make cooldown system read-only (display only)

---

### 3. Damage Calculation and Application

**Current State:**
- Client processes damage in [`src/systems/hit_event_system.rs`](src/systems/hit_event_system.rs:66)
- Client applies damage to `HealthPoints` component locally
- Client spawns damage digits independently

**Exploit Vectors:**
- Damage immunity by blocking damage application
- Fake damage reporting to other clients
- HP manipulation by modifying health values

**Server Authority Required:**
- Server must calculate and apply all damage
- Client should only display damage received from server
- HealthPoints should be read-only on client except via server message

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/damage_system.rs`](../rose-offline/rose-offline-server/src/game/systems/damage_system.rs:1)
- Client: Make HealthPoints mutation server-only

---

### 4. Skill Target Validation

**Current State:**
- Client validates targets in [`src/systems/player_command_system.rs`](src/systems/player_command_system.rs:45-51)
- Client checks team, party, guild, dead/alive status locally
- Client decides if target is valid before sending request

**Exploit Vectors:**
- Targeting invalid entities (dead, friendly) by bypassing checks
- Range cheating by modifying distance calculations
- Skill abuse on protected targets

**Server Authority Required:**
- Server must validate ALL target eligibility
- Client checks should be UX-only (visual feedback)
- Server must send explicit reject messages with reasons

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/command_system.rs`](../rose-offline/rose-offline-server/src/game/systems/command_system.rs:1) - Already has validation, needs explicit reject messages
- Client: Remove authoritative target validation

---

### 5. Status Effect Application

**Current State:**
- Client has [`src/systems/status_effect_system.rs`](src/systems/status_effect_system.rs:11) that processes status effects
- Client applies poison/dot damage locally
- Item effects can apply status effects client-side

**Exploit Vectors:**
- Immunity to negative effects by blocking application
- Self-buffing with invalid status effects
- Effect duration manipulation

**Server Authority Required:**
- Server must apply all status effects
- Server must tick effect durations and damage
- Client should only display effects received from server

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/status_effect_system.rs`](../rose-offline/rose-offline-server/src/game/systems/status_effect_system.rs:1)
- Client: Make status effect system read-only

---

## High Priority (Medium Exploit Risk)

### 6. Item Use and Consumption

**Current State:**
- Client processes item use in [`src/systems/use_item_event_system.rs`](src/systems/use_item_event_system.rs:18)
- Client can apply item effects locally (commented out but structure exists)

**Exploit Vectors:**
- Item duplication by reusing consumed items
- Effect stacking by rapid-use during sync delay
- Using items without meeting requirements

**Server Authority Required:**
- Server must validate item requirements
- Server must consume items from inventory
- Server must apply all item effects

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs`](../rose-offline/rose-offline-server/src/game/systems/use_item_system.rs:1)

---

### 7. Quest Progression

**Current State:**
- Client triggers quest events in [`src/systems/quest_trigger_system.rs`](src/systems/quest_trigger_system.rs:1)
- Client sends quest trigger requests to server

**Exploit Vectors:**
- Quest skipping by sending false completion
- Duplicate quest rewards
- Triggering quests without prerequisites

**Server Authority Required:**
- Server must validate all quest conditions
- Server must track quest state authoritatively
- Client should only display quest state from server

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/quest_system.rs`](../rose-offline/rose-offline-server/src/game/systems/quest_system.rs:1)

---

### 8. Interaction Range Checks

**Current State:**
- Client has distance constants in [`src/systems/command_system.rs`](src/systems/command_system.rs:29-31):
  - `NPC_MOVE_TO_DISTANCE = 250.0`
  - `CHARACTER_MOVE_TO_DISTANCE = 1000.0`
  - `ITEM_DROP_MOVE_TO_DISTANCE = 150.0`
- Client uses these to decide interaction validity

**Exploit Vectors:**
- Remote NPC interaction
- Remote item pickup
- Remote store access

**Server Authority Required:**
- Server must validate all interaction distances
- Client constants should be soft hints only
- Server should have canonical distance values

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/npc_store_system.rs`](../rose-offline/rose-offline-server/src/game/systems/npc_store_system.rs:1)
- Server: [`../rose-offline/rose-offline-server/src/game/systems/pickup_item_system.rs`](../rose-offline/rose-offline-server/src/game/systems/pickup_item_system.rs:1)

---

### 9. Inventory and Economy

**Current State:**
- Client maintains local inventory state
- Client sends inventory update requests

**Exploit Vectors:**
- Item duplication through sync manipulation
- Money duplication
- Invalid item transfers

**Server Authority Required:**
- Server must be authoritative for all inventory changes
- All transactions must be server-validated
- Client inventory is a cache of server state

---

## Medium Priority (Lower Risk but Important)

### 10. Chat Command Processing

**Current State:**
- Client parses chat commands in [`src/systems/chat_command_system.rs`](src/systems/chat_command_system.rs:76)
- Client determines chat type and routing

**Recommendation:**
- Server should validate all chat commands
- Rate limiting should be server-side
- Chat spam prevention should be server-side

**Files to Modify:**
- Server: [`../rose-offline/rose-offline-server/src/game/systems/chat_commands_system.rs`](../rose-offline/rose-offline-server/src/game/systems/chat_commands_system.rs:1)

---

### 11. Experience and Level Progression

**Current State:**
- Client has `ExperiencePoints` and `Level` components that can be modified

**Exploit Vectors:**
- Level manipulation
- XP gain exploitation

**Server Authority Required:**
- Server must calculate all XP gains
- Server must handle level-ups
- Client should display server-authoritative values

---

### 12. Party and Clan Systems

**Current State:**
- Client maintains party/clan information locally

**Server Authority Required:**
- Server must validate all party/clan operations
- Server must be source of truth for membership
- Server must handle invites, joins, leaves

---

## Low Priority (UX/Presentation)

### 13. Animation and Visual Effects

**Current State:**
- Client handles all animation and effects

**Recommendation:**
- Keep client-side for performance
- Server should validate state changes that trigger animations
- Server can force animation sync for critical events

---

### 14. Camera and Input

**Current State:**
- Client handles all camera and input

**Recommendation:**
- Keep client-side (input must be local)
- Server should validate resulting actions
- No changes needed for anti-cheat

---

## Implementation Priority Order

1. **Phase 1 (Critical):** Movement/Position, Cooldowns, Damage
2. **Phase 2 (High):** Skill Validation, Status Effects, Item Use
3. **Phase 3 (Medium):** Quest Progression, Interaction Range, Inventory
4. **Phase 4 (Lower):** Chat, Economy, Social Systems

---

## Network Message Requirements

The following server-to-client messages need implementation or enhancement:

| Message Type | Opcode | Status | Priority |
|-------------|--------|--------|----------|
| UpdateCooldown | 0x7ba | Exists, needs client handler | Critical |
| UpdateConsumableCooldown | 0x7bb | Exists, needs client handler | Critical |
| AdjustPosition | - | Exists, expand usage | Critical |
| UpdateHealthPoints | - | Exists, verify authority | Critical |
| UpdateManaPoints | - | Exists, verify authority | Critical |
| CancelCastingSkill | - | Exists, add reasons | High |
| SkillUseReject | - | Needs implementation | High |
| ItemUseReject | - | Needs implementation | High |
| InteractionDenied | - | Needs implementation | Medium |
| QuestUpdate | - | Exists, verify authority | Medium |

---

## Client Architecture Changes Required

### Component Access Patterns

Components should be categorized by authority:

**Server-Authoritative (Client Read-Only):**
- `HealthPoints`
- `ManaPoints`
- `ExperiencePoints`
- `Level`
- `StatusEffects`
- `Cooldowns` (state, not visual timer)
- `Inventory`
- `Position` (final authority)

**Client-Predicted (Server-Reconciled):**
- `Position` (short-term prediction)
- `Command` (movement intent)
- `NextCommand`

**Client-Local (No Server Authority):**
- Visual effects components
- Camera components
- Input components
- UI components

---

## Testing and Validation

For each migrated system, validate:

1. **Desync Recovery:** Client state converges to server state after correction
2. **Exploit Resistance:** Modified clients cannot bypass server checks
3. **Latency Tolerance:** System behaves acceptably under lag
4. **Visual Continuity:** User experience remains smooth

---

## Related Documentation

- Existing audit: [`docs/server-authority-audit-and-migration-plan.md`](docs/server-authority-audit-and-migration-plan.md)
- Server architecture: `../rose-offline/docs/llm-bot-system-architecture.md`
- Networking pitfalls: [`pitfalls/networking.md`](pitfalls/networking.md)

---

## Conclusion

The client codebase currently has too much authority over gameplay-critical systems. The highest priority items (movement, cooldowns, damage) present immediate exploit vectors that should be addressed before any production deployment.

The server codebase already has good foundations for authority in many systems (skill validation, damage calculation, quest processing), but needs to:

1. Enforce authority more strictly (reject invalid client messages)
2. Send more synchronization messages to clients
3. Provide explicit error feedback for rejected actions

This migration should be done incrementally, with thorough testing at each phase to ensure gameplay remains smooth while security improves.
