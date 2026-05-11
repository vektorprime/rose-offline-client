# Combat Teleport Investigation (2026-04-18)

## Issue
Player appears to teleport a short distance forward/backward right before entering combat range on monster attack.

## Scope
- Client command pipeline, network handlers, movement/collision, and scheduling.
- Server command pipeline, movement simulation, input validation, and replication.

## Attempts and Findings

### Attempt 1: Prior docs and pitfalls review
- Reviewed `pitfalls/networking.md` and architecture docs.
- Result: Existing notes already identify previous attack→move transition bug and risk from split frame transitions.

### Attempt 2: Client-side flow trace
- Reviewed `player_command_system`, `game_connection_system`, `command_system`, `update_position_system`, `collision_system`, and schedule in `lib.rs`.
- Result: Client receives server `AttackEntity`, applies `NextCommand::with_attack`, then in client `command_system` out-of-range transition rewrites current command to Move and computes standoff destination.

### Attempt 3: Server-side flow trace
- Reviewed server `game_server_system`, `command_system`, `update_position_system`, and `position_reconciliation_system`.
- Result: Server repeats same attack out-of-range transition with standoff math, but also accepts `MoveCollision` by inserting both `NextCommand::with_move(position, None, None)` and direct `Position` insert.

### Attempt 4: Bevy scheduling/deferral verification
- Reviewed Bevy ECS command/deferred internals.
- Result: Commands are deferred and applied at `ApplyDeferred`; system order can expose one-frame state where command and position updates are not yet aligned.

### Attempt 5: Implemented server-side `MoveCollision` guard during targeted pursuit
- Modified `rose-offline-server/src/game/systems/game_server_system.rs` in `ClientMessage::MoveCollision` handling.
- Added `preserve_targeted_pursuit` condition for current command states:
  - `CommandData::Attack { .. }`
  - `CommandData::Move { target: Some(_), .. }`
- Behavior change:
  - Always allow validated authoritative `Position` updates from `MoveCollision`.
  - Do **not** inject targetless `NextCommand::with_move(position, None, None)` while preserving targeted pursuit.
  - Keep old targetless move insertion only when not in targeted pursuit states.
- Expected effect:
  - Prevent attack-range boundary command oscillation and short forward/backward visual snaps.

### Attempt 6: Build validation
- Ran `cargo build` in required separate subtask.
- Result: success, no errors.

## Current Hypothesis
Likely micro-teleport was caused by a short-lived disagreement between:
1. current authoritative `Position`,
2. deferred command transition (`Attack` -> `Move`), and
3. immediate `MoveCollision`-driven targetless `NextCommand::Move` overwrite.

The implemented guard removes (3) during targeted pursuit.

This appears specifically at the pre-combat boundary where attack and move modes rapidly alternate.

## Candidate Fix Directions
1. Server: tighten `MoveCollision` acceptance while in attack pursuit (do not convert collision reports into free-form `NextCommand::Move` with no target).
2. Server: gate/smooth direct `Position` overwrite from `MoveCollision` to avoid abrupt pre-range correction.
3. Client: suppress local `NextCommand::with_stop()` collision interrupts when command target is combat target and server is already driving pursuit.
4. Client/server: add transition hysteresis near attack range boundary to prevent oscillation.

## Validation Plan
- Record logs around attack range boundary for command transitions and incoming packets (`MoveEntity`, `AttackEntity`, `AdjustPosition`, `MoveCollision`).
- Verify no one-frame position jump at transition while preserving collision correctness.
- Re-test moving combat and standing combat scenarios.
