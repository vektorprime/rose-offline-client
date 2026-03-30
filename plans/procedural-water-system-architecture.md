# Procedural Water System Architecture

## Executive Summary

This document outlines the architecture for replacing the current texture-based water system with a procedurally generated water system that features depth, realistic rendering, and improved visual quality.

## Current Water System Analysis

### Existing Implementation

The current water system (`src/render/water_material.rs` and `src/render/shaders/water_material.wgsl`) uses:

1. **25 Animated Texture Frames** - Pre-rendered water textures (`3DDATA/JUNON/WATER/OCEAN01_01.DDS` through `OCEAN01_25.DDS`)
2. **Texture Array Binding** - Uses `binding_array<texture_2d<f32>, 25>` for frame animation
3. **Additive Blending** - `SrcAlpha + One` blend mode for water transparency
4. **Procedural Wave Normals** - Already implemented for dynamic surface detail
5. **Fresnel Effect** - Angle-dependent reflectivity
6. **Subsurface Scattering** - Light penetration approximation
7. **Foam Effects** - Organic noise-based foam on wave crests
8. **Zone Fog Integration** - Blends with scene fog

### Current Strengths

- Already has procedural wave normals
- Has foam and SSS effects
- Integrates with zone lighting
- Has underwater post-processing effect

### Current Limitations

1. **No Depth Variation** - Water is a flat plane with no depth-based color variation
2. **Texture-Dependent** - Relies on pre-rendered textures instead of procedural generation
3. **No Shoreline Depth Gradient** - Water doesn't get shallower near edges
4. **No Bottom Visibility** - Cannot see through shallow water to the bottom
5. **Limited Refraction** - Uses pseudo-refraction via UV distortion only
6. **No Caustics on Surfaces** - Light patterns don't project onto nearby surfaces

## Proposed Procedural Water System

### Design Goals

1. **Procedural Generation** - Generate water appearance procedurally instead of using textures
2. **Depth-Based Rendering** - Water color and transparency vary with depth
3. **Shoreline Integration** - Smooth transition from deep to shallow water
4. **Bottom Visibility** - See through shallow water to terrain below
5. **Improved Refraction** - Better light bending simulation
6. **Dynamic Depth Map** - Procedural depth variation across water surface

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Water System                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────┐    ┌──────────────────┐                  │
│  │  WaterMaterial   │    │  WaterSettings   │                  │
│  │  (Custom Material)│    │  (Resource)      │                  │
│  └────────┬─────────┘    └────────┬─────────┘                  │
│           │                       │                             │
│           ▼                       ▼                             │
│  ┌──────────────────────────────────────────────┐              │
│  │         Water Shader (WGSL)                  │              │
│  │  ┌────────────────────────────────────────┐  │              │
│  │  │ Vertex Shader                          │  │              │
│  │  │ - Displace vertices by wave height     │  │              │
│  │  │ - Calculate wave normals               │  │              │
│  │  └────────────────────────────────────────┘  │              │
│  │  ┌────────────────────────────────────────┐  │              │
│  │  │ Fragment Shader                        │  │              │
│  │  │ - Procedural water color generation    │  │              │
│  │  │ - Depth-based color gradient           │  │              │
│  │  │ - Bottom visibility in shallow water   │  │              │
│  │  │ - Improved refraction                  │  │              │
│  │  │ - Dynamic foam and splash              │  │              │
│  │  │ - Caustics projection                  │  │              │
│  │  └────────────────────────────────────────┘  │              │
│  └──────────────────────────────────────────────┘              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Component Design

#### 1. Enhanced WaterSettings Resource

```rust
#[derive(Resource, Debug, Clone)]
pub struct WaterSettings {
    // Existing settings (keep for compatibility)
    pub foam_intensity: f32,
    pub foam_threshold: f32,
    pub sss_intensity: f32,
    pub refraction_strength: f32,
    pub wave_speed: f32,
    pub fresnel_strength: f32,
    pub specular_intensity: f32,
    pub water_surface_y: f32,
    
    // New depth-related settings
    pub min_depth: f32,        // Minimum water depth (meters)
    pub max_depth: f32,        // Maximum water depth (meters)
    pub shallow_threshold: f32, // Depth below which bottom is visible
    pub deep_color: Vec4,      // Color of deep water
    pub shallow_color: Vec4,   // Color of shallow water
    pub bottom_visibility: f32, // How much bottom shows through (0-1)
    pub depth_gradient_scale: Vec2, // Scale for depth variation pattern
    
    // New wave settings
    pub wave_amplitude: f32,   // Height of waves
    pub wave_frequency: f32,   // How many waves per unit distance
    pub wave_layers: u32,      // Number of wave layers for complexity
    
    // New caustics settings
    pub caustics_intensity: f32,
    pub caustics_scale: f32,
    pub caustics_speed: f32,
}
```

#### 2. Procedural Water Shader Features

**Vertex Shader:**
- Displace vertices based on procedural wave height
- Calculate per-vertex wave normals
- Pass depth information to fragment shader

**Fragment Shader:**
- **Procedural Color Generation**: Generate water color using layered sine waves and noise
- **Depth Gradient**: Interpolate between shallow and deep colors based on position
- **Bottom Sampling**: Sample depth texture to see terrain through shallow water
- **Improved Refraction**: Use wave normals to distort view direction
- **Dynamic Foam**: Enhanced foam based on wave steepness
- **Caustics**: Project light patterns based on wave normals

#### 3. Depth Map Generation

Two approaches for depth information:

