# Optimization Review 07: Asset Loading & Zone Loading

## 1. Title & Scope

**Scope covered by this review**

- VFS bridge to Bevy's asset system (`src/vfs_asset_io.rs`, `src/resources/virtual_filesystem.rs`)
- Zone load pipeline: async load task (`src/zone_loader/loading.rs`), state machine (`src/zone_loader/systems.rs`, `src/zone_loader.rs`), main-thread spawning (`src/zone_loader/spawning.rs` + `spawning/{terrain,objects,water}.rs`)
- Asset loaders: `ZMS`/`ZMO`/`ZMD` meshes (`src/zms_asset_loader.rs`), DDS textures (`src/dds_image_loader.rs`), EXE cursors (`src/exe_resource_loader.rs`, `src/resources/ui_resources.rs`)
- Caches: `EffectCache` (`src/effect_loader.rs`), global VFS byte cache (`vfs_asset_io.rs`), `SoundCache` (`src/resources/sound_cache.rs`), name-tag cache (`src/systems/name_tag_system.rs`)
- Game-data resources: `GameData` (`src/resources/game_data.rs`), `ModelLoader` (`src/model_loader.rs`), zone-content spawners (`src/zone_content/*`)
- Underlying VFS cost semantics in `rose-file-readers` (`virtual_filesystem.rs`, `aruavfs.rs`, `irosephvfs.rs`)

**Existing architecture docs (read during review)**

- `system-architecture/Assets.md` — VfsAssetIo, DdsImageLoader, the design decision that zones bypass the AssetServer
- `system-architecture/README.md` — subsystem index
- `pitfalls/zone-loading.md` — crash history (missing `Assets<ZoneLoaderAsset>` init), zone load path
- `pitfalls/performance-memory.md` — GPU buffer leak pattern: `Assets::add()` without handle removal
- `pitfalls/model-viewer.md` — duplicate bundle component

**Verification discipline:** All behavior statements below were validated against the actual sources listed above; nothing is assumed from documentation alone. Line numbers are accurate as of the reading.

---

## 2. Methodology

1. **Prior knowledge pass** — read `pitfalls/index.md` entries for zone loading, performance/memory, model viewer; read `system-architecture/Assets.md` first.
2. **Hot-path identification** — traced the zone load flow end to end: `LoadZoneEvent` → `zone_loader_system` → async `load_zone` → channel → `zone_loaded_from_vfs_system` → `spawn_zone` → per-block/per-object/per-particle spawning.
3. **Cost-semantics validation from source** — verified in `rose-file-readers` how `VfsFile::Buffer` vs `VfsFile::View` behave, where decompression happens (Arua zlib + 32-byte XOR), and that `normalise_path` uppercases (per-open allocation cost).
4. **Cache lifecycle audit** — for every cache (`VFS_FILE_CACHE`, `EffectCache`, `ZoneLoaderCache`, `Assets<ZoneLoaderAsset>`, `SoundCache`, `NameTagCache`, `MemoryTrackingResource`): where entries are inserted, whether they are ever removed, and what the invalidation triggers are.
5. **Main-thread vs async separation** — classified every VFS read/parse/mesh-build as synchronous-on-main-thread vs off-thread.
6. **Quantified with per-file data sizes** — e.g., 4096 terrain blocks/zone, 100+ tile textures/zone, per-object `mesh_cache` allocs, full-buffer `VecReader` clones of multi-MB files.
7. **Cross-checked call sites** — `clear_vfs_file_cache()` (systems.rs:332, 502), `EffectCache::clear()` (effect_loader.rs:74 — no call site found), `NameTagCache` Resource (dead code — `Local` variant used at name_tag_system.rs:361).

**Not executed:** no `cargo build`, no game runs, no `.rs` changes. This is research + writing only.

---

## 3. Findings

> Impact estimates are order-of-magnitude, based on data sizes observed in the code paths (zone geometry counts, texture formats, cache lifetimes). "Zone size" refers to a 4096-block village/town zone (e.g., zone 1 or 200).

### F1 — All parsed zone data is kept resident forever; memory grows per visited zone (HIGH)

- `src/zone_loader/systems.rs` (~:332, ~:398, ~:500) — `ZoneLoaderCache` (`Local<Vec<Option<CachedZone>>>`, zone_loader.rs:505) stores a `ZoneLoaderAsset` handle per visited zone and never evicts. `Assets<ZoneLoaderAsset>` grows by one entry per zone visit until app exit. `clear_asset_handles()` (zone_loader.rs:488) only clears the *untyped child-handle list*, not the parsed zone data.
- Why it matters: each `ZoneLoaderAsset` holds the parsed HIM heightmap (~8 KB × 1000–4096 blocks), TIL tile indices (~64×64 × 4096 blocks), IFO spawn lists (hundreds of records with strings), and LIT lightmaps. Visiting 20–50 zones retains on the order of hundreds of MB of parsed (non-GPU) data that is never used again.
- Impact: High — slow multi-zone sessions (town ↔ dungeon hopping) accumulate RAM; also defeats the purpose of `clear_vfs_file_cache()` (only the raw byte cache is cleared).
- Fix sketch: on zone unload/replacement, `zone_assets.remove(&handle)` and clear the `ZoneLoaderCache` entry (`cache[zone_index] = None`); or cap the cache to the N most recent zones (LRU). The parsed data is reproducible from the byte cache, so eviction is free.

### F2 — Full zone spawn runs synchronously on the main thread and stalls the frame (HIGH)

- `src/zone_loader/spawning.rs:11` `spawn_zone` is called from `zone_loaded_from_vfs_system` in `Update` (lib.rs:1224); it loops 4096 blocks (`spawning/terrain.rs:4`) and hundreds of objects (`spawning/objects.rs:3`).
- Per block: vertex + UV + normal generation, optional normal smoothing, **4 noise evaluations per vertex** (terrain enhancement), a new `TerrainMaterial`, and a full collider setup — all on the main thread. Per object: mesh asset creation, material creation, lightmap texture loads via `AssetServer::load` (which is async, but the *spawn bookkeeping* is not).
- Why it matters: zone teleport = visible multi-second hitch; the async part (parse) is only ~half the work. Collider generation itself is off-thread (`AsyncCollider`), but everything else is not.
- Impact: High — every zone change causes a long main-thread stall proportional to zone size.
- Fix sketch: (a) chunk the spawn — spawn N blocks/objects per frame over several frames (system state machine keyed off `CurrentZone`); (b) move mesh generation (vertices, normals, noise) into the async task that already runs `load_zone`, returning ready-to-add mesh data through the channel; (c) at minimum, move the terrain-noise + vertex building into `AsyncComputeTaskPool` and send results back as events.

