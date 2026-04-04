# Server-Side Migration Proposals

This document outlines game logic currently implemented on the client that should be migrated to the server to ensure authority, prevent cheating, and maintain state synchronization.

## 1. Combat & Life Cycle (High Priority)
The client currently acts as the authority for damage application and death determination.

### Logic to Migrate
- **Damage Application:** The subtraction of health points from entities should move from `src/systems/hit_event_system.rs` and `src/systems/pending_damage_system.rs` to the server.
- **Death Determination:** The check for `hp <= 0` and the subsequent insertion of the `Dead` component should be handled by the server.
- **Pending Damage Queue:** The management of `PendingDamageList` should be server-authoritative.

### Proposed Workflow
- **Client:** Sends an attack request/event to the server.
- **Server:** Validates the attack, calculates damage based on stats, updates HP, and determines if the entity died.
- **Server $\rightarrow$ Client:** Sends a notification of damage dealt (for rendering damage digits) and death events (for playing death animations).

---

## 2. Inventory & Economy (High Priority)
The client currently applies beneficial item effects directly to the player state.

### Logic to Migrate
- **Item Effect Application:** Logic in `src/systems/use_item_event_system.rs` that applies potion effects or other item bonuses.
- **Store Transactions:** Final validation of currency subtraction and item transfer in `src/events/npc_store_event.rs`.

### Proposed Workflow
- **Client:** Sends a `UseItemEvent` or `NpcStoreEvent` to the server.
- **Server:** Validates item ownership, checks requirements, subtracts the item/currency, and applies the effect to the player's server-side state.
- **Server $\rightarrow$ Client:** Sends a state update (e.g., updated HP, updated inventory) and a confirmation to trigger visual effects.

---

## 3. World & Quests (Medium Priority)
The client currently has the ability to trigger rewards and quest completions.

### Logic to Migrate
- **Reward Application:** The `ApplyRewards` variant of `QuestTriggerEvent` in `src/events/quest_trigger_event.rs`.
- **Trigger Validation:** The logic that decides if a quest trigger is valid (`DoTrigger`).

### Proposed Workflow
- **Client:** Notifies the server when it interacts with a quest NPC or enters a trigger volume.
- **Server:** Validates if the quest conditions are met, updates quest progress, and grants rewards (XP, items, currency).
- **Server $\rightarrow$ Client:** Sends a quest update notification to the client UI.

---

## 4. Movement & Physics (Medium Priority)
The client currently maintains authority over its own movement speed and basic collision.

### Logic to Migrate
- **Move Speed Authority:** The determination of maximum movement speed in `src/events/move_speed_event.rs`.
- **Position Validation:** Final authority on entity position to prevent teleportation/wall-hacking in `src/components/collision.rs`.

### Proposed Workflow
- **Client:** Uses client-side prediction for smooth movement and collision.
- **Server:** Tracks movement speed and performs sanity checks on the player's reported position.
- **Server $\rightarrow$ Client:** Sends authoritative position corrections if the client deviates too far.

---

## 5. Social (Low Priority)
Party and Clan management are currently handled with significant client-side trust.

### Logic to Migrate
- **Membership & Permissions:** All changes to party/clan membership and permission checks in `src/events/party_event.rs` and `src/events/clan_dialog_event.rs`.

### Proposed Workflow
- **Client:** Sends requests to join/leave/modify groups.
- **Server:** Validates permissions and updates the group state.
- **Server $\rightarrow$ Client:** Broadcasts group state changes to all members.