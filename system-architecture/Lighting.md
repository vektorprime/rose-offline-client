# Bevy Lighting Features in Rose Offline Client

Comprehensive documentation of Bevy 0.18.1 lighting features used in rose-offline-client, including directional lights, shadows, atmosphere, volumetric fog, and custom implementations.

## Table of Contents

1. [Overview](#overview)
2. [Directional Light](#directional-light)
3. [Shadow Systems](#shadow-systems)
4. [Volumetric Fog](#volumetric-fog)
5. [Atmosphere](#atmosphere)
6. [Environment Lighting](#environment-lighting)
7. [Distance Fog](#distance-fog)
8. [Custom Implementations](#custom-implementations)
9. [Day/Night Cycle](#daynight-cycle)
10. [Configuration Options](#configuration-options)
11. [Common Patterns](#common-patterns)
12. [Troubleshooting](#troubleshooting)
13. [Bevy API References](#bevy-api-references)
14. [Source File References](#source-file-references)

---

## Overview

This document covers all lighting-related features in the Rose Offline Client, including:

- **Directional lights** for sun/moon illumination
- **Cascaded shadow maps** for realistic shadows
- **Volumetric fog** for light shafts and god rays
- **Physical atmosphere** with Rayleigh/Mie scattering
- **Distance fog** for depth perception
- **Custom starry sky** with procedural stars and moon
- **Day/night cycle** with dynamic lighting transitions

### Bevy Version Notes

This documentation is for **Bevy 0.18.1**. Key changes from earlier versions:

| Feature | Bevy 0.14 | Bevy 0.18 |
|---------|-----------|-----------|
| Shadow field | `shadows_enabled` | `shadow_maps_enabled` |
| Ambient light | Resource only | Component + Resource |
| Light bundles | `DirectionalLightBundle` | Individual components |
| Fog volume | Bevy 0.15+ | Enhanced in 0.18 |

---

## Directional Light

The `DirectionalLight` component represents light sources infinitely far away, such as the sun or moon. Light shines along the forward direction of the entity's transform.

**Source:** `bevy-0.18.1/crates/bevy_light/src/directional_light.rs:73`

### Structure

```rust
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,           // Lux (lumens per square meter)
    pub shadow_maps_enabled: bool,  // Bevy 0.18: renamed from shadows_enabled
    pub contact_shadows_enabled: bool,
    #[cfg(feature = "experimental_pbr_pcss")]
    pub soft_shadow_size: Option<f32>,
    pub affects_lightmapped_mesh_diffuse: bool,
    pub shadow_depth_bias: f32,      // Default: 0.02
    pub shadow_normal_bias: f32,     // Default: 1.8
}
```

**Migration Note:** Bevy 0.18 renamed `shadows_enabled` to `shadow_maps_enabled` for consistency.

### Illuminance Values (Lux)

| Lux Value | Description |
|-----------|-------------|
| 0.0001 | Moonless, overcast night sky (starlight) |
| 0.05-0.3 | Full moon on a clear night |
| 3.4 | Dark limit of civil twilight |
| 50 | Family living room lights |
| 320-500 | Office lighting |
| 1,000 | Overcast day; TV studio lighting |
| 10,000 | Ambient daylight (Bevy default) |
| 100,000 | Direct sunlight |

### Usage in Rose Offline Client

```rust
// src/render/zone_lighting.rs:146
let light_entity = commands.spawn((
    DirectionalLight {
        illuminance: 15000.0,       // Balanced PBR lighting
        shadow_maps_enabled: true,  // REQUIRED for volumetric lighting
        ..Default::default()
    },
    default_light_transform(),
    CascadeShadowConfig {
        bounds: vec![20.0, 80.0, 300.0, 1000.0],
        overlap_proportion: 0.3,
        minimum_distance: 0.1,
    },
    VolumetricLight, // Enable light shafts
)).id();
```

### Sun Disk Rendering

The `SunDisk` component controls the visible solar disk in the sky (requires `Atmosphere` on camera):

```rust
pub struct SunDisk {
    pub angular_size: f32,  // Diameter in radians
    pub intensity: f32,     // Brightness multiplier
}

// Constants
pub const EARTH: SunDisk = SunDisk {
    angular_size: 0.00930842,  // ~32 arcminutes
    intensity: 1.0,
};

pub const OFF: SunDisk = SunDisk {
    angular_size: 0.0,
    intensity: 0.0,
};
```

---

## Shadow Systems

### DirectionalLightShadowMap

Controls shadow map resolution for directional and spot lights:

**Source:** `bevy-0.18.1/crates/bevy_light/src/directional_light.rs:193`

```rust
pub struct DirectionalLightShadowMap {
    pub size: usize,  // Must be power of two, default: 2048
}

// Usage
app.insert_resource(DirectionalLightShadowMap { size: 4096 });
```

### CascadeShadowConfig

Configures cascaded shadow maps (CSM) for directional lights:

**Source:** `bevy-0.18.1/crates/bevy_light/src/cascade.rs`

```rust
pub struct CascadeShadowConfig {
    pub bounds: Vec<f32>,        // Near/far bounds per cascade
    pub overlap_proportion: f32, // Cascade overlap (0.0-1.0)
    pub minimum_distance: f32,   // Minimum camera distance
}

// Rose Offline Client configuration
CascadeShadowConfig {
    bounds: vec![20.0, 80.0, 300.0, 1000.0],
    overlap_proportion: 0.3,
    minimum_distance: 0.1,
}
```

### ShadowFilteringMethod

Anti-aliasing method for shadow edges via PCF (Percentage Closer Filtering):

**Source:** `bevy-0.18.1/crates/bevy_light/src/lib.rs:275`

```rust
pub enum ShadowFilteringMethod {
    Hardware2x2,  // Fast, poor quality
    Gaussian,     // Default - 9 samples, smart filtering (The Witness technique)
    Temporal,     // 8 samples spiral, TAA-compatible (CoD: Advanced Warfare)
}
```

**Usage:** Add as component to camera:

```rust
commands.spawn((
    Camera3d::default(),
    ShadowFilteringMethod::Gaussian,
));
```

### Shadow Bias

Control shadow acne and Peter Panning:

- `shadow_depth_bias`: Tradeoff between self-shadowing artifacts and shadow proximity
- `shadow_normal_bias`: Bias along surface normal, scaled to texel size

---

## Volumetric Fog

Volumetric fog creates light shafts/god rays through raymarching in screen space.

**Source:** `bevy-0.18.1/crates/bevy_light/src/volumetric.rs`

### VolumetricFog Component

Add to camera to enable volumetric rendering:

```rust
pub struct VolumetricFog {
    pub ambient_color: Color,      // Default: WHITE
    pub ambient_intensity: f32,    // Default: 0.1
    pub jitter: f32,               // Random offset for TAA, default: 0.0
    pub step_count: u32,           // Raymarching steps, default: 64
}
```

### VolumetricLight Component

Add to directional lights with `shadows_enabled: true` to create light shafts:

```rust
// Marker component - must be paired with DirectionalLight
#[derive(Component)]
pub struct VolumetricLight;
```

### FogVolume Component

Defines a volumetric fog region (unit cube at origin, transformed via Transform):

```rust
pub struct FogVolume {
    pub fog_color: Color,                     // Default: WHITE
    pub density_factor: f32,                  // Default: 0.1
    pub density_texture: Option<Handle<Image>>,
    pub density_texture_offset: Vec3,
    pub absorption: f32,                      // Default: 0.3
    pub scattering: f32,                      // Default: 0.3
    pub scattering_asymmetry: f32,            // Default: 0.5
    pub light_tint: Color,                    // Default: WHITE
    pub light_intensity: f32,                 // Default: 1.0
}
```

### Rose Offline Client Implementation

**Source:** `src/render/zone_lighting.rs:193`

```rust
commands.spawn((
    FogVolume {
        fog_color: Color::srgb(0.85, 0.9, 1.0),  // Soft blue-white
        density_factor: 0.05,                     // Balanced visibility
        absorption: 0.1,                          // Depth perception
        scattering: 0.11,                         // Light shaft intensity
        scattering_asymmetry: 0.7,                // Forward-scattering (Mie)
        ..Default::default()
    },
    Transform::from_translation(Vec3::new(5120.0, 0.0, -5120.0))
        .with_scale(Vec3::splat(2000.0)),
    VolumetricFogVolume,
));
```

**Critical Notes:**
1. Fog volume must be positioned at game world center (5120, 0, -5120)
2. `DirectionalLight::shadow_maps_enabled` MUST be true for volumetric lighting
3. Scattering asymmetry > 0.5 creates forward-scattering effect (Mie scattering)

---

## Atmosphere

Physically-based atmospheric scattering implementing Hillaire's 2020 paper.

**Source:** `bevy-0.18.1/crates/bevy_light/src/atmosphere.rs`

### Atmosphere Component

**Source:** `bevy-0.18.1/crates/bevy_light/src/atmosphere.rs:24`

Add to `Camera3d` to enable atmospheric scattering:

```rust
pub struct Atmosphere {
    pub bottom_radius: f32,              // Planet radius (m)
    pub top_radius: f32,                 // Atmosphere top (m)
    pub ground_albedo: Vec3,             // Surface reflectance
    pub medium: Handle<ScatteringMedium>,
}
```

// Presets
Atmosphere::earth(medium_handle)  // Earth-like (R=6,360,000m, H=100,000m)
Atmosphere::mars(medium_handle)   // Martian (R=3,389,500m, H=120,000m)
```

### ScatteringMedium Asset

Defines how light scatters through the atmosphere:

```rust
pub struct ScatteringMedium {
    pub label: Option<Cow<'static, str>>,
    pub falloff_resolution: u32,   // Default: 256
    pub phase_resolution: u32,     // Default: 256
    pub terms: SmallVec<[ScatteringTerm; 1]>,
}

// Earth atmosphere preset
ScatteringMedium::earth(256, 256)
```

### ScatteringTerm

Individual scattering element (e.g., Rayleigh, Mie, ozone):

```rust
pub struct ScatteringTerm {
    pub absorption: Vec3,    // Light absorbed per meter (m^-1)
    pub scattering: Vec3,    // Light scattered per meter (m^-1)
    pub falloff: Falloff,    // Density distribution
    pub phase: PhaseFunction, // Scattering direction
}
```

#### Falloff Types

```rust
pub enum Falloff {
    Linear,                              // f(p) = p
    Exponential { scale: f32 },          // Scale height (e.g., 8km for Rayleigh)
    Tent { center: f32, width: f32 },    // Triangular peak (e.g., ozone layer)
    Curve(Arc<dyn Curve<f32>>),          // Custom distribution
}
```

#### PhaseFunction Types

```rust
pub enum PhaseFunction {
    Isotropic,                           // Even scattering
    Rayleigh,                            // Gas molecules (blue sky)
    Mie { asymmetry: f32 },              // Dust/aerosols (forward-scattering)
    Curve(Arc<dyn Curve<f32>>),          // Custom
    ChromaticCurve(Arc<dyn Curve<LinearRgba>>),  // Wavelength-dependent
    ChromaticTexture(Handle<Image>),     // Texture-based (N×1 RGBA)
}
```

### AtmosphereSettings

Controls LUT resolution and sampling quality:

**Source:** `bevy-0.18.1/crates/bevy_pbr/src/atmosphere/mod.rs:258`

```rust
pub struct AtmosphereSettings {
    pub transmittance_lut_size: UVec2,       // Default: (256, 128)
    pub transmittance_lut_samples: u32,      // Default: 40
    pub multiscattering_lut_size: UVec2,     // Default: (32, 32)
    pub multiscattering_lut_dirs: u32,       // Default: 64
    pub multiscattering_lut_samples: u32,    // Default: 20
    pub sky_view_lut_size: UVec2,            // Default: (400, 200)
    pub sky_view_lut_samples: u32,           // Default: 16
    pub aerial_view_lut_size: UVec3,         // Default: (32, 32, 32)
    pub aerial_view_lut_samples: u32,        // Default: 10
    pub aerial_view_lut_max_distance: f32,   // Default: 32,000m
    pub scene_units_to_m: f32,               // Default: 1.0
    pub sky_max_samples: u32,                // Default: 16
    pub rendering_method: AtmosphereMode,    // Default: LookupTexture
}
```

### AtmosphereMode

```rust
pub enum AtmosphereMode {
    LookupTexture,  // Fast, LUT-based (default)
    Raymarched,     // Accurate, raymarching-based
}
```

### AtmosphereEnvironmentMapLight

Generates environment map from atmosphere for PBR lighting:

```rust
pub struct AtmosphereEnvironmentMapLight {
    pub intensity: f32,                          // Default: 1.0
    pub affects_lightmapped_mesh_diffuse: bool,  // Default: true
    pub size: UVec2,                             // Default: (512, 512)
}
```

---

## Environment Lighting

### EnvironmentMapLight

HDR environment maps for PBR image-based lighting:

**Source:** `bevy-0.18.1/crates/bevy_light/src/probe.rs:104`

```rust
pub struct EnvironmentMapLight {
    pub diffuse_map: Handle<Image>,              // Blurry, for diffuse
    pub specular_map: Handle<Image>,             // Sharp/mipmapped, for specular
    pub intensity: f32,                          // cd/m² multiplier
    pub rotation: Quat,                          // World-space rotation
    pub affects_lightmapped_mesh_diffuse: bool,  // Default: true
}

// Convenience methods
EnvironmentMapLight::solid_color(assets, color)
EnvironmentMapLight::hemispherical_gradient(assets, top, mid, bottom)
```

### GeneratedEnvironmentMapLight

Runtime-filtered environment map:

```rust
pub struct GeneratedEnvironmentMapLight {
    pub environment_map: Handle<Image>,          // Source cubemap (power-of-two)
    pub intensity: f32,
    pub rotation: Quat,
    pub affects_lightmapped_mesh_diffuse: bool,
}
```

### Skybox

Visual sky rendering (does NOT affect lighting):

```rust
pub struct Skybox {
    pub image: Handle<Image>,
    pub brightness: f32,        // cd/m²
    pub rotation: Quat,
}
```

### LightProbe

Region-based global illumination:

```rust
pub struct LightProbe {
    pub falloff: Vec3,  // Falloff ratio per axis (0.0-1.0)
}
```

Pair with `EnvironmentMapLight` or `IrradianceVolume` for local GI.

---

## Distance Fog

Classic distance-based fog for PBR materials.

**Source:** `bevy-0.18.1/crates/bevy_pbr/src/fog.rs`

### DistanceFog Component

```rust
pub struct DistanceFog {
    pub color: Color,
    pub directional_light_color: Color,     // Glow effect (Color::NONE to disable)
    pub directional_light_exponent: f32,    // Default: 8.0
    pub falloff: FogFalloff,
}
```

### FogFalloff Modes

```rust
pub enum FogFalloff {
    Linear {
        start: f32,  // Transparent distance
        end: f32,    // Opaque distance
    },
    Exponential {
        density: f32,  // Higher = closer fog
    },
    ExponentialSquared {
        density: f32,  // Slower initial falloff
    },
    Atmospheric {
        extinction: Vec3,   // Per-channel light absorption
        inscattering: Vec3, // Per-channel light scattering
    },
}
```

#### Convenience Methods

```rust
// Exponential fog with object visible at 100 units
FogFalloff::from_visibility(100.0)

// Atmospheric fog with blue tint
FogFalloff::from_visibility_color(100.0, Color::srgb(0.5, 0.7, 1.0))

// Separate extinction/inscattering colors
FogFalloff::from_visibility_colors(
    100.0,
    Color::srgb(0.8, 0.8, 0.9),  // Extinction
    Color::srgb(0.5, 0.7, 1.0)   // Inscattering
)
```

---

## Custom Implementations

### StarrySkyMaterial

Procedural star field with moon rendering.

**Source:** `src/render/starry_sky_material.rs`

```rust
pub struct StarrySkyMaterial {
    pub time: f32,
    pub star_density: f32,      // 0.0-1.0
    pub star_brightness: f32,
    pub night_factor: f32,      // 0.0=day, 1.0=night
    pub moon_phase: f32,        // 0.0=new, 0.5=full, 1.0=new
    pub moon_direction: Vec3,
}
```

### StarrySkySettings Resource

```rust
pub struct StarrySkySettings {
    pub star_density: f32,
    pub star_brightness: f32,
    pub moon_phase: f32,
    pub moon_direction: Vec3,
    pub night_factor: f32,
}
```

### MoonLight Component

Marker component for moon directional light:

```rust
#[derive(Component)]
pub struct MoonLight;
```

### CloudLayer

TODO: Document cloud layer implementation when added.

---

## Day/Night Cycle

### ZoneLighting Resource

Central lighting configuration for zones:

**Source:** `src/render/zone_lighting.rs:533`

```rust
pub struct ZoneLighting {
    // Base lighting
    pub map_ambient_color: Vec3,
    pub character_ambient_color: Vec3,
    pub character_diffuse_color: Vec3,
    pub light_direction: Vec3,
    
    // Distance fog
    pub color_fog_enabled: bool,
    pub fog_color: Vec3,
    pub fog_density: f32,
    
    // Time of day
    pub time_of_day: f32,        // 0.0=night, 1.0=day
    pub day_color: Vec3,
    pub night_color: Vec3,
    
    // Volumetric fog
    pub volumetric_fog_enabled: bool,
    pub volumetric_fog_color: Vec3,
    pub volumetric_density_factor: f32,
    pub volumetric_absorption: f32,
    pub volumetric_scattering: f32,
    pub volumetric_scattering_asymmetry: f32,
}
```

### SkySettings Resource

Player-controllable sky settings:

```rust
pub struct SkySettings {
    pub mode: SkyMode,           // Automatic or Manual
    pub manual_time: f32,        // Hours (0-24)
    pub atmosphere_intensity: f32, // 0.0-2.0
}

pub enum SkyMode {
    Automatic,  // Follows ZoneTime
    Manual,     // User-controlled
}
```

### Sun Position System

Dynamic sun rotation based on time of day:

**Source:** `src/render/zone_lighting.rs:316`

```rust
// Time mapping (with +19 hour shift for extended daylight)
// 6:00  → Sunrise (horizon, east)
// 12:00 → Sun climbing
// 17:00 → Sun at highest point
// 18:00+ → Sunset
// 23:00+ → Night (sun below horizon)

let day_fract = (shifted_time / 24.0).clamp(0.0, 1.0);
transform.rotation = Quat::from_euler(
    EulerRot::ZYX,
    std::f32::consts::PI / 3.0,  // Earth tilt (60°)
    0.0,
    -day_fract * std::f32::consts::TAU,
);
```

### Shadow State by Time

| Time State | Sun Shadows | Moon Shadows |
|------------|-------------|--------------|
| Morning | Enabled | Disabled |
| Day | Enabled | Disabled |
| Evening | Disabled | Disabled |
| Night | Disabled | Disabled |

### Atmosphere Toggle System

Disables atmosphere at night to show stars:

```rust
let should_enable_atmosphere = match zone_time.state {
    ZoneTimeState::Night => false,   // Show stars
    ZoneTimeState::Evening => true,  // Transition
    ZoneTimeState::Morning => true,  // Transition
    ZoneTimeState::Day => true,      // Full atmosphere
};
```

### Night Factor Calculation

Smooth star visibility transition:

```rust
let night_factor = match zone_time.state {
    ZoneTimeState::Night => 1.0,
    ZoneTimeState::Evening => {
        if zone_time.state_percent_complete > 0.5 {
            (zone_time.state_percent_complete - 0.5) * 2.0
        } else {
            0.0
        }
    }
    ZoneTimeState::Morning => {
        if zone_time.state_percent_complete < 0.5 {
            1.0 - zone_time.state_percent_complete * 2.0
        } else {
            0.0
        }
    }
    ZoneTimeState::Day => 0.0,
};
```

---

## System Integration

### Zone Lighting Plugin

```rust
pub struct ZoneLightingPlugin;

impl Plugin for ZoneLightingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, ZONE_LIGHTING_SHADER_HANDLE, "shaders/zone_lighting.wgsl", Shader::from_wgsl);
        
        app.register_type::<ZoneLighting>()
            .init_resource::<ZoneLighting>()
            .init_resource::<SkySettings>();
        
        app.add_systems(Startup, spawn_lights)
            .add_systems(Update, (
                update_volumetric_fog_system,
                update_sun_position_system,
                apply_sky_settings_to_zone_time,
                sync_zone_lighting_to_bevy_lights_system,
                update_shadows_for_time_of_day_system,
            ));
    }
}
```

### System Order

1. `zone_time_system` - Updates game time
2. `apply_sky_settings_to_zone_time` - Applies manual time overrides
3. `update_sun_position_system` - Rotates sun based on time
4. `update_shadows_for_time_of_day_system` - Enables/disables shadows
5. `sync_zone_lighting_to_bevy_lights_system` - Syncs to Bevy lights
6. `update_volumetric_fog_system` - Updates fog parameters

---

## Configuration Options

### Shadow Quality Settings

Controlled via `GraphicsSettings` resource:

```rust
// src/graphics/apply_systems.rs:56
pub enum ShadowQuality {
    Off,      // No shadows
    Low,      // 1024x1024 shadow maps
    Medium,   // 2048x2048 shadow maps (default)
    High,     // 4096x4096 shadow maps
}
```

### Volumetric Fog Settings

Via `ZoneLighting` resource:

```rust
// src/render/zone_lighting.rs:533
pub struct ZoneLighting {
    pub volumetric_fog_enabled: bool,
    pub volumetric_fog_color: Vec3,
    pub volumetric_density_factor: f32,      // 0.0-1.0
    pub volumetric_absorption: f32,          // 0.0-1.0
    pub volumetric_scattering: f32,          // 0.0-1.0
    pub volumetric_scattering_asymmetry: f32, // 0.0-1.0
}
```

### Atmosphere Settings

Via `AtmosphereSettings` resource:

```rust
// bevy-0.18.1/crates/bevy_pbr/src/atmosphere/mod.rs:258
pub struct AtmosphereSettings {
    pub transmittance_lut_size: UVec2,       // Default: (256, 128)
    pub multiscattering_lut_size: UVec2,     // Default: (32, 32)
    pub sky_view_lut_size: UVec2,            // Default: (400, 200)
    pub rendering_method: AtmosphereMode,    // LookupTexture or Raymarched
}
```

---

## Common Patterns

### Creating a Sun Light with Shadows

```rust
// src/render/zone_lighting.rs:146
commands.spawn((
    DirectionalLight {
        color: Color::srgb(1.0, 0.95, 0.9),  // Warm sunlight
        illuminance: 15000.0,
        shadow_maps_enabled: true,
        ..Default::default()
    },
    Transform::from_rotation(Quat::from_euler(
        EulerRot::ZYX,
        std::f32::consts::PI / 6.0,  // 30° elevation
        0.0,
        0.0,
    )),
    CascadeShadowConfig {
        bounds: vec![20.0, 80.0, 300.0, 1000.0],
        overlap_proportion: 0.3,
        minimum_distance: 0.1,
    },
    VolumetricLight,
    SunDisk::EARTH,
));
```

### Creating a Moon Light

```rust
// src/lib.rs:2320
commands.spawn((
    DirectionalLight {
        color: Color::srgb(0.8, 0.85, 0.95),  // Blue-white moonlight
        illuminance: 5000.0,
        shadow_maps_enabled: true,
        shadow_depth_bias: 0.0,
        shadow_normal_bias: 0.0,
        ..Default::default()
    },
    MoonLight,  // Marker component
));
```

### Enabling Volumetric Fog on Camera

```rust
commands.spawn((
    Camera3d::default(),
    VolumetricFog {
        ambient_color: Color::WHITE,
        ambient_intensity: 0.1,
        step_count: 64,
        ..Default::default()
    },
));
```

### Setting Up Atmosphere

```rust
let earth_medium = assets.add(ScatteringMedium::earth(256, 256));

commands.spawn((
    Camera3d::default(),
    Atmosphere::earth(earth_medium.clone()),
    AtmosphereEnvironmentMapLight::default(),
));
```

---

## Troubleshooting

### Bevy 0.18 Migration Issues

#### Issue: `shadows_enabled` field not found

**Symptom:**
```
error[E0560]: struct `DirectionalLight` has no field named `shadows_enabled`
```

**Cause:** Bevy 0.18 renamed `shadows_enabled` to `shadow_maps_enabled`.

**Solution:**
```rust
// Before (Bevy 0.14-0.17)
DirectionalLight {
    shadows_enabled: true,
    ..Default::default()
}

// After (Bevy 0.18+)
DirectionalLight {
    shadow_maps_enabled: true,
    ..Default::default()
}
```

**Source:** `bevy-0.18.1/crates/bevy_light/src/directional_light.rs:93`

---

#### Issue: Volumetric fog not visible

**Symptoms:**
- No light shafts visible
- Fog volume appears as black box
- Volumetric lighting not rendering

**Causes and Solutions:**

1. **Shadow maps not enabled**
   ```rust
   // Ensure shadow_maps_enabled is true
   DirectionalLight {
       shadow_maps_enabled: true,  // MUST be true
       ..Default::default()
   }
   ```

2. **VolumetricLight component missing**
   ```rust
   // Add VolumetricLight marker component
   commands.spawn((
       DirectionalLight { .. },
       VolumetricLight,  // Required
   ));
   ```

3. **Fog volume at wrong position**
   ```rust
   // Position fog volume at game world center
   Transform::from_translation(Vec3::new(5120.0, 0.0, -5120.0))
       .with_scale(Vec3::splat(2000.0))
   ```

4. **VolumetricFog not on camera**
   ```rust
   commands.spawn((
       Camera3d::default(),
       VolumetricFog::default(),  // Required on camera
   ));
   ```

**Source:** `src/render/zone_lighting.rs:146-204`

---

#### Issue: Stars not visible at night

**Symptoms:**
- Black sky at night instead of stars
- Atmosphere blocking star visibility

**Solutions:**

1. **Remove Atmosphere component at night**
   ```rust
   // src/render/starry_sky_material.rs
   let should_enable_atmosphere = match zone_time.state {
       ZoneTimeState::Night => false,
       _ => true,
   };
   ```

2. **Update night_factor in StarrySkySettings**
   ```rust
   starry_sky_settings.night_factor = match zone_time.state {
       ZoneTimeState::Night => 1.0,
       ZoneTimeState::Day => 0.0,
       _ => /* interpolate */,
   };
   ```

3. **Ensure starry sky sphere at world origin**
   ```rust
   // Shader uses normalize(world_position), sphere must be at origin
   Transform::from_translation(Vec3::ZERO).with_scale(Vec3::splat(10000.0))
   ```

**Source:** `src/render/starry_sky_material.rs`

---

#### Issue: Shadow acne or Peter Panning

**Symptoms:**
- Shadow acne: dark speckles on surfaces that should be lit
- Peter Panning: shadows detached from objects

**Solution:**
```rust
DirectionalLight {
    shadow_depth_bias: 0.02,   // Increase for acne, decrease for panning
    shadow_normal_bias: 1.8,   // Adjust along surface normal
    ..Default::default()
}
```

**Tuning:**
- Shadow acne → increase `shadow_depth_bias`
- Peter Panning → decrease `shadow_depth_bias`
- Still issues → adjust `shadow_normal_bias`

**Source:** `bevy-0.18.1/crates/bevy_light/src/directional_light.rs:139-144`

---

#### Issue: Cascade shadow map banding

**Symptoms:**
- Visible seams between shadow cascades
- Pop-in when moving camera

**Solutions:**

1. **Increase cascade overlap**
   ```rust
   CascadeShadowConfig {
       overlap_proportion: 0.3,  // Higher = more overlap, smoother transitions
       ..Default::default()
   }
   ```

2. **Adjust cascade bounds**
   ```rust
   CascadeShadowConfig {
       bounds: vec![20.0, 80.0, 300.0, 1000.0],  // Tighter = better quality
       ..Default::default()
   }
   ```

3. **Enable shadow filtering**
   ```rust
   commands.spawn((
       Camera3d::default(),
       ShadowFilteringMethod::Gaussian,  // or Temporal with TAA
   ));
   ```

**Source:** `src/render/zone_lighting.rs:153-157`

---

#### Issue: Atmosphere performance issues

**Symptoms:**
- Frame time spikes when atmosphere enabled
- High GPU memory usage

**Solutions:**

1. **Reduce LUT resolutions**
   ```rust
   app.insert_resource(AtmosphereSettings {
       transmittance_lut_size: UVec2::new(128, 64),   // From (256, 128)
       multiscattering_lut_size: UVec2::new(16, 16),  // From (32, 32)
       sky_view_lut_size: UVec2::new(200, 100),       // From (400, 200)
       ..Default::default()
   });
   ```

2. **Use lookup texture mode**
   ```rust
   app.insert_resource(AtmosphereSettings {
       rendering_method: AtmosphereMode::LookupTexture,  // Faster than Raymarched
       ..Default::default()
   });
   ```

3. **Disable atmosphere at night**
   ```rust
   // Show stars instead of atmosphere during night
   if zone_time.state == ZoneTimeState::Night {
       camera_query.remove::<Atmosphere>(camera_entity);
   }
   ```

**Source:** `bevy-0.18.1/crates/bevy_pbr/src/atmosphere/mod.rs:258`

---

#### Issue: Atmosphere colors too saturated

**Symptoms:**
- Sky too orange/blue
- Unrealistic scattering colors

**Solution:**
```rust
// Adjust atmosphere intensity
sky_settings.atmosphere_intensity = 0.5;  // Reduce from 1.0

// Or adjust ground albedo
Atmosphere {
    ground_albedo: Vec3::splat(0.1),  // Darker ground = less bounce
    ..
}
```

---

#### Issue: Volumetric fog too dense/performance

**Symptoms:**
- Light shafts too opaque
- Performance degradation

**Solutions:**

1. **Reduce density**
   ```rust
   FogVolume {
       density_factor: 0.05,  // Lower = more transparent
       ..Default::default()
   }
   ```

2. **Reduce step count**
   ```rust
   VolumetricFog {
       step_count: 32,  // From 64, lower = faster but less accurate
       ..Default::default()
   }
   ```

3. **Adjust scattering**
   ```rust
   FogVolume {
       scattering: 0.11,           // Light shaft intensity
       scattering_asymmetry: 0.7,  // Forward-scattering (0.5-1.0)
       ..Default::default()
   }
   ```

**Source:** `src/render/zone_lighting.rs:193-204`

---

#### Issue: Sun disk not visible

**Symptoms:**
- No visible sun in sky
- Atmosphere scattering works but no disk

**Solutions:**

1. **Add SunDisk component**
   ```rust
   commands.spawn((
       DirectionalLight { .. },
       SunDisk::EARTH,  // Required for visible sun
   ));
   ```

2. **Enable bloom**
   ```rust
   commands.spawn((
       Camera3d::default(),
       Bloom::default(),  // Required for sun glow effect
   ));
   ```

3. **Ensure Atmosphere is enabled**
   ```rust
   // SunDisk requires Atmosphere on camera
   commands.spawn((
       Camera3d::default(),
       Atmosphere::earth(medium_handle),
   ));
   ```

**Source:** `bevy-0.18.1/crates/bevy_light/src/directional_light.rs:267`

---

## Bevy API References

| Component/Resource | Source File | Description |
|-------------------|-------------|-------------|
| `DirectionalLight` | `bevy_light/src/directional_light.rs:73` | Sun/moon light source |
| `DirectionalLightShadowMap` | `bevy_light/src/directional_light.rs:193` | Shadow map resolution |
| `CascadeShadowConfig` | `bevy_light/src/cascade.rs` | CSM configuration |
| `ShadowFilteringMethod` | `bevy_light/src/lib.rs:275` | Shadow anti-aliasing |
| `VolumetricFog` | `bevy_light/src/volumetric.rs` | Camera volumetric settings |
| `VolumetricLight` | `bevy_light/src/volumetric.rs` | Light shafts marker |
| `FogVolume` | `bevy_light/src/volumetric.rs` | Volumetric fog region |
| `Atmosphere` | `bevy_light/src/atmosphere.rs:171` | Atmospheric scattering |
| `ScatteringMedium` | `bevy_light/src/atmosphere.rs:110` | Scattering properties |
| `AtmosphereSettings` | `bevy_pbr/src/atmosphere/mod.rs:258` | LUT quality settings |
| `AtmosphereEnvironmentMapLight` | `bevy_light/src/probe.rs` | Atmosphere-based IBL |
| `EnvironmentMapLight` | `bevy_light/src/probe.rs:104` | HDR environment lighting |
| `DistanceFog` | `bevy_pbr/src/fog.rs` | Classic distance fog |
| `SunDisk` | `bevy_light/src/directional_light.rs:267` | Visible solar disk |
| `GlobalAmbientLight` | `bevy_light/src/ambient_light.rs` | Global ambient resource |
| `AmbientLight` | `bevy_light/src/ambient_light.rs` | Ambient light component |

---

## Source File References

### Bevy Source Files (bevy-0.18.1)

| Path | Description |
|------|-------------|
| `crates/bevy_light/src/directional_light.rs` | DirectionalLight, SunDisk, shadow maps |
| `crates/bevy_light/src/cascade.rs` | CascadeShadowConfig, CSM logic |
| `crates/bevy_light/src/volumetric.rs` | VolumetricFog, VolumetricLight, FogVolume |
| `crates/bevy_light/src/atmosphere.rs` | Atmosphere, ScatteringMedium, scattering terms |
| `crates/bevy_light/src/probe.rs` | EnvironmentMapLight, LightProbe, Skybox |
| `crates/bevy_light/src/ambient_light.rs` | AmbientLight, GlobalAmbientLight |
| `crates/bevy_light/src/lib.rs` | ShadowFilteringMethod, LightPlugin |
| `crates/bevy_pbr/src/fog.rs` | DistanceFog, FogFalloff |
| `crates/bevy_pbr/src/atmosphere/mod.rs` | AtmosphereSettings, LUT generation |
| `crates/bevy_pbr/src/atmosphere/environment.rs` | AtmosphereEnvironmentMapLight |
| `crates/bevy_pbr/src/volumetric_fog/mod.rs` | VolumetricFogPlugin |
| `crates/bevy_pbr/src/volumetric_fog/render.rs` | Volumetric rendering pipeline |

### Project Source Files

| Path | Description |
|------|-------------|
| `src/render/zone_lighting.rs` | ZoneLighting resource, sun/moon lights, fog volume |
| `src/render/starry_sky_material.rs` | Procedural star field, moon rendering |
| `src/render/cloud_material.rs` | Cloud layer rendering |
| `src/resources/zone_time.rs` | ZoneTime resource, day/night state |
| `src/graphics/apply_systems.rs` | Shadow quality settings application |