### F3 — Zone data read path bypasses the byte cache and re-decompresses everything per visit (HIGH)

- `src/zone_loader/loading.rs:20` `read_bytes_with_priority` reads real-FS-first, then `vfs.open_file` — it does **not** consult the global `VFS_FILE_CACHE` (vfs_asset_io.rs:34–50). `load_block_files` (loading.rs:173) then re-opens each HIM/TIL file a second time for an existence probe (see F14).
- Why it matters: Arua VFS data is zlib-compressed (+ XOR of first 32 bytes, `rose-file-readers/src/aruavfs.rs`); every zone visit re-reads and re-decompresses ~10k+ files even if the identical bytes were cached a minute earlier. The byte cache was designed for exactly this, but the zone path does not use it.
- Impact: High — repeated zone visits (dungeon grinding, town returns) pay full decompress cost each time; the parse task is also fully sequential (F15).
- Fix sketch: route `read_bytes_with_priority` through `vfs_asset_io::get_from_cache`/`store_in_cache` (or a dedicated zone-block cache keyed by normalized path). Cache invalidation on zone change already exists (`clear_vfs_file_cache()`).

### F4 — `clear_vfs_file_cache()` nukes *all* cached assets on every zone change (MEDIUM-HIGH)

- `src/zone_loader/systems.rs:332` and `:502` call `clear_vfs_file_cache()` (vfs_asset_io.rs:44), which clears the **entire** global cache — including shared UI textures (ui_resources.rs), the specular texture, skybox data, and models that are identical across zones.
- Why it matters: right after the clear, every Bevy asset that was evicted re-reads + re-decompresses on first use in the new zone; and `clear_vfs_file_cache()` itself takes an exclusive lock on a `RwLock` that all asset loads contend for.
- Impact: Medium-High — repeats a large chunk of the startup decompress cost at every zone change.
- Fix sketch: tag cache entries (e.g., `zone:<n>:` vs `global:` prefix) and only evict entries tagged with the outgoing zone; or LRU-evict down to a byte budget instead of clearing.

### F5 — DDS loader converts everything to R8G8B8A8, CPU-decompresses, and drops mipmaps (HIGH)

- `src/dds_image_loader.rs:42–47` — every format (DXT1/DXT3/DXT5, A8R8G8B8, R5G6B5, R8G8B8A8) is converted to R8G8B8A8. Mip count is parsed but mip chains are not preserved.
- Why it matters: 4 bytes/pixel × (terrain tiles + object textures + lightmaps + UI). A 256×256 DXT1 tile costs 256 KB uncompressed vs 64 KB on disk (4×); lightmaps are the same. Without mipmaps, the GPU samples minification at full res (aliasing + cache misses), and every load pays a full CPU decode. Terrain alone is 100+ tiles × ~256 KB.
- Impact: High — VRAM footprint multiplies 2–8× for textures; load-time CPU cost per texture; visible shimmer on distant terrain.
- Fix sketch: pass the source format through when it is GPU-native (Bevy `Image` can hold compressed formats; the crate renders them via wgpu); request mip generation (`ImageLoaderSettings`/`Image::from_buffer` with `ImageMipGenerator`); keep R8G8B8A8 only for formats Bevy can't upload natively (e.g., R5G6B5).

### F6 — A new `ExtendedMaterial` instance is created per object part per instance (HIGH)

- `src/zone_loader/spawning/objects.rs:3` `spawn_object` calls `object_materials.add(...)` per part; hundreds of objects × multiple parts per zone = hundreds–thousands of material assets that are identical whenever two parts share the same texture set. Same pattern in `spawn_model` (`src/model_loader.rs`, used by every character/NPC/vehicle).
- Why it matters: each material is a distinct Bevy asset (bind-group layout, CPU struct, GPU descriptor set); duplicate materials waste GPU descriptors and memory, and more shader instances to manage. `Assets<Material>` never evicts these on zone change either.
- Impact: High for object-heavy zones (cities, dungeons with many props); memory + descriptor pressure.
- Fix sketch: dedupe with a `HashMap<(Vec<Handle<Image>>, flags), Handle<...>>` per zone (mirror the `tile_texture_map` pattern in terrain.rs), insert once, clone handles.

### F7 — `tile_textures` cloned into every terrain block material (MEDIUM)

- `src/zone_loader/spawning/terrain.rs:249` — each block creates a `TerrainMaterial { textures: tile_textures.clone(), ... }`; the contents are identical for all blocks in a zone (same ~100–300 tile handles), so every block duplicates the full handle list and creates a duplicate material asset.
- Why it matters: hundreds of blocks × 100+ handles = tens of thousands of redundant handle clones + duplicate material assets with identical bind groups.
- Impact: Medium — per-zone one-time cost, but multiplied across zone loads and never deduped.
- Fix sketch: create one `TerrainMaterial` per zone, `terrain_materials.add(...)` once, and give every block a clone of that single handle.

### F8 — `mesh_cache` allocated per object spawn (`vec![None; len]`) (LOW)

- `src/zone_loader/spawning/objects.rs:40` — `let mut mesh_cache: Vec<Option<Handle<Mesh>>> = vec![None; zsc.meshes.len()];` per `spawn_object` call (same at `map_editor/systems/model_placement_system.rs:293`).
- Why it matters: one allocation per object part per zone load (thousands of small allocs per zone); trivial individually, but pure waste on a hot path.
- Impact: Low — allocation churn only.
- Fix sketch: reuse a `Local<Vec<Option<Handle<Mesh>>>>`, `clear()` + `resize()`, or `Vec::with_capacity` reuse across calls.

