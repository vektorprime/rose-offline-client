# Simplification Report: Zone Loading & Asset Loading

**PLAN ONLY — no code changes were made.** This document is a research report produced by a read-only analysis agent. All line numbers refer to the codebase as read on 2026-07-31.

## Modules analyzed and LOC

| Module | LOC |
|---|---|
| `src/zone_loader.rs` | 709 |
| `src/zone_loader/loading.rs` | 729 |
| `src/zone_loader/systems.rs` | 783 |
| `src/zone_loader/spawning.rs` | 475 |
| `src/zone_loader/spawning/terrain.rs` | 517 |
| `src/zone_loader/spawning/objects.rs` | 696 |
| `src/zone_loader/spawning/water.rs` | 88 |
| `src/loader.rs` | 8 |
| `src/model_loader.rs` | 1594 |
| `src/effect_loader.rs` | 565 |
| `src/zms_asset_loader.rs` | 267 |
| `src/dds_image_loader.rs` | 995 |
| `src/exe_resource_loader.rs` | 51 |
| `src/vfs_asset_io.rs` | 648 |
| `src/terrain/mod.rs` + `noise_overlay.rs` | 821 |
| `src/animation/` (6 files) | 1481 |
| **Total** | **~10,427** |

Estimated removable/simplifiable: **~2,500–3,000 LOC** (mostly dead code, duplication, and commented-out logging).

---

## Finding 1 — `zmo_asset_loader_fixed.rs` is a dead near-verbatim duplicate of `zmo_asset_loader.rs`

- **Location:** `src/animation/zmo_asset_loader_fixed.rs` (394 lines) vs `src/animation/zmo_asset_loader.rs` (417 lines)
- **What's duplicated:** The two files are ~95% identical: same `ZmoAssetLoader`, `ZmoTextureAssetLoader`, `ZmoAsset`, `ZmoAssetBone`, `ZmoAssetAnimationTexture`, identical channel-splitting logic, identical texture-baking logic. Differences are cosmetic (a few debug logs, `async fn` style, missing `RenderAssetUsages` import path).
- **Proof of dead code:** `src/animation/mod.rs:11` declares `mod zmo_asset_loader;` only. Nothing anywhere references `zmo_asset_loader_fixed` (grep across `src/` finds only the file itself).
- **Suggestion:** Delete `zmo_asset_loader_fixed.rs`.
- **Savings:** ~394 LOC.

## Finding 2 — Two parallel zone-loading pipelines; the AssetServer one is effectively dead

- **Location:** `src/zone_loader/loading.rs:60-206` (`load_zone`, used by `ZoneLoader` AssetLoader impl at `loading.rs:18-58`) vs `loading.rs:260-415` (`load_zone_direct`, used by `zone_loader_system` via `AsyncComputeTaskPool` + `mpsc` channel).
- **What's duplicated:** Both functions are ~90% identical: resolve zone from `ZoneList` via `OnceLock`, load ZON/ZSC-cnst/ZSC-deco, iterate 64x64 blocks, extract NPCs with the same `objects_offset` math, and build the same `ZoneLoaderAsset`.
- **Dead-path evidence:**
  - `ZoneLoader` is registered at `src/lib.rs:1021`, and `vfs_asset_io.rs:316-329` even fabricates 1-byte `.zone_loader` assets, but **no code in the repo ever calls `asset_server.load("...zone_loader")`**. The only `.zone_loader` string uses are a commented-out diagnostic (`lib.rs:865`) and the vfs_asset_io handling itself.
  - `systems.rs` only ever spawns `load_zone_direct` tasks; the `loading_via_async_task: false` polling branch (`systems.rs:335-368`) and the `LoadingZone.zone_assets` / `ready_frames` / `clear_asset_handles` machinery exist only to support the dead AssetServer path.
  - Stale comment: `loading.rs:208-212` says the direct path bypasses "the broken asset loading pipeline in Bevy 0.13.2" — the project is on Bevy 0.18.
- **Suggestion:** Remove the `ZoneLoader` AssetLoader (`loading.rs:1-58`, `load_zone`, `load_block_files`), its registration in `lib.rs:1021-1022`, the `zone_loader` asset-source registration + `.zone_loader` special-casing in `vfs_asset_io.rs:316-329, 589-613`, and the AssetServer polling branch in `systems.rs`. Keep `load_zone_direct`; rename to `load_zone`.
- **Savings:** ~200 LOC plus removal of a whole legacy state-machine branch.