**Option A: Procedural Depth Map**
- Generate depth based on distance from water center
- Use noise to create natural depth variation
- Store in a uniform buffer or texture

**Option B: Terrain-Based Depth**
- Sample terrain height at water position
- Calculate water depth as difference between water surface and terrain
- Requires access to terrain mesh or heightmap

### Implementation Plan

#### Phase 1: Core Procedural Water (Week 1)

1. **Remove Texture Dependency**
   - Replace texture array with procedural color generation
   - Use layered sine waves for water pattern
   - Add noise for organic variation

2. **Depth-Based Color**
   - Implement depth gradient from shallow to deep
   - Add configurable shallow/deep colors
   - Create smooth color transitions

3. **Procedural Wave Height**
   - Enhance existing wave functions
   - Add multiple wave layers
   - Configure amplitude and frequency

#### Phase 2: Bottom Visibility (Week 2)

1. **Depth Texture Access**
   - Access scene depth in water shader
   - Calculate distance to terrain below water

2. **Shallow Water Transparency**
   - Increase transparency in shallow areas
   - Blend water color with bottom color

3. **Refraction Improvements**
   - Use proper refraction based on depth
   - Distort bottom view through water

#### Phase 3: Advanced Effects (Week 3)

1. **Caustics Projection**
   - Calculate light focusing through waves
   - Project caustic patterns on nearby surfaces

2. **Shoreline Effects**
   - Enhanced foam at water edges
   - Wet sand/beach transition

3. **Performance Optimization**
   - LOD for wave complexity
   - Distance-based detail reduction

### File Structure

```
src/
├── render/
│   ├── water_material.rs          # Modified - remove texture array
│   ├── water_material.wgsl        # Modified - procedural generation
│   └── shaders/
│       └── water_material.wgsl    # Main shader (enhanced)
├── resources/
│   └── water_settings.rs          # Modified - add depth settings
└── systems/
    └── water_system.rs            # New - procedural water updates
```

### Shader Implementation Details

#### Procedural Water Color Generation

```wgsl
// Generate procedural water color using layered waves
fn generate_water_color(position: vec2<f32>, time: f32, depth: f32) -> vec4<f32> {
    // Layered sine waves for base pattern
    var color = vec3<f32>(0.0);
    var amplitude = 1.0;
    var frequency = 1.0;
    
    for (var i = 0; i < 4; i++) {
        let wave = sin(position * frequency + time * 0.5);
        color += wave * amplitude * vec3<f32>(0.1, 0.3, 0.5); // Blue-green tint
        frequency *= 2.0;
        amplitude *= 0.5;
    }
    
    // Depth-based color gradient
    let depth_factor = saturate((depth - SHALLOW_DEPTH) / (DEEP_DEPTH - SHALLOW_DEPTH));
    let final_color = mix(SHALLOW_COLOR.rgb, DEEP_COLOR.rgb, depth_factor);
    
    // Combine procedural pattern with depth color
    final_color += color * 0.3;
    
    return vec4<f32>(final_color, DEEP_COLOR.a * (1.0 - depth_factor * 0.5));
}
```

#### Bottom Visibility

```wgsl
// Sample bottom through shallow water
fn sample_bottom(world_position: vec3<f32>, depth: f32, view_dir: vec3<f32>) -> vec3<f32> {
    // Only show bottom in shallow water
    let visibility = saturate(1.0 - (depth / SHALLOW_THRESHOLD));
    if (visibility < 0.1) {
        return vec3<f32>(0.0); // No bottom visible in deep water
    }
    
    // Refract view direction to sample bottom
    let eta = 1.0 / 1.33; // Air to water refractive index
    let refracted_dir = refract(view_dir, normalize(vec3<f32>(0.0, 1.0, 0.0)), eta);
    
    // Calculate bottom position
    let bottom_pos = world_position - refracted_dir.y * depth;
    
    // Sample terrain color at bottom position (requires terrain texture or lookup)
    let bottom_color = sample_terrain_color(bottom_pos.xz);
    
    return bottom_color * visibility;
}
```

### Integration with Existing Systems

1. **Zone Loader** - Continue spawning water planes, but with depth parameters
2. **Fish System** - Fish can use depth information for realistic swimming
3. **Underwater Effect** - Integrate depth for better underwater rendering
4. **Settings UI** - Add new water settings controls

### Performance Considerations

1. **Wave Layer LOD** - Reduce wave layers based on distance from camera
2. **Bottom Sampling** - Only sample bottom in shallow areas
3. **Caustics** - Optional, can be disabled for performance
4. **Shader Complexity** - Profile and optimize expensive operations

### Testing Checklist

- [ ] Water renders without texture files
- [ ] Depth gradient visible across water surface
- [ ] Shallow water shows bottom
- [ ] Deep water is opaque and dark
- [ ] Waves animate smoothly
- [ ] Foam appears on wave crests
- [ ] Refraction distorts view correctly
- [ ] Performance is acceptable (60 FPS target)
- [ ] Works with existing underwater effect
- [ ] Fish system still functions

### Migration Path

1. **Keep Existing System** - Don't remove texture-based water immediately
2. **Add Procedural Option** - Implement as alternative material
3. **Test Thoroughly** - Ensure no regressions
4. **Replace Gradually** - Switch zones to procedural water one at a time
5. **Remove Old Code** - Clean up texture loading after full migration

### References

- Current implementation: `src/render/water_material.rs`
- Current shader: `src/render/shaders/water_material.wgsl`
- Underwater effect: `src/render/underwater_effect.rs`
- Bevy 0.18 shader guide: https://bevyengine.org/learn/book/shaders-and-materials/