### F9 — NPC/character skeletons are re-read + re-parsed per spawn; inverse bindposes created per spawn (MEDIUM-HIGH)

- `src/model_loader.rs:253–257` — `self.npc_chr.skeleton_files.get(...).and_then(|p| self.vfs.read_file::<ZmdFile, _>(p).ok())` runs synchronously on the main thread **for every NPC spawn**, including the Arua decompress. `spawn_skeleton` (model_loader.rs:1083) then creates a fresh `SkinnedMeshInverseBindposes` asset per spawned character.
- Why it matters: a village with 30 visible NPCs re-reads + re-decompresses the same ZMD 30 times at login/spawn, and each spawn allocates a new inverse-bindposes GPU asset (never evicted; grows unbounded with spawns over a session). The `ModelLoader` already caches parsed ZSC/ZMD *avatar* skeletons, but not NPC skeletons.
- Impact: Medium-High — login/zone-enter hitches in NPC-dense zones + unbounded asset growth.
- Fix sketch: cache parsed `ZmdFile` in `ModelLoader` (keyed by skeleton path); cache/dedupe `SkinnedMeshInverseBindposes` by skeleton id (`BevyAssets`-style map); reusing bind poses is safe for identical skeletons.

### F10 — PTL particle files re-read per particle per spawn; `EffectCache` unbounded and never cleared (MEDIUM)

- `src/effect_loader.rs:93` `spawn_effect` — EFT is cached (`EffectCache`, :55–70) but every particle's PTL file is read from VFS **per spawn**: `vfs.read_file::<PtlFile, _>` in the particle loop (synchronous, on the main thread). `EffectCache::clear()` exists (effect_loader.rs:74) but has **no call site** in the codebase (verified by grep), so the cache grows unbounded over the session.
- Why it matters: monster idle/attack effects, skill effects, and weather effects re-read PTL per particle per spawn; with cached EFT the savings are partial. `spawn_effect` is also called synchronously inside zone spawn (`spawning/objects.rs:444` `spawn_effect_object`) — every IFO effect object pays this per particle.
- Impact: Medium — repeated VFS opens + parses per effect instance; cache memory growth over long sessions.
- Fix sketch: cache parsed `PtlFile` alongside the EFT (keyed by path); bound the cache (clear on zone change — the intended-but-unwired behavior); reuse the particle mesh/`ShaderStorageBuffer` per particle type instead of per instance.

### F11 — EXE cursor loading pulls the whole `trose.exe` through the byte cache (MEDIUM)

- `src/resources/ui_resources.rs:635–646` — 12 cursors load `"trose.exe#cursor_*"`. The ExeResourceLoader (`src/exe_resource_loader.rs`) is a stub (`processed: true`), but the read path goes through `vfs_asset_io.read`, which reads the **entire** exe (tens of MB) on first cursor request and caches it in the global byte cache; each subsequent cursor `read` then `VecReader::new((*cached).clone())` copies the whole buffer (vfs_asset_io.rs:154).
- Why it matters: one-time ~tens-of-MB read + clone per cursor; also keeps the full exe in the byte cache until the next `clear_vfs_file_cache()` — an oddity because the loader itself does nothing with the bytes.
- Impact: Medium (startup) — wasted startup time and cache residency.
- Fix sketch: parse the exe cursors once at startup into a shared `Vec<Image>`, and make `UiCursor::new` take an existing handle instead of loading through the asset server per cursor; or read the exe once and store handles.

### F12 — Every asset read copies the full file buffer into `VecReader` (MEDIUM-LOW)

- `src/vfs_asset_io.rs:154, 171, 190, 197, 206` — cache stores `Arc<Vec<u8>>`, then `VecReader::new((*cached_data).clone())` performs a full memcpy of the file for every read. Multi-MB DDS/ZMS/zone files are copied per read, even on cache hits.
- Why it matters: cache hits are supposed to be the fast path; they still cost a full file-size copy per read. With hundreds of texture loads per zone, this is many tens of MB of memcpy.
- Impact: Medium-Low — per-load memcpy overhead on the asset-loading threads.
- Fix sketch: implement a `Reader` that borrows/slices the `Arc<Vec<u8>>` (e.g., `ArcReader`/offset reader with `take(n)` support) — avoids the clone entirely; Bevy only needs `AsyncRead` semantics.

### F13 — Cache keys not normalized: case variants create duplicate entries (LOW)

- `src/vfs_asset_io.rs` — cache key is the raw `path_str` as Bevy hands it, while `VfsPath::normalise_path` (rose-file-readers `virtual_filesystem.rs`) uppercases internally. A path requested once as `texture.dds` and once as `TEXTURE.DDS` produces two cache entries and two decompressions.
- Why it matters: inconsistent-case callers (zone loader vs asset server vs direct VFS calls) double the decompress cost for the same file.
- Impact: Low-Medium — depends on how often case differs between callers.
- Fix sketch: uppercase (or run the same `normalise_path` transform) before cache get/insert. Also note `normalise_path` allocates 3–4 strings per open (replace + uppercase) — pre-normalized keys avoid repeated cost.

### F14 — Double file open per block: existence probe + read (LOW)

- `src/zone_loader/loading.rs:173` `load_block_files` — `vfs.open_file(&him_path)` is used as an existence probe, then `read_bytes_with_priority` (loading.rs:20) opens the file again (and does a `base_path.join(...).exists()` stat first). Thousands of files × 2 opens + 1 stat.
- Why it matters: ~2× open/lookup cost and ~10k+ extra syscalls/stats per zone load.
- Impact: Low-Medium — measurable in wall-clock for zone loading.
- Fix sketch: single read path returning `Option<Vec<u8>>` (probe + read merged); skip the real-FS `.exists()` stat when the file is known to be in the VFS index.

### F15 — Zone load task is fully sequential (MEDIUM)

