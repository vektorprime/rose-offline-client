# OCEAN Zone Scaffold (Zone 200)

This folder is the initial scaffold for the ocean sailing zone (zone 200).

## Current status

- Zone `200` now has a minimal playable loose-file map:
  - `OCEAN.ZON`
  - `OCEAN_CNST.ZSC`
  - `OCEAN_DECO.ZSC`
  - Four active terrain blocks around the spawn island/dock
  - A large water plane in block `32_32`
- Runtime client behavior for zone `200` is implemented in code:
  - `src/systems/game_system.rs` applies ocean-specific water settings when zone `200` loads.
- Shared game data registers zone `200` when `3DDATA/MAPS/OCEAN/OCEAN.ZON` exists.

## Authoring notes

1. The current files are intentionally minimal so `/mm 200` and `/boat` can be tested.
2. Future map-editor exports can replace these files with a richer ocean/island layout.
3. Keep `OCEAN.ZON`, at least one parseable `.IFO`, and the two empty ZSC tables unless the zone-list registration is updated.
