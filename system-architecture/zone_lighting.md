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
    - `GlobalAmbientLight` brightness: **80.0 lux** base (Bevy's default), multiplied by the user's `ambient_light_brightness` graphics setting (default 1.5, giving 120.0 lux).

## 4. Dynamic Terrain Lighting & Sun Synchronization
The terrain rendering system has been overhauled to ensure it remains perfectly in sync with the game's dynamic sun and time-of-day cycle.

### The Synchronization Pipeline
Previously, the terrain used a hardcoded light direction and static colors, causing it to look disconnected from the rest of the world. The new pipeline ensures consistency:

1.  **Sun Position**: The `update_sun_position_system` calculates the sun's rotation based on the current game time.
2.  **Light Sync**: The `sync_zone_lighting_to_bevy_lights_system` extracts the actual `forward()` vector from the sun's transform and updates the `ZoneLighting.light_direction` resource.
3.  **Material Update**: The `update_terrain_lighting_system` in `src/render/terrain_material.rs` monitors the `ZoneLighting` resource. When the sun moves or colors change (e.g., at sunset), it pushes the new light direction, light color, and ambient color into the `TerrainMaterial` uniforms.
4.  **Shader Execution**: The terrain shader ([`terrain_material.wgsl`](../src/render/shaders/terrain_material.wgsl)) uses these dynamic uniforms to calculate diffuse and ambient lighting per-pixel.

### Visual Impact
- **Consistent Shadows**: The terrain's highlights and shading now perfectly match the direction of shadows cast by characters and buildings.
- **Day/Night Transitions**: As the sun sets, the terrain naturally transitions from bright daylight to warm evening tones and finally to cool, dark night-time lighting.
- **Atmospheric Integration**: By using the same ambient color as the rest of the scene, the terrain feels like a natural part of the environment rather than a separate layer.

## 5. Atmospheric Effects
- **Volumetric Fog**: `VolumetricFog{step_count: 64}` on the main camera (`src/lib.rs:2051-2055`; comment notes 128 was 2x cost). Fog volume itself lives in `src/render/zone_lighting.rs:191-198`.
- **Atmospheric Scattering**: Integrated Bevy's built-in atmospheric scattering for realistic sky rendering during the day (0.19: standalone `bevy_light::Atmosphere` entity + `AtmosphereSettings` on the camera, toggled by `toggle_atmosphere_based_on_time`).
- **Procedural Starry Sky**: A custom material that renders a dense star field and moon with phases, automatically toggled based on the night factor.

## 6. Post-Processing
- **Tonemapping**: Uses `TonyMcMapface` (`src/lib.rs:1990`) for a high-quality filmic look that preserves detail in both highlights and shadows.
- **Anti-Aliasing**: MSAA is `Off` by default (`src/lib.rs:1967`); SMAA is opt-in via `apply_smaa_system` (Disabled/Low/Medium/High/Ultra). TAA/SSR are not implemented.
- **Motion Blur**: Opt-in only via `apply_motion_blur_system`; not spawned by default (`src/lib.rs:1995-1999`).
