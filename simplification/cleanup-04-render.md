# Cleanup Report: `src/render/` + `src/graphics/` (report `04-render.md` implementation)

**Branch:** `code-simplification`
**Agent scope:** `src/render/` (incl. `src/render/shaders/*.wgsl`) and `src/graphics/`
**Status:** implemented; `cargo check` run once at the end (see "Build check" below).

---

## What was done

### Section 3 — Dead code (priority 1)

| Item | Action |
|---|---|
| `3a. trail_effect.rs` | Rewrote file to keep ONLY the `TrailEffect` component (still spawned by `model_loader.rs:569`). Deleted `TrailEffectPoint`, `TrailEffectPositionHistory` (+Default impl), `TrailEffectRenderPlugin` + `Plugin` impl, `initialise_trail_effects`, `update_trail_effects`, unused imports. Deleted `shaders/trail_effect.wgsl` (unreferenced). |
| `3b. post_processing.rs` | **File deleted entirely** (192 lines). It was an ORPHAN file — NOT declared in `render/mod.rs` and NOT referenced by `lib.rs` (verified by grep: the only `PostProcessingSettings` used anywhere is `ui::PostProcessingSettings` at `lib.rs:1590/2554`). Also deleted `shaders/post_processing.wgsl` (only referenced by the deleted file). |
| `3c. particle_debug.rs` | **File deleted entirely** (86 lines). Only references were the imports at `lib.rs:118-119` (never registered as systems). Removed re-exports from `render/mod.rs`. |
| `3d. starry_sky_material.rs` | 872 → 520 lines. Deleted: empty `diagnose_starry_sky_materials` system + its `Update` registration in the plugin; `sky_sphere_follow_camera_system` (warning-only logger, registered at `lib.rs:1291` and `lib.rs:1403`); all `should_log = false` dead branches in `update_starry_sky_system`, `update_starry_sky_night_factor`, `toggle_atmosphere_based_on_time`; both `FORCE_NIGHT_MODE` consts + branches; all commented-out log blocks; the 7 per-specialize `log::info!` lines in `specialize()`. Live logic preserved byte-for-byte (night-factor math, atmosphere toggling, zone_time-missing fallback). |
| `3e. terrain_material.rs` | Deleted `TerrainLighting` component + `Default` impl and `sync_terrain_lighting_component_system` (never registered; `update_terrain_lighting_system` kept — registered at `lib.rs:1334`). Removed now-unused imports (`MeshPipelineKey`, `PhaseItem/RenderCommand/RenderCommandResult/TrackedRenderPass`, `Component`, `GlobalTransform`, `Query`, `With`, `Resource`, `World`). |
| `3f. zone_lighting.rs` vestigial API | Deleted: `SetZoneLightingBindGroup<const I>` RenderCommand (referenced nowhere), `ZONE_LIGHTING_BIND_GROUP_LAYOUT` static + `OnceLock` import + the `set()` call in `FromWorld`, the duplicate `bind_group_layout_descriptor` construction + `pub` field, the `frame_count` `Local` in `extract_uniform_data`, unused imports (`AmbientLight`, `Local`, `ROQueryItem`, `SRes`, `SystemParamItem`, `PhaseItem`, `RenderCommand`, `RenderCommandResult`, `TrackedRenderPass`, `BindGroupLayoutDescriptor`, `Exposure`). |
| `3h. unused imports` | `MeshPipelineKey` removed from `cloud_material.rs`, `volumetric_cloud.rs`; `AmbientLight` from `zone_lighting.rs`; see 3e/3f for the rest. |
| `3i. particle_material.rs` | 315 → 235 lines. `specialize()` now does only the functional vertex-layout override (all 7 DIAGNOSTIC `info!` calls removed). Plugin build: removed the commented-out bind-group-layout block. `validate_particle_materials`/`log_particle_material_bind_groups` kept (debug-only) but all commented-out `info!` lines removed; functional `error!/warn!/debug!` calls preserved. |

### Section 2 — Extension pattern (priority 2)

