# Sailing Zone 200 Tracking

## Issue

`/mm 200` returns `Invalid zone id 200`.

## Affected systems

- Shared iROSE zone metadata loading (`LIST_ZONE.STB` backed zone database/list)
- Server chat command validation through `ClientEntityList`
- Client zone loader and host/VFS map-file resolution
- Sailing boarding validation, which requires ocean zone `200` and a loaded water plane

## Attempt 1

- Checked existing sailing scaffold under `3DDATA/MAPS/OCEAN`.
- Confirmed only documentation placeholders existed there.
- Confirmed the server only accepts zones present in the loaded `ClientEntityList`.
- Confirmed shared zone loading requires a `LIST_ZONE.STB` entry and at least one parseable `.IFO` block.

Result: Zone `200` needs real map files plus shared metadata registration.

## Attempt 2

- Generated a minimal ocean map under `3DDATA/MAPS/OCEAN`.
- Mirrored the same files into `target/debug/3Ddata/MAPS/OCEAN` so the current loose-data root can load them immediately.
- Added shared iROSE data-loader fallback registration for zone `200` when `3DDATA/MAPS/OCEAN/OCEAN.ZON` exists.

Result: Binary sanity checks passed for `OCEAN.ZON`, `32_32.HIM`, `32_32.TIL`, and `32_32.IFO`.

## Attempt 3

- Regenerated zone `200` as a 17x17 active block ocean region centered on block `32_32`.
- Kept one large water plane in `32_32.IFO` and mirrored all regenerated files into `target/debug/3Ddata/MAPS/OCEAN`.
- Replaced the blocky procedural boat with a tapered hull mesh, keel, rails, rigging, deck/cabin details, and opaque triangular sails.

Result: Build verification passed after the map scale and boat visual changes.

## Build verification

- Required cargo build subtask reported no client-side errors.
- Required cargo build subtask reported no server/shared-workspace errors.
