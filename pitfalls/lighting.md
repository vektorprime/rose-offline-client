# Lighting Pitfalls

This document records lighting-related issues encountered during development.

---

## Dark Shadows / Excessively Dark Non-Illuminated Surfaces (Fixed 2026-02-12)

### Problem
The dark side of 3D models (surfaces not facing the directional light) were very dark, making characters and objects barely visible when facing away from the light source. This created an unpleasant visual experience where players couldn't see their characters properly in certain orientations.

### Root Cause
Bevy 0.14/0.15 changed `AmbientLight` brightness from arbitrary units to **photometric units** (cd/m² - candelas per square meter). The `AmbientLight` brightness was set to `0.3`, which worked in older Bevy versions but is now hundreds of times too low for the new unit system.

### Solution
Increased `AmbientLight` brightness from `0.3` to `500.0` in the ambient light setup.

### Reference Values for AmbientLight Brightness (Bevy 0.15.4)
- Bevy 0.15.4 default: `80.0` cd/m²
- Working value for this project: `500.0` cd/m²
- Bevy examples range: `50.0` to `3000.0` cd/m²

### Code Example
```rust
// Before (too dark in Bevy 0.15+)
commands.insert_resource(AmbientLight {
    color: Color::srgb(0.6, 0.6, 0.6),
    brightness: 0.3,  // Way too low for photometric units
});

// After (proper brightness)
commands.insert_resource(AmbientLight {
    color: Color::srgb(0.6, 0.6, 0.6),
    brightness: 500.0,  // Appropriate for cd/m²
});
```

### Files Modified
- `src/render/zone_lighting.rs` - AmbientLight brightness value

### Lesson Learned
When migrating from Bevy 0.13 or earlier to Bevy 0.14+, be aware that `AmbientLight` brightness now uses photometric units (cd/m²). Values that worked before (like `0.3`, `1.0`, or even `10.0`) are now far too low. Use values in the hundreds:
- For dim ambient: `100.0` - `300.0`
- For normal ambient: `300.0` - `800.0`
- For bright ambient: `800.0` - `2000.0`

See Bevy's migration guide for more details on the lighting unit changes.

---

## AutoExposure Default Makes the Scene Very Bright (Fixed 2026-09-30, Bevy 0.19)

### Problem
With Auto Exposure on, the whole image was very bright and washed out. Night was lifted to roughly daytime brightness.

### Root Cause
Bevy's AE shader sets `target = curve(avg) - avg`, where `avg` is the metered log2 scene luminance. The default `AutoExposureCompensationCurve` is flat 0, so AE drives the scene average to **1.0** pre-tonemap. That is about 2.5 EV above photographic middle grey (0.18), even though the docs say "middle gray". Full adaptation also removes the authored day/night difference.

### Fix
A custom compensation curve (`auto_exposure_compensation_curve` in `src/graphics/apply_systems.rs`):
- Flat at `GraphicsSettings::auto_exposure_target_ev` (default **-1.3**, tuned in-game) for metered averages ≥ -2.
- 50% adaptation below that, so night and caves stay darker.

The shared `AutoExposureCurve` resource is created right after `AutoExposurePlugin` in `lib.rs` and used at camera spawn. `apply_auto_exposure_system` rebuilds it in place only when the Graphics tab "Exposure Target" slider changes.

### Lesson Learned
Never ship `AutoExposure::default()` as-is. The curve's y value is the log2 luminance the scene average lands on, so pick it deliberately. Add a slope for dark scenes, or night becomes day.

---

## Models Look Shiny / Washed Out Only When Zoomed In (Fixed 2026-09-30, Bevy 0.19)

### Problem
Close to the camera, character and monster models had a grey-white "shine". It was strongest on dark or shadowed parts (wings, fur) and disappeared when the camera zoomed out.

### Root Cause
The camera's `VolumetricFog` had Bevy's default `ambient_intensity: 0.1`. Bevy 0.19.1's `volumetric_fog.wgsl` adds that ambient as `exp(-ray_length * (absorption + scattering)) * ambient_color * ambient_intensity`, blended additively. It ignores `density_factor`, so it applies even with fog density 0. It is not scaled by exposure either. The fog volume covers the whole zone, so the camera is always inside it and `ray_length` is just the depth to the surface. With `absorption + scattering = 0.21`, the veil is about 10x stronger at 4 m than at 15 m.

### Fix
Set `VolumetricFog::ambient_intensity` to `0.0` in both places the component is inserted in `src/lib.rs`: the camera spawn in `load_common_game_data` and `apply_post_processing_settings`. Light shafts still work when fog density is above 0.

### Lesson Learned
`VolumetricFog`'s ambient term is not a uniform fog tint. It is an additive haze that gets stronger as the camera gets closer to a surface, and density does not turn it off. Keep it at 0 unless that near-camera glow is wanted.
