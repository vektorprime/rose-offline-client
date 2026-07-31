# Simplification Report: `src/render/` + `src/graphics/`

**Module analyzed:** `src/render` (25 files) + `src/graphics` (3 files)
**LOC analyzed:** ~7,635 Rust lines (plus ~101 KB / ~2,500 lines of WGSL in `src/render/shaders/`)

**This is a PLAN ONLY. No code changes were made.** All findings below are research results; every suggestion is a proposal to be confirmed before implementation.

---

## 1. Duplicated Material Boilerplate (the biggest structural finding)

### 1a. Identical plugin pattern in 8 material plugins

Every custom material plugin follows the exact same shape:

```
fn build(&self, app) {
    log banner
    load_internal_asset!(HANDLE, "shaders/X.wgsl")
    app.init_asset::<T>() (some)
    app.add_plugins(MaterialPlugin::<T>::default())
    log banner
}
```

- `terrain_material.rs:64-84`
- `water_material.rs:28-48`
- `particle_material.rs:155-204`
- `damage_digit_material.rs:80-99`
- `starry_sky_material.rs:30-63`
- `cloud_material.rs:29-76`
- `volumetric_cloud.rs:29-56`
- `extension_material_plugin.rs:71-111` (same pattern for 4 shader loads)

Each also implements `Material` with ~10 near-identical methods: `vertex_shader()/fragment_shader()` returning the same handle, `alpha_mode()`, `enable_prepass() -> false`, `enable_shadows() -> false`, `specialize()` that sets a vertex layout and/or blend state. The blend-state block (same 8 lines of `BlendState { color: BlendComponent {...SrcAlpha/OneMinusSrcAlpha...} }`) appears verbatim in `starry_sky_material.rs:289-307`, `cloud_material.rs:365-381`, `terrain_material.rs:254-269`, `world_ui.rs:260-271`.

**Suggestion:** Introduce a `bevy::log`-free shared macro or helper (e.g. `register_internal_shader_material!(app, HANDLE, "path", MaterialType)`) for the plugin body, and a `default_specialize_blend(descriptor)` helper for the blend block. Do **not** attempt to merge the materials themselves (they differ in bind groups) — only the boilerplate.

**Estimated savings:** ~150-250 lines across the 8 files.

### 1b. Four nearly identical manual `AsBindGroup` impls

`CloudMaterial`, `VolumetricCloudMaterial`, `StarrySkyMaterial`, and `WaterMaterial` each hand-write `AsBindGroup` with the same skeleton:
- `label()`, `bind_group_data()` returning a unit struct, `unprepared_bind_group()` returning `Err(AsBindGroupError::CreateBindGroupDirectly)`, `bind_group_layout_entries()` with a single uniform-buffer entry, and `as_bind_group()` doing: get layout from cache → `create_buffer_with_data` → `create_bind_group` → `PreparedBindGroup`.

- `cloud_material.rs:191-295` (105 lines)
- `volumetric_cloud.rs:149-240` (92 lines)
- `starry_sky_material.rs:143-223` (81 lines)
- `water_material.rs:162-312` (151 lines, storage buffer variant)

The only real difference is the float array being packed (`cloud_material.rs:212-249` vs `volumetric_cloud.rs:168-197` vs `starry_sky_material.rs:166-179`).

**Suggestion:** A shared helper `fn uniform_material_bind_group(layout, device, cache, label, data: &[f32]) -> Result<PreparedBindGroup, AsBindGroupError>` plus a small derive-style macro for the "pack floats into one uniform buffer" case. Also, `#[derive(AsBindGroup)]` with `#[uniform(0)] pub params: Vec4` fields is an alternative, but the code chose manual packing for wgpu-27 binding-array constraints only in terrain — the other three do NOT need manual impls and could use the derive (verify against shader layout before switching).

**Estimated savings:** ~200-280 lines.

### 1c. `calculate_cloud_lighting` duplicated byte-for-byte

- `cloud_material.rs:541-614` and `volumetric_cloud.rs:579-627` are the **same function** (same time-of-day mapping, same sun color/ambient/tod_factor math, only comments removed).

**Suggestion:** Move to a shared location (e.g. `zone_lighting.rs` or a small `time_of_day_lighting.rs`) and have both modules call it.

**Estimated savings:** ~45-50 lines (one copy deleted).

### 1d. Time-of-day intensity multiplier duplicated

The `ZoneTimeState` → intensity match (`Morning 2.0 / Day 2.5 / Evening 2.0 / Night 1.0`, then `/ 5.0`) appears twice in `terrain_material.rs:111-120` and again `terrain_material.rs:157-163` — in two sibling systems that are both live but serve different sinks (material assets vs. dead component, see §3e).

