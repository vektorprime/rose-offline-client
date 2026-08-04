# Optimization Review 02 — Rendering Pipeline, Materials & Graphics Settings

**Repo:** `rose-offline-client` (single crate, Bevy 0.18.1, bevy_egui 0.39, wgpu v27)
**Date:** 2026-08-03
**Reviewer role:** research/writing only — no game code was modified, no build was run, no game run.

## 1. Scope

### Files covered

- `src/render/` — all modules: `terrain_material.rs`, `water_material.rs`, `cloud_material.rs`, `volumetric_cloud.rs`, `starry_sky_material.rs`, `particle_material.rs`, `particle_render_data.rs`, `damage_digit_material.rs`, `damage_digit_render_data.rs`, `zone_lighting.rs`, `water_reflection.rs`, `underwater_effect.rs`, `world_ui.rs`, `wing_material.rs`, `trail_effect.rs`, `object_material_extension.rs`, `effect_mesh_extension.rs`, `extension_material_plugin.rs`, `skinned_mesh_fix.rs`, `mod.rs`
- `src/render/shaders/` — all 15 `.wgsl` files (read in full or key sections): `zone_lighting.wgsl`, `particle.wgsl`, `damage_digit.wgsl`, `terrain_material.wgsl`, `water_material.wgsl`, `cloud.wgsl`, `volumetric_cloud.wgsl`, `starry_sky.wgsl`, `underwater_effect.wgsl`, `object_material_extension.wgsl`, `rose_effect_extension.wgsl`, `world_ui.wgsl`, etc.
- `src/dds_image_loader.rs` — DDS → Bevy `Image` conversion (read in full)
- `src/graphics/` — `graphics_settings.rs` (full 544 lines), `apply_systems.rs` (full)
- `src/systems/` — `particle_sequence_system.rs`, `damage_digit_render_system.rs`, `zone_time_system.rs`, `memory_diagnostics.rs`
- `src/components/` — `water_settings.rs`, `render_configuration.rs`, `specular_texture.rs`
- `src/lib.rs` — render/graphics registration (lines ~740-770, 847, 1937-2297) and `src/render/mod.rs`

### Architecture docs that existed

- `system-architecture/Render.md`, `Lighting.md`, `Camera.md`, `zone_lighting.md`, `planar-water-reflection.md`, `sky_stars_architecture.md`, `SUN_DOCUMENTATION.md`, `weather-season-system.md` — noted (sky/water/lighting domain). Subsystem docs for UI/Window/Animation/Physics/Assets/ECS/Transform/Audio read in prior reviews.
- Pitfalls read: `pitfalls/index.md`, `rendering-camera.md`, `materials-transparency.md`, `lighting.md`, `water-system.md`, `performance-memory.md`, `login-sky-determinism.md`.
- Pitfall `performance-memory.md` ("every-frame asset creation leaks") applies directly here: the leak variant (overwriting `Assets<ShaderStorageBuffer>` handles per frame) was fixed, but the per-frame recreation pattern itself remains and is the dominant rendering finding (F1/F2).

## 2. Methodology