## Finding 3 — `read_bytes_with_priority_sync` vs nested `read_bytes_with_priority`: byte-identical, and triplicated in `VfsAssetIo`

- **Location:** `loading.rs:215-258` and `loading.rs:515-558` (a nested fn inside `load_block_files_direct`).
- **What's duplicated:** The two functions are the same 44 lines: real-filesystem-first read, `VfsFile::Buffer`/`View` handling, identical error messages.
- **Triplicated:** `vfs_asset_io.rs:369-448` (`VfsAssetIo::read`) implements the same "real FS priority → VFS fallback" policy, plus its own `std::fs::read(path)` fallback at line 436.
- **Suggestion:** Extract one shared helper (e.g., `fn read_with_real_fs_priority(vfs, base_path, path) -> Result<Vec<u8>, anyhow::Error>` in `vfs_asset_io.rs` or `rose_file_readers`) and call it from all three sites.
- **Savings:** ~90 LOC + single source of truth for the priority policy.

## Finding 4 — `systems.rs`: duplicate `Spawned` match arm (unreachable) and raw-pointer borrow hacks

- **Location:** `src/zone_loader/systems.rs:310-566`
- **What's duplicated/over-complex:**
  1. **Two `LoadingZoneState::Spawned` arms** — `systems.rs:371-531` and `systems.rs:533-564`. The second is unreachable dead code (Rust pattern-matching would flag it; it performs the "wait 2 frames for physics" logic that can never run).
  2. **Unsafe raw-pointer calls to `spawn_zone`** — `systems.rs:460-468` and `systems.rs:702-709` convert `&mut SpawnZoneParams` to `*mut` and dereference inside `unsafe` blocks, with multi-line comments justifying why it's "safe". This is a borrow-checker workaround for the oversized `SpawnZoneParams` (a single `SystemParam` of 20+ fields, `zone_loader.rs:606-629`).
  3. `zone_loader_system` is ~560 lines (`systems.rs:5-567`).
- **Suggestion:** Delete the second `Spawned` arm. Split `spawn_zone` to take a struct of plain `&mut` resources (or return data the system inserts) so no raw pointers are needed — `spawn_zone` itself already destructures the params (`spawning.rs:50-70`), so this is a mechanical change. Break `zone_loader_system` into per-responsibility systems (channel polling, event dispatch, state advancement).
- **Savings:** ~90 LOC + removal of 2 `unsafe` blocks.

## Finding 5 — `noise_overlay.rs`: ~350 lines of never-called public API

- **Location:** `src/terrain/noise_overlay.rs` (821 lines)
- **What's dead:** Grep across `src/` shows only these are used:
  - `GlobalTerrainNoise::get_noise` (used in `spawning/terrain.rs:132`), `TerrainEnhancementPlugin` (`lib.rs:1001`), `TerrainEnhancementSettings` (`ui_settings_system.rs:17`), `get_thread_local_noise` (`zone_loader.rs:561`), `init_thread_local_noise` (internal).
  - **Unused:** `apply_noise_to_height`, `apply_noise_to_height_with_elevation`, `apply_noise_to_height_with_blend`, `get_terrain_noise`, `ImportantPositions` + all methods, `calculate_blend_factor`, `calculate_blend_factor_smoothstep`, `calculate_blend_factor_smootherstep`, `calculate_height_based_blend`, `calculate_elevation_multiplier`, `calculate_zone_center_blend`, `FlatZone`, `calculate_flat_zones_blend`, `calculate_combined_blend_factor`, `smoothstep`, `smootherstep`, and the `TerrainNoiseGenerator` pub re-export.
- **Over-engineering:** The module maintains **two noise instances** — a thread-local (`TERRAIN_NOISE`, `noise_overlay.rs:149-171`) and a `GlobalTerrainNoise` resource (`noise_overlay.rs:175-200`) — initialized from the same settings, plus three different "apply noise" wrappers for a feature (`noise_enabled` defaults to `false`).
- **Suggestion:** Delete all unused functions and `ImportantPositions`; keep only `GlobalTerrainNoise`, `get_thread_local_noise`, `init_thread_local_noise`, settings, plugin. Consider dropping the thread-local copy and passing the resource to `get_terrain_height` instead (already accessible via `CurrentZone.handle` → `zone_loader_assets`).
- **Savings:** ~350 LOC + state duplication.

## Finding 6 — `ZmsNoSkinAssetLoader` duplicates `ZmsAssetLoader` load body

