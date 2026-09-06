# Bevy 0.18.1 → 0.19 Upgrade Plan

**Status:** in progress (Phase 0 recon done, Phase 1 starting)
**Target:** `bevy 0.19.1` (released 2026-06-19; MSRV Rust **1.95.0**)
**Migration guide:** https://bevy.org/learn/migration-guides/0-18-to-0-19
**Scope:** 424 `.rs` files. `bevy_procedural_grass` already disabled (incompatible since 0.18).

> Note: repo `plans/` + `docs/` are retired per AGENTS.md §10, so this plan lives at repo root.

## 0. Pre-work analysis (done)

- `pitfalls/index.md` reviewed — no prior Bevy-upgrade entries; relevant watch-outs: custom
  materials/shaders (`materials-transparency.md`, `water-system.md`), camera/post-process setup
  (`rendering-camera.md`), multi-view world-UI buffer issues (`water-system.md`).
- `system-architecture/README.md` reviewed — affected subsystems: Render, Lighting, Camera,
  zone-pipeline, model-spawning, combat-effects (particles), asset-loaders.
- `Cargo.toml` reviewed (full custom granular feature set, no default features).
- Full 0.18→0.19 migration guide fetched and grepped against `src/`. Hits below are verified
  with file:line references, not assumptions.

## 1. Dependency matrix (crates.io verified 2026-09-05)

| Dependency | Current | Target | Evidence |
|---|---|---|---|
| `bevy` (+ `bevy_mesh/camera/image/light/shader/post_process`) | 0.18 | **0.19** (`"0.19"`) | bevy 0.19.1 on crates.io, MSRV 1.95.0 |
| `bevy_egui` | 0.39 | **0.40** | bevy_egui 0.40.0 reqs `bevy ^0.19.0`, `egui ^0.34` |
| `bevy-inspector-egui` | 0.36 | **0.37** | changelog: 0.37.0 = bevy 0.19 + bevy_egui 0.40 |
| `egui` / `egui_extras` | 0.33 | **0.34** | bevy_egui 0.40 reqs `egui ^0.34` |
| `bevy_rapier3d` | 0.33 | **0.35** | changelog: v0.35.0 (2026-07-12) = bevy 0.19. Was hard-blocked before; now unblocked |
| `glam` | 0.29 | **0.32** | bevy_math 0.19.1 reqs `glam ^0.32.0`. No direct `glam::` use in `src/` (bevy re-export only), low risk |
| `encase` | 0.8 | **0.12** | bevy_egui 0.40 render reqs `encase ^0.12`; `zone_lighting.rs:908` writes a `#[derive(ShaderType)]` struct (`:776`) into `encase::UniformBuffer` — trait versions must match |
| `rand` | 0.8 | **keep 0.8** | Bevy moved to rand 0.10 (`Rng`→`RngExt`, `thread_rng()` gone), but ours is gameplay-only RNG (~30 files, `thread_rng`/`gen_range`/`SliceRandom`) with no Bevy API interop. Independent crate version — no conflict. Do NOT bump unless resolver complains |
| `image` | 0.24 | **keep 0.24** | Direct use is bevy types + test-only `image::load_from_memory_with_format` (`dds_image_loader.rs:987`). No interop. Keep unless compiler says otherwise |
| `cpal` | 0.15 | **keep 0.15** | Custom oddio audio stack, independent of Bevy's rodio 0.22/cpal 0.17. Keep |
| `accesskit` | 0.17 | **keep** | No `use accesskit` in `src/` (transitive/unused direct dep). Keep |
| `uuid = "1"`, `bytemuck 1.x`, rest | — | **keep** | Float / independent |
| `bevy_procedural_grass` (path, bevy 0.17) | — | **REMOVE dep** | Only commented-out uses (`lib.rs:37-38,932-935`). Still costs a full second Bevy 0.17 build today. Removal is safe |

Rust toolchain: installed `1.93.1` < MSRV `1.95.0` → **must `rustup update stable` first**.

## 2. Code hit list (verified by grep)

### P0 — will not compile without fix
1. **Render-graph-as-systems** — `src/render/underwater_effect.rs:20,219,229,237,341` uses
   removed API (`ViewNode`, `RenderGraphContext`, `NodeRunError`, `add_render_graph_node`).
   Rewrite node as system fn `(world: &World, view: ViewQuery<(&ExtractedCamera, &ViewTarget)>, mut ctx: RenderContext)` + `render_app.add_systems(Core3d, …after/before…in_set(Core3dSystems::MainPass))`.
   Also check `world_ui.rs` custom pipeline registration for graph usage.
2. **`ShaderStorageBuffer` → `ShaderBuffer`** (+ `GpuShaderStorageBuffer` → `GpuShaderBuffer`):
   `effect_loader.rs:15,99,412,481-485`, `model_loader.rs:11,248,893`, `zone_loader.rs:453`,
   `zone_loader/spawning/objects.rs:488`, `render/particle_material.rs:6,20-26,154`,
   `systems/memory_diagnostics.rs:18,53`, `systems/move_destination_effect_system.rs:37`,
   `systems/npc_model_system.rs:44`, `systems/particle_sequence_system.rs:14,521,580-598`.
