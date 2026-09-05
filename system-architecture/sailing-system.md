# Sailing System

Boat travel reuses the shared client/server simulation in `rose-game-common::sailing` (re-exported by `src/sailing.rs`: `sailing_step`, trim/speed helpers) so prediction matches the server.

## Pieces

- Systems: `sailing_movement_system.rs` (shared-math stepping), `boat_buoyancy_system.rs` (float height), `boat_wake_system.rs` (+ `components/boat_wake.rs`), `boat_spawn_system.rs`, `remote_boat_system.rs` (+ `components/remote_boat.rs`), `sail_animation_system.rs`, `sail_camera_system.rs` (boat-follow camera; see [Camera.md](Camera.md)).
- Components: `components/boat.rs`, `components/remote_boat.rs`, `components/boat_wake.rs`.
- Zone data: `src/zone_content/{boats,docks}.rs` (spawn positions, dock definitions).
- Events: `src/events/boat_event.rs`.
- Audio/HUD: `src/audio/boat_sound.rs`, `src/ui/ui_sailing_hud_system.rs`.
- Chat: `/boat`-style commands are parsed alongside `/fly` (`is_boat_command` in the chatbox path; see [flying-system-architecture.md](flying-system-architecture.md)).

## Interaction notes

- `MoveMode::Sail` flows through `Command`/`update_position_system` like walk/run, unlike flight which bypasses `Command`.
- Water volumes/colliders come from `src/zone_loader/spawning/water.rs`; underwater camera state is `CameraUnderwaterState` (`src/render/underwater_effect.rs`).