- Deleted files: `terrain_material_extension.rs` (71), `water_material_extension.rs` (42), `blood_overlay_material.rs` (78), `blood_overlay_shader.rs` (12). Verified dead: `RoseTerrainExtension`/`RoseWaterExtension` were registered in `lib.rs` but never instantiated; `BloodOverlayExtension`/`BloodOverlayUniform` never registered (live blood system paints into `RoseObjectExtension.blood_overlay_texture` instead).
- Deleted shaders: `shaders/rose_terrain_extension.wgsl`, `shaders/rose_water_extension.wgsl` (byte-identical pass-throughs), `shaders/blood_overlay.wgsl` (1,877 B).
- `extension_material_plugin.rs` (111 → 78): removed `ROSE_TERRAIN_EXTENSION_SHADER_HANDLE`/`ROSE_WATER_EXTENSION_SHADER_HANDLE` consts + their `load_internal_asset!` calls; removed unused imports (`MaterialExtension`, `MaterialPipeline`, `MaterialPipelineKey`, `RenderPipelineDescriptor`, `SpecializedMeshPipelineError`, `MeshVertexBufferLayoutRef`, `ShaderRef`); kept `RoseObjectMaterialPlugin` + `ExtensionMaterialPlugin` (live, registered at `lib.rs:1058/1078`).
- `render/mod.rs` (134 → 116): removed re-exports for `particle_debug`, `trail_effect::TrailEffectRenderPlugin` (kept `TrailEffect`), `terrain_material_extension`, `water_material_extension`, `blood_overlay_material`, `blood_overlay_shader`, and `sky_sphere_follow_camera_system`.

### Section 1c/1d — Dedup (priority 3)

- **1c:** `calculate_cloud_lighting` moved to a single `pub(crate)` copy in `zone_lighting.rs:742-815`; both `cloud_material.rs` and `volumetric_cloud.rs` now call `crate::render::zone_lighting::calculate_cloud_lighting`. Identical math preserved (used the commented version).
- **1d:** became **moot** — the duplicated `ZoneTimeState` → intensity match existed only because `sync_terrain_lighting_component_system` (3e) mirrored `update_terrain_lighting_system`. Deleting the dead system removes the duplication; no helper needed for a single call site.
- **world_ui.rs bug fix:** removed the duplicated `pipelines.specialize(...)` call (`world_ui.rs:574-575`, copy-paste — GPU pipeline-cache lookup ran twice per frame) plus a commented log line.

### Section 6 — graphics presets (bonus, verified-safe)

