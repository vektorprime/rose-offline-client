# Flying Pitfalls

This document records issues encountered with the `/fly` flight system and related
collision/spawning behavior.

---

## Character Launches Upward at Spawn and Never Stops (Fixed 2026-08-02)

### Problem
As soon as the character spawned in, it started flying up and never stopped.
Typing `/fly` made the character snap to the ground.

### Root Cause
`find_object_top_height()` in `src/systems/collision_system.rs` (added to place
NPCs spawned underneath castle steps on top of the object) runs **every frame**
for the player. Its rapier query used:

```rust
CollisionGroups::new(
    COLLISION_FILTER_MOVEABLE | COLLISION_FILTER_INSPECTABLE,
    !COLLISION_GROUP_PHYSICS_TOY & !COLLISION_GROUP_ZONE_TERRAIN & !COLLISION_GROUP_ZONE_WATER,
)
```

Rapier tests interaction with `query.memberships & collider.filter != 0 &&
collider.memberships & query.filter != 0`. The player's own model collider
(`character_model_add_collider_system.rs`) has
`memberships = COLLISION_GROUP_PLAYER` and
`filter = INSPECTABLE | CLICKABLE | PHYSICS_TOY`, so it **matched the query**.
Each frame the feet-sphere intersected the player's own collider, the upward ray
climbed to the top of their own ~1.8m collider, and
`target_y = max(terrain, feet + 1.8)` launched the player ~1.8m higher every
frame. NPCs had the same self-intersection via `collision_height_only_system`.

Typing `/fly` appeared to "fix" it only because the flight branch of
`collision_player_system` snaps `transform.y` to `position.z / 100` (the
server-authoritative ground height), bypassing the buggy ground path.

### Solution
Exclude entity-class groups from the query memberships so only zone objects
(bridges, castle steps, buildings) match:

```rust
let object_groups = CollisionGroups::new(
    COLLISION_FILTER_MOVEABLE | COLLISION_FILTER_INSPECTABLE,
    !COLLISION_GROUP_PHYSICS_TOY
        & !COLLISION_GROUP_ZONE_TERRAIN
        & !COLLISION_GROUP_ZONE_WATER
        & !COLLISION_GROUP_PLAYER
        & !COLLISION_GROUP_NPC
        & !COLLISION_GROUP_CHARACTER
        & !COLLISION_GROUP_ITEM_DROP,
);
```

Applied to `find_object_top_height()` and the downward ground ray in
`collision_height_only_system()`.

### Files Modified
- `src/systems/collision_system.rs`

### Lesson Learned
When writing rapier scene queries, verify which collision groups the query
matches against **your own entity's colliders**. An "everything except X/Y/Z"
membership mask still matches the entity itself (player/NPC colliders include
`INSPECTABLE` in their filter), causing self-intersection. Exclude the
entity-class membership groups (`PLAYER`, `NPC`, `CHARACTER`, `ITEM_DROP`)
explicitly. Verify filter semantics from the rapier source
(`interaction_groups.rs`: interaction allowed iff
`memberships & filter` in both directions).

---

## /fly Character Runs in Mid-Air (Server-Authoritative) (Fixed 2026-08-02)

### Problem
Typing `/fly` and holding Space made the character look like it was trying to
walk/run in mid-air instead of flying.

### Root Cause
Flight is a client-local command: the client moves `Position` locally and
reports it via `ClientMessage::MoveCollision`. The server accepts the position
but also sets a server-side move command, then echoes
`ServerMessage::MoveEntity` back to the client with `move_mode: Run`. The
client's `game_connection_system` turned that into `NextCommand::with_move(...)`,
so `command_system` played the Run animation and `update_position_system` dragged
the character toward the stale echoed destination at ground speed — every frame,
fighting the flight movement.

### Solution
- `game_connection_system.rs`: in the `MoveEntity` handler, if the entity is
  flying (`FlightState::is_flying`), insert `NextCommand::with_stop()` instead of
  `NextCommand::with_move(...)`.
- `command_system.rs`: in `Command::Move` handling, if the player is flying, keep
  the idle/stop motion instead of run/walk (covers a move command already active
  when `/fly` is toggled).
- `flight_movement_system.rs`: also send `MoveCollision` with the current
  position while hovering (speed == 0) so the server-authoritative position
  stays in lockstep and stale pre-flight move commands can't drag the server
  entity away (which would later trigger teleport-rejection snap-backs).

The server stays authoritative; it accepts flight positions (per-frame movement
is well under the 500cm teleport limit).

### Files Modified
- `src/systems/game_connection_system.rs`
- `src/systems/command_system.rs`
- `src/systems/flight_movement_system.rs`

### Lesson Learned
Client-predicted movement that reports via `MoveCollision` will receive the
server's `MoveEntity` echo with the ground move mode. Ignore that echo for the
predicted entity (keep it on `Stop`) or the client will animate run/walk and
chase a stale destination.