1. Read all render modules, all shaders, settings, loaders end-to-end (multiple passes for `water_material.rs`, `underwater_effect.rs`, `graphics_settings.rs`).
2. Grep-based verification: dead settings (`fxaa_enabled`, `smaa_quality`, `motion_blur_*`, `mip_bias`), `CascadeShadowConfig` usage, `generate_mipmaps`/`MipmapMode`/`mip_level_count`, material-apply system registration, per-frame mutation sites.
3. Validated Bevy 0.18.1 / wgpu 27 semantics against `bevy-collection\bevy-0.18.1\crates\`:
   - `bevy_pbr/src/render/material_bind_groups.rs:1866` — non-bindless `allocate_unprepared` re-prepares a **fresh bind group + buffer** per material change; `bevy_pbr/src/material.rs:563-598` — `RenderMaterialInstances` change-tick tracking ⇒ any material asset mutation triggers re-prepare.
   - `bevy_render/src/storage.rs:59,81,95,106` — `ShaderStorageBuffer::new/set_data/resize/resize_in_place` exist (buffer **reuse** API for the F1/F2 fix).
   - `bevy_image/src/compressed_image_saver.rs:46` — the only `generate_mipmaps` reference in Bevy 0.18.1; **`Image::generate_mipmaps` API removed** (fix F7 must generate mips in the loader).
   - `bevy_light/src/cascade.rs` — `CascadeShadowConfig` is a **Component** (default = `CascadeShadowConfigBuilder::default()`, 4 cascades); game never adds it ⇒ cascade presets dead (F12).
4. No code was changed; no `cargo build`; no game run.

---

## 3. Findings

> Naming: each finding lists `file:line` refs, impact estimate, and a concrete fix sketch.

---

### F1. [HIGH] Particle storage buffers (4/system) recreated + 4 Vec clones per frame

**Files:** `src/systems/particle_sequence_system.rs:563-601` (hardcoded `should_recreate_buffers = true` at line 569; the comment at 567-568 documents the intended-but-disabled optimization), `src/render/shaders/particle.wgsl` (bindings)

**Description:** Every frame, for every particle system: `positions/sizes/colors/textures` are cloned from the render data and `Assets<ShaderStorageBuffer>::add`-ed (4 new GPU storage buffers), then the old handles `remove`-d (4 frees). Each new handle mutates `ParticleMaterial` ⇒ with bindless disabled (`lib.rs:760-765`), Bevy's non-bindless path re-prepares a **fresh bind group + fresh uniform buffer per material per frame** (`material_bind_groups.rs:1866`, `material.rs:563-598`). On top of that, `particle.wgsl` declares 4 **separate scalar `u32` uniform bindings** (group 2, bindings 6-9: `blend_op`, `src_blend_factor`, `dst_blend_factor`, `billboard_type`) — each becomes its own 16-byte uniform buffer, re-uploaded every frame even though these values rarely change.

**Impact:** Constant GPU allocator churn + 4 full `Vec` clones per system per frame on CPU. With ~20-50 active particle systems in a fight scene this is thousands of alloc/free pairs/sec and a bind-group prepare per system per frame.

**Fix sketch:**
```rust
// Reuse buffers; only recreate when capacity is insufficient:
let capacity_needed = render_data.positions.len() * 4; // 4 verts/particle, see shader
let Some(mat) = materials.get_mut(&existing_material_handle.0) else { continue; };
for (slot, data) in [(&mut mat.positions, &render_data.positions),
                     (&mut mat.sizes,     &render_data.sizes),
                     (&mut mat.colors,    &render_data.colors),
                     (&mut mat.textures,  &render_data.textures)] {
    let Some(buf) = storage_buffers.get_mut(slot) else { continue; };
    if buf.data_len() < data.len() { buf.resize(bytes_needed); } // storage.rs:95
    buf.set_data(data.clone());                                   // storage.rs:81
}
```
- Only mark material changed when buffer handles actually change (capacity growth) — data upload no longer touches the material/bind group.
- Pack the 4 scalar uniforms into one `vec4<u32>` binding (shader + AsBindGroup change) — one buffer instead of four.

---

### F2. [MED-HIGH] Damage digit storage buffers (3/digit entity) recreated per frame

**Files:** `src/systems/damage_digit_render_system.rs:145-171`, `src/render/shaders/damage_digit.wgsl`

**Description:** Same pattern as F1: every frame each active digit entity does 3× `add(ShaderStorageBuffer::from(clone))` + 3× `remove` (lines 153-171). Digit entities are short-lived (a few seconds), so the count is bounded by active digits, but each frame allocates/frees 3 GPU buffers + re-preps the material bind group per digit.

**Impact:** Bursty during combat (many floating digits) — allocator churn + per-digit bind group re-prepare per frame.

**Fix sketch:** Same as F1 — keep handles, `resize` on capacity growth (digits rarely exceed max size mid-life: 10 digits × 6 vertices max), `set_data` per frame. Guard material mutation on handle change.

---

### F3. [MED] Terrain lighting storage buffer + sampler + bind group recreated every frame

**Files:** `src/render/terrain_material.rs:73-118` (`update_terrain_lighting_system`, guard `zone_time.is_changed()`), `:228-321` (`as_bind_group` allocates storage buffer + sampler + bind group per call)

**Description:** `update_terrain_lighting_system` mutates the (single, per-zone) `TerrainMaterial` whenever `zone_time.is_changed()`. `ZoneTime` is written every frame by `zone_time_system` ⇒ the guard is always true ⇒ material touched every frame ⇒ `as_bind_group` allocates a fresh storage buffer (lighting vec), a fresh sampler, and a fresh bind group every frame. The terrain lighting data (sun/dir/ambient/fog colors) actually only changes when time-of-day state transitions.

**Impact:** Per-frame GPU allocation + bind group prepare for the terrain material (shared by the whole zone); the change tick also forces re-extract/re-prep of the material every frame.

**Fix sketch:** Compare the packed lighting payload (e.g. `Vec4::to_array` hashing/`abs_diff_eq` against a cached copy) and only mutate the material when it changed. Better: move lighting to a persistent uniform buffer written with `write_buffer` each frame (same pattern as `zone_lighting.rs:729-739`), leaving the bind group stable.

---

### F4. [MED] Water material storage buffer + bind group recreated every frame

**Files:** `src/lib.rs:2166-2184` (`apply_water_settings` guard `water_settings.is_changed() || zone_lighting.is_changed()`), `src/render/water_material.rs:193-336` (`as_bind_group` allocates storage buffer + reflection texture view + sampler + bind group per call)

**Description:** `apply_water_settings` iterates **all** `WaterMaterial` assets and mutates `settings`/fog fields. `zone_lighting` is written every frame (`sync_zone_lighting_to_bevy_lights_system`, `update_sun_position_system`) ⇒ the guard is true every frame ⇒ every water material (1 per zone) is mutated every frame ⇒ per-frame `as_bind_group` allocation churn (storage buffer + bind group; also the reflection image view/sampler are re-created each call).

**Impact:** Per-frame allocation churn on the water material; also forces re-prepare of all water pipelines' bind groups each frame.

**Fix sketch:** Diff the actual water-setting fields (only mutate when a value changed — they only change when the user opens the water settings panel), and/or mirror F3's persistent-buffer approach so per-frame data changes don't rebuild bind groups.

---

### F5. [LOW-MED] Animated sky/cloud materials: fresh uniform buffer + bind group per frame (6 sites)

**Files:** `src/render/cloud_material.rs:405-488` (`update_cloud_material_system` writes `material.time` every frame), `:491-538` (`update_cloud_lighting_system` writes every frame, no guard), `src/render/volumetric_cloud.rs:531-553` + `:554-575` (same pattern), `src/render/starry_sky_material.rs:337-396` (`update_starry_sky_system`) + `:397-…` (`update_starry_sky_night_factor`) — all with manual `AsBindGroup` impls whose `prepare()`/`as_bind_group` allocate a fresh GPU uniform buffer + bind group per call.

**Description:** Each system mutates material fields every frame (time-of-day based, so they genuinely change) ⇒ per-frame uniform buffer + bind group allocation for: cloud plane, volumetric cloud blobs, starry sky sphere. Quantities are small (1 material each), so per-site impact is low — but combined with F3/F4 this is a constant stream of GPU allocations from the same anti-pattern ("material asset as per-frame data channel").

**Impact:** ~4-6 small GPU allocations + bind group prepares per frame; amplification by the non-bindless path (F18).

**Fix sketch:** Give these materials one shared bind group whose uniform is a persistent `Buffer` written once per frame via `write_buffer` (pattern already used by `zone_lighting.rs:729-739`). Material assets then only change when *settings* change (density, brightness, etc.).

---

### F6. [LOW-MED] Underwater full-screen pass + fresh bind group every frame, even on land

**Files:** `src/render/underwater_effect.rs:219-291` (`UnderwaterEffectNode::run`), `src/render/shaders/underwater_effect.wgsl:196` (shader-side early-out only)

**Description:** The `ViewNode` runs unconditionally for every camera view: allocates a bind group via `render_device().create_bind_group(...)` (lines 271-279) and issues a full-screen draw (0..3) every frame. `CameraUnderwaterState` is queried but **unused** (`_underwater_state`); the only mitigation is the shader early-returning when `is_underwater < 0.5`. So even standing on dry land, every frame costs: a full-screen pass (source → destination copy through the shader) + a bind group allocation.

**Impact:** One full-screen pass + one bind group alloc per frame per camera, always.

**Fix sketch:** Early-out in `run()`: `if !state.is_underwater { return Ok(()); }` (the query already fetches `CameraUnderwaterState` — just use it). Cache the bind group across frames (it depends only on `source`/`sampler`/uniform buffer, which are stable per view).

---

### F7. [HIGH] DDS loader: everything decompressed to uncompressed, mip-less RGBA8

**Files:** `src/dds_image_loader.rs` (header comment + `load()` 12-97; `create_rgba_image` 345-373; BC1/BC2/BC3 decode loops 750+), graphics `TextureQuality` interaction

**Description:** Every DDS (BC1/BC2/BC3/… or raw) is block-decoded on the CPU to `TextureFormat::Rgba8UnormSrgb` with **no mip chain** and `ImageSampler::linear()`. Grep across `src/` for `generate_mipmaps|MipmapMode|mip_level_count` = no hits. Bevy 0.18.1 removed the `Image::generate_mipmaps` API entirely (verified: only `compressed_image_saver.rs:46` compressor param mentions it) — so nothing generates mips anywhere. Consequences:
- ~4× VRAM vs BC1/BC3 (8:1 / 4:1 compression) for every texture, including the terrain 100-slot binding array and all world/character textures.
- No LOD chain ⇒ aliasing/shimmer on terrain and objects at distance; texture-cache misses (each mip-less texture samples full-res everywhere).
- Load-time CPU block-decode cost (a decode loop over all blocks per texture).

**Impact:** Largest single VRAM + bandwidth + visual-quality issue in the renderer. It also voids the existing `TextureQuality::mip_bias` setting (F8).

**Fix sketch (two stages):**
1. (Short term, big win) Generate mip levels **inside the loader** during decode: down-sample after each level into the same `Vec<u8>`, set `texture_descriptor.mip_level_count`, so every image ships with a full chain. No external API needed.
2. (Longer term) Keep BC1/BC3 formats: pass through `TextureFormat::Bc1RgbaUnormSrgb` / `Bc3RgbaUnormSrgb` (view format must match for binding arrays). Validate Bevy 0.18.1 compressed-image support first — the loader's own comment records past `pixel_size` panics on compressed textures (0.13 era) — see Risks.

---

### F8. [LOW] `TextureQuality::mip_bias` is dead code

**Files:** `src/graphics/graphics_settings.rs:172` (definition), no consumer (grep)

**Description:** `mip_bias()` exists but no system ever reads it (the UI exposes texture quality; nothing applies it to materials/cameras). It is also moot until F7 lands (no mips to bias).

**Fix:** Wire to the camera sampler override once mips exist (Bevy 0.18: `bevy_pbr` has no per-camera mip-bias; apply via `ImageSampler` anisotropy/`MipmapFilter` per material or a render-arg) — or remove the setting.

---

### F9. [HIGH] Always-on post FX with no settings control; motion blur / FXAA / SMAA settings are dead UI

**Files:** `src/lib.rs:1965-1991` (camera spawn: `Smaa::default()`, `ScreenSpaceReflections::default()` 1985, `MotionBlur::default()` 1987, `AutoExposure::default()` 1989, `ContrastAdaptiveSharpening::default()` 1991), `src/graphics/graphics_settings.rs:385-388` (`motion_blur_enabled/intensity`), `:409` (`fxaa_enabled`), `:412` (`smaa_quality`) — grep: **no apply system reads any of these**; `src/ui/…/ui_settings_system.rs:1107-1119` (UI toggles that do nothing)

**Description:** The camera unconditionally carries SSR, MotionBlur, AutoExposure, CAS and SMAA components. The settings UI offers motion blur toggle/intensity, FXAA, and SMAA quality, but no system applies them (only `bloom`, `shadow_*`, `tonemapping`, `color_grading`, `msaa`, `ambient_light` have apply systems — `apply_systems.rs`). SSR and MotionBlur are among the most expensive full-screen effects (multi-pass); there is no way to turn them off.

**Impact:** 2-3 full-screen passes per frame (SSR + motion blur; CAS/AutoExposure are cheap) that cannot be disabled, plus misleading UI settings.

**Fix sketch:** Add apply systems following the existing `apply_bloom_system` pattern (`apply_systems.rs:121-137`): on settings change, insert/remove `ScreenSpaceReflections`, `MotionBlur` (map `motion_blur_intensity` → `MotionBlur { shutter_angle, samples_per_frame }`), keep `AutoExposure`/`CAS`/`SMAA` gated on a quality setting. Defaults should match current visuals so nothing changes until the user opts out.

---

### F10. [MED] "Disabled" post FX still run minimal versions

**Files:** `src/lib.rs:2143-2148` (SSAO off → `quality_level: Low`, pass still rendered), `:2152-2159` (volumetric fog off → `step_count = 1`, pass still rendered), `:2080-2105` (`apply_depth_of_field_settings`: disabled → `DepthOfFieldMode::Gaussian` at 2099, gather pass still rendered), `src/graphics/apply_systems.rs:130-136` (bloom disabled → `intensity = 0.0`, bloom chain still runs)

**Description:** Three "disable" paths reduce quality to a minimum instead of removing the component, so the passes still execute: SSAO Low (full-screen depth+normal sampling), volumetric fog at 1 step (pass still queued; density is also zeroed by `zone_lighting.rs:249-253`), DoF Gaussian (full-screen gather), bloom intensity 0 (downsample/upsample chain still runs). Note there are **two divergent bloom-off paths**: `apply_bloom_system` sets intensity 0.0 (component stays) while `apply_post_processing_settings` removes the component entirely (`lib.rs:2127-2139`) — depending on which setting changed, the result differs.

**Impact:** Up to 3-4 full-screen passes run per frame that the user asked to disable. Volumetric fog is the most expensive when enabled (64 steps) — its "off" mode still costs a pass.

**Fix sketch:** Follow the Bloom-removal pattern (`lib.rs:2127-2139`): remove `ScreenSpaceAmbientOcclusion`, `VolumetricFog`, `DepthOfField` components when disabled; re-insert from settings on enable. Unify the two bloom paths (remove component; delete `apply_bloom_system` intensity=0 branch).

---

### F11. [LOW] Camera spawn hardcodes Ultra-equivalent FX regardless of preset

**Files:** `src/lib.rs:2007-2028` (spawn: DoF `Bokeh` 2007-2014, `VolumetricFog { step_count: 128 }` 2017-2022, `ScreenSpaceAmbientOcclusion { Ultra }` 2025-2028), `src/lib.rs:2107-2161` (normalized only when settings change — fires frame 1)

**Description:** The spawn values (Bokeh DoF, 128 fog steps, SSAO Ultra) contradict the settings system (which normalizes to `step_count 64` / SSAO Medium / DoF per settings on first change tick). Because `apply_post_processing_settings` runs on `is_changed()` (true on the first frame after resource init), the mismatch exists only for frame 0 — cosmetic, but the spawn constants are misleading and DoF is enabled at spawn even if the user saved DoF off.

**Fix:** Spawn camera FX from the settings resources (one-time cost; removes the frame-0 artifact and the "DoF always on at boot" behavior).

---

### F12. [LOW-MED] Shadow preset cascade count / max distance are dead: `CascadeShadowConfig` never added to lights

**Files:** `src/graphics/apply_systems.rs:45-92` (`apply_shadow_quality_system` — cascade bounds applied only `if let Some(mut config)` at 73-89), `src/graphics/graphics_settings.rs:89-95` (`cascade_count()`), `src/lib.rs:847` (`DirectionalLightShadowMap { size: 4096 }` hardcoded)

**Description:** Repo-wide grep: `CascadeShadowConfig` appears only in the `apply_systems.rs` query import and the `Option<&mut CascadeShadowConfig>` — it is **never inserted onto any DirectionalLight**. So the cascade branch (bounds, `minimum_distance`, `overlap_proportion`) never runs: `cascade_count()` and `shadow_max_distance` presets have no effect, and the game silently uses Bevy 0.18.1's default `CascadeShadowConfigBuilder` (4 cascades, verified `bevy_light/src/cascade.rs`). Preset "Shadow Quality" currently changes only the map size (Low 1024 … Ultra 4096) and the on/off flag; "Low = 1 cascade" cannot actually lower cascade count. The hardcoded 4096 at `lib.rs:847` is corrected on frame 1 by the apply system.

**Fix sketch:** Add `CascadeShadowConfig::default()` to the sun light at spawn (`lib.rs:2286` region) — the existing `apply_shadow_quality_system` then applies cascade counts/bounds per preset. Spawn `DirectionalLightShadowMap` from the current preset instead of 4096.

---

### F13. [INFO] Shadows correctly disabled at night — keep as-is

**Files:** `src/render/zone_lighting.rs:499-533` (`update_shadows_for_time_of_day_system` disables shadows at Evening/Night), `src/lib.rs:2286` (moon light `shadows_enabled: true` but `moon_shadows` is false in all states)

**Description:** Sun shadow map is skipped at night; moon light never renders a shadow map. Good existing behavior — no change needed; cited so future work on F12 doesn't accidentally enable night shadows.

---

### F14. [MED] Starry sky: ~300+ hash calls per pixel at night, full-screen

**Files:** `src/render/shaders/starry_sky.wgsl:101-144` (4 star layers × 27 cells × `hash3v` ≈ 324 hash evals/pixel; plus nebula `noise3`), `src/lib.rs:2188-2205` (sky sphere radius 50 000, camera ~7 200 units from origin; night-only via `night_factor`), `src/render/starry_sky_material.rs` (AlphaMode::Add, Transparent3d)

**Description:** At night the starry sky shader is a full-screen procedural star field with 4 layer passes and cell-based hashing — hundreds of hash evaluations per pixel with no LOD, plus nebula noise. Runs whenever `night_factor > 0`.

**Impact:** Fragment-bound cost at night on mid/low GPUs; worse on ultrawide/4K.

**Fix sketch:** Pre-bake a tileable star texture (stars + brightness) at load/init time and sample it (2 layers max); keep the twinkle in the vertex stage or a cheap time-based modulation. This changes the look slightly — keep density/brightness knobs (`starry_sky_material.rs` settings).

---

### F15. [MED] Water fragment shader: heavy procedural noise stack per pixel

**Files:** `src/render/shaders/water_material.wgsl:380-512` (`warp_noise` = 2 warp layers × 3 octaves ≈ 7 `gradient_noise` ≈ 28 hash calls, `organic_foam_noise`, `fbm` 4 octaves, edge splash, caustics), plus planar-reflection sample + refraction

**Description:** Every water pixel runs a multi-octave procedural noise cascade. Cost scales with water screen coverage (large lakes/surfaces in several zones) and resolution.

**Impact:** Fragment-bound on water-heavy views; the cost also applies during the reflection pass (F20).

**Fix sketch:** Pre-bake flow/foam/caustic noise into 2-3 small textures (bake at load, sample in shader); fade octaves by distance; consider precomputing the warp offset per vertex. Keep `WaterSettings` knobs intact.

---

### F16. [MED] Cloud plane: full-screen fBm with `NoFrustumCulling`; two cloud systems exist

**Files:** `src/render/shaders/cloud.wgsl:114-159` (fbm 4 octaves ≈ 32 `hash3` ≈ 96 hash calls + coverage fbm 2 octaves per pixel), `src/render/cloud_material.rs:541-609` (`spawn_cloud_layer`: 100 000 × 100 000 plane, `NoFrustumCulling`), `src/render/volumetric_cloud.rs` (second cloud system), `src/lib.rs:1364` (both spawners registered)

**Description:** The cloud plane is always full-screen (no culling) and runs a per-pixel 4+2 octave fBm with density/coverage shaping. A **second** cloud implementation (volumetric blob system) is also registered — verify only one is enabled by default (`CloudSettings.enabled` defaults true; volumetric default needs checking) or both costs stack.

**Impact:** Fragment-bound full-screen cost whenever clouds are enabled; two systems would double it.

**Fix sketch:** Bake the noise field into a texture (e.g. 3D noise atlas) sampled by the shader; drop octave count at low density; ensure only one cloud system is enabled by default (settings-driven switch).

---

### F17. [MED] Zone lighting state is triple-maintained (group-3 uniform vs per-material copies)

**Files:** `src/render/shaders/zone_lighting.wgsl` (`ZoneLightingData` at **group(3)** binding 0), `src/render/shaders/terrain_material.wgsl` (lighting in RO storage buffer), `src/render/shaders/water_material.wgsl` (lighting in storage buffer), `src/render/zone_lighting.rs:729-739` (uniform `write_buffer` per frame), plus `update_terrain_lighting_system` / `update_cloud_lighting_system` / `apply_water_settings`

**Description:** wgpu 27 forbids mixing binding arrays with uniform buffers in one bind group (comment in `terrain_material.wgsl`), and custom materials only get bind groups 0-2 — so the shared `ZoneLightingData` uniform at group(3) is unreachable from terrain/water/cloud materials. Each material therefore maintains its **own lighting copy** with its **own per-frame sync system** (F3, F4, F5): the same sun/ambient/fog data is computed and uploaded 3-4 times per frame in 3 different layouts.

**Impact:** Redundant per-frame uploads + the maintenance burden of keeping 3 representations in sync (source of the always-true guards in F3/F4).

**Fix sketch:** Establish one "lighting group(3)" bind group created once per frame (single buffer + `write_buffer`), and have all custom materials include the group-3 binding in their shader bind layouts (`AsBindGroup` supports multiple bind groups via `#[uniform]` groups or a shared layout) — deletes the per-material lighting storage buffers and their sync systems.

