# Evening/Night Shadow Solution Design

## Problem Statement
During evening and night times, models create shadows as if the sun is still out. Since there's a starry night sky that appears in the evening and at night (which is already bright all over the sky), these directional sun shadows shouldn't exist during those times.

## Current Architecture Analysis

### 1. Zone Time System
**File**: [`src/resources/zone_time.rs`](../src/resources/zone_time.rs)

```rust
pub enum ZoneTimeState {
    Morning,  // 6:00-12:00
    Day,      // 12:00-17:00
    Evening,  // 17:00-19:00 (2-hour transition)
    Night,    // 19:00-6:00 (11 hours, wraps midnight)
}
```

The `ZoneTime` resource tracks:
- `state`: Current ZoneTimeState
- `state_percent_complete`: Progress through current state (0.0-1.0)
- `time`: Current time in ticks
- `debug_overwrite_time`: Optional manual time override

### 2. Zone Time System Logic
**File**: [`src/systems/zone_time_system.rs`](../src/systems/zone_time_system.rs:84)

The `zone_time_system` function determines time state based on FIXED hour thresholds:
- **Morning**: 6:00-12:00 (6 hours)
- **Day**: 12:00-17:00 (5 hours)
- **Evening**: 17:00-19:00 (2 hours - dusk transition)
- **Night**: 19:00-6:00 (11 hours, wraps around midnight)

### 3. Directional Lights Configuration

#### Sun Light
**File**: [`src/render/zone_lighting.rs`](../src/render/zone_lighting.rs:140-159)

```rust
let light_entity = commands.spawn((
    DirectionalLight {
        illuminance: 15000.0,
        shadows_enabled: true,  // ALWAYS TRUE - this is the problem
        ..Default::default()
    },
    default_light_transform(),
    CascadeShadowConfig {
        bounds: vec![20.0, 80.0, 300.0, 1000.0],
        overlap_proportion: 0.3,
        minimum_distance: 0.1,
    },
    RenderLayers::default(),
    VolumetricLight,
)).id();
```

#### Moon Light
**File**: [`src/lib.rs`](../src/lib.rs:2241-2254)

```rust
let moon_entity = commands.spawn((
    MoonLight,
    DirectionalLightComponent {
        illuminance: 5000.0,
        color: Color::srgb(0.8, 0.85, 0.95),
        shadows_enabled: true,  // Also always true
        ...
    },
    ...
)).id();
```

### 4. Shadow Control System
**File**: [`src/graphics/apply_systems.rs`](../src/graphics/apply_systems.rs:40-88)

The `apply_shadow_quality_system` controls shadows based on GraphicsSettings:
- Iterates all directional lights
- Sets `light.shadows_enabled` based on `ShadowQuality` setting
- Does NOT consider time of day

### 5. Existing Time-Based Lighting Updates
**File**: [`src/render/zone_lighting.rs`](../src/render/zone_lighting.rs:267-311)

The `sync_zone_lighting_to_bevy_lights_system`:
- Updates ambient light color and brightness
- Updates directional light color
- Updates `zone_lighting.light_direction`
- Does NOT modify shadow settings

### 6. Night Factor System (for stars)
**File**: [`src/render/starry_sky_material.rs`](../src/render/starry_sky_material.rs:429-555)

The `update_starry_sky_night_factor` system calculates visibility:
- Night = 1.0 (stars fully visible)
- Evening (2nd half) = 0.0-1.0 (fade in)
- Morning (1st half) = 1.0-0.0 (fade out)
- Day = 0.0 (stars invisible)

## Root Cause
The `shadows_enabled` property on both the sun and moon directional lights is set to `true` at startup and never modified based on time of day. The shadow quality system only considers graphics settings, not game time.

## Proposed Solution

### Option A: Disable Shadows During Evening/Night (Recommended)

Create a new system that disables sun shadows during evening and night:

**New System Location**: [`src/render/zone_lighting.rs`](../src/render/zone_lighting.rs)

