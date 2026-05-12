# Zone Loader Split Tracking

## Task
Split `src/zone_loader.rs` into smaller modules without changing behavior.

## Affected systems
- Zone asset loading via Bevy `AssetLoader`.
- Async direct VFS zone loading and channel handoff.
- Zone loading ECS systems and `Local` cache state.
- Zone spawning for terrain, water, objects, effects, sounds, and map editor components.
- Exported zone data used by collision, minimap, map editor, UI, and zone events.

## Pre-work results
- Reviewed `pitfalls/zone-loading.md`; important asset handle tracking must be preserved for spawned zone dependencies.
- Reviewed `pitfalls/terrain-physics.md`; zone assets must be inserted into `Assets<ZoneLoaderAsset>` before dependent events/systems use them.
- Reviewed relevant architecture docs for assets, ECS, rendering, and zone lighting.
- Checked Bevy 0.18.1 source:
  - `AssetLoader::load` is async and returns the top-level asset from a byte reader.
  - `AssetServer::get_load_state` reports the root asset load state.
  - `Assets<T>::add` allocates a strong handle and inserts the asset.
  - `Assets<T>::get` reads by asset id/handle.
  - `Commands` is a deferred system param and `Local<T>` derefs to per-system local state.

## Attempts
- Attempt 1: Kept `src/zone_loader.rs` as the public root and moved implementation into:
  - `src/zone_loader/loading.rs` for `ZoneLoader` and raw zone/block loading.
  - `src/zone_loader/systems.rs` for zone loading systems.
  - `src/zone_loader/spawning.rs` for zone entity spawning helpers.
- Attempt 2: Split `src/zone_loader/spawning.rs` further into:
  - `src/zone_loader/spawning/terrain.rs`
  - `src/zone_loader/spawning/water.rs`
  - `src/zone_loader/spawning/objects.rs`
- Result: Refactor split completed and touched files formatted directly with `rustfmt`; build check pending.