- **Location:** `src/zms_asset_loader.rs:36-158` vs `:168-266`
- **What's duplicated:** Both `load()` bodies are ~90 lines of identical ZMS→Mesh conversion (position/normal/tangent/color UV1-4 + `material_num_faces` labeled asset); the only differences are the `no_skin` variant skipping joint attributes and the registered extension.
- **Suggestion:** Merge into a single `AssetLoader` with a `Settings` flag (or a `bool` field on the loader struct) for "skip joints". Removes one full loader.
- **Savings:** ~90 LOC.

## Finding 7 — Two complete terrain pipelines (`spawn_terrain` vs `spawn_new_terrain`)

- **Location:** `src/zone_loader/spawning/terrain.rs:4-317` (HIM/TIL mesh + noise) vs `:319-517` (custom `.mesh.bin` binary parser + albedo/normal PNGs)
- **What's duplicated/over-complex:**
  - Two independent mesh-building pipelines selected by `render_config.use_new_terrain` (`spawning.rs:163-184`).
  - `spawn_new_terrain` contains a hand-rolled binary mesh format parser (`read_u32`/`read_f32` closures, `terrain.rs:348-407`) with no serde/bincode — a `block_*_*.mesh.bin` file format documented nowhere in-repo.
  - The final entity-spawn block (EditorSelectable, ZoneObject::Terrain, MapEditorTerrainBlock, TerrainMeshForGrass, Mesh3d, MeshMaterial3d, Transform, Aabb, RenderLayers, NotShadowCaster, RigidBody::Fixed, trimesh Collider, CollisionGroups) is duplicated nearly verbatim (`terrain.rs:257-302` vs `:465-514`).
- **Suggestion:** Decide on one terrain pipeline (the legacy HIM/TIL one is what map-editor saves and `get_terrain_height` assume); remove `spawn_new_terrain` + `new_terrain_mesh` fields (`zone_loader.rs:387`, `loading.rs:480-487/692-717`) if the experimental path is unneeded, or extract the shared entity-bundle spawn into one helper.
- **Savings:** ~150–200 LOC.

## Finding 8 — `spawn_object`: 420-line function with heuristic wind-sway classification and inline material construction

- **Location:** `src/zone_loader/spawning/objects.rs:3-426`
- **What's over-complex:**
  - Does everything: transform conversion, per-object mesh cache, lightmap lookup, `ExtendedMaterial<RoseObjectExtension>` construction, collision-filter flag logic, wind-sway string matching, effect attachment (currently disabled).
  - Wind sway uses **five chained `if/else` lowercase substring checks** on mesh paths (`objects.rs:302-356`): "grass", "leaf/leaves", "foliage/canopy", "bush/shrub/plant", "tree" with a trunk-detection heuristic (`ends_with("b.zms")` etc.). This should be data-driven (config table or a single regex/predicate).
  - Material construction duplicates `create_rose_object_material` (`model_loader.rs:55-83`) — the same `RoseObjectExtension` fields (lightmap_params/lightmap_texture/specular_texture/blink_state/blood_overlay) are set inline in two more places (`objects.rs:157-189`, `objects.rs:497-515`) with slightly different defaults.
  - `ZscEffectType`/`NightTimeEffect` imports in `zone_loader.rs:346,353` are only needed by the commented-out effect block `objects.rs:392-422`.
- **Suggestion:** Split into helpers: `build_object_part_material`, `classify_wind_sway(path) -> Option<WindSway>`, `rose_object_transform(&IfoObject) -> Transform` (this transform pattern — `(x, z, -y)`, quat swizzle, scale swap — is copy-pasted in 5+ places across `objects.rs` and `effect_loader.rs`). Remove commented-out effect code and the `spawn_effect`/`ZscEffectType`/`NightTimeEffect` imports.
- **Savings:** ~100 LOC + removes import dead-ends.

## Finding 9 — Memory/diagnostics scaffolding: `memory_monitor` + `MemoryTrackingResource` + unregistered profiler plugin