3. **`light.shadows_enabled` → `shadow_maps_enabled`** (contact-shadows split):
   `graphics/apply_systems.rs:88-103`, `lib.rs:2368`, `render/zone_lighting.rs:197-235,660-700`.
4. **Atmosphere becomes a separate entity + moves to `bevy_light`**:
   `lib.rs:13` (`bevy::pbr::{Atmosphere, …}`), `lib.rs:2036`
   (`Atmosphere::earthlike(medium)` on camera + `AtmosphereSettings`), `lib.rs:1931`
   (`Assets<bevy::pbr::ScatteringMedium>`), `render/starry_sky_material.rs:498,527-591`
   (toggle logic inserts/removes `Atmosphere`/`AtmosphereSettings` on camera).
   Migrate: `Atmosphere::earthlike` → `Atmosphere::earth` (entity, own `Transform` scale),
   camera keeps only `AtmosphereSettings` (no `scene_units_to_m`), imports →
   `bevy::light::{Atmosphere, AtmosphereSettings}` / `bevy::light::atmosphere::ScatteringMedium`.
   Day/night toggle system (`starry_sky_material.rs:510-598`) must spawn/despawn the atmosphere
   entity instead of inserting/removing the component on the camera.
5. **Camera TextureFormat rework** — `render/world_ui.rs:257-258` (`MeshPipelineKey::HDR`,
   `ViewTarget::TEXTURE_FORMAT_HDR` deprecated), `:581` (`MeshPipelineKey::from_hdr(view.hdr)`),
   `:517` (`&ExtractedView` query), `render/underwater_effect.rs:557` (`TEXTURE_FORMAT_HDR`).
   Migrate: `ExtractedView::hdr` → `ExtractedCamera::hdr`, `ViewTarget::is_hdr` removed,
   source format from `ExtractedView::target_format` in specialization keys.
6. **`bevy_material` crate split** — `AlphaMode` (from `bevy_render`) and
   `SpecializedMeshPipelineError` (from `bevy_render`/`bevy_pbr`) moved to `bevy_material`.
   Touches ~25 files via `bevy::render::alpha::AlphaMode` and 8 render files
   (`terrain/cloud/starry_sky/water/volumetric_cloud/particle_material.rs`,
   `object_material_extension.rs`, `effect_mesh_extension.rs`). Fix import paths
   (prefer `bevy::material::…` facade if re-exported there), likewise
   `DefaultOpaqueRendererMethod` (`lib.rs:13,843`).
7. **`experimental` occlusion path** — `lib.rs:27`
   `render::experimental::occlusion_culling::OcclusionCulling` →
   `render::occlusion_culling::OcclusionCulling`.
8. **`insert_non_send_resource` → `insert_non_send`** — `audio/mod.rs:133` (deprecated alias;
   harmless but fix while here).
9. **`AssetServer::load_with_settings`** — `zone_loader/spawning/terrain.rs:446`. Old load
   variants deprecated in favor of `load()` / `load_builder()`; compiler will confirm.

### P1 — verify during build, fix if errors
- `Frustum`/`HalfSpace` (`render/water_reflection.rs:26,286,399-412`): partly moved
  `bevy_camera::primitives` → `bevy_math::primitives::{HalfSpace, ViewFrustum}`; `Frustum` is now a tuple wrapper.
- `ExtractComponent` refactor (`components/blink_clip.rs:6,30,54`, `underwater_effect.rs:17,99`):
  may need `SyncComponent` impl / `extract_component_sync_target` attribute.
- `MeshPipelineViewLayouts::get_view_layout` now by value; `generate_view_layouts` removed;
  `MeshViewBindGroup` offsets → `main_offsets` (only if custom view-bind-group code exists — grep says no direct use; world_ui/underwater pipelines to confirm).
- `MeshPipelineKey::from_msaa_samples` (`world_ui.rs:581`) — check strip-index-format bits requirement
  (`from_primitive_topology_and_strip_index`) if pipeline key construction errors appear.
- `bevy_reflect` root reorg — fix `use` paths per compiler hints (inspector-driven code most exposed).
- `Ref::clone()` semantics change — only matters if we clone `Ref<T>` (grep during Phase 2).
- `System::type_id` → `system_type`, `World::entities_allocator` rename — grep during Phase 2 (no hits in first pass).
- rapier 0.33→0.35 (two jumps): 0.34 changed joint APIs (`ImpulseJoint::data` enum, public `data`
  fields/getters removed, `has_any_active_contacts` renamed, `ColliderDebugColor` → `Hsla`).
  Audit physics call sites after first build.