**Suggestion:** Extract one `terrain_time_intensity(graphics, zone_time) -> f32` helper; also dedupe the `color/ambient` packing done in both.

**Estimated savings:** ~20 lines + removes drift risk between the two copies.

### 1e. Particle and DamageDigit materials are the same material with different uniforms

- `particle_material.rs:17-153` and `damage_digit_material.rs:13-78`: both are storage-buffer-driven materials with an empty vertex layout (procedural geometry via `vertex_index`), `prepass_vertex/fragment_shader -> ShaderRef::Default`, `enable_prepass/shadows -> false`, and an identical `specialize()` body (`vertex.buffers = vec![VertexBufferLayout { array_stride: 0, step_mode: Vertex, attributes: vec![] }]`).

**Suggestion:** A shared `fn empty_vertex_layout_specialize(descriptor)` helper, and consider a generic `StorageBufferMaterial<T>` abstraction. Low priority — the savings (~30 lines) are small, but the conceptual duplication is real.

---

## 2. The Extension Pattern is Over-Engineered (and partly dead)

Architecture today: 4 extension shaders registered via `weak_handle!` + `load_internal_asset!` in `extension_material_plugin.rs:26-111`, each `MaterialExtension` impl in its own file returning the handle by path string.

Findings:

1. **`rose_terrain_extension.wgsl` and `rose_water_extension.wgsl` are byte-identical** (MD5 `C2BD6E52...`). Both are 32-line pass-through shaders that do nothing but call `pbr_input_from_standard_material` + `apply_pbr_lighting`. They read **none** of the extension's bindings (water UV params/texture at `water_material_extension.rs:20-26`, terrain 5 textures + count at `terrain_material_extension.rs:25-51`). All those bindings are dead weight uploaded per-frame for zero visual effect.

2. **`RoseTerrainExtension` and `RoseWaterExtension` are registered as `MaterialPlugin`s in `lib.rs:1062-1070` but never instantiated anywhere** — grep across `src/` finds no construction of `ExtendedMaterial<StandardMaterial, RoseTerrainExtension>` or `RoseWaterExtension`. Entities use `TerrainMaterial` (custom) and `WaterMaterial` (custom). The two extensions, their shader handles, their plugin registrations, and one of the two identical WGSL files are 100% dead code that still ships pipelines.

3. `RoseObjectMaterialPlugin` (`extension_material_plugin.rs:45-63`) is just `MaterialPlugin::<RoseObjectMaterial>::default()` plus log banners — the entire struct is redundant; `extension_material_plugin.rs` also imports `MaterialExtensionPipeline`, `RenderPipelineDescriptor`, `SpecializedMeshPipelineError`, `MeshVertexBufferLayoutRef` for nothing.

4. The `specialize()` stubs in `object_material_extension.rs:86-95` and `blood_overlay_material.rs:69-77` are no-ops (`Ok(())` + comment).

5. `BloodOverlayExtension`/`BloodOverlayUniform`/`blood_overlay_shader.rs`/`blood_overlay.wgsl` are **never registered or used** — the live blood system (`systems/blood_overlay_system.rs:136-141`) paints into `RoseObjectExtension.blood_overlay_texture` instead. `blood_overlay_material.rs:20-78`, `blood_overlay_shader.rs:12`, `shaders/blood_overlay.wgsl` (1,877 B) are dead.

**Suggestion (no visual change, verified shaders are pass-through):**
- Delete `terrain_material_extension.rs`, `water_material_extension.rs`, both identical WGSL files, and their `lib.rs` plugin registrations.
- Delete `blood_overlay_material.rs`, `blood_overlay_shader.rs`, `blood_overlay.wgsl`, and their re-exports in `render/mod.rs:66-70`.
- Keep the `load_internal_asset!` + weak-handle pattern (it is the robust Bevy idiom) but consider folding `ExtensionMaterialPlugin` + `RoseObjectMaterialPlugin` into one plugin struct, or deleting both and loading the two remaining shaders from within `object_material_extension.rs`/`effect_mesh_extension.rs` themselves (each extension file already contains the `MaterialExtension` impl; loading its own shader there removes the global handle indirection entirely).

**Estimated savings:** ~330 Rust lines + 2,300 B WGSL, minus keeping ~40 lines of registration.

---

## 3. Dead Code, Unused Imports, Stubs