---

### F18. [LOW] Bindless disabled ⇒ non-bindless path amplifies every material change

**Files:** `src/lib.rs:760-765` (WgpuSettings disables `BUFFER_BINDING_ARRAY | STORAGE_RESOURCE_BINDING_ARRAY | PARTIALLY_BOUND_BINDING_ARRAY`), `:806-812` (bevy_egui bindless workaround)

**Description:** With those features disabled, Bevy 0.18.1 uses the non-bindless material path where every changed material gets a fresh bind group + buffer (`material_bind_groups.rs:1866`, `material.rs:563-598`). F1-F5 all pay this amplification.

**Fix sketch (considered, risky):** Re-enable `STORAGE_RESOURCE_BINDING_ARRAY` + `BUFFER_BINDING_ARRAY` (keep `PARTIALLY_BOUND_BINDING_ARRAY` off) and validate the bindless path with bevy_egui — reduces per-frame CPU churn, but requires a visual/regression pass (see Risks). The F1-F5 fixes are the primary path; this is additive.

---

### F19. [INFO] `memory_diagnostics` module is temporary tooling

**Files:** `src/systems/memory_diagnostics.rs` (logs entity/asset counts every 30 s incl. `ShaderStorageBuffer`, `ParticleMaterial`, `DamageDigitMaterial`)

