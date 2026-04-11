# Sailing System Implementation Tracking

## Scope
Implement client-side sailing based on `plans/sailing-system-plan.md` in `rose-offline-client`.

## Attempt Log

### Attempt 1 - Analysis
- Reviewed prior knowledge:
  - `pitfalls/index.md`
  - `pitfalls/water-system.md`
  - `pitfalls/terrain-physics.md`
  - `system-architecture/flying-system-architecture.md`
- Reviewed sailing plan:
  - `plans/sailing-system-plan.md`
- Identified integration points:
  - Components/modules registration (`src/components/mod.rs`, `src/resources/mod.rs`, `src/systems/mod.rs`, `src/events/mod.rs`, `src/ui/mod.rs`)
  - App registration/scheduling (`src/lib.rs`)
  - Input + collision interactions (`src/systems/game_keyboard_input_system.rs`, `src/systems/game_mouse_input_system.rs`, `src/systems/collision_system.rs`)
  - Chat command routing (`src/ui/ui_chatbox_system.rs`)
- Validated Bevy 0.18.1 behavior from source:
  - Message API (`MessageReader`, `#[derive(Message)]`)
  - Hierarchy APIs (`add_child`)
  - Mesh component type (`Mesh3d`)

### Next Planned Implementation Steps
1. Add boat components/events/resources/systems/ui modules.
2. Wire command handling (`/boat`) and movement/collision gating.
3. Register modules/systems/resources in `lib.rs`.
4. Run required separate `cargo build` check and fix errors.

### Attempt 2 - Implementation Completed
- Added new sailing/wind data modules:
  - `src/components/boat.rs`
  - `src/resources/wind_state.rs`
  - `src/events/boat_event.rs`
- Added new systems:
  - `src/systems/wind_system.rs`
  - `src/systems/boat_spawn_system.rs`
  - `src/systems/sailing_movement_system.rs`
  - `src/systems/boat_buoyancy_system.rs`
  - `src/systems/sail_camera_system.rs`
  - `src/ui/ui_sailing_hud_system.rs`
- Integrated module exports and registration:
  - `src/components/mod.rs`
  - `src/resources/mod.rs`
  - `src/events/mod.rs`
  - `src/systems/mod.rs`
  - `src/ui/mod.rs`
  - `src/lib.rs`
- Added `/boat` command handling in chat UI:
  - `src/ui/ui_chatbox_system.rs`
- Added sailing movement gating in existing controls/collision:
  - `src/systems/game_keyboard_input_system.rs`
  - `src/systems/game_mouse_input_system.rs`
  - `src/systems/collision_system.rs`
- Added sailing graphics settings data to:
  - `src/graphics/graphics_settings.rs`

### Build Validation
- Per project rule, build was validated in a separate subtask using the required prompt.
- Build result: **no compilation errors**.
- Follow-up build validation subtask was re-run after final schedule ordering adjustment in `src/lib.rs`.
- Final result: **no compilation errors**.

## Current Status
- Sailing client foundation implementation is complete for this task scope.
- Tracking document is now updated to reflect implementation and validation results.

### Attempt 3 - Sailing Runtime Fixes (Water Height + Collision)
- User-reported issues:
  - Boat floated at the activation altitude when `/boat` was used.
  - Boat could pass through terrain/walls.
- Applied fixes:
  - Updated `src/systems/sailing_movement_system.rs` to sample current water surface from runtime water volumes (`UnderwaterVolumes`) each frame and set `position.z` from the sampled water height.
  - Kept fallback behavior to previous known water height or global `WaterSettings.water_surface_y` when no local volume match exists.
  - Updated `src/systems/collision_system.rs` sailing branch behavior:
    - Sailing no longer uses the flying bypass path.
    - Sailing now performs wall/object shape-cast collision response (same collision path style used by player movement).
    - Added shoreline/land blocking check by comparing terrain height to current water surface, preventing sailing movement onto land.
    - Syncs transform from post-collision sailing position while preserving water-surface Y.
- Build validation:
  - Executed required separate `cargo build` subtask after fixes.
  - Result: **no compilation errors**.

## Updated Status
- Sailing now uses dynamic water-surface height sampling instead of locking to activation altitude.
- Sailing now collides with walls/obstacles and is blocked from moving onto land above the water surface.
