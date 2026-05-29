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

### Attempt 4 - Visual Upgrade (Sailboat Shape + Sails)
- User request: make the boat look more realistic (actual sailboat silhouette, not raft-like).
- Updated procedural model construction in [`spawn_boat_visual()`](src/systems/boat_spawn_system.rs:81):
  - Reworked hull into a multi-part form (core hull, angled port/starboard sides, tapered bow pieces).
  - Added deck and cabin volume to break the flat-raft look.
  - Added sailboat rig details: fore mast, boom, bowsprit.
  - Kept and enlarged mainsail; added a second forward sail (jib-like sail).
  - Adjusted rider seat and rudder placement to match new hull profile.
  - Updated materials/colors for clearer wood + canvas visual separation.
- Build validation:
  - Executed required separate `cargo build` subtask after visual changes.
  - Result: **no compilation errors**.

## Latest Status
- Sailing visuals now render as a stylized sailboat (hull + mast rig + multiple sails), replacing the previous raft-like profile.

### Attempt 5 - Detailed Expansion Integration (A + C + G)
- Added sail subdivision + deformation data support:
  - Expanded [`SailMesh`](src/components/boat.rs:70) with base vertex storage, dimensions, and subdivision level.
  - Added subdivided sail mesh generation in [`create_subdivided_sail_mesh()`](src/systems/boat_spawn_system.rs:86) and quality selection in [`create_sail_mesh_for_quality()`](src/systems/boat_spawn_system.rs:149).
  - Boat spawn now uses [`GraphicsSettings.sailing.sail_deformation_quality`](src/systems/boat_spawn_system.rs:68) for sail mesh density.
- Added runtime sail deformation system:
  - New system [`sail_animation_system()`](src/systems/sail_animation_system.rs:45) with billow/luffing behavior and port/starboard side selection.
  - Registered module export in [`src/systems/mod.rs`](src/systems/mod.rs:76) and app scheduling in [`src/lib.rs`](src/lib.rs:1556).
- Expanded sailing HUD implementation:
  - Replaced minimal HUD with custom compass/speed/trim/prompt rendering in [`ui_sailing_hud_system()`](src/ui/ui_sailing_hud_system.rs:243).
  - Added wind compass painter in [`draw_wind_compass()`](src/ui/ui_sailing_hud_system.rs:34), speed gauge in [`draw_speed_gauge()`](src/ui/ui_sailing_hud_system.rs:113), and trim indicator in [`draw_trim_indicator()`](src/ui/ui_sailing_hud_system.rs:179).
- Expanded sail camera behavior:
  - Added smooth behind-boat yaw tracking, speed-adaptive follow distance, right-mouse free-look override compatibility, and pitch settling in [`sail_camera_system()`](src/systems/sail_camera_system.rs:6).

### Build Validation
- Executed required separate `cargo build` subtask after these continued changes.
- Result: **no compilation errors**.

## Current Status
- Section A (sail deformation), Section C (HUD), and Section G (camera) are now integrated client-side per the detailed expansion document scope for this pass.

### Attempt 6 - Detailed Expansion Integration (B: Wake + Bow Spray)
- Implemented wake/spray components in [`src/components/boat_wake.rs`](src/components/boat_wake.rs):
  - `WakeEmitter`
  - `WakeParticle`
  - `BowSprayParticle`
  - `WakeSource`
- Exported new components via [`src/components/mod.rs`](src/components/mod.rs).
- Implemented wake/spray systems in [`src/systems/boat_wake_system.rs`](src/systems/boat_wake_system.rs):
  - `setup_boat_wake_assets` (shared quad mesh + wake/spray materials)
  - `ensure_boat_wake_emitter_system`
  - `boat_wake_spawn_system`
  - `boat_wake_update_system`
- Registered system module and exports in [`src/systems/mod.rs`](src/systems/mod.rs).
- Registered scheduling in [`src/lib.rs`](src/lib.rs):
  - Post-startup wake asset setup
  - Game update emitter/spawn/update ordering
