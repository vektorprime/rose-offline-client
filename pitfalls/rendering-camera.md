# Rendering and Camera Pitfalls

This document records rendering and camera-related issues encountered during development.

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

## Shadow/Shader Quality (Bevy 0.15) (Fixed 2026-02-19)

### Problem 1: SSAO and TAA Require Msaa::Off
**Error:** `SSAO is being used which requires Msaa::Off, but Msaa is currently set to Msaa::Sample4`

**Cause:** Both Screen Space Ambient Occlusion (SSAO) and Temporal Anti-Aliasing (TAA) are computationally intensive techniques that are incompatible with Multi-Sample Anti-Aliasing (MSAA).

**Solution:** Add `Msaa::Off` component to the camera when using SSAO or TAA:
```rust
commands.spawn((
    Camera3d::default(),
    Msaa::Off,  // Required for SSAO/TAA
    ScreenSpaceAmbientOcclusion::default(),
    TemporalAntiAliasing::default(),
));
```

---

### Problem 2: ExtendedMaterial Limited Bind Group Access
**Error:** `Shader global ResourceBinding { group: 3, binding: 0 } is not available in the pipeline layout`

**Cause:** `ExtendedMaterial` in Bevy 0.15 only has access to bind groups 0, 1, and 2:
- Group 0: View uniforms (camera, view-projection matrices)
- Group 1: Mesh uniforms (transform data)
- Group 2: Material uniforms (StandardMaterial + extension data)

Cannot access group 3+ where custom zone lighting data might be stored.

**Solution:** Use Bevy's built-in fog systems (`DistanceFog`, `FogMetadata`) instead of trying to access custom zone lighting bind groups in material extensions. Pass any needed additional data through the extension's own bindings at group 2.

---

### Problem 3: PbrInput Struct Missing Direct `view` Field
**Error:** Cannot access `pbr_input.view.z` directly in Bevy 0.15 shaders.

**Cause:** The `PbrInput` struct structure changed in Bevy 0.15. The view vector is not directly accessible as a simple field.

**Solution:** Calculate view_z using the `view.view_from_world` matrix transformation, or use Bevy's built-in shader functions for depth calculations.

---

### Problem 4: Shadow Casting and Transparency Artifacts
**Problem:** Alpha-blended objects (like tree leaves with `AlphaMode::Blend`) caused shadow artifacts when casting shadows.

**Cause:** Alpha-blended materials don't have well-defined opacity for shadow mapping, causing visual artifacts.

**Solution:** Configure shadow casting based on transparency type:
- **Opaque objects:** Should cast shadows (`casts_shadow: true`)
- **Alpha-blended objects:** Should NOT cast shadows (`casts_shadow: false`)
- **Alpha-masked objects:** CAN cast shadows (binary transparency works with shadow mapping)

```rust
match material.alpha_mode {
    AlphaMode::Opaque | AlphaMode::Mask(_) => {
        // Cast shadows
    }
    AlphaMode::Blend | AlphaMode::Premultiplied | AlphaMode::Add | AlphaMode::Multiply => {
        // Don't cast shadows to avoid artifacts
    }
}
```

---

### Problem 5: High Ambient Light Washes Out Shadows
**Problem:** Shadows appeared washed out and had poor contrast.

**Cause:** Ambient light brightness was set too high (500.0 cd/m²), which fills in shadowed areas and reduces shadow contrast.

**Solution:** Reduce ambient light brightness to improve shadow contrast:
```rust
// Before (shadows washed out)
AmbientLight { brightness: 500.0, ... }

// After (better shadow contrast)
AmbientLight { brightness: 150.0, ... }
```

**Note:** This is a trade-off - lower ambient means darker shadows but also darker non-illuminated surfaces. Balance based on scene requirements.

---

### Problem 6: Vegetation Appears Too Shiny
**Problem:** Trees and vegetation had an unrealistic shiny appearance.

**Cause:** Default `StandardMaterial` roughness (0.5) is too low for vegetation, which typically has a matte appearance.

**Solution:** Increase roughness for vegetation materials:
```rust
// For trees, grass, and other vegetation
material.perceptual_roughness = 0.8;  // More realistic matte appearance
```

---

### Problem 7: Foliage Alpha Masks Not Working in ExtendedMaterial
**Problem:** Foliage and objects with alpha masks appeared as opaque squares instead of having proper transparency.

**Cause:** When using `ExtendedMaterial` in Bevy 0.15, the extension's fragment shader replaces the base material's fragment function. The shader imported `alpha_discard` but never called it, so pixels that should be transparent were not being discarded.

**Solution:** Add `alpha_discard()` call in the fragment shader after creating the PBR input:
```wgsl
// In the fragment shader's main function
pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);
```

This ensures that pixels below the alpha threshold are discarded before rendering, allowing alpha-masked textures (like tree leaves, grass, fences) to render with proper transparency.

**Note:** The `alpha_discard` function must be imported from `bevy_pbr::pbr_fragment::pbr_types`:
```wgsl
#import bevy_pbr::pbr_fragment::pbr_types::{alpha_discard, PbrInput}
```

---

### Files Modified
- `src/lib.rs` - Camera setup with `Msaa::Off`
- `src/render/zone_lighting.rs` - Ambient light brightness adjustment
- `src/model_loader.rs` - Vegetation roughness, shadow casting configuration
- `src/zone_loader.rs` - Shadow casting based on alpha mode

### Lesson Learned
1. SSAO and TAA require `Msaa::Off` - they are fundamentally incompatible with MSAA
2. `ExtendedMaterial` can only access bind groups 0-2; use built-in Bevy systems for fog/lighting effects
3. Shadow casting should be disabled for alpha-blended materials to avoid artifacts
4. Ambient light brightness directly affects shadow contrast - balance carefully
5. Vegetation materials need higher roughness values (0.7-0.9) for realistic appearance
6. **ExtendedMaterial fragment shaders must call `alpha_discard()`** - when using `AlphaMode::Mask`, the extension's fragment shader replaces the base material's fragment function, so you must explicitly call `alpha_discard(pbr_input.material, pbr_input.material.base_color)` to discard transparent pixels
