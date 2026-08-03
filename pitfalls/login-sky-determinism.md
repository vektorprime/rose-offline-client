# Login Screen Sky Inconsistency (Random Day/Night per Launch)

## Problem

On the login screen (and character select), the sky was inconsistent between
relaunches: sometimes a bright daytime atmosphere sky, other times a nearly
black sky (no sky at all). The appearance changed randomly on every launch.

## Root Cause

Two compounding issues:

1. **Random startup world time.** `WorldTime::default()` initialized the game
   clock with `rand::thread_rng().gen_range(0..=9999)` ticks. On the login
   screen no game server is connected, so this random value was never replaced
   by server time (`game_connection_system` only inserts real server ticks when
   the player spawns into a zone). `zone_time_system` derives the time-of-day
   from those ticks, so the login screen was randomly Day (~54%) or Night
   (~46%) per launch.

2. **Night mode removes the atmosphere.** `toggle_atmosphere_based_on_time`
   removes the Bevy `Atmosphere` component from the camera at night so the
   procedural star field is visible. At night the sun illuminance is also forced
   to 0 and the camera clear color is near-black, so the login screen rendered
   as a dark scene with sparse stars — perceived as "no sky".

3. **Secondary state-desync race.** The toggle set `AtmosphereState.enabled`
   *before* checking whether the camera query succeeded. If
   `camera_query.single()` ever failed (0 or 2+ `Camera3d`), the flag was
   flipped but the camera components were never touched, and there was no
   retry — the flag stayed permanently out of sync with the camera.

## Fix

- `src/resources/world_time.rs`: `WorldTime::default()` now starts at a
  deterministic midday tick (`WorldTicks(WORLD_TICKS_PER_DAY / 2)` = 80) instead
  of a random value.
- `src/systems/zone_time_system.rs`: while in `AppState::GameLogin` or
  `GameCharacterSelect`, the world time is forced to `day_cycle / 2` (noon), so
  menu screens always show the daytime atmosphere sky. In-game time is
  unaffected (server ticks replace `WorldTime` on zone join).
- `src/render/starry_sky_material.rs`: `toggle_atmosphere_based_on_time` only
  updates `AtmosphereState.enabled` *after* `camera_query.single()` succeeds; if
  the query fails, the flag is left unchanged so the toggle retries next frame
  instead of permanently desyncing from the camera.

## Lesson Learned

Non-deterministic startup values (random seeds) for systems that drive visuals
make bugs appear intermittent and are very hard to reproduce. The day/night
cycle is a game-world simulation: menu screens should show a fixed sky, and the
clock should only be randomized (or set from the server) once real game time is
available.