- Runtime behavior added:
  - V-shaped wake spawn behind boat while moving
  - Bow spray burst at higher speeds
  - Camera-distance budget (~50m) and per-boat particle caps
  - Graphics toggles honored (`wake_particles_enabled`, `bow_spray_enabled`)

### Build Validation
- Executed required separate `cargo build` subtask after wake/spray implementation.
- Result: **no compilation errors**.

## Latest Status
- Detailed expansion section B is now integrated client-side in this repository.

### Attempt 7 - Detailed Expansion Integration (I: Disembark Mechanics)
- Implemented disembark input binding:
  - Updated [`game_keyboard_input_system()`](../src/systems/game_keyboard_input_system.rs:26) to emit [`DisembarkBoatEvent`](../src/events/boat_event.rs:9) on `E` while sailing.
  - Sailing input path now handles `E` without re-enabling normal WASD walk movement.
- Refined `/boat` command semantics:
  - Updated chat command path in [`ui_chatbox_system()`](../src/ui/ui_chatbox_system.rs:490) so `/boat` now sends only [`BoardBoatEvent`](../src/events/boat_event.rs:4) (no implicit toggle/disembark).
- Implemented boarding validation and disembark shore placement in [`boat_toggle_system()`](../src/systems/boat_spawn_system.rs:138):
  - Added client-side boarding validation checks:
    - Zone must be ocean zone `200` (from [`CurrentZone`](../src/resources/current_zone.rs:8)).
    - Must be near a water volume (10m horizontal threshold) using [`UnderwaterVolumes`](../src/render/underwater_effect.rs:124).
    - Cannot board while dead.
    - Cannot board while in combat-like states (`Attack` / `CastSkill`) from [`Command`](../src/components/command.rs:62).
    - Cannot board while in drive mode.
  - Added helper functions for water and shore logic:
    - [`distance_to_volume_horizontal_m()`](../src/systems/boat_spawn_system.rs:24)
    - [`nearest_water_surface_height_cm()`](../src/systems/boat_spawn_system.rs:35)
    - [`is_near_water_plane()`](../src/systems/boat_spawn_system.rs:49)
    - [`find_nearest_shore_position()`](../src/systems/boat_spawn_system.rs:60)
  - Implemented disembark placement:
    - On `DisembarkBoatEvent`, scans 8 compass directions every 2m up to 20m.
    - Finds nearest terrain point above water by margin (`+50cm`) using zone height sampling.
    - Teleports player to shore candidate and deactivates boat.
    - Rejects disembark if no shoreline candidate is found.
  - Added player visual visibility toggling:
    - [`set_character_model_visibility()`](../src/systems/boat_spawn_system.rs:101) hides character mesh parts when boarding and restores on disembark.
  - Added system feedback via [`ChatboxEvent::System`](../src/events/chatbox_event.rs:12) for invalid board/disembark attempts.

### Build Validation
- Executed required separate `cargo build` subtask after disembark mechanics changes.
- Result: **no compilation errors**.

## Latest Status
- Detailed expansion section I is now integrated client-side for this pass:
  - `E` key disembark flow
  - nearest-shore terrain placement
  - client-side boarding validation
  - character visibility toggling during sailing

### Attempt 8 - Next Zone Work (Section D Initial Integration)
- User-requested follow-up: proceed with next zone work after disembark implementation.
- Implemented client-side ocean-zone behavior for zone `200`:
  - Updated [`game_zone_change_system()`](../src/systems/game_system.rs:74) to apply zone-specific water tuning on [`ZoneEvent::Loaded`](../src/events/zone_event.rs:23).
  - Added ocean constants + helper application paths in [`src/systems/game_system.rs`](../src/systems/game_system.rs):
    - [`OCEAN_ZONE_ID`](../src/systems/game_system.rs:15)
    - [`apply_ocean_zone_water_settings()`](../src/systems/game_system.rs:26)
    - [`apply_default_zone_water_settings()`](../src/systems/game_system.rs:19)
  - Ocean-zone water tuning now applies:
    - `wave_amplitude = 1.5`
    - `wave_frequency = 0.8`
    - `foam_intensity = 1.2`
