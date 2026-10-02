# Zone Lighting Documentation

This document describes the high-quality graphics and lighting improvements implemented for the Bevy 0.18.1 client.

> Single source of truth note: Bevy light-type reference lives in [Lighting.md](Lighting.md); sun-movement detail in [SUN_DOCUMENTATION.md](SUN_DOCUMENTATION.md); night sky in [sky_stars_architecture.md](sky_stars_architecture.md). Values below match current code (`src/lib.rs`, `src/render/zone_lighting.rs`, `src/graphics/`).

## 1. High-Resolution Shadows
The shadow mapping system has been significantly upgraded to provide sharp, detailed shadows across the game world.

- **Shadow Map Resolution**: Default **2048** (`src/lib.rs:842`, Medium default). 4096 applies only at `ShadowQuality::Ultra` (`src/graphics/graphics_settings.rs:104`, applied by `apply_shadow_quality_system` in `src/graphics/apply_systems.rs:68-121`).
- **Cascaded Shadow Maps (CSM)**: Default bounds in `src/render/zone_lighting.rs:147-153` are `[50.0, 100.0]` with `overlap_proportion: 0.2` (Medium default). `apply_shadow_quality_system` recomputes bounds/count from `cascade_count()`/`max_distance()` per quality (Off/Low/Medium/High/Ultra = 0/1/2/3/4 cascades).
- **Shadow Filtering**: Uses `ShadowFilteringMethod::Gaussian` for stable, high-quality soft edges (`src/lib.rs:1994`).

## 2. Advanced Lighting & Reflections
Modern PBR features have been integrated to increase visual richness and material depth.

- **Environment Map Light**: Added an `EnvironmentMapLight` to the main camera (`src/lib.rs:2020-2024`, `diffuse_map`/`specular_map` from `ETC/SPECULAR_SPHEREMAP.DDS#cube`, `intensity: 100.0`) to provide realistic reflections and irradiance to all PBR materials.
- **SSAO**: Screen Space Ambient Occlusion defaults to **Medium** (`src/lib.rs:2058-2061`); Ultra is only via `SsaoQuality::Ultra` (`src/graphics/apply_systems.rs:263`).
- **Bloom**: The natural bloom effect (`Bloom::NATURAL`, `src/lib.rs:1992`) enhances HDR highlights and light-emitting materials.

## 3. Synchronized Time-of-Day System
The lighting system has been fully synchronized to ensure all world elements, including custom shaders, react consistently to the day/night cycle.

- **Light Synchronization**: A new system `sync_zone_lighting_to_bevy_lights_system` in `src/render/zone_lighting.rs` bridges the `ZoneLighting` resource with Bevy's built-in `GlobalAmbientLight` resource and `DirectionalLight` component.
- **Balanced Intensities**:
    - `DirectionalLight` illuminance: **15,000 lux** (balanced for PBR).
    - `GlobalAmbientLight` brightness: **80.0 lux** base (Bevy's default), multiplied by the user's `ambient_light_brightness` graphics setting (default 1.0, giving 80.0 lux). Kept constant so day/night variation comes from sun + fill, preserving shadow contrast.
- **Change detection**: `zone_time_system` writes `ZoneLighting` and `ZoneTime` through `bypass_change_detection()` and calls `set_changed()` only when a value moved past its epsilon (1e-4 colours/densities, 1e-3 `state_percent_complete`), a state switched, or the tick advanced. Writing `&mut res.field` through `ResMut` is itself a change, which used to flag both resources every frame. They now change roughly once a second (`ZoneTime`) or only during dawn/dusk lerps and ticks (`ZoneLighting`), so the fog volume, sun, shadow and cloud systems gated on them do real work only then. Any system that reads them must gate on every input it uses (see `update_shadows_for_time_of_day_system`), not on "ZoneTime changes every frame".
- There is no render-world `ZoneLighting` uniform: one used to be extracted and uploaded every frame, but no pipeline bound it.

## 4. Dynamic Terrain Lighting & Sun Synchronization
The terrain rendering system has been overhauled to ensure it remains perfectly in sync with the game's dynamic sun and time-of-day cycle.

### The Synchronization Pipeline
Previously, the terrain used a hardcoded light direction and static colors, causing it to look disconnected from the rest of the world. The new pipeline ensures consistency:

1.  **Sun Position**: The `update_sun_position_system` calculates the sun's rotation based on the current game time.
2.  **Light Sync**: The `sync_zone_lighting_to_bevy_lights_system` writes the sun transform's `back()` vector (the direction **toward** the sun) into `ZoneLighting.light_direction`. This matches the `ZoneLighting` default and the terrain shader's `max(dot(N, L), 0)`. Until 2026-09-30 it wrote `forward()` (the direction light travels), which lit terrain from the wrong side: flat ground got no sun by day and was lit from below at night.
3.  **Material Update**: The `update_terrain_lighting_system` in `src/render/terrain_material.rs` mirrors the PBR lights with no time-of-day table:
    - Sun: strength is `terrain_light_intensity × 0.5 × (Sky sun brightness / 5000) × sun_light_factor(sun_height)`, tinted by `character_diffuse_color`.
    - Moon: a second light toward `StarrySkySettings.moon_direction`, with strength `MOON/SUN × moon_light_factor`, tinted by `MOON_COLOR`.

    Both change continuously with sun elevation, so there are no jumps at Morning/Day/Evening/Night changes (this replaced a 2.0/2.5/2.0/1.0 step multiplier).
4.  **Shader Execution**: The terrain shader ([`terrain_material.wgsl`](../src/render/shaders/terrain_material.wgsl)) reads a 5×vec4 storage buffer (sun dir, sun color, ambient, moon dir, moon color) and computes `ambient + sun·max(N·Lsun,0) + moon·max(N·Lmoon,0)` per pixel. It does not receive shadow maps or SSAO (`enable_prepass() = false`); `--new-terrain` does.

### Visual Impact
- **Consistent Shadows**: The terrain's highlights and shading now perfectly match the direction of shadows cast by characters and buildings.
- **Day/Night Transitions**: As the sun sets, the terrain naturally transitions from bright daylight to warm evening tones and finally to cool, dark night-time lighting.
- **Atmospheric Integration**: By using the same ambient color as the rest of the scene, the terrain feels like a natural part of the environment rather than a separate layer.

## 5. Atmospheric Effects
- **Volumetric Fog**: `VolumetricFog{step_count: 64}` on the main camera (`src/lib.rs:2051-2055`; comment notes 128 was 2x cost). Fog volume itself lives in `src/render/zone_lighting.rs:191-198`.
- **Atmospheric Scattering**: Integrated Bevy's built-in atmospheric scattering for realistic sky rendering (0.19: standalone `bevy_light::Atmosphere` entity + `AtmosphereSettings` on the camera; `toggle_atmosphere_based_on_time` keeps the entity permanently spawned — despawning triggers Bevy 0.19.1 bug #24808, see `pitfalls/atmosphere-flash.md`).
- **Procedural Starry Sky**: A custom material that renders a dense star field and moon with phases, automatically toggled based on the night factor.

## 6. Post-Processing
- **Tonemapping**: Uses `TonyMcMapface` (`src/lib.rs:1990`) for a high-quality filmic look that preserves detail in both highlights and shadows.
- **Anti-Aliasing**: MSAA is `Off` by default (`src/lib.rs:1967`); SMAA is opt-in via `apply_smaa_system` (Disabled/Low/Medium/High/Ultra). TAA/SSR are not implemented.
- **Motion Blur**: Opt-in only via `apply_motion_blur_system`; not spawned by default (`src/lib.rs:1995-1999`).
