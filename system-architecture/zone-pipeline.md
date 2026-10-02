# Zone Loading Pipeline

How zones go from `LoadZoneEvent` to visible, collidable world. Full loader logic lives in `src/zone_loader/` (re-exported by `src/zone_loader.rs`).

## Flow

1. `LoadZoneEvent(ZoneId)` is written (game flow, zone viewer, map editor all use it; viewers default to `ZoneId(1)` — `src/lib.rs:621-652`, `src/main.rs`).
2. `zone_loader_system` (`src/zone_loader/systems.rs:5`) picks it up in `Update`. It does NOT use the `AssetServer`: it spawns an `AsyncComputeTaskPool` task calling `load_zone()` (`src/zone_loader/loading.rs`), which reads `.zon` / `.zsc` / block files (`.him`, `.til`, `.ifo`, `.lit`) straight from the VFS.
   - Blocks load in parallel: one `AsyncComputeTaskPool` task per block position (at most `thread_num x 2` in flight, so render pipeline compiles on the same pool are not starved), awaited in the original order, so the block list is identical to the old serial loop. Each block task also builds that block's terrain geometry. Each HIM is read once (the existence probe's data is reused), and an on-disk override is read with a single `fs::read` (NotFound falls back to the VFS).
   - **Same-zone reload:** a `LoadZoneEvent` for the zone that is already spawned and current (same-zone respawn/teleport, character select back to login on zone 4) used to re-read the whole zone, only for `zone_loaded_from_vfs_system` to skip the spawn ("already exists"). `ZoneReuseState` stores the `ZoneLoadKey` of the last load (zone id, `use_new_terrain`, the `GlobalTerrainNoise` change tick and a file generation that map-editor writes bump via `notify_zone_files_changed()`); on a match it writes `ZoneLoadedFromVfsEvent(id, CurrentZone.handle)` next frame, which takes the same skip path and emits the same single `ZoneEvent::Loaded`.
   - **Freeing zone data:** `Assets<ZoneLoaderAsset>` is a plain `init_resource` (no asset handle tracking), so replaced zones used to stay in memory forever. `free_unreferenced_zone_assets` removes a loader-added zone asset once the loader's own handle is its last strong handle (every user reaches zone data through `CurrentZone.handle` or an event's strong handle) and drops it on the `AsyncComputeTaskPool`.
3. The background task result returns via mpsc channel (`ZoneLoadChannelSender` / `ZoneLoadChannelReceiver`). The system inserts the result into `Assets<ZoneLoaderAsset>` and writes `ZoneLoadedFromVfsEvent(zone_id, handle)`.
4. `zone_loaded_from_vfs_system` (`src/zone_loader/systems.rs:436`) spawns terrain, objects, and water (`src/zone_loader/spawning/{terrain,objects,water}.rs`, dispatched from `src/zone_loader/spawning.rs`).
   - Terrain block meshes, AABBs and colliders are built on the async load task (`load_zone` → `build_terrain_geometry` / `build_new_terrain_geometry`, stored in `ZoneLoaderBlock::terrain_geometry`); `spawn_zone` moves them into the entities and only rebuilds as a fallback. All legacy terrain blocks share one `TerrainMaterial` per zone.
   - Zone object materials are shared zone-wide (`ObjectMaterialCache`, keyed by texture, sidedness, alpha, ZSC specular flag and lightmap page) and forward-rendered (`rose_object_material`). A lit part's cell in its lightmap page is its `MeshTag`; `rose_object_extension.wgsl` rebuilds the cell (column, row) from it when `lightmap_params.w` (parts per row) > 0 and samples the page at `(uv_b + cell) / parts_per_row` (as the original `lightmap_nolit.vsh`), blending it as the original MODULATE2X. The lightmap UVs are the ZMS second UV set (`MESH_ATTRIBUTE_UV_1`), bound to the forward pipeline by `RoseObjectExtension::specialize`.
   - Animated zone objects (`spawn_animated_object`) use a per-object `RoseEffectExtension` material (forward) with `NoFrustumCulling`, animated by `mesh_animation_system`.
   - Object part trimesh colliders use `SharedMeshCollider` (`src/systems/shared_mesh_collider_system.rs`): one unscaled shape per (mesh, flags), shared by all parts using that mesh; bevy_rapier still applies each entity's scale.
5. `game_zone_change_system` (`src/systems/game_system.rs:59`) runs last and finishes the transition (camera, lighting, player placement).

Ordering is enforced in `src/lib.rs:1227-1253`: `zone_loader_system` → `zone_loaded_from_vfs_system` → `game_zone_change_system`, all before `PhysicsSet::SyncBackend` so colliders exist before queries run.

## Key facts

- Zone data bypasses `VfsAssetIo`/`AssetServer`; everything else (models, textures, dialogs) goes through them. See [asset-loaders.md](asset-loaders.md).
- Terrain height comes from the heightmap for base following; Rapier raycasts add bridges/platforms on top (see [Physics.md](Physics.md)).
- `DeletedZoneObjects` / `MapEditorTerrainBlock` / `MapEditorWaterPlane` let the map editor overlay edits on the same pipeline (see [map-editor-architecture.md](map-editor-architecture.md)).
- Diagnose stuck loads via `[ZONE LOADER SYSTEM]` / `[ZONE_TIME]` logs; NPC/player fall-through right after a transition means `CurrentZone` or zone data was not ready yet.
