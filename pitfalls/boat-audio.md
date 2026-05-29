# Boat Audio System (Section F)

## Issue
The sailing system had no audio integration. No sound entities were created or managed when boarding/disembarking a boat.

## Root Cause
Section F (Audio System) of the `sailing-system-detailed-expansion.md` plan had not been implemented. The existing `vehicle_sound_system.rs` pattern demonstrated how to attach looped `SpatialSound` entities to vehicle model roots, but no equivalent existed for the sailing system.

## Fix
- Created [`src/audio/boat_sound.rs`](src/audio/boat_sound.rs) with three systems:
  - `ensure_boat_sound_state_system` — adds/removes `BoatSoundState` component and spawns/despawns loop sound entities (wind, creak, flap) when `BoatState.active` changes
  - `boat_loop_sound_update_system` — adjusts `SoundGain::Ratio` on loop entities based on speed ratio, luff factor, and wave motion
  - `boat_one_shot_sound_system` — manages periodic bow splash timers and detects sail trim changes for rope sounds
- Registered the module in [`src/audio/mod.rs`](src/audio/mod.rs) with public re-exports
- Scheduled in [`src/lib.rs`](src/lib.rs): `ensure_boat_sound_state_system` after `boat_toggle_system`, and the update systems after `sailing_movement_system`

## Key Design Decision
The current implementation spawns placeholder child entities without actual `SpatialSound` components (since production sound asset paths aren't known yet). This establishes the entity lifecycle pattern so real sound data can be plugged in later:

1. When `BoatState.active` becomes true: create wind/creak/flap child entities
2. When `BoatState.active` becomes false: despawn those child entities
3. Per-frame: update `SoundGain` on each loop entity based on sailing state

## Future Work
- Replace `spawn_loop_sound` with `SpatialSound::new_repeating(handle)` using real sound IDs from `SoundCache`
- Implement one-shot sounds via actual `SpatialSound::new(handle)` instead of immediate despawn
- Add `SoundRadius` components with proper radius values per sound type
- Consider per-particle material pooling for wake/spray before supporting many remote boats