- **Location:** `src/zone_loader.rs:13-307` (`memory_monitor`, Windows API structs, `MemorySnapshot`, `format_bytes`), `zone_loader.rs:418-524` (`MemoryTrackingResource`), `src/systems/zone_memory_profiler_system.rs` (424 lines)
- **What's dead/duplicated:**
  - `MemoryTrackingResource` (`zone_loader.rs:419-524`): **every `info!` log call is commented out** (lines 446-451, 457-458, 467-472, 478-479, 485, 497-507, 521); only two `warn!` branches can fire. ~105 lines of instrumentation scaffold whose per-call hooks (`log_mesh_handle_created` etc.) are still invoked from hot spawn paths.
  - `format_bytes` is duplicated in `zone_loader.rs:75-87`, `vfs_asset_io.rs:21-33` (and a non-Windows copy `zone_loader.rs:262-274`).
  - `MemorySnapshot` exists twice with different shapes: `zone_loader.rs:199-255` and `zone_memory_profiler_system.rs:37-59`.
  - `zone_memory_profiler_system.rs` (424 lines) defines `ZoneMemoryProfilerPlugin` + 2 systems, but the plugin is **never registered** in `lib.rs` — dead code (also uses `static mut` counters at lines 374-381, 387-408).
  - `ZoneLoaderCache`/`LoadingZone`/`CachedZone` carry `memory_snapshot_start` and `assets_cleared` fields feeding the dead paths.
- **Suggestion:** Delete the unregistered profiler module or register it; delete `memory_monitor`'s unused pieces (keep `format_bytes` in one place); strip `MemoryTrackingResource` down to live warnings only (or remove entirely if diagnostics are no longer needed — nothing user-visible depends on it).
- **Savings:** ~250 LOC + reduced instrumentation overhead.

## Finding 10 — `vfs_asset_io.rs`: dead `CursorWrapper`, redundant cache layer, stale comments

- **Location:** `src/vfs_asset_io.rs:83-176` (`CursorWrapper`), `:180-289` (global `VFS_FILE_CACHE`), `:464-507` (`read_directory`)
- **What's dead/over-complex:**
  - `CursorWrapper` (94 lines of hand-rolled `AsyncRead`/`AsyncSeek`/`Read`/`Seek`) is **never used** — the codebase already uses Bevy's `VecReader` (`vfs_asset_io.rs:328` etc.).
  - The global file cache (`VFS_FILE_CACHE`, `get_from_cache`/`store_in_cache`) clones `Arc<Vec<u8>>` and copies the full buffer into a fresh `VecReader` on every hit (`vfs_asset_io.rs:366, 388, 416`), i.e., cache-bypass via full copies, layered on top of Bevy's own asset cache. `clear_vfs_file_cache()` is called from `systems.rs:412, 661` every zone switch, meaning the cache rarely survives anyway.
  - `read_directory` carries a 40-line "CRITICAL FIX — DO NOT REMOVE" comment block (`:474-500`) describing a Bevy 0.13-era OOM bug.
  - `new_without_cache` (`:235-246`) is `#[allow(dead_code)]` — unused.