- `src/zone_loader/systems.rs:173` — one `pool.spawn` per zone; inside `load_zone` (loading.rs:65) the 4096 block reads + parses run sequentially on a single async task.
- Why it matters: a zone's I/O + zlib-decompress work could be split across the (many-core) `AsyncComputeTaskPool`; today one core does everything while others idle.
- Impact: Medium — zone load time scales linearly with zone size instead of dividing across cores.
- Fix sketch: fan out per-block loads (e.g., chunked 8–64 block reads) with `Arc<VirtualFilesystem>` (mmap-backed devices are `Send + Sync`), join with `futures_lite::future::zip_all`; keep spawn-to-ECS on the main thread via events (F2).

### F16 — Terrain block components duplicate the zone's raw data (MEDIUM)

- `src/zone_loader/spawning/terrain.rs` — each block stores `him_heights_cm` and `til_tiles` clones (`MapEditorTerrainBlock`), while the same data still lives in the `ZoneLoaderAsset` (which is retained forever per F1).
- Why it matters: two live copies of every block's heightmap + tilemap for the whole zone (and the block copies die with the entity on zone change, so the asset copy remains the permanent one).
- Impact: Medium — memory, plus per-block clone cost at spawn.
- Fix sketch: store only block coordinates/index in the component; keep the canonical data in the `ZoneLoaderAsset` (or Arc-share the arrays).

### F17 — `spawn_zone` logs per block at info level (LOW)

- `src/zone_loader/spawning.rs` — `log::info!` per block during spawn (4096+ lines per zone), each with format allocation; the session logger writes every line to `logs/<session>/structured.jsonl`.
- Why it matters: log file volume and formatting cost during the most time-critical section; masks real progress in the logs.
- Impact: Low — log noise + I/O, not gameplay.
- Fix sketch: one summary `log::info!` per zone (blocks, objects, materials, timing); per-block at `debug`.

### F18 — `is_directory` logs info per call (LOW)

- `src/vfs_asset_io.rs:280` — every `is_directory` call (Bevy calls it during asset discovery) writes an info log line. `read_directory` correctly returns an empty stream (vfs_asset_io.rs:230–273) — that behavior must NOT be changed (see Risks R5).
- Why it matters: pure log noise.
- Impact: Low.
- Fix sketch: demote to `trace` or remove.

### F19 — `update_ui_resources` rescans all spritesheets each frame until done (LOW)

- `src/resources/ui_resources.rs` (`update_ui_resources`, scheduled in EguiPrimaryContextPass, lib.rs:134) — each frame iterates all sprite-sheets/`egui` textures while any is pending; premultiply conversion is O(w×h) per texture.
- Why it matters: bounded one-time cost, but it sits in the UI render pass on the hot path.
- Impact: Low — one frame blip at login/zone change.
- Fix sketch: track a "pending count" and skip iteration when zero; process a bounded number of textures per frame.

### F20 — `resources/name_tag_cache.rs` is dead code; live cache is a `Local` (LOW)

- `src/resources/name_tag_cache.rs` — `NameTagCache` Resource has no `Res/ResMut` usage anywhere (verified by grep). The working cache is `Local<NameTagCache>` in `name_tag_system.rs:361`, rebuilt on `LoadZoneEvent`/pixels-per-point change.
- Why it matters: dead resource + misleading docs; the live cache is correctly scoped (fine).
- Impact: Low — cleanup.
- Fix sketch: delete the unused Resource or wire it in; keep the `Local` behavior.

### F21 — NPC/model spawns via ModelLoader run synchronous VFS reads (MEDIUM-LOW)

- `src/model_loader.rs` `spawn_npc_model`/`spawn_model` — ZSC lookups use the cached parsed ZSC (good), but the ZMD skeleton read (F9) and per-part texture loads go through synchronous VFS reads on the main thread at spawn time (login, NPC spawn-in, skill models).
- Why it matters: per-character spawn cost on the main thread; in NPC-dense zones this compounds F9.
- Impact: Medium-Low — combined with F9, the main hitches.
- Fix sketch: covered by F9's skeleton cache + pre-loading shared per-zone textures through the AssetServer (async) before spawn.

### F22 — Zone-content systems poll `Assets<ZoneLoaderAsset>` per frame (LOW, OK)

- `src/zone_content/{boats,docks,monsters}.rs` + `game_connection_system.rs:2608` — per-frame `zone_query.iter().find(...)` / `assets.get(&zone.handle)` are cheap hash lookups; `spawn_docks_system` uses a `Local<bool>` one-shot guard (good).
- Why it matters: no action needed — recorded as evidence that the zone ECS layer itself is already lean.
- Impact: Negligible.
- Fix sketch: none.

### F23 — `MemoryTrackingResource` grows unbounded across the session (LOW)

- `src/zone_loader.rs` `memory_monitor` + unique-asset-path set — the `HashSet<String>` of seen asset paths never shrinks; strings are small.
- Why it matters: trivial memory; the *value* of the metric is fine.
- Impact: Low.
- Fix sketch: none required; cap or ignore.

### F24 — Repeated `remove`/`insert` churn on `Assets<ZoneLoaderAsset>` (MEDIUM-LOW)

- `src/zone_loader/systems.rs` (~:490–502) and `zone_loader.rs:496` — the handle-dance (`zone_assets.remove(&old)` then re-insert) plus `clear_asset_handles`/`shrink_to_fit` runs at every zone load; combined with F1's retention, this is churn without eviction.
- Why it matters: re-hash + re-alloc per zone load; the memory the "memory fix" claims to free is not actually freed (F1).
- Impact: Medium-Low — allocation churn + misleading memory accounting.
- Fix sketch: fold into F1's real eviction; remove `shrink_to_fit` on hot path (keep spare capacity).

### F25 — Lightmap textures decoded per object part per zone (MEDIUM-LOW)

- `src/zone_loader/spawning/objects.rs` — each part loads its lightmap via `AssetServer::load(path)`; Bevy dedups by path (good), but each unique lightmap pays the F5 full decode cost, and the DDS→R8G8B8A8 conversion applies per lightmap (LIT lightmaps are 64×64×4 per block, hundreds per zone).
- Why it matters: one-time per-zone CPU decode of hundreds of lightmaps.
- Impact: Medium-Low — folded into F5's fix; listed for completeness of the spawn-time cost inventory.

### F26 — VFS device layer: no caching of decompressed buffers (MEDIUM-LOW, design-level)

