# Starry Sky & Atmosphere Architecture

**Date:** February 28, 2026
**Bevy Version:** 0.16.1
**Project:** ROSE Offline Client

**Status:** ✅ **FULLY FUNCTIONAL** - Stars render correctly at night with real-time UI controls

---

## Table of Contents

1. [Executive Summary](#executive-summary)
2. [Star Generation Algorithm](#star-generation-algorithm)
3. [Day/Night Cycle Architecture](#daynight-cycle-architecture)
4. [UI Settings Integration](#ui-settings-integration)
5. [Render Pipeline](#render-pipeline)
6. [Resources & Components](#resources--components)
7. [System Execution Order](#system-execution-order)
8. [Configuration & Tuning](#configuration--tuning)
9. [Debug & Diagnostics](#debug--diagnostics)
10. [Historical Issues (Resolved)](#historical-issues-resolved)

---

## Executive Summary

The starry sky implementation provides a procedurally generated star field that:
- **Renders only at night** (controlled by game time or manual override)
- **Uses grid-based procedural generation** with multiple density layers
- **Supports real-time UI adjustments** for star density, brightness, and moon settings
- **Integrates with Bevy's atmosphere** by toggling it off at night
- **Includes moon rendering** with phase support and directional lighting

### Key Features

| Feature | Status | Description |
|---------|--------|-------------|
| Procedural Stars | ✅ | Grid-based generation with 4 density layers |
| Day/Night Cycle | ✅ | Automatic (game time) or Manual (UI slider) |
| Atmosphere Toggle | ✅ | Enabled day / Disabled night |
| Moon Rendering | ✅ | Phases, direction, and lighting |
| Real-time UI | ✅ | Settings menu for all parameters |
| Twinkling Animation | ✅ | Time-based per-star animation |
| Nebula Background | ✅ | Subtle noise-based depth |

---

## Star Generation Algorithm

### Overview

Stars are generated procedurally in the GPU shader using a **grid-based sampling approach**. This ensures consistent star placement across frames while allowing for high star counts with minimal memory.

### Implementation Location

`src/render/shaders/starry_sky.wgsl` - `star_layer()` function (lines 91-132)

### Algorithm Breakdown

#### 1. Grid-Based Sampling

```wgsl
fn star_layer(dir: vec3<f32>, scale: f32, brightness_base: f32, twinkle_speed: f32) -> f32 {
    // Grid-based star placement
    let p = dir * scale;
    let grid_id = floor(p);
    let grid_fract = fract(p);
```

- **`dir`**: Normalized direction vector from sphere center to pixel
- **`scale`**: Controls grid density (higher = more cells = more potential stars)
- **`grid_id`**: Integer grid cell coordinates
- **`grid_fract`**: Position within the current grid cell

#### 2. Neighboring Cell Check

```wgsl
    // Check neighboring cells for stars (3×3×3 cube = 27 cells)
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let cell_offset = vec3<f32>(f32(x), f32(y), f32(z));
                let cell_id = grid_id + cell_offset;
```

Checking neighbors ensures stars near cell boundaries are visible from all adjacent pixels.

#### 3. Per-Cell Star Generation

```wgsl
                // Random star position within cell
                let rand_vals = hash3v(cell_id);
                let star_pos = rand_vals.xyz;

                // Star exists based on density threshold
                let star_exists = step(rand_vals.x, star_density);

                // Distance from current pixel to star position
                let diff = grid_fract - cell_offset - star_pos;
                let dist = length(diff);

                // Star size varies based on random value
                let star_size = 0.02 + rand_vals.y * 0.03;

                // Star intensity with smooth falloff
                let intensity = smoothstep(star_size, 0.0, dist);

                // Twinkling effect
                let twinkle_phase = rand_vals.z * 6.28318;
                let twinkle = 0.7 + 0.3 * sin(time * twinkle_speed + twinkle_phase);

                star_brightness += intensity * star_exists * brightness_base * twinkle;
            }
        }
    }
```

**Key Parameters:**
- **`star_density`** (0.0-1.0): Probability a cell contains a star
- **`star_size`** (0.02-0.05): Angular size of the star
- **`twinkle`** (0.7-1.0): Brightness modulation over time

#### 4. Multiple Layers for Depth

Four star layers are rendered with different scales:

| Layer | Scale | Brightness | Purpose |
|-------|-------|------------|---------|
| Distant | 80.0 | 0.4× | Many small background stars |
| Medium | 40.0 | 0.7× | Mid-distance stars |
| Bright | 20.0 | 1.2× | Prominent stars |
| Rare | 10.0 | 2.0× | Very bright, rare stars |

### Star Count Calculation

**Formula:**
```
Stars per layer ≈ (scale / 2)² × π × star_density
```

**Example with star_density = 0.50:**
- Distant (scale=80): ~5,000 × 0.50 = **2,500 stars**
- Medium (scale=40): ~1,250 × 0.50 = **625 stars**
- Bright (scale=20): ~312 × 0.50 = **156 stars**
- Rare (scale=10): ~78 × 0.50 = **39 stars**
- **Total: ~3,320 stars**

**Recommended star_density values:**
- **0.15**: Sparse (~1,000 stars) - minimalist aesthetic
- **0.50**: Moderate (~3,300 stars) - **recommended default**
- **0.70**: Dense (~5,000+ stars) - photorealistic

---

## Day/Night Cycle Architecture

### Overview

The day/night cycle is controlled by the `zone_time_system` which reads from either:
1. **Game time** (`WorldTime` resource from server ticks)
2. **Manual override** (`ZoneTime.debug_overwrite_time` set by UI)

### Core Resources

#### `WorldTime` (from `rose-game-common`)
```rust
pub struct WorldTime {
    pub ticks: u64,              // Server tick count
    pub time_since_last_tick: Duration,
}
```

#### `ZoneTime` (`src/resources/zone_time.rs`)
```rust
pub struct ZoneTime {
    pub state: ZoneTimeState,           // Morning/Day/Evening/Night
    pub state_percent_complete: f32,    // 0.0-1.0 progress within state
    pub time: u32,                      // Current tick within day_cycle
    pub debug_overwrite_time: Option<u32>, // Manual override (from UI)
}
```

#### `SkySettings` (`src/render/zone_lighting.rs`)
```rust
pub struct SkySettings {
    pub mode: SkyMode,                  // Automatic or Manual
    pub manual_time: f32,               // 0-24 hours when Manual
    pub atmosphere_intensity: f32,      // 0.0-2.0 multiplier
}
```

### Time State Thresholds

From zone data (STB file):
- `morning_time`: Tick when morning begins (typically ~6:00)
- `day_time`: Tick when day begins (typically ~12:00)
- `evening_time`: Tick when evening begins (typically ~18:00)
- `night_time`: Tick when night begins (typically ~22:00)
- `day_cycle`: Total ticks for 24-hour cycle (typically 160)

### State Detection Logic

```rust
// Night wraps around midnight
let is_night = if zone_data.night_time >= zone_data.morning_time {
    day_time >= zone_data.night_time || day_time < zone_data.morning_time
} else {
    day_time >= zone_data.night_time && day_time < zone_data.morning_time
};

// Evening may also wrap
let is_evening = if zone_data.night_time >= zone_data.evening_time {
    day_time >= zone_data.evening_time && day_time < zone_data.night_time
} else {
    day_time >= zone_data.evening_time || day_time < zone_data.night_time
};

// Day and morning are simple ranges
let is_day = day_time >= zone_data.day_time && day_time < zone_data.evening_time;
let is_morning = day_time >= zone_data.morning_time && day_time < zone_data.day_time;
```

### Night Factor by State

| Time State | night_factor | Atmosphere | Stars Visible |
|------------|--------------|------------|---------------|
| **Day** | 0.0 | Enabled | No |
| **Morning (1st half)** | 1.0→0.0 | Enabled | Fading out |
| **Morning (2nd half)** | 0.0 | Enabled | No |
| **Evening (1st half)** | 0.0 | Enabled | No |
| **Evening (2nd half)** | 0.0→1.0 | Enabled | Fading in |
| **Night** | 1.0 | **Disabled** | **Yes** |

---

## UI Settings Integration

### Sky Tab (Time Control)

**Location:** `src/ui/ui_settings_system.rs` - `SettingsPage::Sky`

**Controls:**
- **Time Mode**: Automatic (game time) / Manual (slider)
- **Time of Day**: 0-24 hours slider (only in Manual mode)
- **Atmosphere Intensity**: 0.0-2.0 multiplier

**Data Flow:**
```
UI Slider → SkySettings.manual_time
    → apply_sky_settings_to_zone_time system
        → ZoneTime.debug_overwrite_time = Some(tick_value)
            → zone_time_system uses override instead of WorldTime
```

### Stars Tab (Star Appearance)

**Location:** `src/ui/ui_settings_system.rs` - `SettingsPage::Stars`

**Controls:**
- **Star Density**: 0.0-1.0 (probability of star per grid cell)
- **Star Brightness**: 0.0-5.0 (overall brightness multiplier)
- **Moon Phase**: 0.0-1.0 (0/1=new, 0.5=full)
- **Moon Direction X/Y/Z**: -1.0 to 1.0 (direction vector)
- **Normalize Button**: Normalizes moon direction to unit vector
- **Night Factor**: Read-only display (auto-controlled)

**Data Flow:**
```
UI Sliders → StarrySkySettings
    → update_starry_sky_system
        → StarrySkyMaterial uniforms updated
            → GPU shader uses new values immediately
```

### Manual Time Bridge System

**Location:** `src/render/zone_lighting.rs` - `apply_sky_settings_to_zone_time()`

```rust
fn apply_sky_settings_to_zone_time(
    sky_settings: Res<SkySettings>,
    current_zone: Option<Res<CurrentZone>>,
    game_data: Res<GameData>,
    mut zone_time: ResMut<ZoneTime>,
) {
    if !sky_settings.is_changed() {
        return;
    }

    let Some(current_zone) = current_zone else { return };
    let Some(zone_data) = game_data.zone_list.get_zone(current_zone.id) else { return };

    match sky_settings.mode {
        SkyMode::Manual => {
            // Convert hours (0-24) to ticks
            let manual_time_hours = sky_settings.manual_time.clamp(0.0, 24.0);
            let tick_value = ((manual_time_hours / 24.0) * zone_data.day_cycle as f32) as u32;
            zone_time.debug_overwrite_time = Some(tick_value);
        }
        SkyMode::Automatic => {
            zone_time.debug_overwrite_time = None;
        }
    }
}
```

**Key Points:**
- Only runs when `SkySettings` changes (efficient)
- Requires `CurrentZone` to get `day_cycle` value
- Conversion: `tick = (hours / 24.0) × day_cycle`
- Example: 12:00 noon with day_cycle=160 → tick 80

---

## Render Pipeline

### Bevy Render Graph (Simplified)

```
EarlyPrepasses
    ↓
MainOpaquePass (solid objects)
    ↓
Atmosphere::RenderSky (fullscreen quad, additive blend) ← REMOVED at night
    ↓
MainTransparentPass (transparent objects)
    ↓
Transparent3d (StarrySkyMaterial) ← Stars render here
    ↓
Tonemapping / Post-processing
```

### Starry Sky Material Pipeline

**Material:** `StarrySkyMaterial` (implements `bevy::pbr::Material`)

**Specialization** (`src/render/starry_sky_material.rs`):
```rust
fn specialize(...) {
    // 1. Vertex layout: position only
    descriptor.vertex.buffers = vec![vertex_layout];

    // 2. Disable backface culling (camera is INSIDE sphere)
    descriptor.primitive.cull_mode = None;

    // 3. Additive blending for stars
    color_target_state.blend = Some(BlendState {
        color: BlendComponent {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::One,
            operation: BlendOperation::Add,
        },
        ...
    });

    // 4. Disable depth writes, always pass depth test
    depth_stencil.depth_write_enabled = false;
    depth_stencil.depth_compare = CompareFunction::Always;
}
```

**Why these settings?**
- **No culling**: Camera is inside the sphere, all triangles face away
- **Additive blend**: Stars accumulate brightness naturally
- **No depth write**: Sky should always render behind everything
- **Always compare**: Prevents depth issues at far plane

### Atmosphere Toggle

**System:** `toggle_atmosphere_based_on_time()` (in `starry_sky_material.rs`)

```rust
match zone_time.state {
    ZoneTimeState::Night => {
        // Remove atmosphere components
        commands.entity(camera).remove::<Atmosphere>();
        commands.entity(camera).remove::<AtmosphereSettings>();
    }
    _ => {
        // Add atmosphere components
        commands.entity(camera).insert((
            Atmosphere::EARTH,
            AtmosphereSettings::default(),
        ));
    }
}
```

**Why toggle instead of fade?**
- Bevy atmosphere is a **fullscreen post-process effect**, not a traditional skybox
- It uses **additive blending** which would wash out stars even at low opacity
- Removing the component is cleaner and more performant

---

## Resources & Components

### Resources

| Resource | Location | Purpose |
|----------|----------|---------|
| `WorldTime` | `rose-game-common` | Server tick-based time |
| `ZoneTime` | `src/resources/zone_time.rs` | Current zone's time state |
| `StarrySkySettings` | `src/render/starry_sky_material.rs` | Star appearance settings |
| `SkySettings` | `src/render/zone_lighting.rs` | Time mode and manual override |
| `ZoneLighting` | `src/render/zone_lighting.rs` | Ambient/diffuse colors, fog |
| `CurrentZone` | `src/resources/current_zone.rs` | Active zone ID and data |

### Components

| Component | Entity Type | Purpose |
|-----------|-------------|---------|
| `StarrySky` | Sky sphere | Marker for starry sky entity |
| `Atmosphere` | Camera | Enables Bevy atmosphere |
| `AtmosphereSettings` | Camera | Atmosphere configuration |
| `MoonLight` | Directional light | Moon's directional light source |

### Materials

| Material | Shader | Purpose |
|----------|--------|---------|
| `StarrySkyMaterial` | `starry_sky.wgsl` | Procedural stars, moon, nebula |

---

## System Execution Order

### Update Schedule (in `lib.rs`)

```rust
app.add_systems(Update, (
    // 1. Time system (runs first)
    zone_time_system,
    
    // 2. UI bridge (converts SkySettings to ZoneTime override)
    apply_sky_settings_to_zone_time,
    
    // 3. Atmosphere toggle (depends on ZoneTime)
    toggle_atmosphere_based_on_time.after(zone_time_system),
    
    // 4. Night factor update (depends on ZoneTime)
    update_starry_sky_night_factor.after(zone_time_system),
    
    // 5. Material update (depends on night_factor)
    update_starry_sky_system.after(update_starry_sky_night_factor),
    
    // 6. Sun position (depends on SkySettings or ZoneTime)
    update_sun_position_system,
    
    // 7. Color grading (depends on ZoneTime)
    color_grading_time_of_day_system,
));
```

### Data Dependency Graph

```
WorldTime (server) ──┐
                     ├→ zone_time_system → ZoneTime
SkySettings (UI) ────┘ (via debug_overwrite_time)
       ↓
apply_sky_settings_to_zone_time
       ↓
ZoneTime.debug_overwrite_time
       ↓
zone_time_system (uses override if set)
       ↓
┌─────────────────┬─────────────────┬──────────────────┐
↓                 ↓                 ↓                  ↓
toggle_atmo...  update_starry...  update_sun...    color_grading...
       ↓                 ↓                 ↓                  ↓
Atmosphere      night_factor      sun rotation     color grading
component       (0.0 or 1.0)
       ↓                 ↓
[removed at    update_starry_sky_system
 night]                 ↓
              StarrySkyMaterial uniforms
       ↓
GPU shader renders stars
```

---

## Configuration & Tuning

### Default Values

#### `StarrySkySettings` (in `starry_sky_material.rs`)

```rust
impl Default for StarrySkySettings {
    fn default() -> Self {
        Self {
            star_density: 0.50,        // 50% cell occupancy (~3,300 stars)
            star_brightness: 1.0,      // Normal brightness
            moon_phase: 0.5,           // Full moon
            moon_direction: Vec3::new(0.3, 0.8, 0.5).normalize(),
            night_factor: 0.0,         // Auto-controlled
        }
    }
}
```

#### `SkySettings` (in `zone_lighting.rs`)

```rust
impl Default for SkySettings {
    fn default() -> Self {
        Self {
            mode: SkyMode::Automatic,  // Follow game time
            manual_time: 12.0,         // Noon (if Manual)
            atmosphere_intensity: 1.0, // Normal intensity
        }
    }
}
```

### Tuning Guidelines

#### Increasing Star Count

**Option 1: Increase `star_density`** (recommended)
- Range: 0.0-1.0
- Effect: Linear increase in star count
- Performance: Minimal impact (GPU-side)
- Recommended: 0.50-0.70 for dense sky

**Option 2: Increase layer scales**
- Current: 80, 40, 20, 10
- Effect: Exponential increase (scale²)
- Performance: Higher (more hash computations)
- Caution: May cause performance issues

#### Adjusting Star Brightness

**Individual layer brightness** (in shader):
```wgsl
let distant_stars = star_layer(dir, 80.0, 0.4 * star_brightness, 2.0);
let medium_stars = star_layer(dir, 40.0, 0.7 * star_brightness, 3.0);
let bright_stars = star_layer(dir, 20.0, 1.2 * star_brightness, 4.0);
let rare_stars = star_layer(dir, 10.0, 2.0 * star_brightness, 5.0);
```

**Overall multiplier** (via UI):
- `star_brightness` uniform: 0.0-5.0
- Affects all layers equally

#### Moon Configuration

**Phase cycle:**
- 0.0-0.05: New Moon (invisible)
- 0.05-0.25: Waxing Crescent
- 0.25-0.35: First Quarter
- 0.35-0.55: Waxing Gibbous
- 0.55-0.65: Full Moon (brightest)
- 0.65-0.75: Waning Gibbous
- 0.75-0.95: Last Quarter
- 0.95-1.0: New Moon (invisible)

**Direction:**
- Use normalized vector pointing where moon should appear
- Y component should be positive (above horizon)
- Example: `Vec3::new(0.3, 0.8, 0.5).normalize()` → upper right

---

## Debug & Diagnostics

### Log Messages to Monitor

#### Starry Sky Pipeline
```
[STARRY SKY PLUGIN] ========== PLUGIN BUILD START ==========
[STARRY SKY PLUGIN] Internal shader asset loaded: <handle>
[STARRY SKY SPECIALIZE] Specializing pipeline for StarrySkyMaterial
[STARRY SKY PREPARE] night_factor = 1.0 - stars SHOULD BE VISIBLE
[STARRY SKY UPDATE] Updated X material(s)
```

#### Time System
```
[ZONE_TIME] ========== CURRENT TIME ==========
[ZONE_TIME]   tick: 128 / 160
[ZONE_TIME]   game time: 19:12
[ZONE_TIME]   state: Evening
[ZONE_TIME]   state_percent: 75.0%
```

#### Sky Settings Bridge
```
[SKY SETTINGS] Manual time enabled: 0.0 hours -> 0 ticks (day_cycle: 160)
[SKY SETTINGS] Automatic time enabled - following game time
```

#### Night Factor
```
[NIGHT_FACTOR_UPDATE] State is NIGHT -> night_factor = 1.0
[NIGHT_FACTOR_UPDATE] State is EVENING (2nd half, 75.0%) -> night_factor = 0.5
```

#### Atmosphere
```
[ATMOSPHERE] ✗ DISABLED atmosphere: true -> false (state: Night)
[ATMOSPHERE] ✓ ENABLED atmosphere: false -> true (state: Day)
```

### Shader Debug Modes

In `src/render/shaders/starry_sky.wgsl`, set `DEBUG_MODE`:

| Mode | Output | Purpose |
|------|--------|---------|
| 0 | Normal rendering | Production |
| 1 | Yellow fullscreen | Verify shader executes |
| 2 | Red gradient | Visualize night_factor |
| 3 | Green gradient | Visualize star calculation |
| 4 | Blue gradient | Visualize horizon check (dir.y) |
| 5 | Bright stars (5x) | Debug visibility issues |

### Common Issues & Solutions

#### Stars Not Visible

**Checklist:**
1. ✅ Is it night time? Check `[ZONE_TIME] state: Night`
2. ✅ Is atmosphere disabled? Check `[ATMOSPHERE] ✗ DISABLED`
3. ✅ Is night_factor = 1.0? Check `[STARRY SKY PREPARE] night_factor`
4. ✅ Is star_density > 0? Check `[STARRY SKY PREPARE] star_density`
5. ✅ Is shader compiling? Check for `[STARRY SKY SPECIALIZE]`

**Quick Test:**
- Set `DEBUG_MODE = 1` in shader
- If you see yellow, shader works but stars are hidden by logic
- If you see nothing, shader pipeline failed

#### Time Not Changing

**Checklist:**
1. ✅ Is SkySettings.mode = Manual?
2. ✅ Did you change the slider (triggers is_changed())?
3. ✅ Is CurrentZone resource present?
4. ✅ Check `[SKY SETTINGS] Manual time enabled` log

**Debug:**
```rust
// Add temporary log in apply_sky_settings_to_zone_time
log::info!("[DEBUG] sky_settings.is_changed() = {}", sky_settings.is_changed());
log::info!("[DEBUG] manual_time = {}", sky_settings.manual_time);
```

#### Stars Too Dim

**Solutions:**
1. Increase `star_brightness` in UI (0.0-5.0)
2. Increase individual layer brightness multipliers in shader
3. Check HDR/tone mapping settings (stars may be clipped)
4. Try `DEBUG_MODE = 5` for 5x brightness boost

---

## Historical Issues (Resolved)

### Issue #1: Shader Import Paths (February 27, 2026)

**Problem:** Bevy 0.16.1 changed shader import syntax

**Before:**
```wgsl
#import bevy_pbr::{
    mesh_functions,
    view_transformations::position_world_to_clip,
}
```

**After:**
```wgsl
#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_world, mesh_position_local_to_clip}
#import bevy_pbr::mesh_view_bindings view
```

**Impact:** Shader now compiles successfully

---

### Issue #2: Default night_factor Caused Permanent Night (February 27, 2026)

**Problem:** `StarrySkySettings::default()` had `night_factor: 1.0`

**Fix:** Changed to `night_factor: 0.0`

**Impact:** Day/night cycle now respects game time

---

### Issue #3: UI Settings Not Connected to ZoneTime (February 28, 2026)

**Problem:** `SkySettings.manual_time` was not bridging to `ZoneTime.debug_overwrite_time`

**Fix:** Added `apply_sky_settings_to_zone_time()` system in `zone_lighting.rs`

**Impact:** Manual time control now works correctly

---

### Issue #4: Low Star Count (February 28, 2026)

**Problem:** Default `star_density = 0.15` produced only ~1,000 stars

**Fix:** Changed to `star_density = 0.50` (~3,300 stars)

**Impact:** Much denser, more realistic star field

---

## Files to Review

| File | Purpose |
|------|---------|
| `src/render/shaders/starry_sky.wgsl` | Star generation shader |
| `src/render/starry_sky_material.rs` | Material, systems, settings |
| `src/render/zone_lighting.rs` | SkySettings, time bridge system |
| `src/systems/zone_time_system.rs` | Day/night cycle logic |
| `src/resources/zone_time.rs` | ZoneTime resource definition |
| `src/ui/ui_settings_system.rs` | UI controls (Sky and Stars tabs) |

## Bevy Source References

| File | Purpose |
|------|---------|
| `bevy-0.16.1/crates/bevy_pbr/src/atmosphere/node.rs` | Atmosphere rendering |
| `bevy-0.16.1/crates/bevy_pbr/src/render/mesh_functions.wgsl` | Mesh transformation functions |
| `bevy-0.16.1/crates/bevy_mesh/src/primitives/dim3/sphere.rs` | Sphere mesh generation |

---

## Summary

The starry sky system is a **fully integrated, production-ready feature** that:

1. **Generates 3,000+ stars procedurally** using efficient GPU-based grid sampling
2. **Respects day/night cycle** from game time or manual override
3. **Provides real-time UI controls** for all visual parameters
4. **Integrates cleanly with Bevy's atmosphere** by toggling it off at night
5. **Includes moon rendering** with phases and directional lighting

**For future engineers:**
- Star density is the primary tuning parameter (0.50 recommended)
- Time system uses `ZoneTime.debug_overwrite_time` for manual control
- Atmosphere must be removed (not faded) for stars to be visible
- All settings are exposed in the in-game Settings menu

---

*Last updated: February 28, 2026*
*ROSE Offline Client - Bevy 0.16.1*