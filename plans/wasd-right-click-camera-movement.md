# WASD Right-Click Camera Movement

## Affected systems

- `src/systems/game_keyboard_input_system.rs`: sends periodic WASD `PlayerCommandEvent::Move` commands in `AppState::Game`.
- `src/systems/orbit_camera_system.rs`: uses right mouse drag to rotate the orbit camera and locks the cursor while dragging.
- `src/systems/game_mouse_input_system.rs`: suppresses click-to-move while the cursor is grabbed; this should remain mouse-only behavior.

## Findings

- Architecture docs identify game WASD movement in `game_keyboard_input_system` and orbit camera right-click drag in `orbit_camera_system`.
- `orbit_camera_system` sets `CursorGrabMode::Locked` while right mouse is held for camera rotation.
- `game_keyboard_input_system` returned early whenever the primary window cursor grab mode was not `None`.
- Result: holding right mouse to rotate the camera made the keyboard movement system skip WASD processing entirely.

## Attempts

### Attempt 1

- Change: removed the cursor-grab-mode gate from `game_keyboard_input_system`.
- Expected result: keyboard movement continues while right-click camera drag locks the cursor.
- Mouse click behavior remains guarded in `game_mouse_input_system`, so right-drag camera mode still prevents click-to-move.
- Build result: `cargo build` subtask reported no errors.