- `graphics_settings.rs` (605 → 544): `low_preset`/`medium_preset`/`high_preset`/`ultra_preset` now build on `Self { ... ..Default::default() }`. Every field value verified identical to the previous explicit struct (the `low` preset's `motion_blur_intensity: 0.0` vs default `0.5`, `bloom_enabled: false` vs default `true`, `vsync_mode` etc. all preserved; only non-default fields listed).

### Deleted shader files (5)

`post_processing.wgsl`, `trail_effect.wgsl` (1,183 B), `rose_terrain_extension.wgsl`, `rose_water_extension.wgsl`, `blood_overlay.wgsl` (1,877 B). Total ≈ 4.5 KB.

### Skipped (per instructions / risk)

- **§1a plugin macro / §1b AsBindGroup helper:** SKIPPED. The blend-state blocks are NOT identical (starry_sky/cloud use `alpha.src = One`; terrain/world_ui use `SrcAlpha`), and a material-registration macro risks subtle behavior differences; medium-risk per the report.
- **§5 WGSL noise dedup (`#import`/`import_path`):** SKIPPED — requires confirming Bevy 0.18.1 naga import support; left for a later round.
- **`cloud_material.rs` module:** KEPT dormant per decision (only the `calculate_cloud_lighting` dedup was applied). Its `should_log = false` branches / `diagnose_cloud_layer_system` commented block were left untouched (not flagged in the report; note for a future round if the module stays dead).
- **`RoseRenderPlugin` consolidation (report §6):** left as-is (registered at `lib.rs:1086`, only adds Terrain+Water plugins); changing it would churn `lib.rs`.
- **§4 long-function splits:** not performed except what fell out of dead-code removal (`update_starry_sky_system` 73→26, `update_starry_sky_night_factor` 157→38, `toggle_atmosphere_based_on_time` 149→62). `queue_world_ui_meshes` (254 lines) left intact — pure refactor, no dead code.

---

## LOC accounting

| Item | Lines before | Lines after | Delta |
|---|---|---|---|
| `post_processing.rs` (deleted) | 192 | 0 | −192 |
| `particle_debug.rs` (deleted) | 86 | 0 | −86 |
| `terrain_material_extension.rs` (deleted) | 71 | 0 | −71 |
| `water_material_extension.rs` (deleted) | 42 | 0 | −42 |
| `blood_overlay_material.rs` (deleted) | 78 | 0 | −78 |
| `blood_overlay_shader.rs` (deleted) | 12 | 0 | −12 |
| `trail_effect.rs` | 84 | 14 | −70 |
| `starry_sky_material.rs` | 872 | 520 | −352 |
| `terrain_material.rs` | 434 | 371 | −63 |
| `zone_lighting.rs` (incl. +74 shared fn) | 814 | 816 | +2 |
| `cloud_material.rs` | 773 | 697 | −76 |
| `volumetric_cloud.rs` | 627 | 577 | −50 |
| `particle_material.rs` | 315 | 235 | −80 |
| `extension_material_plugin.rs` | 111 | 78 | −33 |
| `world_ui.rs` | 758 | 756 | −2 |
| `render/mod.rs` | 134 | 116 | −18 |
| `graphics/graphics_settings.rs` | 605 | 544 | −61 |
| **Total Rust** | | | **≈ −1,284** |
| **WGSL deleted** | | | **5 files ≈ 4.5 KB** |

---

## LIB.RS CHANGES NEEDED

The following `src/lib.rs` items reference deleted code and MUST be removed by the lib.rs agent (verified by grep; line numbers current on this branch):

1. **`use render::{...}` block (lines 116-152)** — remove these import lines:
   - Line **118**: `debug_particle_rendering,`
   - Line **119**: `particle_performance_monitor,`
   - Line **120**: `sky_sphere_follow_camera_system,`
   - Line **135**: `RoseTerrainExtension,`
   - Line **136**: `RoseWaterExtension,`
   - Line **141**: `TrailEffectRenderPlugin,`
2. **Lines 1062-1065** — remove the whole block:
   ```rust
   app.add_plugins((MaterialPlugin::<
       ExtendedMaterial<StandardMaterial, RoseTerrainExtension>,
   >::default(),));
   log::info!("[MATERIAL PLUGIN] RoseTerrainExtension plugin registered successfully");
   ```
3. **Lines 1067-1070** — remove the whole block:
   ```rust
   app.add_plugins((MaterialPlugin::<
       ExtendedMaterial<StandardMaterial, RoseWaterExtension>,
   >::default(),));
   log::info!("[MATERIAL PLUGIN] RoseWaterExtension plugin registered successfully");
   ```
4. **Line 1083** — remove `TrailEffectRenderPlugin,` from the `add_plugins((...))` tuple (leaves `ZoneLightingPlugin, WorldUiRenderPlugin, RoseRenderPlugin, ...`).
5. **Lines 1142-1147** — remove the two `log::info!(...)` lines for `RoseTerrainExtension` (1143-1144) and `RoseWaterExtension` (1146-1147) from the "Material Plugin Diagnostic Logging" section.
6. **`sky_sphere_follow_camera_system` is registered TWICE** — remove both:
   - Lines **1288-1292** (comment at 1288 "// Sky sphere follows camera..." + `app.add_systems(PostUpdate, sky_sphere_follow_camera_system.after(TransformSystems::Propagate));`)
   - Lines **1400-1404** (same statement again, inside `add_systems(PostUpdate, ...)`).

   `TransformSystems` import (line 41) is still used at lines 1889/1894 — KEEP it.

**Note (no action needed):** `src/components/blood_overlay.rs:74` contains a doc comment linking `[`BloodOverlayExtension`](crate::render::BloodOverlayExtension)`. This is now a broken intra-doc link (doc-only, does not affect compilation). `components/` is not in my scope; the owner may fix the doc text.

---

## Build check

Ran `cargo check` (once, at the end, per instructions). Results:

- **No errors in `src/render/` or `src/graphics/`** — all my changes compile cleanly.
- The only error caused by MY deletions is the expected one in `lib.rs:117`: `unresolved imports render::debug_particle_rendering, render::particle_performance_monitor, render::sky_sphere_follow_camera_system, render::RoseTerrainExtension, render::RoseWaterExtension, render::TrailEffectRenderPlugin` — resolved by the lib.rs agent applying section "LIB.RS CHANGES NEEDED" above.
- All other `cargo check` errors are pre-existing on this branch in other agents' files (`src/systems/*`, `src/zone_loader/*`, `src/lib.rs:73` missing `diagnostics` module, `src/lib.rs:909` ZoneLoader AssetLoader bound) and were not touched.
