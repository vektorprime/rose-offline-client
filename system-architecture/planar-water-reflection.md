# Planar Water Reflections and Water Shading Architecture

## Overview

The client renders **real planar reflections** of the 3D environment (terrain, objects, characters, clouds) onto water surfaces using the *mirrored camera* technique from the official Bevy `mirror` example (see `bevy-collection/bevy-0.18.1/examples/3d/mirror.rs`; the technique is unchanged in 0.19.1).

A dedicated reflection camera mirrors the main camera's transform across the water plane and renders the scene into an off-screen texture. The water material's fragment shader samples that texture at each fragment's own screen-space UV (plus a wave distortion offset) — no plane-reflection math in the shader is needed, because the reflected scene maps 1:1 to the mirrored camera's view.

The water surface itself is shaded physically (Fresnel, absorption, glints) with the scene's real lights; see *Water Shading* below.

## Design Goals

1. **Correct reflections**: mirrored, not upside-down; nothing below the water surface in the reflection (oblique near clip plane)
2. **No recursion**: water must never render into its own reflection (handled with `RenderLayers`)
3. **No world UI in the reflection**: name tags / chat bubbles are skipped for the reflection view (`NoWorldUi`)
4. **Performance**: reflection pass gated by distance to water and by water being in view; half-resolution render target
5. **Never sample a stale texture**: the water materials get a status (0 = camera disabled) and fall back to an analytic sky
6. **Debuggability**: on-water debug view showing the raw reflection texture plus a camera status encoding (colors)
7. **Settings integration**: enable/disable, resolution scale, debug view toggle in the settings UI

---

## Architecture Diagram

