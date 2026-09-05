# Zone Loading Pipeline

How zones go from `LoadZoneEvent` to visible, collidable world. Full loader logic lives in `src/zone_loader/` (re-exported by `src/zone_loader.rs`).

## Flow

1. `LoadZoneEvent(ZoneId)` is written (game flow, zone viewer, map editor all use it; viewers default to `ZoneId(1)` — `src/lib.rs:621-652`, `src/main.rs`).
2. `zone_loader_system` (`src/zone_loader/systems.rs:5`) picks it up in `Update`. It does NOT use the `AssetServer`: it spawns an `AsyncComputeTaskPool` task calling `load_zone()` (`src/zone_loader/loading.rs`), which reads `.zon` / `.zsc` / block files (`.him`, `.til`, `.ifo`, `.lit`) straight from the VFS.
3. The background task result returns via mpsc channel (`ZoneLoadChannelSender` / `ZoneLoadChannelReceiver`). The system inserts the result into `Assets<ZoneLoaderAsset>` and writes `ZoneLoadedFromVfsEvent(zone_id, handle)`.
4. `zone_loaded_from_vfs_system` (`src/zone_loader/systems.rs:436`) spawns terrain, objects, and water (`src/zone_loader/spawning/{terrain,objects,water}.rs`, dispatched from `src/zone_loader/spawning.rs`).
5. `game_zone_change_system` (`src/systems/game_system.rs:59`) runs last and finishes the transition (camera, lighting, player placement).

Ordering is enforced in `src/lib.rs:1227-1253`: `zone_loader_system` → `zone_loaded_from_vfs_system` → `game_zone_change_system`, all before `PhysicsSet::SyncBackend` so colliders exist before queries run.

## Key facts

- Zone data bypasses `VfsAssetIo`/`AssetServer`; everything else (models, textures, dialogs) goes through them. See [asset-loaders.md](asset-loaders.md).
- Terrain height comes from the heightmap for base following; Rapier raycasts add bridges/platforms on top (see [Physics.md](Physics.md)).
- `DeletedZoneObjects` / `MapEditorTerrainBlock` / `MapEditorWaterPlane` let the map editor overlay edits on the same pipeline (see [map-editor-architecture.md](map-editor-architecture.md)).
- Diagnose stuck loads via `[ZONE LOADER SYSTEM]` / `[ZONE_TIME]` logs; NPC/player fall-through right after a transition means `CurrentZone` or zone data was not ready yet.
