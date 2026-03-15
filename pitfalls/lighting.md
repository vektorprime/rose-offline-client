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
