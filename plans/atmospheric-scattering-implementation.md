# Atmospheric Scattering Implementation Plan

## Overview

This document outlines the plan to replace the existing `CartoonSkyMaterial` system with Bevy 0.16's built-in `Atmosphere` system for high-quality procedural atmospheric scattering.

### Key Finding

**Bevy 0.16 includes a production-ready atmospheric scattering implementation** in `bevy::pbr::{Atmosphere, AtmosphereSettings}`. This is based on [Hillaire's 2020 paper](https://sebh.github.io/publications/egsr2020.pdf) and provides:

- Rayleigh and Mie scattering calculations
- Compute shader-based LUT generation (Transmittance, Multiscattering, Sky View, Aerial View)
- Integration with directional lights for dynamic time-of-day
- Aerial perspective/fog for distant terrain
- HDR-compatible rendering

**Recommendation**: Use the built-in Bevy 0.16 Atmosphere system rather than implementing a custom solution.

---

## Current Implementation Analysis

### CartoonSkyMaterial (`src/render/cartoon_sky_material.rs`)

The current system uses a custom material approach:

```
CartoonSkyMaterial
├── MaterialPlugin<CartoonSkyMaterial>
├── Shader: shaders/cartoon_sky.wgsl
├── Sky dome mesh (sphere viewed from inside)
└── Features:
    ├── Procedural gradient sky
    ├── Stylized sun/moon discs
    ├── Animated procedural clouds (FBM noise)
    ├── Stars at night
    └── ZoneTime integration for day/night cycle
```

**Limitations**:
- Stylized/cartoon look (not realistic)
- No atmospheric scattering through terrain
- No aerial perspective fog
- Manual color blending instead of physics-based

### Camera Setup (`src/lib.rs` lines 1753-1845)

The camera already has required components:
- `Camera { hdr: true, ... }` - HDR enabled
- `Tonemapping::TonyMcMapface` - Tone mapping
- `Bloom::NATURAL` - Bloom effect
- `ColorGrading` - Color correction

---

## Bevy 0.16 Atmosphere System

### Architecture

```
AtmospherePlugin (built into Bevy)
├── Components:
│   ├── Atmosphere - Physical atmosphere parameters
│   │   ├── bottom_radius (planet radius)
│   │   ├── top_radius (atmosphere height)
│   │   ├── rayleigh_scattering (wavelength-dependent)
│   │   ├── mie_scattering, mie_absorption, mie_asymmetry
│   │   └── ozone_absorption
│   └── AtmosphereSettings - LUT configuration
│       ├── transmittance_lut_size
│       ├── sky_view_lut_size
│       ├── aerial_view_lut_size
│       └── scene_units_to_m (scale factor)
│
├── Render Graph Nodes:
│   ├── AtmosphereLutsNode (after prepasses)
│   │   └── Computes transmittance, multiscattering LUTs
│   └── RenderSkyNode (between opaque/transparent passes)
│       └── Renders sky and applies aerial perspective
│
└── Compute Shaders:
    ├── transmittance_lut.wgsl
    ├── multiscattering_lut.wgsl
    ├── sky_view_lut.wgsl
    ├── aerial_view_lut.wgsl
    └── render_sky.wgsl
```

### Presets

```rust
// Earth-like atmosphere (default)
Atmosphere::EARTH

// Customizable
Atmosphere {
    bottom_radius: 6_360_000.0,  // meters
    top_radius: 6_460_000.0,     // meters
    rayleigh_scattering: Vec3::new(5.802e-6, 13.558e-6, 33.100e-6),
    mie_scattering: 3.996e-6,
    mie_absorption: 0.444e-6,
    mie_asymmetry: 0.8,
    // ... more parameters
}
```

---

## Implementation Plan

### Phase 1: Add Atmosphere Component to Camera

**File**: `src/lib.rs`

Add `Atmosphere` and `AtmosphereSettings` to the camera spawn:

```rust
// In load_common_game_data function, add to camera entity:
commands.spawn((
    Camera3d::default(),
    Camera { hdr: true, ... },
    // ... existing components ...
    
    // ADD: Atmospheric scattering
    bevy::pbr::Atmosphere::EARTH,
    bevy::pbr::AtmosphereSettings {
        scene_units_to_m: 1.0,  // 1 scene unit = 1 meter
        aerial_view_lut_max_distance: 3.2e4,
        ..default()
    },
));
```

### Phase 2: Integrate with ZoneTime for Sun Position

**File**: `src/systems/directional_light_system.rs` (or new file)

Create a system that updates the directional light rotation based on `ZoneTime`:

```rust
/// Updates directional light (sun) position based on time of day
/// The Atmosphere system automatically uses this for scattering calculations
fn atmosphere_sun_update_system(
    zone_time: Res<ZoneTime>,
    mut sun_query: Query<&mut Transform, With<DirectionalLight>>,
) {
    if zone_time.is_changed() {
        // Calculate sun angle from ZoneTime state
        let sun_angle = match zone_time.state {
            ZoneTimeState::Morning => {
                -FRAC_PI_4 + zone_time.state_percent_complete * FRAC_PI_4
            }
            ZoneTimeState::Day => {
                FRAC_PI_4 + zone_time.state_percent_complete * FRAC_PI_2
            }
            ZoneTimeState::Evening => {
                FRAC_PI_2 + FRAC_PI_4 + zone_time.state_percent_complete * FRAC_PI_4
            }
            ZoneTimeState::Night => -FRAC_PI_2,
        };
        
        // Update directional light transform
        for mut transform in sun_query.iter_mut() {
            transform.rotation = Quat::from_euler(
                EulerRot::ZYX,
                0.0,
                0.0,
                -sun_angle
            );
        }
    }
}
```

### Phase 3: Configure Scene Scale

The atmosphere system uses real-world meters. The game world needs proper scaling:

```rust
AtmosphereSettings {
    // Game units are ~1 meter each
    scene_units_to_m: 1.0,
    
    // Aerial view distance (for fog effect on distant terrain)
    aerial_view_lut_max_distance: 32000.0,  // 32km
    
    // LUT sizes (defaults are fine for most cases)
    transmittance_lut_size: UVec2::new(256, 128),
    sky_view_lut_size: UVec2::new(400, 200),
    ..default()
}
```

### Phase 4: Remove or Deprecate CartoonSkyMaterial

**Option A**: Complete removal
- Remove `CartoonSkyMaterialPlugin` from `src/lib.rs`
- Remove `spawn_cartoon_sky` from `src/zone_loader.rs`
- Delete `src/render/cartoon_sky_material.rs` and `src/render/shaders/cartoon_sky.wgsl`

**Option B**: Keep as fallback
- Add feature flag to switch between systems
- Useful for lower-end hardware without compute shader support

### Phase 5: Add Night Skybox (Optional)

For stars at night, add a Skybox component:

```rust
// Load a night sky cubemap
let night_sky: Handle<Image> = asset_server.load("environment_maps/night_sky.ktx2");

commands.spawn((
    Camera3d::default(),
    // ... other components ...
    Skybox {
        image: night_sky,
        brightness: 500.0,
        rotation: Quat::default(),
    },
));
```

The Atmosphere system renders *on top* of the skybox, so stars will automatically show through at night when the sun is below the horizon.

---

## File Changes Summary

### Files to Modify

| File | Changes |
|------|---------|
| `src/lib.rs` | Add `Atmosphere` and `AtmosphereSettings` to camera spawn |
| `src/zone_loader.rs` | Remove `spawn_cartoon_sky` call and related code |
| `src/systems/directional_light_system.rs` | Update to work with `ZoneTime` for sun position |

### Files to Remove (Option A - Complete Removal)

| File | Reason |
|------|--------|
| `src/render/cartoon_sky_material.rs` | Replaced by Bevy Atmosphere |
| `src/render/shaders/cartoon_sky.wgsl` | Replaced by Bevy shaders |
| `src/render/mod.rs` | Remove CartoonSkyMaterialPlugin export |

### Files to Create (Optional)

| File | Purpose |
|------|---------|
| `src/render/atmosphere_settings.rs` | Custom atmosphere configurations |
| `src/systems/atmosphere_time_system.rs` | ZoneTime → sun position integration |

---

## Bevy 0.16 Specific Considerations

### GPU Requirements

The Atmosphere system requires:
- Compute shader support (`DownlevelFlags::COMPUTE_SHADERS`)
- `TextureFormat::Rgba16Float` with `STORAGE_BINDING` usage

If GPU doesn't support these, the plugin logs a warning and doesn't load. Consider keeping `CartoonSkyMaterial` as a fallback for older hardware.

### Render Graph Integration

The Atmosphere nodes are automatically inserted into the 3D render graph:

```
Node3d::EndPrepasses → AtmosphereNode::RenderLuts → Node3d::StartMainPass
Node3d::MainOpaquePass → AtmosphereNode::RenderSky → Node3d::MainTransparentPass
```

No manual render graph configuration needed.

### HDR Requirement

The Atmosphere component **requires HDR** on the camera. Our camera already has `hdr: true`, so this is satisfied.

### VolumetricFog Compatibility

From Bevy docs: "Using both [Atmosphere and VolumetricFog] at once is untested, and might not be physically accurate."

Our current camera has `VolumetricFog` for light shafts. Consider:
- **Option 1**: Remove VolumetricFog, rely on Atmosphere's aerial perspective
- **Option 2**: Keep both, accept potential visual inconsistencies
- **Option 3**: Use VolumetricFog for indoor/underground, Atmosphere for outdoor

---

## Visual Comparison

| Feature | CartoonSkyMaterial (Current) | Bevy Atmosphere (New) |
|---------|------------------------------|----------------------|
| Sky color | Manual gradient blending | Physics-based Rayleigh scattering |
| Sun | Stylized disc | Realistic scattering + glow |
| Moon | Stylized disc | Can use Skybox for moon/stars |
| Clouds | Procedural FBM noise | Not included (use separate system) |
| Stars | Procedural points | Use Skybox texture |
| Fog | Separate fog system | Built-in aerial perspective |
| Terrain scattering | None | Automatic based on distance |
| Performance | Simple shader | Compute shaders + LUTs |

---

## Testing Checklist

- [ ] Atmosphere renders correctly during day
- [ ] Sun position matches ZoneTime state
- [ ] Sunrise/sunset colors are realistic
- [ ] Night sky is dark (add Skybox for stars)
- [ ] Aerial perspective fog on distant terrain
- [ ] No performance regression
- [ ] Works with existing VolumetricFog (or decide to remove)
- [ ] Shadows still work correctly
- [ ] Bloom and tone mapping still function

---

## Migration Steps (For Code Mode)

1. **Add Atmosphere to camera** - Single line addition to camera spawn
2. **Update directional light system** - Connect ZoneTime to sun rotation
3. **Test basic functionality** - Verify sky renders
4. **Remove CartoonSkyMaterial** - Delete old system
5. **Add night skybox** - Optional, for stars
6. **Tune parameters** - Adjust AtmosphereSettings for game scale
7. **Test performance** - Ensure 60fps maintained

---

## References

- [Bevy 0.16 Atmosphere Source](file://C:/Users/vicha/RustroverProjects/bevy-collection/bevy-0.16.1/crates/bevy_pbr/src/atmosphere/mod.rs)
- [Bevy Atmosphere Example](https://github.com/aevyrie/bevy/blob/atmosphere_showcase/examples/3d/atmosphere.rs)
- [Hillaire 2020 Paper](https://sebh.github.io/publications/egsr2020.pdf) - A Scalable and Production Ready Sky and Atmosphere Rendering Technique
- [Unreal Engine Implementation](https://github.com/sebh/UnrealEngineSkyAtmosphere)
