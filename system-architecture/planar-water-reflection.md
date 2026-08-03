# Planar Water Reflections Architecture

## Overview

The client renders **real planar reflections** of the 3D environment (terrain, objects, clouds, name tags) onto water surfaces using the *mirrored camera* technique from the official Bevy `mirror` example (Bevy 0.18.1, see `bevy-collection/bevy-0.18.1/examples/3d/mirror.rs`).

A dedicated reflection camera mirrors the main camera's transform across the water plane and renders the scene into an off-screen texture. The water material's fragment shader samples that texture using each fragment's own screen-space UV — no plane-reflection math in the shader is needed, because the reflected scene maps 1:1 to the mirrored camera's view.

## Design Goals

1. **Correct reflections**: mirrored, not upside-down; culling matches what the main camera would see mirrored
2. **No recursion**: water must never render into its own reflection (handled with `RenderLayers`)
3. **Performance**: reflection pass gated by distance to water; half-resolution render target; camera disabled when irrelevant
4. **Debuggability**: on-water debug view showing the raw reflection texture plus a camera status encoding (colors)
5. **Settings integration**: enable/disable, resolution scale, debug view toggle in the settings UI

---

## Architecture Diagram

```mermaid
flowchart TB
    subgraph Main World
        MainCam[Main Camera<br/>Camera3d, RenderLayers [0,1]]
        WaterMesh[Water Mesh<br/>RenderLayers layer(1)]
        Env[Environment<br/>terrain/objects: Mesh3d + RenderLayers layer(0)]
        WaterMat[WaterMaterial<br/>reflection_texture + reflection_status]
        Settings[WaterSettings<br/>reflection_enabled, reflection_scale, debug_show_reflection]
    end

    subgraph WaterReflectionPlugin src/render/water_reflection.rs
        Setup[setup_water_reflection<br/>PostStartup: spawn camera + render target]
        Manage[manage_reflection_image<br/>recreate target on resize/scale change]
        SyncTex[sync_reflection_textures<br/>point every WaterMaterial at the target]
        SyncCam[sync_reflection_camera<br/>PostUpdate, before TransformSystems::Propagate]
    end

    subgraph Render Target
        Img[Image<br/>Bgra8UnormSrgb, LDR, half resolution]
    end

    subgraph GPU Shader src/render/shaders/water_material.wgsl
        Sample[sample_water_reflection<br/>clip_from_world * world_pos -> UV -> sample]
        Blend[blend = max(fresnel, 0.5)<br/>mix water color with reflection]
        Debug[debug status colors<br/>red/orange/magenta]
    end

    MainCam --> SyncCam
    SyncCam --> MainCam
    SyncCam --> Env
    Env --> Img
    SyncCam --> Img
    Manage --> Img
    SyncTex --> Img
    Img --> WaterMat
    WaterMat --> Sample
    Sample --> Blend
    Settings --> SyncCam
    Settings --> Manage
    Settings --> WaterMat
    WaterMesh --> WaterMat
```

---

## Key Components

| File | Responsibility |
|---|---|
| `src/render/water_reflection.rs` | Plugin, reflection camera, render target lifecycle, per-frame camera sync |
| `src/render/water_material.rs` | `WaterMaterial` bind groups: reflection texture/sampler, 14-vec4 storage buffer |
| `src/render/shaders/water_material.wgsl` | `sample_water_reflection()`, fresnel blending, debug status colors |
| `src/resources/water_settings.rs` | `WaterSettings` — `reflection_enabled` (default true), `reflection_scale` (0.5), `debug_show_reflection` |
| `src/ui/ui_settings_system.rs` | Water settings page: Reflections checkbox, Resolution slider, Debug Show Reflection toggle |
| `src/zone_loader/spawning/water.rs` | Water planes spawned on `RenderLayers::layer(1)` |
| `src/lib.rs` | `WaterReflectionPlugin` registration; main camera uses `RenderLayers::from_layers(&[0, 1])`; `EguiGlobalSettings { auto_create_primary_context: false }` |

---

## Reflection Camera Setup

Spawned in `PostStartup` (after the main camera and its egui context exist):

```rust
Camera3d::default(),
Msaa::Off,
Camera {
    order: -1,
    is_active: false,          // enabled per-frame by the sync system
    invert_culling: true,      // mirrored geometry faces the mirrored camera
    clear_color: ClearColorConfig::Custom(Color::BLACK),
    ..Default::default()
},
RenderTarget::Image(handle.into()),       // off-screen texture
Projection::Perspective(PerspectiveProjection::default()),
RenderLayers::layer(0),                    // never renders water (layer 1)
WaterReflectionCamera,                     // marker used by Without<> filters
```