```mermaid
flowchart TB
    subgraph Main World
        MainCam[Main Camera<br/>Camera3d, RenderLayers [0,1]]
        WaterMesh[Water Mesh<br/>RenderLayers layer(1)]
        Env[Environment<br/>terrain/objects: Mesh3d + RenderLayers layer(0)]
        WaterMat[WaterMaterial<br/>settings + reflection_texture + reflection_status + sky_night_factor]
        Settings[WaterSettings<br/>reflection_enabled, reflection_scale, debug_show_reflection]
        Volumes[UnderwaterVolumes<br/>world-space water rectangles]
    end

    subgraph WaterReflectionPlugin src/render/water_reflection.rs
        Setup[setup_water_reflection<br/>PostStartup: spawn camera + render target]
        Manage[manage_reflection_image<br/>recreate target on resize/scale change]
        SyncTex[sync_reflection_textures<br/>point every WaterMaterial at the target]
        SyncCam[sync_reflection_camera<br/>PostUpdate, before TransformSystems::Propagate]
    end

    subgraph Render Target
        Img[Image<br/>Rgba16Float, linear HDR, half resolution<br/>alpha 0 = sky]
    end

    subgraph GPU Shader src/render/shaders/water_material.wgsl
        Sample[planar_reflection<br/>frag UV + wave offset, sky where alpha 0]
        Sky[sky_radiance<br/>analytic sky from view lights]
        Blend[Schlick Fresnel F0 0.02<br/>premultiplied alpha]
    end

    MainCam --> SyncCam
    Volumes --> SyncCam
    SyncCam --> Img
    Env --> Img
    Manage --> Img
    SyncTex --> Img
    SyncCam -->|status| WaterMat
    Img --> WaterMat
    WaterMat --> Sample
    Sky --> Sample
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
| `src/render/water_reflection.rs` | Plugin, reflection camera, render target lifecycle, per-frame camera sync, status push |
| `src/render/water_material.rs` | `WaterMaterial` (settings, reflection texture, status), premultiplied blending, 8-vec4 storage buffer |
| `src/render/shaders/water_material.wgsl` | Waves, Fresnel, reflection sampling, analytic sky, absorption, glints, caustics, foam, underside |
| `src/render/underwater_effect.rs` | `UnderwaterStatePlugin`: `UnderwaterVolumes` + `CameraUnderwaterState` (no screen effect) |
| `src/render/world_ui.rs` | `NoWorldUi` camera marker; world UI skips such views |
| `src/resources/water_settings.rs` | `WaterSettings` — shading settings plus `reflection_enabled` (default true), `reflection_scale` (0.5), `debug_show_reflection` |
| `src/ui/ui_settings_system.rs` | Water settings page (`render_water_page`) |
| `src/zone_loader/spawning/water.rs` | Water planes spawned on `RenderLayers::layer(1)`; volume center at the plane's height |
| `src/lib.rs` | `WaterReflectionPlugin` / `UnderwaterStatePlugin` registration; main camera uses `RenderLayers::from_layers(&[0, 1])`; `apply_water_settings` copies `WaterSettings` into every water material; `EguiGlobalSettings { auto_create_primary_context: false }` |

---

## Reflection Camera Setup

Spawned in `PostStartup` (after the main camera and its egui context exist):

```rust
Camera3d::default(),
Msaa::Off,
Hdr,
Tonemapping::None,
Camera {
    order: -1,
    is_active: false,          // enabled per-frame by the sync system
    invert_culling: true,      // mirrored geometry faces the mirrored camera
    clear_color: ClearColorConfig::Custom(Color::NONE), // alpha 0 marks sky
    ..Default::default()
},
RenderTarget::Image(handle.into()),       // off-screen texture
Projection::Perspective(PerspectiveProjection::default()),
RenderLayers::layer(0),                    // never renders water (layer 1)
NoWorldUi,                                 // no name tags / chat bubbles
WaterReflectionCamera,                     // marker used by Without<> filters
```

Render target: `Rgba16Float` (linear HDR, untonemapped). The water draws the reflection into the main HDR pass, where the main camera's Auto Exposure and tonemapping apply once to the whole frame, so reflections match the scene's brightness at every time of day. Target size = window size × `reflection_scale` (default 0.5).

**Sky**: the reflection camera has no atmosphere (`AtmosphereSettings` is deliberately not added: it previously caused a wgpu bind-group panic, and the mirrored camera sits below the ground the atmosphere model assumes). The target is cleared to transparent black, so pixels where nothing rendered keep alpha 0. Opaque geometry writes alpha 1, blended geometry (clouds) accumulates coverage, additive effects leave alpha unchanged. The shader composites `rgb + (1 - a) * sky`.

**Why a second `Camera3d` is safe**: every game system that called `.single()` on `With<Camera3d>` queries got a `Without<WaterReflectionCamera>` filter. Without those filters the reflection camera breaks camera-follow logic, input, minimap, lights, audio, etc.

---

## Per-Frame Sync (`sync_reflection_camera`)

Runs in `PostUpdate` **before** `TransformSystems::Propagate` so the engine propagates the new transform to `GlobalTransform` in the same frame. Each frame it:

1. **Picks the mirror plane**: the surface of the nearest water volume (XZ distance to its rectangle; ties, e.g. the camera above overlapping planes, go to the highest surface below the camera). Falls back to `WaterSettings::water_surface_y` (the first volume) when there are no volumes.
2. **Mirrors the transform** across that plane (`y = surface_y`):

   ```rust
   let plane_offset = Mat4::from_translation(Vec3::Y * surface_y);
   let reflect = Mat4::from_mat3a(reflection_matrix(Vec3::Y));
   let mirror_matrix = plane_offset * reflect * plane_offset.inverse();
   let mirrored_transform = Transform::from_matrix(mirror_matrix * main_transform.to_matrix());
   ```

   The transform and `camera.is_active` are written only on difference.
3. **Builds the projection**: the main perspective with a halved far plane (`max(far * 0.5, 2000) + 500`) and, while the camera is above the plane, the **oblique near clip plane** (see below). Written only when far/fov/near/clip plane differ (aspect is re-derived by Bevy's `camera_system` from the target).
4. **Writes the frustum directly** (see *The Frustum Trap* below).
5. **Gates the camera**: `camera.is_active` is true only when reflections are enabled in settings, a water volume exists, the camera is not underwater, the nearest water volume is ≤ 300 m away, **and a water plane intersects the main camera's frustum** (AABB vs frustum built from this frame's main transform, far plane ignored, same test as `check_visibility`; water spawned this frame counts as visible). With no water fragment on screen the reflection texture is never sampled, so the whole second scene render and its shadow cascades are skipped.
6. **Pushes the status** into every water material whose status differs (`push_reflection_status`): 0 when the camera is off for any reason except "no water in view" (that case leaves the status unchanged, since no water is drawn), otherwise 1-3 from the visible entity count. Checking every frame (read-only scan) also covers materials created by a zone load, which start at 0.
7. **Mirrors the `EnvironmentMapLight`** onto the reflection camera (when present) so the reflected scene is lit identically.

`sync_reflection_textures` keeps a `WaterMaterialRegistry` of water material ids. It drops ids whose material no longer exists before scanning, so the per-frame scans do not grow over a session.

---

## Oblique Near Clip Plane

`near_clip_plane = (view_from_world(main) * -Y).normalize().extend(surface_y - camera_y)` — the same construction as Bevy's `mirror` example. Bevy's `adjust_perspective_matrix_for_clip_plane` (Lengyel) replaces the projection's near plane with the water plane, so everything below the surface (lake bed, wading characters' legs, sunken objects) is clipped from the reflection.

It is only applied while the camera is above the plane (`surface_y - camera_y < -0.01`): the plane must face away from the camera or the projection degenerates.

Verified numerically against Bevy 0.19.1's math (perspective_infinite_reverse_rh + adjust + `from_clip_from_world_custom_far`): above-water points keep their clip depth (no extra clipping out to 4000 m), below-water points get NDC z > 1, and the frustum's near half-space (`row3 + row2`) stays loose, so CPU culling is unaffected. (It was disabled earlier only while the frustum trap below was being debugged.)

---

## The Mirror Matrix Math

The reflection camera's transform is the composition `mirror_matrix * main_camera_matrix`. `reflection_matrix(Vec3::Y)` from `bevy::math` builds the standard planar reflection matrix (determinant −1). `plane_offset * reflect * plane_offset.inverse()` shifts the reflection plane to `y = surface_y`.

`Transform::from_matrix` decomposes this correctly: glam's `to_scale_rotation_translation` detects the negative determinant and returns scale `(-1,-1,-1)` with a compensating rotation, so `to_matrix()` reproduces the exact reflection.

---

## The Frustum Trap (critical lesson)

The reflection camera's `Frustum` component **is written manually every frame**:

```rust
*frustum = reflection_perspective.compute_frustum(&GlobalTransform::from(*transform));
```

Why: Bevy's `update_frusta` recomputes a camera's frustum only when its `GlobalTransform` or `Projection` is change-detected. If that never fires for the reflection camera, the `Frustum` keeps its degenerate default (all-zero half-spaces), and `check_visibility` then culls **everything** except `NoFrustumCulling` entities.

Symptom: only clouds and name tags appeared in the reflection (both are `NoFrustumCulling`), the debug status stayed magenta (status 2, < 300 visible entities).

---

## Render Layers & Culling Strategy

| Entity | Layer | Purpose |
|---|---|---|
| Reflection camera | 0 | renders everything except water |
| Main camera | 0, 1 | renders everything |
| Water planes | 1 | excluded from the reflection camera → no recursion |
| Fish | 1 | excluded from reflections (perf; also fish are under the surface) |
| Terrain, objects, characters | 0 | appear in reflections |

Clouds (`cloud_material.rs`, `volumetric_cloud.rs`) use `NoFrustumCulling` and render in the reflection regardless of the frustum. Name tags and chat bubbles are **not** drawn in the reflection: world UI is a custom render pipeline that queues per view, and it skips views whose camera has `NoWorldUi`.

---

## Water Shading (`water_material.wgsl`)

All lighting comes from Bevy's `lights` uniform (every directional light: sun, sky fill, moon; plus ambient) and is scaled by `view.exposure`, like the PBR materials. The water therefore follows the day/night cycle exactly; nothing in the shader is a constant emissive color. Output is **premultiplied alpha** (`AlphaMode::Premultiplied`, blend `PREMULTIPLIED_ALPHA_BLENDING`):

```
color = F * reflection + (1 - F) * body + caustics + crest glow + glint   (foam mixed on top)
alpha = 1 - (1 - F) * T
```

so the opaque scene behind the surface (lake bed) shows through by exactly `(1 - F) * T`, and reflections/glints are not diluted in clear water.

| Part | Implementation |
|---|---|
| Waves | Up to 8 directional deep-water waves (2 per `wave_layers` octave, wavelengths 6.3 m → 0.22 m at `wave_frequency` 2.0, longest octaves slightly gentler) with analytic slopes, a slow domain warp, per-octave noise wave-group envelopes (no endless parallel stripes) and a ~1 m noise warp on the short octaves (no glitter lattice). Each wave fades out below ~2-6 pixels per wavelength (`fwidth` footprint); its slope variance is added to the glint roughness. |
| Fresnel | Schlick, F0 = 0.02 (IOR 1.33), × `fresnel_strength * 2` (0.5 = physical). |
| Reflection | Fragment UV + offset between the projections of `-V` and the mirrored wave-reflected ray `(R.x, -R.y, R.z)` (exact for distant scenery), × `refraction_strength * 5` (0.2 = physical, default 0.05 = a quarter), clamped to 0.08. Fades to the analytic sky where the offset leaves the texture. Used only when `reflection_enabled` and status ≥ 1. |
| Analytic sky | Single-scattering Rayleigh + Mie sky lit by each directional light (air mass, light reddening/fade at the horizon), × 2.5 to match Bevy's multi-scattering atmosphere. Fills alpha-0 reflection pixels (the starry sky sphere is in the texture itself when visible). When the reflection is off/stale it is blended toward the starry sky's night background by `WaterMaterial::sky_night_factor` (`StarrySkySettings::night_factor`, quantized to 1/32 and synced by `sync_water_sky_night_factor` in `water_material.rs`), because at night the starry sphere covers the atmosphere in the main view. |
| Body / absorption | Refracted path length through a smooth procedural depth field (`min_depth`..`max_depth`, biased shallow, scale `depth_gradient_scale`), or the exact depth of a depth-prepass object behind the surface if shallower. Absorption σ = −ln(`bottom_visibility`) / `shallow_threshold` (bottom visibility at the clarity depth), red ×2.2, blue ×0.85. Body = (1 − T) × mix(shallow, deep color) × 0.2 × E/π. |
| Glint | GGX + height-correlated Smith from the **brightest** directional light (sun by day, moon by night), × `specular_intensity * 2`, capped at 40. |
| Caustics | Ridged-noise pattern on the bed point under the refracted ray, lit by the key light through the water, weighted by the visible bed. |
| Crest glow | Key light through wave crests when looking toward a low sun/moon (`sss_intensity`). |
| Foam | Whitecaps above `foam_threshold` only with steep waves (`wave_amplitude` > ~0.5), plus contact foam where a depth-prepass object is < 0.35 m below the surface; lit like a diffuse surface. Soft fade where an object meets the surface. |
| Underside | Camera below the plane: Snell's window (TIR beyond ~48.6°). |

**Depth prepass**: the main camera has `DepthPrepass`, so the water pipeline gets the `DEPTH_PREPASS` def and `prepass_depth()`. The **terrain does not write the prepass** (`TerrainMaterial::enable_prepass() == false`), so the real lake-bed depth and terrain shorelines are unknown to the shader; only objects/characters/boats get exact thickness and contact foam.

**Fog**: the water no longer applies the zone fog itself (terrain and objects do not either); it was graying distant water against unfogged shores.

### Debug status colors

With `debug_show_reflection` on, the water shows either the raw reflection sample or a status color:

| Status | Meaning |
|---|---|
| Red (status 0) | camera disabled (setting, underwater, > 300 m, no water) |
| Orange (status 1) | camera active but 0 visible entities |
| Magenta (status 2) | active but < 300 visible entities (suspicious, broken frustum) |
| raw sample (status 3) | active with a normal entity count |

---

## Material Bindings (`water_material.rs`)

- Binding 0: read-only storage buffer, 8 vec4s:
  - `[0]` wave_amplitude, wave_frequency, wave_speed, wave_layers
  - `[1]` fresnel_strength, specular_intensity, sss_intensity, refraction_strength
  - `[2]` foam_intensity, foam_threshold, caustics_intensity, caustics_scale
  - `[3]` min_depth, max_depth, shallow_threshold, bottom_visibility
  - `[4]` deep_color, `[5]` shallow_color
  - `[6]` depth_gradient_scale.xy, caustics_speed, water_surface_y
  - `[7]` reflection enabled, debug_show_reflection, status, sky night factor
- Binding 1: `reflection_texture` (`Handle<Image>`; fallback image until the target exists)
- Binding 2: reflection sampler (clamp, linear)

Returned from `unprepared_bind_group` so Bevy's allocator frees the old bind group when the material changes (status transitions, settings edits).

---

## Interaction With Other Systems

- **`.single()` camera queries**: ~20 systems (input, minimap, lights, audio, clouds, weather, debug UI, map editor, model/zone viewers, etc.) got `Without<WaterReflectionCamera>` filters — required, or the second `Camera3d` panics/breaks them.
- **Egui**: `EguiGlobalSettings { auto_create_primary_context: false }` — prevents the reflection camera from stealing the primary egui context (`MultipleEntities` panic).
- **Atmosphere**: intentionally **not** enabled on the reflection camera (see *Reflection Camera Setup*).
- **EnvironmentMapLight**: mirrored (cloned) onto the reflection camera so lighting matches.
- **Underwater state**: `UnderwaterStatePlugin` (`underwater_effect.rs`) tracks volumes and `CameraUnderwaterState`; there is no underwater screen effect. Volumes of despawned water entities are dropped (they used to survive zone changes, and since every zone sits at the same world offset they could flag the camera as underwater or move the mirror plane in the next zone).

---

## Diagnostics

Periodic log (every 150 frames):

```
[WATER REFLECTION] cam=... surface_y=... volumes=... underwater=... settings_enabled=...
  active=... water_dist=... refl_pos=... refl_visible_entities=... refl_visible_mesh3d=...
```

- `surface_y` — the mirror plane actually used (nearest volume).
- `refl_visible_mesh3d` — how many terrain/object meshes the reflection camera sees (the real indicator of working culling).
- `[WATER REFLECTION] status changed to N` on transitions of the status.

---

## Known Issues & Future Work

1. **One mirror plane**: water at other heights in view reflects about the nearest volume's plane.
2. **No terrain depth**: shoreline softening/foam and real lake-bed depth need the terrain in the depth prepass (custom terrain prepass shader).
3. **Debug threshold** (300 visible entities) is a heuristic.
4. **Performance**: the reflection pass renders the whole scene again while water is on screen (gated by the 300 m distance check and the main-frustum water test).