- Added initial data scaffold for zone content authoring:
  - [`3DDATA/MAPS/OCEAN/.gitkeep`](../3DDATA/MAPS/OCEAN/.gitkeep)
  - [`3DDATA/MAPS/OCEAN/OCEAN-zone-scaffold.md`](../3DDATA/MAPS/OCEAN/OCEAN-zone-scaffold.md)
  - Scaffold documents expected exported zone artifacts and map-editor workflow handoff.

### Build Validation
- Executed required separate `cargo build` subtask after zone integration changes.
- Result: **no compilation errors**.

## Latest Status
- Initial section D (Ocean Zone) integration is in place for this pass:
  - zone-200 runtime water behavior in client code
  - OCEAN folder scaffold ready for map export content

### Attempt 9 - Runtime Crash Fix (Bevy Query B0001)
- User-reported runtime crash:
  - Bevy error `B0001` in [`boat_wake_update_system()`](../src/systems/boat_wake_system.rs:221)
  - conflicting mutable access to `Transform` across wake and spray queries.
- Applied fix in [`src/systems/boat_wake_system.rs`](../src/systems/boat_wake_system.rs):
  - Added disjoint filters to the two queries:
    - wake query now uses [`Without<BowSprayParticle>`](../src/systems/boat_wake_system.rs:232)
    - spray query now uses [`Without<WakeParticle>`](../src/systems/boat_wake_system.rs:240)
- This guarantees query disjointness for Bevy's runtime borrow checker and prevents the crash.

### Build Validation
- Executed required separate `cargo build` subtask after the crash fix.
- Result: **no compilation errors**.

### Attempt 10 - Usage Documentation for Ocean Map
- Added end-user run instructions for zone 200 in `docs/how-to-run-game.md`.
- That standalone guide was later removed during markdown cleanup after it became stale; current ocean-zone status lives in [`3DDATA/MAPS/OCEAN/OCEAN-zone-scaffold.md`](../3DDATA/MAPS/OCEAN/OCEAN-zone-scaffold.md).
- Documentation includes:
  - current scaffold status
  - prerequisites (exported `ZON/HIM/TIL/IFO` + server registration)
  - zone viewer and map editor launch examples for zone 200
  - notes on server integration requirements for in-game travel.

### Attempt 11 - Sailing Plan Documentation Expansion
- User request: analyze the sailing plan and expand the planning documents with enough implementation detail for a mid-level engineer to implement remaining work.
- Reviewed project-specific context before editing:
  - `pitfalls` notes for water, terrain/physics, hierarchy/asset readiness, rendering/camera, networking, and zone loading lessons.
  - `system-architecture` documentation, especially ECS scheduling, movement/input/camera, physics coordinates, and the flying-system pattern.
  - Current sailing source files for components, resources, events, systems, UI, graphics settings, zone water behavior, input gating, collision behavior, and implementation history.
- Validated relevant Bevy 0.18.1 behavior from source:
  - `ChildOf`/`Children` hierarchy relationship and recursive despawn behavior.
  - `Message`/`MessageReader`/`MessageWriter` APIs.
  - `ButtonInput<KeyCode>` held/pressed input behavior.
  - `Time::delta_secs`, `Timer`, and schedule registration APIs.
  - `Mesh`, `Mesh3d`, and mutable mesh asset usage requirements.
  - Runtime query disjointness constraints that caused the prior wake/spray B0001 fix.