| Location | What | Evidence |
|---|---|---|
| a. `trail_effect.rs:47-84` (entire file) | Plugin build empty, both systems empty and unregistered; only the 2 components remain "for API compatibility" | `model_loader.rs:569` still spawns `TrailEffect`, so components accumulate with zero processing. `trail_effect.wgsl` (1,183 B) unused. |
| b. `post_processing.rs:74-112, 143-188` | `PostProcessingNode::run` iterates a query and does nothing; `DrawPostProcessing` and `SetPostProcessingBindGroup` are empty stubs; `POST_PROCESSING_SHADER_HANDLE`/`post_processing.wgsl` unused by any actual pass | Node does no rendering. `setup_post_processing` (`:190-192`) never registered. ~150 of 192 lines are placeholder. |
| c. `cloud_material.rs` (entire module, 773 lines + 8.6 KB cloud.wgsl) | `CloudMaterialPlugin` commented out in `lib.rs:1130`, `spawn_cloud_layer` commented out (`lib.rs:146, 1603`); the volumetric cloud module replaced it | Registered systems only via the unregistered plugin. Keep only if 2D cloud layer is planned to be re-enabled. |
| d. `starry_sky_material.rs` dead scaffolding | `diagnose_starry_sky_materials` (`:68-85`) empty but **registered** at `:58`; `sky_sphere_follow_camera_system` (`:468-508`) registered in `lib.rs:1403` but only logs a warning once a second; `should_log = false` gates ~120 lines of dead logging branches in `update_starry_sky_system`/`update_starry_sky_night_factor`/`toggle_atmosphere_based_on_time`; `FORCE_NIGHT_MODE` consts (`:550, :736`) always false; sphere radius inconsistent (mesh `:341` = 500, comments/system `:463-483` = 50000) | ~250 lines removable. |
| e. `terrain_material.rs:143-181` `sync_terrain_lighting_component_system` + `TerrainLighting` component (`:46-61`) | Never registered (`lib.rs` only registers `update_terrain_lighting_system`), component never inserted anywhere; shader lighting comes from the storage buffer built in `as_bind_group` | ~50 lines dead. |
| f. `particle_debug.rs` (86 lines) | `debug_particle_rendering` and `particle_performance_monitor` imported in `lib.rs:118-119` but never registered as systems | Dead; delete or wire behind a debug cfg. |
| g. `zone_lighting.rs` vestigial public API | `SetZoneLightingBindGroup<const I>` RenderCommand (`:793-814`), `ZONE_LIGHTING_BIND_GROUP_LAYOUT` static (`:77`), `ZoneLightingUniformMeta::bind_group_layout_descriptor` (`:643`) referenced nowhere else; `extract_uniform_data`'s `frame_count` Local only feeds commented logs (`:715-726`) | ~40 lines dead API. |
| h. Unused imports | `render_phase::{PhaseItem, RenderCommand, RenderCommandResult, TrackedRenderPass}` in `terrain_material.rs:23`; `MeshPipelineKey` in `terrain_material.rs:14`, `starry_sky_material.rs:17`, `cloud_material.rs:16`, `volumetric_cloud.rs:12`; `AmbientLight` in `zone_lighting.rs:16` | Compiler warnings today. |
| i. `particle_material.rs` debug noise | 7 `info!` DIAGNOSTIC logs in `specialize` (`:97-145`) — runs on every pipeline specialization; ~20 lines of commented-out logs in plugin build and validators (`:192-202, 223-269, 300-305`); `validate_particle_materials`/`log_particle_material_bind_groups` registered only in debug builds | Trim to 1 log or 0. |

**Estimated savings from §3 (a-i):** ~1,400-1,600 lines of Rust + 2 dead WGSL files (~12 KB), most with **zero risk** (delete-only, no behavior change).

---

## 4. Excessively Long Functions

| Function | Location | Lines |
|---|---|---|
| `queue_world_ui_meshes` | `world_ui.rs:505-757` | 254 (largest in module) |
| `spawn_volumetric_clouds` | `volumetric_cloud.rs:323-483` | 161 |
| `update_starry_sky_night_factor` | `starry_sky_material.rs:543-699` | 157 (mostly dead `should_log` branches) |
| `toggle_atmosphere_based_on_time` | `starry_sky_material.rs:724-872` | 149 (same) |
| `TerrainMaterial::as_bind_group` | `terrain_material.rs:291-384` | 94 |
| `WaterMaterial::as_bind_group` | `water_material.rs:175-268` | 94 |
| `update_sun_position_system` | `zone_lighting.rs:348-442` | 95 |
| `update_starry_sky_system` | `starry_sky_material.rs:383-455` | 73 (with ~50 lines of dead logging) |
| `CloudMaterial::as_bind_group` | `cloud_material.rs:203-264` | 62 |
| `ParticleMaterial::specialize` | `particle_material.rs:91-148` | 58 (half is logging) |

**Notable bug:** `world_ui.rs:574-575` calls `pipelines.specialize(...)` **twice with identical arguments** (copy-paste) — wasteful GPU pipeline cache lookup each frame; remove line 575.