- egui 0.33→0.34 + inspector 0.37 feature reorg (`bevy_camera/mesh/light` optional, `2d`/`3d`
  collections): fix UI compile errors per compiler.
- `bevy::ui::IsDefaultUiCamera` (`lib.rs:1984`) — must survive via feature unification through
  bevy_egui; confirm at build.
- `custom_cursor` feature (explicit in our list) still exists in 0.19 (moved collection only). Confirm at build.

### Explicitly NOT affected (verified absent)
`bevy_scene`/GLTF-subasset paths, `TextFont`/`TextLayout`/`TextSection`, `DynamicScene`,
`MorphWeights`, `AnimationTargetId` serialization, `Skybox` component (ours is game-data
`SkyboxDatabase`), `DespawnOnEnter/Exit` reliance, custom `SystemParam`/`Command` impls,
`SavedAsset`/`AssetSaver`, `ViewportNode`, `InputFocus`, `bevy_picking`, `PlaneMeshBuilder`,
`Image::pixel_bytes`, `get_full_extension`, `define_atomic_id`, `SystemBuffer`.

## 3. Execution phases

- [x] **Phase 0 — recon** (this plan; toolchain 1.98.1 installed via `rustup update stable`)
- [x] **Phase 1 — manifest**: bump versions per §1 table, remove `bevy_procedural_grass` dep,
  add `bevy_material` feature; **also bumped `rose-offline` workspace `bevy 0.18.1 → 0.19`**
  (shared `rose-game-common` components must impl the 0.19 `Component` trait — ~400 E0277s
  otherwise; server-side fallout tracked as risk below).
  Baseline `cargo build` (separate subtask) → collected error list, all predicted hits confirmed
- [x] **Phase 2a — mechanical**: `ShaderBuffer` rename, `shadow_maps_enabled`,
  `insert_non_send`, occlusion path, `bevy_material::AlphaMode` imports,
  removed `IsDefaultUiCamera` (no bevy_ui usage; avoids pulling `bevy_ui` crate)
- [x] **Phase 2b — structural**: render-graph-as-systems for underwater effect
  (`ViewNode` → `ViewQuery` system in `Core3dSystems::PostProcess` after `tonemapping`),
  Atmosphere-as-entity (+ day/night toggle now spawns/despawns), HDR rework
  (`MeshPipelineKey::HDR` gone → custom `WorldUiPipelineKey` with `target_format`)
- [x] **Phase 2c — fallout**: compiler-driven, all fixed —
  `Assets::get_mut` → `AssetMut` (`mut` bindings, scoped borrows, `into_inner()`),
  `DepthStencilState` `Option` fields, `MipmapFilterMode`, `immediate_size: 0`
  (replaces `push_constant_ranges`), `multiview_mask: None`,
  `Transparent3d::sorting_info` + `add_transient` (change-list phases),
  `RenderCreation::Automatic(Box::new(..))`, `contact_shadows_enabled` field,
  egui 0.34 (`Frame::NONE`, `smooth_scroll_delta`, `RectShape::angle`,
  `TextFormat::coords`), glam 0.32 (`angle_between` → `angle_to().abs()`)
- [x] **Build green**: `cargo build` exit 0 (2026-09-06, toolchain 1.98.1)
- [ ] **Phase 3 — verify (USER)**: launch client and check: zone load, day/night
  atmosphere, water reflection, underwater effect, particles/blood, physics/colliders,
  egui windows. Logs in `logs/<session>/`
- [ ] **Phase 4 — closeout**: user confirms fixed → add `pitfalls/` note per AGENTS.md §9
- [ ] **Phase 3 — verify**: `cargo build` clean (subtask); user launches client and checks:
  zone load, day/night atmosphere, water reflection, underwater effect, particles/blood,
  physics/colliders, egui windows, logs in `logs/<session>/`
- [ ] **Phase 4 — closeout**: remove dead code found on the way, user confirms fixed → add
  `pitfalls/` note per AGENTS.md §9

## 4. Risks
- ~~rapier double-jump (0.33→0.35) may have joint/character-controller API churn~~ —
  no rapier errors encountered; 0.35 compiled clean against our usage.
- **SERVER FALLOUT (known, out of scope)**: `rose-offline/Cargo.toml` workspace `bevy`
  bumped 0.18.1 → 0.19 (forced: shared `rose-game-common` components must impl the 0.19
  `Component` trait). `rose-offline-server` + `libs/big-brain` (pins bevy 0.18) will NOT
  compile until the server is migrated separately.
- wgpu 27→29 transitive: GPU behavior/visual diffs (bloom now linear-space — may look dimmer;
  raise `Bloom.intensity` if user reports it).
- Minimap arrow: `angle_between` (unsigned) → `angle_to().abs()` preserves old behavior
  exactly, but a signed angle would be more correct (arrow may mirror when turning).
  Future improvement, not a regression.
- First 0.19 build compiles a new full Bevy — slow (`[profile.dev.package."*"] opt-level = 3`).
  Do not mistake for a hang.