- **Suggestion:** Delete `CursorWrapper`; evaluate whether the global cache earns its complexity (it duplicates Bevy's cache and is wholesale-cleared on zone change — likely removable); trim the warning-wall comments; remove `new_without_cache`.
- **Savings:** ~120 LOC.

## Finding 11 — `dds_image_loader.rs`: 995 lines of hand-rolled DDS decoding with a crates fallback

- **Location:** `src/dds_image_loader.rs`
- **What's over-complex:** ~500 lines of manual pixel decoders (`convert_rgb/bgr/rgba/a8/l8/l8a8/a1r5g5b5/r5g6b5/b5g6r5/a4r4g4b4`, BC1/2/3 block decompression) when the `image` crate — already a dependency and already used as the fallback (`try_image_crate`, `:976-995`) — supports DDS including BC1/2/3 and RGBA8. Formats Bc4/Bc5/Bc6H/Bc7 (`parse_dx10_header`, `:317-339`) are parsed but then simply fall through to the image crate anyway.
  - Note: Rose Online's 16-bit legacy formats (R5G6B5, A1R5G5B5, A4R4G4B4) may or may not be covered by the image crate's DDS support — this needs a quick empirical test before deleting the hand-rolled path (per "validate before acting").
- **Suggestion:** Attempt to load every DDS through `image::load_from_memory` first; keep only the converters the image crate cannot handle (likely the 16-bit Rose formats). Worst case: reduce to ~4 converter fns instead of 12.
- **Savings:** 300–700 LOC (needs validation).

## Finding 12 — Commented-out logging and legacy comments (197 lines in scope)

- **Location:** across all analyzed files, especially `zone_loader/systems.rs` (67), `vfs_asset_io.rs` (42), `zone_loader/loading.rs` (23), `spawning/objects.rs` (22), `zone_loader.rs` (19), `dds_image_loader.rs` (13), plus big commented blocks:
  - `spawning.rs:425-475` — commented-out `spawn_cartoon_sky` (47 lines)
  - `model_loader.rs:592-652` — commented-out `spawn_character_gem_effect` (~60 lines)
  - `model_loader.rs:850-900` — commented-out gem-effect call sites
  - `objects.rs:392-422` — commented-out effect spawning block
  - `zms_asset_loader.rs:60-69` etc. — commented diagnostic blocks
- **Suggestion:** Remove all commented-out code and dead log lines (they are in git history). Trim the remaining active `log::info!` calls that fire per-entity/per-block during zone spawn (e.g., `loading.rs` logs per block, `objects.rs` logs per part).
- **Savings:** ~250 LOC of comments/cruft (plus runtime log overhead).

## Finding 13 — Three parallel file-reading stacks with different caching behavior

- **Location:** `vfs_asset_io.rs` (AssetServer default source, global cache), `effect_loader.rs:37-95` (`EffectCache` for EFT files, manual `vfs.read_file`), `zone_loader/loading.rs` (manual real-FS-priority reads for zone blocks, no cache)
- **What's duplicated:** The same VFS/real-FS file access exists in three flavors: Bevy AssetServer (textures/meshes/motions), `vfs.read_file` (EFT/PTL/ZSC/ZON), and `read_bytes_with_priority*` (zone blocks). Each has its own fallback policy and caching (or lack thereof).
- **Suggestion:** Standardize on one entry point (either route everything through `VfsAssetIo`/AssetServer — the modern, intended path — or extract a single `RoseFileCache`-style reader used by both `effect_loader` and `loading.rs`). At minimum, unify the real-FS-priority helper (Finding 3).
- **Savings:** architectural; ~100 LOC of immediate duplication, more if full unification is pursued.

## Finding 14 — `exe_resource_loader.rs` is a no-op stub

- **Location:** `src/exe_resource_loader.rs:24-46`
- **What's over-engineered:** `ExeResourceLoader::load` reads the file and returns a marker asset `ExeResourceCursor { processed: true }`; the comment (lines 36-37) admits the real cursor work happens elsewhere (`ui_resources.rs`). A full AssetLoader round-trip exists only to deliver a boolean.
- **Suggestion:** If `ui_resources.rs` only needs the handle to exist, replace with a `Resource` + `Event`, or load cursors directly in `ui_resources.rs` and drop the loader/asset entirely (requires checking `ui_resources.rs` dependency).
- **Savings:** ~40 LOC.

## Finding 15 — `loader.rs` is an orphaned file that does not compile standalone

- **Location:** `src/loader.rs` (8 lines)
- **What's dead:** `pub async fn read_asset_bytes(&self, ...)` with no `impl` block, no `use` statements, referencing undefined `self` and `LoadError`. Not declared in `lib.rs`'s module list — not part of the build.
- **Suggestion:** Delete the file.
- **Savings:** 8 LOC (and eliminates confusion).

---

## Prioritized summary — top 5 quick wins

1. **Delete `zmo_asset_loader_fixed.rs` + `loader.rs`** (dead duplicates, 400 LOC). Zero risk — neither file is in the module tree.
2. **Delete the dead AssetServer zone path**: `ZoneLoader` AssetLoader + `load_zone` (`loading.rs:18-206`), `.zone_loader` special-casing (`vfs_asset_io.rs:316-329, 589-613`), AssetServer polling branch + second `Spawned` arm in `systems.rs` (≈350 LOC). Grep confirms nothing ever loads a `.zone_loader` asset; keep `load_zone_direct`.
3. **Strip unused noise-overlay API** (`noise_overlay.rs`, ≈350 LOC) — ~20 exported functions have zero call sites; keep `GlobalTerrainNoise` + thread-local + plugin + settings.
4. **Delete commented-out logging/code** (≈250 LOC: 197 commented log lines + 5 commented-out function blocks) and the unregistered `ZoneMemoryProfilerPlugin` (424 LOC or register it).
5. **Deduplicate file reads**: merge `read_bytes_with_priority_sync`/`read_bytes_with_priority` and align with `VfsAssetIo::read` (≈90 LOC), then merge `ZmsAssetLoader`/`ZmsNoSkinAssetLoader` (≈90 LOC).

Follow-ups (larger effort, still high value): remove raw-pointer `unsafe` blocks in `systems.rs` (Finding 4), pick one terrain pipeline (Finding 7), replace hand-rolled DDS decoders after validation (Finding 11), delete dead `CursorWrapper`/cache layers in `vfs_asset_io.rs` (Finding 10), and data-drive the wind-sway string matching (Finding 8).

**Note:** this is a plan only. No files were modified.
