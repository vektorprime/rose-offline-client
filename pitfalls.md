# Pitfalls and Lessons Learned

This document records issues encountered during development and their solutions, to help avoid similar problems in the future.

---

## Depth of Field Not Visible (Fixed 2026-02-12)

### Problem
Depth of field (DoF) effect was added to the camera but wasn't visible in the game.

### Root Cause
1. **Missing Tonemapping**: HDR must be enabled on the camera (`hdr: true`), and Tonemapping must be added for HDR to work properly with DoF
2. **Missing Bloom**: Bloom enhances the visibility of the DoF effect
3. **No runtime adjustment**: DoF settings couldn't be tuned live, making it difficult to find appropriate values

### Solution
1. Added `Tonemapping::TonyMcMapface` component to the camera
2. Added `Bloom::NATURAL` component to the camera
3. Created `DepthOfFieldSettings` resource in `src/ui/ui_settings_system.rs` with live UI controls
4. Added `apply_depth_of_field_settings` system to apply settings from resource to camera
5. Added "Depth of Field" tab to Settings UI with sliders for all DoF parameters

### Key DoF Parameters (Bevy 0.15.4)
- `mode`: `DepthOfFieldMode::Bokeh` or `DepthOfFieldMode::Gaussian`
- `focal_distance`: Distance in meters to the focal plane (objects at this distance are sharp)
- `aperture_f_stops`: Lower values = more blur (e.g., 0.05 = very blurry, 3.3 = subtle)
- `sensor_height`: Affects blur characteristics (0.01866 = Super 35 format)
- `max_circle_of_confusion_diameter`: Maximum blur circle size in pixels
- `max_depth`: Clamps depth for distant objects

### Working Default Values
```rust
DepthOfField {
    mode: DepthOfFieldMode::Bokeh,
    focal_distance: 10.0,
    aperture_f_stops: 3.3,
    sensor_height: 0.01866,
    max_circle_of_confusion_diameter: 64.0,
    max_depth: 2000.0,
}
```

### Files Modified
- `src/lib.rs` - Camera spawn with DoF, Tonemapping, Bloom; `apply_depth_of_field_settings` system
- `src/ui/ui_settings_system.rs` - `DepthOfFieldSettings` resource and UI controls
- `src/ui/mod.rs` - Export `DepthOfFieldSettings`

### Lesson Learned
When using Bevy's depth of field effect:
1. Always enable HDR on the camera
2. Always add Tonemapping (required for HDR to render properly)
3. Consider adding Bloom for better visual results
4. Import path is `bevy::core_pipeline::dof::{DepthOfField, DepthOfFieldMode}` (not `depth_of_field`)
5. Use `DetectChanges` trait from `bevy::ecs::change_detection` for `is_changed()` on resources

---

## Tree/Grass Transparency Not Working (Fixed 2026-02-12)

### Problem
Trees and grass textures were not showing transparency in their leaves. The texture alpha channel was being ignored.

### Root Cause
When creating Bevy materials in `model_loader.rs` and `zone_loader.rs`, the code always used `AlphaMode::Opaque` regardless of the ZSC material's `alpha_enabled` and `alpha_test` properties.

### Solution
Both files were updated to properly set `alpha_mode` based on ZSC material properties:
- `AlphaMode::Mask(threshold)` when `alpha_enabled` with `alpha_test` threshold
- `AlphaMode::Blend` when `alpha_enabled` without threshold
- `AlphaMode::Opaque` when alpha is disabled

### Files Modified
- `src/model_loader.rs` (lines ~1357-1375)
- `src/zone_loader.rs` (lines ~2649-2669)

### Lesson Learned
When working with Bevy's StandardMaterial or custom materials, always explicitly set `alpha_mode` based on the source material's transparency properties. The default `AlphaMode::Opaque` will ignore any alpha channel in textures.

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

