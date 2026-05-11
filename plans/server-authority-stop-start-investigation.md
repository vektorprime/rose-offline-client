# Server Authority Stop/Start Investigation

## Scope
- Investigate movement starting, stopping, then restarting.
- Investigate combat continuing logically while client-side combat animation stops.
- Analyze both client and server before proposing a fix.

## Working Hypotheses
- Command/state transitions may oscillate between move/stop/attack states.
- Authoritative movement packets may conflict with local command/animation systems.
- Combat animation completion or command replacement may stop local playback while server damage continues.

## Evidence Log

### 2026-04-18 Initial review
- Reviewed networking pitfalls and authority migration docs.
- Confirmed `system-architecture/` exists and contains animation/input/ECS/transform references.
- Identified likely client touchpoints:
  - `src/protocol/irose/game_client.rs`
  - `src/systems/game_connection_system.rs`
  - `src/systems/command_system.rs`
  - `src/systems/update_position_system.rs`
  - `src/animation/animation_state.rs`
  - `src/animation/skeletal_animation.rs`
  - `src/systems/animation_effect_system.rs`
  - `src/systems/hit_event_system.rs`

### 2026-04-18 Implemented fixes
- Client `AdjustPosition` handling now applies authoritative position correction directly instead of converting it into a new local move command.
- Server periodic reconciliation no longer emits unconditional `AdjustPosition` packets every snapshot interval.
- Server attack processing now rebroadcasts `AttackEntity` at each actual attack cycle start so client combat animations restart in sync with repeated server damage ticks.

### 2026-04-18 Follow-up downhill terrain regression
- Grounded `AdjustPosition` corrections were snapping `Transform.y` from server `position.z`.
- Server ground movement currently preserves/derives horizontal authority better than vertical terrain-following authority.
- Fix: for grounded, non-sailing, non-flying entities, preserve local ground height during `AdjustPosition` and only correct horizontal position immediately.

### 2026-04-18 Follow-up combat-initiation teleport
- Both client and server attack-out-of-range transitions were first setting move destination to the target's exact position.
- Because movement update and command resolution are split across frames, this caused a one-frame lunge toward the target before the regular move-to-range logic corrected it.
- Fix: when transitioning from attack to move, compute the attack-range standoff destination immediately on both client and server.

## Next Steps
- Read client movement/combat packet handling in detail.
- Read server command/movement/combat replication flow in detail.
- Cross-check Bevy timing/transform scheduling assumptions against source.
- Build a packet-to-command-to-animation timeline for movement and combat.
