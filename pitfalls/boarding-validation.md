# Boarding Validation Restoration

## Issue
All client-side boarding validation checks were commented out in `boat_spawn_system.rs` during prototyping, making `/boat` permissive in any zone, while dead, in combat, driving, or far from water.

## Root Cause
The prototype phase intentionally removed validation gates to simplify testing. The sailing plan documents ([`plans/sailing-system-plan.md`](plans/sailing-system-plan.md) line 40) noted this: "some client-side boarding validations... are commented out in the current source."

## Fix
Restored all five validation checks in [`boat_toggle_system`](src/systems/boat_spawn_system.rs:241) in `src/systems/boat_spawn_system.rs`:
1. **Dead check** — `dead.is_some()` prevents boarding while dead
2. **Combat check** — `Command::Attack` / `Command::CastSkill` prevents boarding in combat
3. **Zone restriction** — must be in `OCEAN_ZONE_ID` (200)
4. **Driving check** — `MoveMode::Drive` prevents boarding while driving a vehicle
5. **Water proximity** — must be within 10m of a `WaterVolume`

Each check writes a `ChatboxEvent::System` with a descriptive rejection message for user feedback.

## Future Work
- These are client-side fast feedback checks only. The server must independently validate the same rules for authority.
- The zone restriction (OCEAN_ZONE_ID) assumes the ocean zone exists and is registered. Without real zone data, sailing will only work via the zone viewer or map editor.