- Updated planning docs:
  - [`plans/sailing-system-plan.md`](sailing-system-plan.md): added current client baseline, handoff order, updated file inventory, updated risks, and revised priority order.
  - [`plans/sailing-system-detailed-expansion.md`](sailing-system-detailed-expansion.md): added current architecture snapshot, source-validated Bevy rules, updated section statuses, concrete server/networking plan, ocean-zone MVP checklist, audio integration details, remote boat rendering plan, and production disembark/boarding notes.

### Build Validation
- Executed required separate `cargo build` subtask after documentation edits.
- Result: **no compilation errors**.

### Attempt 12 - Full Sailing Plan Client Pass
- User request: analyze the sailing system plan and implement everything feasible in the Bevy 0.18.1 Rust client.
- Affected systems identified:
  - Sailing/boat components and reusable boat visual spawning.
  - Local sailing movement and sail mesh deformation.
  - Remote/player-mode sailing rendering and interpolation readiness.
  - Wake and bow-spray particle material lifecycle.
  - Sailing HUD prompt conditions.
  - Boat audio lifecycle and placeholder sound assets.
  - Shared `MoveMode`/network move-mode encoding needed for server-driven sailing state.
- Planned implementation:
  - Extract duplicated sailing math into a shared pure module with focused unit tests.
  - Add remote boat state/components/systems and make sail animation/wake systems work for non-local boats.
  - Pool wake/spray alpha materials instead of cloning a material per particle.
  - Replace placeholder boat audio entities with generated in-memory `AudioSource` handles.
  - Update HUD disembark prompts to reflect actual shore availability.
  - Run formatting and the required separate `cargo build` validation subtask.
- Implemented:
  - Added `src/sailing.rs` with shared pure sailing math and unit tests.
  - Added `RemoteBoatState` and `remote_boat_sync_system` for remote `MoveMode::Sail` visual spawning/despawning.
  - Exposed the procedural boat visual helper so local and remote boats share the same mesh construction.
  - Updated sail animation, buoyancy, wake emitter, and wake particles so they operate on remote boats as well as the local player boat.
  - Replaced per-particle wake/spray material cloning with pooled alpha-bucket materials.
  - Replaced placeholder no-op boat audio entities with generated in-memory placeholder `AudioSource` handles using existing `SpatialSound` systems.
  - Updated the HUD disembark prompt to show "Press E" only when terrain shore placement is available.
  - Added `MoveMode::Sail` to `rose-game-common` and existing move-mode byte encoding/decoding in `rose-network-irose`.
- Build validation:
  - Executed required separate `cargo build` subtask after implementation.
  - Result: **no compilation errors**.

## Latest Status
- Client-side sailing plan work is complete for the player-mode/server-ready path.
- Remaining plan items that cannot be completed purely in this client repository:
  - Actual ocean zone data files, server zone registration, NPC/vendor/quest data, and warp connections.
  - Server-authoritative sailing simulation and anti-cheat validation.
  - Optional standalone boat entity packets if the server chooses that design instead of `MoveMode::Sail` player-mode sailing.
  - Replacement of generated placeholder boat sounds with production catalogued sound files/IDs.

### Attempt 13 - Server Build Follow-up
- User request: make sure the server builds too.
- Initial required separate server build subtask found `MoveMode::Sail` non-exhaustive match errors in:
  - `C:\Users\vicha\RustroverProjects\rose-offline\rose-offline-server\src\game\systems\game_server_system.rs`
- Applied fixes:
  - Server run/drive toggle handling now explicitly rejects toggles while in `MoveMode::Sail`.
  - Added `PacketServerMoveToggleType::Sail` to `rose-network-irose`.
  - Server protocol now maps `MoveMode::Sail` to the new move-toggle packet type.
  - Client protocol now accepts server `Sail` move-toggle packets and applies `MoveMode::Sail`.
- Build validation:
  - Required separate server `cargo build` subtask in `C:\Users\vicha\RustroverProjects\rose-offline`: **no compilation errors**.
  - Required separate client `cargo build` subtask after the client protocol patch: **no compilation errors**.