**Description:** Instrumentation for the storage-buffer leak fix; the file itself says "remove once the leak is confirmed fixed". Keep active while validating F1/F2 fixes (the counters will directly show the effect); remove afterwards.

---

### F20. [MED-HIGH] Planar reflection re-renders the scene (half-res) whenever the camera is near water

**Files:** `src/render/water_reflection.rs:286-304` (enabled when camera within ~300 units of a water volume, `!is_underwater`, reflections enabled; `:422` underwater gating), `:375-379` (EnvironmentMapLight cloned onto the reflection camera)

**Description:** When the gate passes, a mirrored camera re-renders the whole visible scene (layer 0 only) at `reflection_scale` 0.5 every frame — geometry, all materials, plus the envmap IBL cost on the reflection camera. In water-heavy zones (or standing at a shore) this can be a large fraction of frame time. The gating is good (no reflection when underwater/far), but there is no quality/resolution control.

**Impact:** Biggest single variable render cost in the rendering path; unbounded by settings.

**Fix sketch:** Tie reflection resolution/distance to a settings slider (e.g. GraphicsSettings); cull far/distant objects from the reflection view (its own LOD/frustum); optionally skip when water occupies a small screen fraction. Keep the existing gating.

---

### F21. [LOW] Fixed-size procedural meshes for particles/damage digits (wasted vertex invocations)