Render target: `Bgra8UnormSrgb` (LDR) — identical to the official `mirror` example. WGSL sampling of an sRGB texture returns linear values. Target size = window size × `reflection_scale` (default 0.5).

**Why a second `Camera3d` is safe**: every game system that called `.single()` on `With<Camera3d>` queries got a `Without<WaterReflectionCamera>` filter. Without those filters the reflection camera breaks camera-follow logic, input, minimap, lights, audio, etc.

---

## Per-Frame Sync (`sync_reflection_camera`)

Runs in `PostUpdate` **before** `TransformSystems::Propagate` so the engine propagates the new transform to `GlobalTransform` in the same frame. Each frame it:

1. **Mirrors the transform** across the water plane (`y = surface_y`):

   ```rust
   let plane_offset = Mat4::from_translation(Vec3::Y * surface_y);
   let reflect = Mat4::from_mat3a(reflection_matrix(Vec3::Y));
   let mirror_matrix = plane_offset * reflect * plane_offset.inverse();
   *transform = Transform::from_matrix(mirror_matrix * main_transform.to_matrix());
   ```

2. **Copies the projection**: `*projection = Projection::Perspective(main_perspective.clone())`.
3. **Writes the frustum directly** (see *The Frustum Trap* below).
4. **Gates the camera**: `camera.is_active` is true only when reflections are enabled in settings, a water volume exists, the camera is not underwater, and the nearest water volume is ≤ 300 m away.
5. **Mirrors the `EnvironmentMapLight`** onto the reflection camera (when present) so the reflected scene is lit identically.
6. **Pushes status + logs** (see *Diagnostics*).

The mirrored camera *position* is logged as `refl_pos` — it should always be `(x, 2*surface_y - y, z)` relative to the main camera.

---

## The Mirror Matrix Math

The reflection camera's transform is the composition `mirror_matrix * main_camera_matrix`. `reflection_matrix(Vec3::Y)` from `bevy::math` builds the standard planar reflection matrix (determinant −1). `plane_offset * reflect * plane_offset.inverse()` shifts the reflection plane to `y = surface_y`.

`Transform::from_matrix` decomposes this correctly: glam's `to_scale_rotation_translation` detects the negative determinant and returns scale `(-1,-1,-1)` with a compensating rotation, so `to_matrix()` reproduces the exact reflection. The view matrix is therefore `T⁻¹·R` — a valid (mirrored, left-handed) view, and the extracted frustum is a valid frustum.

---

## The Frustum Trap (critical lesson)

The reflection camera's `Frustum` component **is written manually every frame**:

```rust
*frustum = main_perspective.compute_frustum(&GlobalTransform::from(*transform));
```

Why: Bevy's `update_frusta` recomputes a camera's frustum only when its `GlobalTransform` or `Projection` is change-detected. If that never fires for the reflection camera, the `Frustum` keeps its degenerate default (all-zero half-spaces), and `check_visibility` then culls **everything** except `NoFrustumCulling` entities.

Symptom: only clouds and name tags appeared in the reflection (both are `NoFrustumCulling`), the debug status stayed magenta (status 2, < 300 visible entities), and all frustum sphere tests failed. Terrain/objects (normal `Mesh3d` with `Aabb`) were culled even though the frustum math itself was correct.

Verification steps used during debugging:
- Logged `near_plane` and `p0` half-spaces — a valid frustum has `near_plane.normal` ≈ the camera's forward direction and the camera position inside `p0`.
- Counted `VisibleEntities` per visibility class (`refl_visible_entities`, `refl_visible_mesh3d`) — after the fix, hundreds of terrain meshes are visible.

---

## Render Layers & Culling Strategy

| Entity | Layer | Purpose |
|---|---|---|
| Reflection camera | 0 | renders everything except water |
| Main camera | 0, 1 | renders everything |
| Water planes | 1 | excluded from the reflection camera → no recursion |
| Fish | 1 | excluded from reflections (perf; also fish are under the surface) |
| Terrain, objects, characters | 0 | appear in reflections |

Clouds (`cloud_material.rs:602`, `volumetric_cloud.rs:458`), name tags and chat bubbles (`name_tag_system.rs`, `chat_bubble_spawn_system.rs`) use `NoFrustumCulling` and therefore render in the reflection regardless of the frustum.

---