```rust
/// System that adjusts shadow settings based on time of day.
/// During evening and night, sun shadows are disabled since the starry sky
/// provides ambient lighting from all directions.
pub fn update_shadows_for_time_of_day_system(
    zone_time: Res<ZoneTime>,
    mut sun_query: Query<&mut DirectionalLight, (With<VolumetricLight>, Without<MoonLight>)>,
    mut moon_query: Query<&mut DirectionalLight, With<MoonLight>>,
    graphics_settings: Option<Res<GraphicsSettings>>,
) {
    // Check if shadows are enabled in graphics settings
    let shadows_enabled_by_settings = graphics_settings
        .map(|g| *g.shadow_quality != ShadowQuality::Off)
        .unwrap_or(true);
    
    if !shadows_enabled_by_settings {
        return; // Shadows disabled in settings, nothing to do
    }
    
    // Calculate shadow visibility based on time state
    let (sun_shadows, moon_shadows) = match zone_time.state {
        ZoneTimeState::Morning => {
            // Early morning: fade shadows in
            let factor = zone_time.state_percent_complete;
            (true, false) // Sun shadows fade in, no moon shadows
        }
        ZoneTimeState::Day => {
            // Full daylight: sun shadows enabled
            (true, false)
        }
        ZoneTimeState::Evening => {
            // Evening transition: fade sun shadows out
            // During second half, start disabling shadows
            let factor = 1.0 - zone_time.state_percent_complete;
            (factor > 0.3, false) // Disable sun shadows after 70% through evening
        }
        ZoneTimeState::Night => {
            // Night: no sun shadows
            // Moon provides very dim light - optional soft shadows
            (false, true) // Could enable subtle moon shadows
        }
    };
    
    // Apply to sun light
    for mut light in sun_query.iter_mut() {
        light.shadows_enabled = sun_shadows;
    }
    
    // Apply to moon light
    for mut light in moon_query.iter_mut() {
        light.shadows_enabled = moon_shadows;
    }
}
```

**Registration in Plugin**:
```rust
app.add_systems(Update, (
    // ... existing systems ...
    update_shadows_for_time_of_day_system
        .after(zone_time_system)
        .after(apply_shadow_quality_system), // Run after graphics settings apply
));
```

### Option B: Reduce Shadow Intensity (Alternative)

Instead of completely disabling shadows, reduce the shadow depth bias or use illuminance to make shadows less prominent:

```rust
/// System that reduces shadow intensity during evening/night
pub fn update_shadow_intensity_for_time_of_day_system(
    zone_time: Res<ZoneTime>,
    mut sun_query: Query<&mut DirectionalLight, (With<VolumetricLight>, Without<MoonLight>)>,
) {
    for mut light in sun_query.iter_mut() {
        let (illuminance, shadow_bias) = match zone_time.state {
            ZoneTimeState::Morning => {
                let factor = zone_time.state_percent_complete;
                (15000.0 * factor, 0.0) // Increase illuminance through morning
            }
            ZoneTimeState::Day => {
                (15000.0, 0.0) // Full daylight
            }
            ZoneTimeState::Evening => {
                let factor = 1.0 - zone_time.state_percent_complete;
                (15000.0 * factor, 0.5 * (1.0 - factor)) // Reduce illuminance, increase bias
            }
            ZoneTimeState::Night => {
                (0.0, 1.0) // No light, max bias = no visible shadows
            }
        };
        
        light.illuminance = illuminance;
        light.shadow_depth_bias = shadow_bias;
    }
}
```

## Files to Modify

### Primary Changes
1. **[`src/render/zone_lighting.rs`](../src/render/zone_lighting.rs)**
   - Add new system `update_shadows_for_time_of_day_system`
   - Register system in `ZoneLightingPlugin::build()`
   - System should run after `zone_time_system` and `apply_shadow_quality_system`

### Optional Changes
2. **[`src/graphics/apply_systems.rs`](../src/graphics/apply_systems.rs)**
   - Modify `apply_shadow_quality_system` to respect time-based shadow state
   - Add coordination between graphics settings and time-of-day system

## Implementation Notes

1. **System Ordering**: The new system must run AFTER:
   - `zone_time_system` - to get current time state
   - `apply_shadow_quality_system` - to respect user's shadow quality settings

2. **Marker Components**: Use existing marker components:
   - `VolumetricLight` - identifies the sun directional light
   - `MoonLight` - identifies the moon directional light

3. **Graphics Settings Integration**: The system should check `GraphicsSettings.shadow_quality` first. If shadows are disabled in settings, time-based changes are irrelevant.

4. **Smooth Transitions**: Consider using `state_percent_complete` for smooth shadow fade in/out during morning and evening transitions.

## Testing Checklist
- [ ] Verify shadows appear during morning (6:00-12:00)
- [ ] Verify shadows appear during day (12:00-17:00)
- [ ] Verify shadows fade/disable during evening (17:00-19:00)
- [ ] Verify no shadows during night (19:00-6:00)
- [ ] Verify shadow quality settings still work
- [ ] Test with manual time override in debug UI
- [ ] Verify moon light behavior at night (optional subtle shadows)
