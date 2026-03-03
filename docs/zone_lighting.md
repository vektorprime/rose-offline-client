# Zone Lighting Documentation

This document describes the high-quality graphics and lighting improvements implemented for the Bevy 0.16.1 client.

## 1. High-Resolution Shadows
The shadow mapping system has been significantly upgraded to provide sharp, detailed shadows across the game world.

- **Shadow Map Resolution**: Increased to **4096** (from default) in `src/lib.rs`.
- **Cascaded Shadow Maps (CSM)**: Fine-tuned the cascade bounds in `src/render/zone_lighting.rs` to `[20.0, 80.0, 300.0, 1000.0]`. This ensures maximum resolution for objects near the player while maintaining coverage for distant terrain.
- **Shadow Filtering**: Uses `ShadowFilteringMethod::Gaussian` for stable, high-quality soft edges.

## 2. Advanced Lighting & Reflections
Modern PBR features have been integrated to increase visual richness and material depth.

- **Environment Map Light**: Added an `EnvironmentMapLight` to the main camera. This uses a custom-loaded Cubemap (derived from `SPECULAR_SPHEREMAP.DDS`) to provide realistic reflections and irradiance to all PBR materials.
- **Ultra SSAO**: Screen Space Ambient Occlusion has been set to **Ultra** quality, providing deep contact shadows in crevices and where objects meet the ground.
- **Bloom**: Re-enabled the natural bloom effect to enhance HDR highlights and light-emitting materials.

## 3. Synchronized Time-of-Day System
The lighting system has been fully synchronized to ensure all world elements, including custom shaders, react consistently to the day/night cycle.

- **Light Synchronization**: A new system `sync_zone_lighting_to_bevy_lights_system` in `src/render/zone_lighting.rs` bridges the `ZoneLighting` resource with Bevy's built-in `AmbientLight` and `DirectionalLight`.
- **Balanced Intensities**:
    - `DirectionalLight` illuminance: **15,000 lux** (balanced for PBR).
    - `AmbientLight` brightness: **350.0 lux** (balanced with EnvironmentMapLight).

## 3. Dynamic Terrain Lighting & Sun Synchronization
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

## 4. Atmospheric Effects
- **High-Quality Volumetric Fog**: Increased the step count to **128** for much smoother light shafts (god rays) with minimal sampling artifacts.
- **Atmospheric Scattering**: Integrated Bevy 0.16's built-in atmospheric scattering for realistic sky rendering during the day.
- **Procedural Starry Sky**: A custom material that renders a dense star field and moon with phases, automatically toggled based on the night factor.

## 5. Post-Processing
- **Tonemapping**: Uses `TonyMcMapface` for a high-quality filmic look that preserves detail in both highlights and shadows.
- **Anti-Aliasing**: Combines **SMAA** with **Temporal Anti-Aliasing (TAA)** (when enabled) for superior edge smoothing and temporal stability.
- **Motion Blur**: Enabled for smoother visual transitions during fast movement.