## Shader Side (`water_material.wgsl`)

### Screen-space sampling

`sample_water_reflection()` (line ~589) projects each water fragment through the **main camera's** `view.clip_from_world`:

```wgsl
let clip = view.clip_from_world * vec4<f32>(world_pos, 1.0);
let ndc = clip.xy / clip.w;
let uv = vec2<f32>(0.5 * ndc.x + 0.5, 0.5 - 0.5 * ndc.y);
```

Because the reflection camera's view-projection is exactly the main camera's composed with the plane reflection, a fragment on the water plane maps to the same UV in both — no plane reflection in the shader is required. A small wave-based UV distortion (0.008 scale) plus an edge fade keep the surface living and hide render-target seams.

### Blending

```wgsl
let reflection_blend = max(saturate(fresnel), 0.5);
final_color = mix(final_color, reflection_light, reflection_blend);
```

At least 50% reflection everywhere (clearly visible even looking straight down), rising to ~100% at grazing angles. `fresnel` combines `fresnel_schlick(VdotN, 0.2)` with a `fresnel_strength` setting and a 2.0 boost multiplier. When reflections are disabled, `sample_water_reflection` returns black and the fallback procedural sky tint is used instead.

### Debug status colors

With `debug_show_reflection` on, the water shows either the raw reflection sample or a status color:

| Status | Meaning |
|---|---|
| Red (status 0) | camera disabled |
| Orange (status 1) | camera active but 0 visible entities |
| Magenta (status 2) | active but < 300 visible entities (suspicious, broken frustum) |
| raw sample (status 3) | active with a normal entity count |

Status is pushed from the sync system into `WaterMaterial::reflection_status` (only on change) and read by the shader through the material storage buffer.

---

## Material Bindings (`water_material.rs`)

- Bind group 1: `reflection_texture` (`Handle<Image>`)
- Bind group 2: reflection sampler
- Storage buffer grown to **14 vec4s**:
  - `[12]` — reflection plane (normal + water surface y)
  - `[13]` — reflection params (enabled flag, status)

`AsBindGroup::Param` uses `RenderAssets<GpuImage>` so the bind group resolves the texture in the render world; a fallback image covers the frame before the first render.

---

## Interaction With Other Systems

- **`.single()` camera queries**: ~20 systems (input, minimap, lights, audio, clouds, weather, debug UI, map editor, model/zone viewers, etc.) got `Without<WaterReflectionCamera>` filters — required, or the second `Camera3d` panics/breaks them.
- **Egui**: `EguiGlobalSettings { auto_create_primary_context: false }` — prevents the reflection camera from stealing the primary egui context (`MultipleEntities` panic).
- **Atmosphere**: intentionally **not** cloned onto the reflection camera. It caused a fatal wgpu bind-group panic (24 vs 27 bindings) because the matching render-world atmosphere textures are not prepared for this camera.
- **EnvironmentMapLight**: mirrored (cloned) onto the reflection camera so lighting matches.

---

## Diagnostics

Periodic log (every 150 frames):

```
[WATER REFLECTION] cam=... surface_y=... volumes=... underwater=... settings_enabled=...
  active=... water_dist=... refl_pos=... refl_visible_entities=... refl_visible_mesh3d=...
[WATER REFLECTION DBG] gt_pos=... fwd=... | near_plane=... p0=... | hits cam/up100/down100/surf50=...
```

- `refl_visible_mesh3d` — how many terrain/object meshes the reflection camera sees (the real indicator of working culling).
- Frustum sphere-hit probes at the main camera position, ±100 m vertical, and 50 m above the surface — a valid frustum contains the camera and points along `fwd`.
- `[WATER REFLECTION] status changed to N` on transitions of the debug status.

---

## Known Issues & Future Work

1. **Oblique near clip plane disabled**: the Lengyel clip plane (from the mirror example) that would hide the lake bed from reflections is currently disabled (`// DEBUG TEST` in `sync_reflection_camera`) because its sign made the derived frustum cull everything. It must be re-enabled and sign-corrected now that the frustum is written manually.
2. **Debug threshold** (300 visible entities) is a heuristic; re-verify when the oblique plane is back.
3. **Performance**: the reflection pass renders the whole scene again — gated by the 300 m distance check, but terrain-heavy zones still pay for it. Future: reduce to a low-res texture with blur, or skip reflections when few entities are visible.
4. **LDR target** (`Bgra8UnormSrgb`): fine for now; an HDR target would allow exposure/tonemapping consistency with the main view.