- `rose-file-readers/src/aruavfs.rs` / `irosephvfs.rs` — decompression happens per `open_file`; the client-side byte cache (vfs_asset_io.rs) mitigates this only for AssetServer loads, not for direct `vfs.read_file` callers (F3, F9, F10).
- Why it matters: the cache's effectiveness depends on every caller using it; today the zone path and effect path don't.
- Impact: Medium-Low (aggregated into F3/F9/F10).
- Fix sketch: consolidate all reads through one caching read function in the client (single choke point).

---

## 4. Priority-Ranked Table

| # | Finding | Severity | Effort | Main file:line | Verdict |
|---|---------|----------|--------|----------------|---------|
| F1 | Parsed zone data retained forever | High | Low–Med | systems.rs:332/398/500, zone_loader.rs:505 | Do first with F4 |
| F2 | Full zone spawn on main thread | High | Med–High | spawning.rs:11 | Chunk or move to async |
| F3 | Zone reads bypass byte cache (re-decompress per visit) | High | Low–Med | loading.rs:20, 173 | Route through cache |
| F5 | DDS → R8G8B8A8 + no mipmaps | High | Med | dds_image_loader.rs:42–47 | Keep BC + mips |
| F6 | Material per object part per instance | High | Low–Med | objects.rs:3, model_loader.rs | Dedupe per zone |
| F4 | Full cache clear on every zone change | Med-High | Low | systems.rs:332, 502 | Tagged/LRU eviction |
| F9 | NPC skeleton re-read + bindposes per spawn | Med-High | Low–Med | model_loader.rs:253–257, 1083 | Cache skeletons/bindposes |
| F10 | PTL re-read per particle; EffectCache never cleared | Medium | Low | effect_loader.rs:93, 74 | Cache PTL; bound cache |
| F15 | Sequential zone load task | Medium | Med | systems.rs:173 | Parallelize blocks |
| F16 | Terrain data duplicated per block | Medium | Low | terrain.rs | Reference, don't clone |
| F7 | tile_textures cloned per block material | Medium | Low | terrain.rs:249 | One material per zone |
| F11 | Whole trose.exe pulled for cursors | Medium | Low–Med | ui_resources.rs:635–646 | Parse once, share |
| F12 | Full-buffer copy per read | Med-Low | Med | vfs_asset_io.rs:154 | Arc/offset Reader |
| F25 | Lightmap decode per part | Med-Low | — | objects.rs | Fold into F5 |
| F24 | Asset handle churn without eviction | Med-Low | Low | systems.rs:490–502 | Fold into F1 |
| F13 | Cache keys not normalized | Low | Low | vfs_asset_io.rs | Uppercase keys |
| F14 | Double open per block file | Low | Low | loading.rs:173 | Merge probe+read |
| F8 | mesh_cache alloc per object | Low | Low | objects.rs:40 | Reuse Vec |
| F17 | Per-block info logs (4096/zone) | Low | Low | spawning.rs | Summary log |
| F18 | is_directory info log | Low | Low | vfs_asset_io.rs:280 | trace level |
| F19 | UI texture rescan per frame | Low | Low | ui_resources.rs | Pending-count gate |
| F20 | Dead NameTagCache Resource | Low | Low | resources/name_tag_cache.rs | Remove |
| F23 | Unbounded memory-tracking set | Low | Low | zone_loader.rs | Optional cap |
| F21 | Sync VFS reads in ModelLoader | Med-Low | Med | model_loader.rs | Fold into F9 |
| F22 | Per-frame zone lookups | — | — | zone_content/* | No action |
| F26 | No cache at VFS device level | Med-Low | Med | rose-file-readers | Single read choke point |

---

## 5. Quick Wins

Ordered by (effort, risk):

1. **F13 + F14** — normalize cache keys (uppercase) and merge the probe+read in `load_block_files`. Two small edits in `vfs_asset_io.rs`/`loading.rs`; immediately reduces zone-load opens and duplicate decompressions.
2. **F17 + F18** — replace per-block info logs with a per-zone summary; demote `is_directory` log to trace. Zero-risk, instantly cleaner logs and less formatting cost in the hottest section.
3. **F8** — reuse a `Local<Vec>` for `mesh_cache` in `objects.rs` (and the map-editor copy). Pure allocation-churn removal.
4. **F7** — build one `TerrainMaterial` per zone instead of per block in `terrain.rs:249`. Small change, removes hundreds of duplicate material assets per zone.
5. **F20** — delete the dead `NameTagCache` Resource (keep the `Local` in name_tag_system.rs:361).
6. **F6 (reduced scope)** — dedupe `object_materials` by texture set per zone (the biggest asset-count win with the same pattern as the terrain tile map).
7. **F4 (reduced scope)** — skip `clear_vfs_file_cache()` when the new zone shares no files with the old one (heuristic), or clear only zone-prefixed entries once F3's cache routing exists.

---

## 6. Risks & Considerations

- **R1 — `read_directory` must stay empty.** vfs_asset_io.rs:230–273 contains the documented 2 GB/s OOM history when directory listing was enabled (Bevy hot-reload discovery loop). Any change touching this function is prohibited unless the reload path is redesigned with a dedicated, non-AssetReader API.
- **R2 — F5 format changes interact with renderer assumptions.** `RenderConfiguration.passthrough_terrain_textures` (lib.rs:863) and the custom shaders (object/terrain/water/effect materials) may assume 4-channel textures in places (e.g., alpha sampling). Keep R8G8B8A8 for anything sampled as alpha; verify each shader's sampler format before shipping BC textures. Start with mipmap generation (safe) before compressed formats.
- **R3 — Cache eviction correctness (F1/F4).** Zone revisit must remain fast: if the parsed `ZoneLoaderAsset` is evicted, the byte cache should still hold the compressed blocks so re-parse is cheap; otherwise you trade RAM for repeated decompression. Evict parsed assets + byte cache together, and measure revisit time.
- **R4 — Spawn chunking (F2) interacts with `AsyncCollider`, memory tracking, and `force_zone_visibility_system`** (lib.rs:1230). Colliders are computed off-thread after spawn; chunked spawns keep entities appearing progressively — ensure `CurrentZone`/`ZoneEvent::Loaded` semantics (game_zone_change_system ordering, lib.rs:1240) still fire after the *last* chunk, not the first.
- **R5 — F9 skeleton/bindposes caching memory tradeoff.** Caching parsed ZMDs is cheap (small files, bounded by NPC skeleton count). Caching `SkinnedMeshInverseBindposes` must be keyed by skeleton id and should be cleared on zone change to avoid unbounded growth (same pitfall as `pitfalls/performance-memory.md`: added-but-never-removed assets).
- **R6 — Bevy 0.18.1 API surface.** `Image` supports compressed formats and mip generation, but exact signatures were not re-verified against the vendored source (bevy-collection) beyond what the loaders already use (`RenderAssetUsages`, `ImageLoaderSettings`); validate F5's implementation against `bevy_image` in the 0.18.1 source tree before committing to it.
- **R7 — Effect cache clearing (F10) affects spawned effects.** `EffectCache` entries may be in use by live effect entities; clearing is safe because `spawn_effect` holds `Arc<EftFile>`, but re-clearing mid-fight re-reads on next spawn — acceptable, matches the documented intent ("useful for zone transitions").
- **R8 — Log volume vs debuggability.** F17/F18 remove per-block lines; if a zone-spawn bug needs per-block traces later, prefer a runtime-flag `debug` gate rather than re-adding info logs.
- **R9 — SoundCache is the reference-good pattern.** `src/resources/sound_cache.rs` (fixed `Vec<Option<Handle<AudioSource>>>` indexed by `SoundId`) is bounded, indexed, and correctly scoped — F1/F10 evictions should mimic this structure (fixed-size + index) rather than `HashMap`+clear.

---

## 7. Verification Update (2026-08-04)

Independent sub-agent scrutiny of every finding (F1–F26) against the actual source, Bevy 0.18.1, `rose-file-readers`, and the previous implementation attempt on `wip/local-changes-2026-08-04` (regressions in `12-validation.md` §3.5/§3.9). Verdicts per finding:

| Finding | Verdict | Scrutiny result / action |
|---|---|---|
| F1 | CONFIRM (fix on wip, correct) | Claim accurate (cache never evicts; only the untyped child-handle list is cleared; tens of MB–1 GB retained over 20–50 zones). The wip eviction (keep {incoming, current} at both sites — 12-validation §3.9's "keep = incoming only" is **stale**, fixed in the final commit; R3 handled via F3's zone-tagged byte cache, so A↔B bounces are parse-free AND decompress-free) is correct; no LRU needed (town↔dungeon is the real pattern). Two required follow-ups before merge: (1) the stale-drop branch in `zone_loaded_from_vfs_system` should also `remove(&event.zone_handle)` — a superseded load leaks one asset; (2) fix the §3.5 map-editor real-FS priority inversion (same commit). Minor doc nit: the asset holds LIT *file names*, not lightmap pixels. |
| F2 | CONFIRM (claim) / fix: (b)+(a) hybrid, deferred | Claim accurate; two doc errors: "AsyncCollider off-thread" is FALSE (terrain colliders are sync `Collider::trimesh` on the spawn path; object colliders are deferred but built inline on the main thread in the physics schedule), and noise is **5 evals/vertex** (base + 4 neighbors), conditional on the noise toggle — default OFF (spawn ≈ 1–2 s; with noise ON, 8–22 s). Not implemented on wip. Recommended fix: move mesh+collider-vert building into the existing async `load_zone` (task-local generator from the same settings snapshot — consistent with `get_terrain_height`; requires **batched** delivery, all-blocks-at-once ≈ 1.2 GB transient) + chunked drain on the main thread; `ZoneEvent::Loaded`/`CurrentZone` must fire after the last chunk (safe: the player doesn't exist during spawn). Plain chunking alone deferred (spends complexity on splitting without removing work). Land wip's F7/F8/F17 first. |
| F3 | PARTIAL (fix on wip, one HIGH regression to fix) | Bypass CONFIRMED — on main, zone files never enter the byte cache at all. **"Re-decompresses per visit" is REFUTED on main**: revisits reuse the parsed `ZoneLoaderAsset` (load_zone re-runs only on first visit / failure / map-editor reload); the HIGH impact only materializes after F1's eviction lands. The wip fix (cache-first, zone-tagged, normalized keys, 512 MB budget) works — **but inverts the real-FS priority**: map-editor saved HIM/TIL (and `--new-terrain` exports) are shadowed by stale cache entries forever (the §3.5 HIGH regression; suspected contributor to the first-login terrain issue). Required: real-FS check before cache (or invalidate tagged bytes on `SaveZoneEvent`). Also note 12-validation:77's "F3 implemented" refers to wip — current main has zero cache calls. |
| F4 | CONFIRM (fix on wip, with 2 required changes) | Mechanism confirmed; doc's impact examples wrong: UI textures/specular/skybox are held by strong handles for the session — clearing the byte cache does **not** re-read them; the real re-read cost is tile/object textures (freed on zone despawn → cache miss → re-decompress). The wip tagged-eviction + keep-sets + budget is sound (all tagged files genuinely zone-specific). Required before merge: (1) the §3.5 real-FS inversion fix (shared with F3); (2) the budget eviction is an all-or-nothing `retain` (not LRU) that fires inside the store path on **asset-loading threads** under the write lock — replace with real LRU, amortized. Optional: refresh the zone tag on cache hit. |
| F5 | CONFIRM (aligned with 02-F7; NOT implemented) | Claim accurate (all 13 format arms funnel to Rgba8UnormSrgb; mip_count parsed, never used; DXT1 ratio 8×, DXT3/5 4×). Fix plan: **Stage 1** — in-loader mip generation via `Image::new_uninit` + `mip_level_count` (`Image::new` debug-panics on mip data; DDS on-disk layout is already LayerMajor mip0-first, so 52% of files' chains can be reused verbatim); stage 1 alone raises VRAM ~33% (the win is stage 2). **Stage 2** — BC pass-through via `Image::from_buffer`/`dds_buffer_to_image` (exists in 0.18.1; `ImageMipGenerator` does NOT — doc's sketch is half-stale), `is_srgb=true` threading, A8/L8 **must stay on the CPU path** (particle masks sample `.a` — semantic, not just format), R5G6B5-family stays CPU (Bevy hard-errors). `passthrough_terrain_textures` is currently a dead flag (no runtime consumer). Do stage 1 now, stage 2 as a separate validated change. |
| F6 | PARTIAL (implement-with-changes: zone-scope only) | Claim confirmed (per-part `add`; never evicted). A zone-object-scoped cache is SAFE where the 01-F8 character cache is not: `BloodOverlay` exists only on combat entities; zone-static materials are never descendants of a bloodied root and never mutated (verified). **Key must include `lightmap_params`** (per-part LIT atlas cell — "dedupe by texture set" alone is a visible artifact), plus base_color/lightmap texture handles, alpha flags, two_sided (AlphaMode lacks Hash — use discriminant + `to_bits()`). Exclude `spawn_animated_object` (RoseEffectExtension, mutated per frame) and effect objects. Do NOT extend to `spawn_model` (01-F8 verdict stands). Not implemented on wip. |
| F7 | CONFIRM (already fixed on wip — port) | Claim accurate (per-block `TerrainMaterial { textures: tile_textures.clone() }`; tens of thousands of redundant clones). Wip implements one shared material per zone (12-validation:78 OK); `update_terrain_lighting_system` is safe (updates one asset; re-prepares all blocks); no per-block mutation exists anywhere; all paths (zone viewer, map editor) converge. Port to main (wip branch is stale vs the code-simplification cleanup); verify with a day/night pass. |
| F8 | CONFIRM (already fixed on wip — port) | Claim accurate (one alloc per **object**, not per part — minor nit; ~N allocs of up to a few KB per zone). Wip implements clear+resize through a threaded `&mut Vec` (correct — `spawn_object` is a plain fn, so `Local` doesn't fit; no stale handles; no recursion). The map-editor copy (model_placement_system.rs:293) is NOT fixed anywhere — negligible (one alloc per user click); optional. Merge the wip zone-side change. |
| F9 | PARTIAL (implement-with-changes, aligned with 01-F7/F9) | Core accurate; corrections: character/vehicle skeletons are **already struct-cached** (title overstates — only NPCs re-read); "Arua decompress" doesn't apply (the iRose phoenix VFS is zero-copy mmap — 01-F9's decompress claim is also overstated; real per-spawn cost is hash lookup + full parse); IBP-per-spawn applies to all 3 spawn paths (characters, NPCs, vehicles) and is never evicted. Wip has the skeleton cache **keyed by npc_id — re-key to `skeleton_index` (u16)** (~2 lines; covers the "50 NPCs share 8 skeletons" case). The IBP cache is NOT on wip: implement keyed by namespaced skeleton identity (enum/path — a raw u32 collides across groups and breaks skinning); do NOT clear on zone change (bounded by unique skeletons — supersedes R5's clearing note). |
| F10 | CONFIRM (implement-with-changes, deferred) | PTL re-read per particle per spawn confirmed (only `read_file::<PtlFile` site; 8 sync spawn call sites); `clear()` uncalled; "unbounded" is a mild overstatement (bounded by the distinct EFT set, ~few MB). Fix: cache parsed `PtlFile` by path in `EffectCache` + wire `clear()` at the zone-switch site (systems.rs:499-517 — one line, the hook exists); **drop the ShaderStorageBuffer-sharing clause** (unsafe — buffers are recreated per frame per instance, 02-F1 territory; sharing corrupts); optional: share only the static particle mesh by count. Not on wip. |
| F11 | PARTIAL (simplest correct = removal; not the doc's sketch) | Mechanism confirmed but magnitude **refuted**: the exe is **2.6 MB** (not "tens of MB"); worst case 12 reads ≈ 30 MB once at startup — LOW, not MEDIUM. Bigger finding: the entire cursor pipeline is **dead scaffolding** — the loader is a stub, `processed` is never read, and nothing renders custom cursors. The doc's "parse cursors once" is a feature (PE parsing doesn't exist), not an optimization. Correct fix: delete the 12 loads + `cursors` field + poll loop (zero behavior change — nothing shows cursors today), or short-circuit `trose.exe` reads in the reader. Not on wip. |
| F12 | CONFIRM (defer — align with 01-F14) | All 5 clone sites verified. `Bytes` doesn't exist in 0.18.1; `SliceReader<'a>` is unusable (borrow can't outlive the global RwLock guard / concurrent eviction) — the owning-`Arc` reader is the only route; must override `read_to_end` (trait default polls 32-byte chunks) and keep seek semantics. Bevy's own `DataReader` is the proof-of-pattern. Minor overstatement: the zone path doesn't use `VecReader` (only asset-server loads). Implement ONCE (with 01-F14) when the wip cache overhaul merges — it's the natural carrier. |
| F13 | CONFIRM (already fixed on wip — defer standalone) | Mechanism verified (VFS case-insensitive by construction; Bevy passes case through; real mixed-case callers exist). Wip implements `normalize_cache_key` (uppercase + `\`→`/`), 12-validation:80 OK. Cosmetic gap: wip omits `//`-collapse and trim (exotic paths just miss twice). Standalone implementation would conflict with the wip cache overhaul — don't. Residual (out of scope): Bevy-level double *parse* persists (case-sensitive handle dedup). |
| F14 | CONFIRM (probe merge on wip; stat fix needs the corrected form) | Corrections: the probe is **HIM-only** (not HIM+TIL) — and worse than an open: it fully decrypts+decompresses the HIM and discards it (double decompress for existing blocks); ~24.6k failed `.exists()` stats per zone (not 10k). Wip already merges probe+read with missing-HIM handled at `debug!` (also fixes a latent bug: real-FS override HIMs were skipped by the VFS-only probe). The "skip the stat when in the VFS index" half is **REFUTED** — the stat IS the real-FS override mechanism (`--data-path`, map-editor saves); correct form: single `std::fs::read` + `NotFound` fallback, warn only on other errors. |
| F15 | PARTIAL (defer; impact overstated) | Claim confirmed (one task, zero await points — the "async" is cosmetic; up to 6 files/block ≈ 24k opens); corrections: no zlib on the default `data.idx` path (VfsIndex returns raw mmap slices — only the Arua device decompresses), and the AsyncComputeTaskPool caps at **4 threads** (25% of cores, max 4) — not many-core. The VFS is Send+Sync (immutable devices) and already Arc-shared into the task. Fix feasible: 4–8 chunked `pool.spawn`s with Bevy `Task` handles (no `futures_lite` — not in Cargo.toml), skip-on-error, reindex by block coords; NPC spawn order becomes nondeterministic (cosmetic). Impact is LOW-MEDIUM: main-thread spawning (F2) dominates zone-change time. Defer, on top of the wip cache work. |
| F16 | CONFIRM (implement-with-changes, alongside F1) | Claim accurate and conservative: ~33 KB/block duplicated ≈ **135 MB/zone (50% of the terrain footprint)** + spawn-time memcpy. Consumers verified: only `save_system` and `add_water_plane_system` (both can access the asset); **nothing mutates the clones** (editor editing is uniform offset/fill applied at save). Fix: store block coords + offset/fill/dirty only, resolve heights/tiles from the asset with graceful `None` (the F1 keep-set invariant is its safety precondition); Arc-sharing rejected (cross-crate, server-shared `rose-file-readers`). Not implemented. |
| F17 | CONFIRM (already fixed on wip — merge; magnitude understated) | Claim accurate and understated: **8–10 info lines per block ≈ 20k–40k JSON lines per zone** (spawning.rs:156 + loading.rs:244/259 + terrain.rs sites), each serde_json-serialized on the main thread during the spawn hitch. Wip demotes all per-block sites to `debug!` (summary already exists at info, 12-validation:80 OK) — **but main still has all of them**. Merge/cherry-pick; optionally add materials+timing to the summary. |
| F18 | CONFIRM (already fixed on wip — merge) | Impact even lower than claimed: `is_directory` is effectively **never called** (no `load_folder`, no asset-processor/file-watcher features) — the log is dead noise. Wip removed the log (12-validation:80 OK); **main still has it**. R1 protected (read_directory untouched). One-line merge. |
| F19 | PARTIAL (implement-with-changes, re-scoped) | Core confirmed (9 sheets, ~79 refs; premultiply ≈ 16.5M px, ~0.5–1 s); corrections: startup-only (flags never reset — not per zone change); the **real per-frame cost is WARN spam** for pending textures (thousands of lines/sec into structured.jsonl); **new bug found: double-premultiply** on 22 of 79 shared duplicate handles (27% redundant work + subtle darkening). Fixes: log-once per texture (state transition); premultiply once per unique handle; skip the pending-count gate (the early return already exists; a counter desyncs the two-flag semantics) and bounded-per-frame (delays premultiply → transient white-box artifact regression). Not on wip. |
| F20 | CONFIRM (already deleted on wip — merge) | Dead Resource verified (zero uses outside its own file; the richer `Local` variant is the working cache with sound rebuild logic). Wip deleted the file (12-validation:81 OK); **main still has it**. Trivial deletion + `mod` line removal. |
| F21 | PARTIAL (mostly REFUTE — fold into F9) | Skeleton half is verbatim F9 (double-listed). The texture half is **false**: `spawn_model` loads textures/meshes via `asset_server.load` (async) — no sync VFS reads. The actual residual sync reads at NPC spawn are EFT/PTL files (→ F10/F26). Texture preloading is not sound (no well-defined per-zone NPC texture set; wasted memory). Correct the doc; no new code. |
| F22/F23 | CONFIRM (both no-action) | Both verified accurate (per-frame lookups are 1–2-entity scans/O(1) HashMap gets; the tracking set is a few hundred KB max). **No change required — remove both findings from the doc.** |
| F24 | PARTIAL (fold into F1; no standalone work) | Handle-dance real but it's a **borrow-workaround** (`spawn_zone` never uses `zone_loader_assets` — dead field), once per load on a ≤2-entry map; `clear_asset_handles`/`shrink_to_fit` are event-branch-only (not hot path). F1's wip eviction doesn't touch the dance. Residual is negligible; optional cleanliness: drop the dead field + `shrink_to_fit` in the F1/F4 finalization commit. |
| F25 | CONFIRM (fold into F5; doc details wrong) | Per-part lightmap loads + path dedup + F5 decode confirmed. Details wrong: lightmaps are per-block **atlas DDS** (mostly 1024×1024 DXT5 with 10–11 on-disk mips): EJ01 = 194 files ≈ 598 MB RGBA8; JUNON = 2,463 ≈ **6.7 GB** — dramatically understated, which strengthens F5. Caveat to carry into F5: **atlas mip bleeding** — keep lightmap atlases mip-less (or don't auto-generate mips for them); keep case consistent with F13's key normalization (LIT filenames are lowercase vs uppercase on disk). |
| F26 | PARTIAL (defer; doc pairing inaccurate) | Decompression-per-open is TRUE only for the Arua device (the client's actual data.idx device — AES + zlib, no caching of any kind); irosephvfs/titan/vfs are zero-copy mmap (doc pairs the wrong device). A client-side single choke point is feasible (~8 direct-read sites); a device-level cache in `rose-file-readers` is **rejected** (shared crate with the server, `&self`, no eviction hook, server reads once). Practical coverage: F3 + F9 + F10 already address the volume; for parse-heavy reads, *parsed* caches are strictly better than a byte cache. Defer; if ever done: client-only `cached_read_bytes()` with normalized keys + zone tags + real-FS priority preserved. |

**Note (per 2026-08-04 review):** F22 and F23 require no changes — verified no-action; retained in the document for the record (their substance is summarized in this verification section).

**Cross-cutting note for the whole doc set:** the vendored source folder `bevy-collection\bevy-0.18.1` is actually **0.19.0-dev** (its `Cargo.toml` declares `version = "0.19.0-dev"`); the client builds against crates.io Bevy 0.18.1. API citations verified only against that folder must be re-checked against 0.18.1 before implementing (this affects F5's mip/compression citations, e.g. `ImageMipGenerator` does not exist in 0.18.1).