**Suggested splits:** `queue_world_ui_meshes` → extract per-rect logic (vertex building, bind-group creation, phase-item push) into 2-3 helpers; the starry-sky pair shrinks to ~40 lines each once dead logging is removed; the two `as_bind_group` bodies can use the shared helper from §1b.

---

## 5. WGSL Shader Duplication

1. **`rose_terrain_extension.wgsl` == `rose_water_extension.wgsl`** — byte-identical 32-line pass-through; both unused by any entity (see §2). Delete one, or both.
2. **3D noise stack duplicated**: `hash`/`hash3`/`quintic`/`quintic3`/`gradient_noise`/`fbm` are identical between `cloud.wgsl:53-114` and `volumetric_cloud.wgsl:46-98` (~55 lines).
3. **2D hash duplicated** (same `0.1031`/`33.33` formula, minor variants): `water_material.wgsl:320-330`, `underwater_effect.wgsl:59-65`, `wing_material.wgsl:41-46`. Plus overlapping `value_noise`/`gradient_noise`/`noise`/`fbm` blocks in those three files (~20-30 lines each).
4. `fbm` with different signatures exists in 5 files: `cloud.wgsl:114`, `volumetric_cloud.wgsl:98`, `underwater_effect.wgsl:112`, `water_material.wgsl:399`, `wing_material.wgsl:61`.

**Suggestion:** WGSL supports `#import` via `Shader::from_wgsl` with `import_path` — create one shared `common_noise.wgsl` module (`import_path: "rose::noise"`) with the 3D and 2D noise suites, and `#import` it from the 5 shaders. Verify naga import support for these files (they are loaded as internal assets, which supports `import_path` — check against Bevy 0.18.1 source `bevy-shader` for exact syntax before doing this).

**Estimated savings:** ~120-160 WGSL lines + eliminates maintenance drift (hash constant tweaks currently apply to only one copy).

---

## 6. Minor / Hygiene

- `render/mod.rs:122-132`: `RoseRenderPlugin` only registers Terrain+Water plugins and prints two logs; all other plugins are registered ad-hoc in `lib.rs`. Either consolidate all material plugin registrations into `RoseRenderPlugin` or delete it.
- `graphics_settings.rs:478-604`: the 4 presets (`low/medium/high/ultra`) duplicate the ~35-field struct each; could build on `Default::default()` + overrides (like `cloud_material.rs:663` already does). ~30 lines saved, and preset drift (e.g. `shadow_filtering: Temporal` in Ultra at `:583`) becomes explicit.
- `graphics/apply_systems.rs`: 6 systems, each with the same `is_changed` early-return guard — fine as-is, no action needed.
- `wing_material.rs` (37 lines) and `skinned_mesh_fix.rs` (69 lines) are already lean; no action.

---

## Prioritized Summary — Top 5 Quick Wins

| # | Action | Risk | Est. LOC saved |
|---|---|---|---|
| 1 | **Delete dead trail_effect logic** (`trail_effect.rs` systems/plugin), `post_processing.rs` placeholder (or finish it), `particle_debug.rs` unregistered systems | Zero (delete-only) | ~330 |
| 2 | **Delete unused extensions + identical shaders**: `terrain_material_extension.rs`, `water_material_extension.rs`, `blood_overlay_material.rs`, `blood_overlay_shader.rs`, `rose_terrain_extension.wgsl`/`rose_water_extension.wgsl`/`blood_overlay.wgsl`, and the 4 plugin registrations in `lib.rs:1062-1070` | Zero visually — pass-through shaders, never-instantiated materials | ~330 Rust + 5 KB WGSL |
| 3 | **Strip dead logging scaffolding** in `starry_sky_material.rs` (`should_log` branches, empty `diagnose_starry_sky_materials`, `FORCE_NIGHT_MODE`, `sky_sphere_follow_camera_system` if unused) and `particle_material.rs` `specialize` DIAGNOSTIC logs | Zero (logging-only) | ~300 |
| 4 | **Dedupe `calculate_cloud_lighting`** (cloud_material ↔ volumetric_cloud) and terrain time-of-day multiplier; fix duplicated `pipelines.specialize` in `world_ui.rs:575` | Low (shared helper, same math) | ~60 + 1 bug fix |
| 5 | **Unify the 4 manual `AsBindGroup` impls + material plugin boilerplate** into shared helpers (§1a/§1b); optionally retire dormant `cloud_material.rs` module (773 lines) pending a decision on 2D clouds | Medium (needs visual verification per material) | ~400-500 |

**Total realistic savings: ~1,800-2,500 of 7,635 Rust lines (~25-33%) plus ~20 KB of dead WGSL**, most of it delete-only.

---

*End of report. This is a PLAN ONLY — no files were modified.*