**Files:** `src/systems/damage_digit_render_system.rs:140-143` (mesh created for max 10 digits × 6 vertices = 60, `@builtin(vertex_index)` in `damage_digit.wgsl`), `particle.wgsl` (quad from `vertex_index`)

**Description:** Meshes are sized for the maximum particle/digit count and geometry is generated in the vertex shader via `vertex_index`; when counts are below max, the surplus vertices still run the shader. Digit meshes are typically nearly full (up to 10 digits), so the waste is small; particle systems with variable counts waste more.

**Fix:** Shrink meshes when counts drop below half capacity (mesh write is cheap relative to per-frame buffer churn being fixed in F1/F2), or accept as negligible after F1/F2 land.

---

## 4. Priority-ranked summary

| # | Finding | Impact | Effort |
|---|---------|--------|--------|
| F1 | Particle storage buffers + clones recreated every frame | High | Med |
| F7 | DDS → uncompressed mip-less RGBA8 (no mip API in 0.18.1) | High | Med-High |
| F9 | SSR/MotionBlur/CAS/AutoExposure always on; motion-blur/FXAA/SMAA settings dead | High | Low-Med |
| F2 | Damage digit buffers recreated per frame | Med-High | Med |
| F10 | Disabled post FX still run (SSAO Low, fog step 1, DoF Gaussian, bloom intensity 0) | Med | Low |
| F20 | Planar reflection half-res scene re-render, no quality control | Med-High | Med |
| F3 | Terrain material bind group + storage buffer recreated every frame | Med | Med |
| F4 | Water material bind group + storage buffer recreated every frame | Med | Low |
| F14 | Starry sky ~300+ hashes/pixel at night | Med | Med |
| F15 | Water fragment noise stack | Med | High |
| F16 | Cloud plane full-screen fBm; two cloud systems | Med | High |
| F17 | Zone lighting triple-maintained (group-3 vs per-material copies) | Med | High |
| F5 | Sky/cloud materials: uniform buffer + bind group per frame | Low-Med | Med |
| F6 | Underwater pass + fresh bind group every frame on land | Low-Med | Very Low |
| F12 | Shadow cascade presets dead (no `CascadeShadowConfig` on lights) | Low-Med | Very Low |
| F8 | `mip_bias` dead (and moot until F7) | Low | Low |
| F11 | Camera spawn hardcodes Ultra FX for frame 0 | Low | Low |
| F18 | Bindless disabled amplifies F1-F5 | Low | High (risky) |
| F21 | Fixed-size procedural meshes | Low | Low |
| F13 | Night shadow disable (good practice) | — | — |
| F19 | `memory_diagnostics` temporary tooling | — | — |

