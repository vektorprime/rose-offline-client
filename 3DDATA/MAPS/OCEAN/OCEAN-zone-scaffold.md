# OCEAN Zone Scaffold (Zone 200)

This folder is the initial scaffold for the ocean sailing zone described in:
- `plans/sailing-system-detailed-expansion.md` (Section D)

## Current status

- Directory scaffold created.
- Runtime client behavior for zone `200` is now implemented in code:
  - `src/systems/game_system.rs` applies ocean-specific water settings when zone `200` loads.

## Planned data files (to be authored/exported)

The following files should be generated via map editor export pipeline and/or existing ROSE map tooling:

- `OCEAN.ZON`
- Block files (`HIM`, `TIL`, `IFO`) for active ocean/island blocks
- Optional map metadata/minimap assets as needed by existing zone list/content data

## Authoring notes

1. Build ocean + island blocks in map editor.
2. Export zone data to this folder.
3. Register zone metadata/server-side links in the server repository.
4. Validate zone load path in client.

This scaffold intentionally avoids placeholder binary assets and keeps source control clean until exported map data is ready.
