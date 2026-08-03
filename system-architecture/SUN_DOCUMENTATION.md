# Sun Documentation - Rose Online Rust Client

## Overview

The sun in this Rose Online implementation is represented by a **DirectionalLight** that simulates sunlight. The sun's position and color are dynamically updated by the **time-of-day system**, which synchronizes the **ZoneLighting** resource with Bevy's built-in lights to ensure a consistent and high-quality visual experience across all world elements.

---

## Sun Start Position and Configuration

### Initial Light Direction

The sun's directional light is spawned in `zone_lighting.rs` using `default_light_transform()`:

```rust
fn default_light_transform() -> Transform {
    Transform::from_rotation(Quat::from_euler(
        EulerRot::ZYX,
        0.0,                                // Z-axis rotation (none)
        std::f32::consts::PI * (2.0 / 3.0), // Y-axis rotation: ~120°
        -std::f32::consts::PI / 4.0,        // X-axis rotation: -45°
    ))
}
```

This results in a light direction of approximately:
- **Direction**: Pointing toward the sky at a **45° angle upward** and **120° from forward** (southeast direction in Bevy's coordinate system)
- **Illuminance**: 15,000 lux (balanced for PBR, reduced from 50,000)
- **Shadows**: Enabled (required for volumetric lighting)

Note: this rotation is only the spawn-time default. At runtime `update_sun_position_system` (same file) overrides the `DirectionalLight` transform each time the time of day changes, and the default `ZoneLighting.light_direction` is derived from `default_light_transform().back()`.

### Light Spawn Location

The directional light is spawned during `Startup` with:

```rust
commands.spawn((
    DirectionalLight {
        illuminance: 15000.0,
        shadows_enabled: true,
        ..Default::default()
    },
    default_light_transform(),
    CascadeShadowConfig {
        bounds: vec![20.0, 80.0, 300.0, 1000.0],
        overlap_proportion: 0.3,
        minimum_distance: 0.1,
    },
    RenderLayers::default(),
    VolumetricLight,  // Enables light shafts/god rays
)).id();
```

---

## How the Sun "Moves" (Time-of-Day System)

The sun's apparent movement is driven by two cooperating systems:

- **`update_sun_position_system`** (`src/render/zone_lighting.rs`): Rotates the `DirectionalLight` transform along an arc based on the time of day. In `SkyMode::Automatic` it converts `ZoneTime.time` (ticks) to hours using the zone's `day_cycle`, then sets the rotation from a day fraction (with a +19h shift so the sun is up from ~6:00 to ~23:00); in `SkyMode::Manual` it uses `SkySettings.manual_time` instead.
- **`zone_time_system`** (`systems/zone_time_system.rs`): Tracks the four lighting states (Morning/Day/Evening/Night) and continuously updates the `ZoneLighting` resource (fog colors, densities, skybox-based ambient/diffuse colors), while **`update_shadows_for_time_of_day_system`** adjusts the sun's illuminance and shadows per state.

### Day Cycle Structure

Each zone data entry provides a day cycle in ticks:
- `day_cycle`: Total cycle duration in ticks (default 160 = 24 hours)
- `morning_time` / `day_time` / `evening_time` / `night_time`: Zone data tick thresholds, used for tick conversions and debug logging

Note: state determination does NOT use the zone data thresholds. `zone_time_system` derives the state from FIXED hour thresholds:
- **Morning**: 6:00–12:00
- **Day**: 12:00–17:00
- **Evening**: 17:00–19:00
- **Night**: 19:00–6:00 (wraps around midnight)

### Four Time States

The sun simulation has four distinct states:

| State | Time Range | Light Color | Fog Color | Volumetric Fog |
|-------|-----------|-------------|-----------|----------------|
| **Morning** | 6:00 → 12:00 | Warm orange→white | Dark gray→white | Dark blue→orange→light blue |
| **Day** | 12:00 → 17:00 | White-blue | White | Light blue |
| **Evening** | 17:00 → 19:00 | White→orange | White→dark gray | Light blue→pink→dark blue |
| **Night** | 19:00 → 6:00 | Dark blue | Dark gray | Dark blue |

### Transition Logic

Time transitions are handled with **smooth interpolation**:

```rust
// Example: Day to Evening transition
if zone_time.state_percent_complete < 0.5 {
    // First half: Day → Evening
    zone_lighting.volumetric_fog_color = VOLUMETRIC_DAY_COLOR.lerp(
        VOLUMETRIC_EVENING_COLOR,
        zone_time.state_percent_complete * 2.0,
    );
} else {
    // Second half: Evening → Night
    zone_lighting.volumetric_fog_color = VOLUMETRIC_EVENING_COLOR.lerp(
        VOLUMETRIC_NIGHT_COLOR,
        (zone_time.state_percent_complete - 0.5) * 2.0,
    );
}
```

### Key Parameters Updated

The `zone_time_system` updates these ZoneLighting resource fields:

| Parameter | Day | Night | Interpolation |
|-----------|-----|-------|---------------|
| `volumetric_fog_color` | `Vec3(0.9, 0.95, 1.0)` | `Vec3(0.3, 0.35, 0.5)` | Linear |
| `volumetric_density_factor` | `0.05` | `0.03` | Linear |
| `map_ambient_color` | Skybox Day | Skybox Night | Skybox state lerp |
| `character_ambient_color` | Skybox Day | Skybox Night | Skybox state lerp |
| `character_diffuse_color` | Skybox Day | Skybox Night | Skybox state lerp |
| `fog_color` | `Vec3(200/255, 200/255, 200/255)` | `Vec3(10/255, 10/255, 10/255)` | Linear |
| `fog_density` | `0.0018` | `0.0020` | Linear |

---

## Volumetric Fog (Light Shafts)

The sun's light shafts are implemented via **volumetric fog**:

```rust
commands.spawn((
    FogVolume {
        fog_color,          // Time-of-day dependent color
        density_factor,     // Time-of-day dependent density
        absorption: zone_lighting.volumetric_absorption,                          // 0.1 default
        scattering: zone_lighting.volumetric_scattering,                          // 0.11 default
        scattering_asymmetry: zone_lighting.volumetric_scattering_asymmetry,      // 0.7 default
        ..Default::default()
    },
    Transform::from_translation(Vec3::new(5120.0, 0.0, -5120.0))
        .with_scale(Vec3::splat(2000.0)),
    VolumetricFogVolume,
));
```

The volumetric fog is **not attached to the sun** - it's a world-space volume that captures the directional light, creating visible light shafts/god rays. The `VolumetricLight` component is always present on the sun, but the fog volume's `density_factor` starts at 0.0 unless `ZoneLighting.volumetric_fog_enabled` is set (Settings UI checkbox, disabled by default).

---

## Light Synchronization System

A dedicated system, `sync_zone_lighting_to_bevy_lights_system` in `src/render/zone_lighting.rs`, ensures that the game's two lighting systems remain perfectly in sync:

1.  **Ambient Light**: Writes `zone_lighting.map_ambient_color` (multiplied by the user's `GraphicsSettings` ambient color, with brightness scaled from Bevy's base of 80.0) into the `GlobalAmbientLight` resource. In Bevy 0.18, `AmbientLight` is a component; `GlobalAmbientLight` is the resource form used here.
2.  **Directional Light Color**: Synchronizes `zone_lighting.character_diffuse_color` with the `DirectionalLight` component.
3.  **Light Direction**: Automatically updates `zone_lighting.light_direction` from the actual sun position (transform) in the sky.

This synchronization is critical for:
- **Terrain Lighting**: Allows the custom terrain shader to receive dynamic light direction and color updates.
- **Water Reflections**: Ensures specular highlights on water match the sun's position.
- **PBR Consistency**: Guarantees that characters and world objects are lit identically to the terrain.

---

## Shadow Mapping

The directional light uses **Cascaded Shadow Maps (CSM)** with 4 cascades:

```rust
CascadeShadowConfig {
    bounds: vec![20.0, 80.0, 300.0, 1000.0],  // Cascade distances
    overlap_proportion: 0.3,   // Smooth transition between cascades
    minimum_distance: 0.1,
}
```

`directional_light_system.rs` (`src/systems/directional_light_system.rs`) tracks the player position (falling back to the main camera) and constructs shadow view-projection matrices:

```rust
// directional_light_system.rs
pub fn directional_light_system(
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    query_light: Query<&GlobalTransform, With<DirectionalLight>>,
    // ... constructs shadow view-projection matrices
)
```

Note: the cascade shadow maps themselves are built automatically by Bevy's built-in CSM systems. The matrices constructed in this system are currently discarded (the code comments note that manual cascade management is no longer supported since Bevy 0.13).

---

## Summary

| Aspect | Configuration |
|--------|--------------|
| **Sun Type** | DirectionalLight (rotation driven by time of day) |
| **Initial Direction** | ~45° upward, 120° from forward (southeast) |
| **Illuminance** | 15,000 lux by day; 0 lux during Evening/Night |
| **Shadows** | Enabled with 4-cascade CSM |
| **Volumetric Light** | Enabled (light shafts) |
| **"Movement"** | Rotated by `update_sun_position_system` from ZoneTime / SkySettings |
| **Day Cycle** | 4 states: Morning → Day → Evening → Night |
| **Transition** | Smooth linear interpolation over time |

The sun's apparent movement is created by:
1. Rotating the `DirectionalLight` transform along an arc via `update_sun_position_system` (driven by `ZoneTime`, or `SkySettings.manual_time` in manual mode)
2. Fog colors (bright blue for day, dark blue for night, warm for dawn/dusk)
3. Fog density (subtle for day, slightly thicker at night)
4. Ambient and diffuse lighting colors (from skybox data)
5. Volumetric fog color and density
6. Setting sun illuminance to 0 and disabling shadows during Evening/Night (`update_shadows_for_time_of_day_system`)

The resulting light transform is synced into `ZoneLighting.light_direction` by `sync_zone_lighting_to_bevy_lights_system`, so terrain, water reflections, and PBR materials all share the same moving sun.