## 5. Quick wins (small change, big effect)

1. **F6:** early-out `UnderwaterEffectNode::run` when `!is_underwater` (the data is already queried, just unused) — removes a full-screen pass + bind group alloc on land, every frame, every camera.
2. **F12:** insert `CascadeShadowConfig::default()` on the sun light at spawn — activates the existing cascade preset logic in `apply_shadow_quality_system` (one line + re-test presets).
3. **F10:** remove `ScreenSpaceAmbientOcclusion` / `VolumetricFog` / `DepthOfField` components when disabled (mirror Bloom's insert/remove at `lib.rs:2127-2139`); delete the `apply_bloom_system` intensity=0 duplicate.
4. **F4:** change `apply_water_settings` guard from `zone_lighting.is_changed()` to a field-level diff (settings only change when the user edits the panel) — kills the per-frame water bind group churn.
5. **F1 (subset):** pack the 4 scalar particle uniforms (`particle.wgsl` bindings 6-9) into one `vec4<u32>` — 4 uniform buffers become 1, and only re-set on change.
6. **F9 (subset):** wire `motion_blur_enabled` to insert/remove `MotionBlur` on the camera — the single most expensive settings-less effect becomes user-controllable.
7. **F20 (subset):** expose reflection resolution as a setting (default current 0.5) — immediate control over the biggest variable cost.
8. **F19:** run the game with `memory_diagnostics` and watch `ShaderStorageBuffer` counts before/after F1/F2 to confirm the fixes.

## 6. Risks & considerations per fix

- **F1/F2 (buffer reuse):** `ShaderStorageBuffer::resize/set_data` (`bevy_render/src/storage.rs:95,81`) are render-asset mutations — the prepare pass must re-upload the same GPU `Buffer` (verify it doesn't reallocate per upload; it reuses the buffer and only writes). Confirm `Assets<ShaderStorageBuffer>::get_mut` doesn't run in the render schedule (main-world only). Keep the old "remove old handles" removal path ONLY when truly replacing (capacity growth) to avoid leaks — the prior leak pitfall (`performance-memory.md`) is exactly this pattern's failure mode.
- **F7 (mips/compressed):** Bevy 0.18.1 has **no** `Image::generate_mipmaps` (verified) — mips must be computed inside `dds_image_loader.rs` (down-sample loop appended to the image data; `mip_level_count` + per-level data must match the image's size/data layout). Compressed formats: the loader's own comment records past `pixel_size` panics on compressed textures; verify `CompressedImageFormats`/wgpu feature support on D3D12 and that `binding_array<texture_2d>` views share the same format (all tiles must be BC1 or all BC3, not mixed). Do mips first (safe), compression second (validate).
- **F9/F10 (FX removal):** Removing components changes the look: DoF (Bokeh) is a deliberate cinematic choice; SSAO/volumetric fog affect lighting. Keep settings-driven with current values as defaults; test re-enable paths (insert with correct settings, e.g. `step_count 64`, `quality_level Medium`) — the fog/SSAO re-insertion must restore exactly what the apply system would set (F11 spawn constants are a foot-gun here: use the settings resource as the single source of truth).
- **F3/F4/F5/F17 (persistent buffers):** The shared group-3 lighting bind group requires every custom material shader to include that binding in its layout — shader + `AsBindGroup` changes; any material that forgets the binding will look wrong (no lighting). Roll out with terrain/water/cloud together; keep the wgpu-27 "no mixed binding arrays + uniforms" constraint in mind (group 3 is a plain uniform — fine).
- **F12 (cascades):** After inserting `CascadeShadowConfig`, presets actually change cascade count — verify shadow quality visually per preset (Low 1 cascade vs Ultra 4) and watch the "view uniform buffer overrun" note (`graphics_settings.rs:80` — High was already reduced from 4 to 3 to avoid overrun).
- **F14/F15/F16 (noise baking):** Visual changes by construction. The weather-season system and `SkySettings` tie into fog/sky colors (`zone_time_system`, `zone_lighting.rs`) — bake noise textures at load, keep density/octave knobs in settings; run a per-zone visual pass (day/dusk/night).
- **F18 (bindless):** Partially-bound binding arrays were disabled for a reason (bevy_egui interplay, `lib.rs:806-812` workaround). Re-enabling only the storage/buffer array features changes the material pipeline wholesale — validate frame times AND visual correctness across zone load + egui overlays before keeping.
- **F20 (reflection):** Changing resolution/culling alters water reflections (a signature visual). Keep current behavior as the default preset; add the control, don't change defaults.

## 7. Notes for future reviews

- The single root cause across F1-F5 is **"material asset used as a per-frame data channel"** with bindless disabled: every animated renderable (particles, digits, terrain, water, clouds, stars) mutates its material (or its `Assets<ShaderStorageBuffer>` handles) each frame, and Bevy 0.18.1's non-bindless path converts each mutation into fresh GPU allocations + bind group prepares. A dedicated "frame uniforms" mechanism (one persistent buffer + `write_buffer` per frame, bound to all custom materials) plus storage-buffer reuse would remove ~90% of the allocation churn in the renderer. Worth a pitfall entry if the fixes land: *"per-frame material mutation ⇒ per-frame bind-group recreation (non-bindless path)"*.
- `docs/`/`plans/` were removed before this review; the arch docs in `system-architecture/` for sky/water/lighting were accurate on every point checked.
- `memory_diagnostics.rs` counters (F19) make a good before/after measurement harness for F1/F2; also consider enabling Bevy's render diagnostics (render-resource memory + bind-group counts) for the report follow-up.
