# Sun Documentation - Rose Online Rust Client

## Overview

The sun in this Rose Online implementation is represented by a **DirectionalLight** that simulates sunlight. The sun does **not move** in 3D space - instead, the game uses a **static directional light** with a fixed angle, and the time-of-day system changes **lighting colors, fog properties, and skybox textures** to simulate the sun's movement through the sky.

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
        bounds: vec![50.0, 150.0, 500.0, 2000.0],
        overlap_proportion: 0.3,
        minimum_distance: 0.1,
    },
    RenderLayers::default(),
    VolumetricLight,  // Enables light shafts/god rays
)).id();
```

---

## How the Sun "Moves" (Time-of-Day System)

The sun doesn't actually move - instead, the **zone_time_system** (`systems/zone_time_system.rs`) simulates the sun's movement by changing lighting parameters throughout the day cycle.

### Day Cycle Structure

Each zone has a day cycle defined by:
- `morning_time`: Start of morning transition
- `day_time`: Start of full day
- `evening_time`: Start of evening transition
- `night_time`: Start of night
- `day_cycle`: Total cycle duration in ticks

### Four Time States

The sun simulation has four distinct states:

| State | Time Range | Light Color | Fog Color | Volumetric Fog |
|-------|-----------|-------------|-----------|----------------|
| **Morning** | `morning_time` → `day_time` | Warm orange→white | Gray→white | Orange→light blue |
| **Day** | `day_time` → `evening_time` | White-blue | White | Light blue |
| **Evening** | `evening_time` → `night_time` | White→orange | White→gray | Blue→orange |
| **Night** | `night_time` → `morning_time` | Dark blue | Dark gray | Dark blue |

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
| `volumetric_density_factor` | `0.02` | `0.01` | Linear |
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
        absorption: 0.01,   // Minimal for bright scene
        scattering: 0.3,    // Standard for light shafts
        scattering_asymmetry: 0.5,
    },
    Transform::from_translation(Vec3::new(5120.0, 0.0, -5120.0))
        .with_scale(Vec3::splat(2000.0)),
    VolumetricFogVolume,
));
```

The volumetric fog is **not attached to the sun** - it's a world-space volume that captures the directional light, creating visible light shafts/god rays.

---

## Light Direction in Shaders

The light direction is passed to shaders via the `ZoneLighting` resource:

```rust
// zone_lighting.rs
pub struct ZoneLightingUniformData {
    pub light_direction: Vec4,  // Normalized direction vector
    // ...
}
```

In WGSL shaders, this is used for:
- **Water material specular calculations**: `light_direction: Vec3(0.3, -0.8, 0.5)`
- **PBR lighting calculations**: Directional light direction from the transform

---

## Shadow Mapping

The directional light uses **Cascaded Shadow Maps (CSM)** with 4 cascades:

```rust
CascadeShadowConfig {
    bounds: vec![50.0, 150.0, 500.0, 2000.0],  // Cascade distances
    overlap_proportion: 0.3,   // Smooth transition between cascades
    minimum_distance: 0.1,
}
```

The shadow frustum is dynamically updated in `directional_light_system.rs` to track the player:

```rust
// directional_light_system.rs
pub fn directional_light_system(
    query_player: Query<&GlobalTransform, With<PlayerCharacter>>,
    query_light: Query<&GlobalTransform, With<DirectionalLight>>,
    // ... constructs shadow view-projection matrices
)
```

---

## Summary

| Aspect | Configuration |
|--------|--------------|
| **Sun Type** | DirectionalLight (static, not moving) |
| **Initial Direction** | ~45° upward, 120° from forward (southeast) |
| **Illuminance** | 15,000 lux |
| **Shadows** | Enabled with 4-cascade CSM |
| **Volumetric Light** | Enabled (light shafts) |
| **"Movement"** | Simulated via time-of-day lighting transitions |
| **Day Cycle** | 4 states: Morning → Day → Evening → Night |
| **Transition** | Smooth linear interpolation over time |

The sun's apparent movement is an illusion created by changing:
1. Fog colors (bright blue for day, dark blue for night, warm for dawn/dusk)
2. Fog density (subtle for day, slightly thicker at night)
3. Ambient and diffuse lighting colors (from skybox data)
4. Volumetric fog color and density

The actual light direction remains fixed to provide consistent illumination direction throughout the day cycle.