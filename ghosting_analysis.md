# Ghosting Issue Analysis: Starry Sky & Model Overlap

**Date:** February 28, 2026  
**Bevy Version:** 0.16.1  
**Project:** ROSE Offline Client  
**Issue:** Ghosting artifacts when models overlap with the night sky

---

## Executive Summary

When models overlap with the starry night sky and the camera moves, ghosting artifacts appear where the model was previously positioned. The ghost appears as an "imprint" of the night sky that lingers in the scene. This is a **depth buffer and blending interaction issue** between the starry sky sphere and opaque/transparent objects in the scene.

---

## Problem Description

### Symptoms
- When a model overlaps with the starry sky background
- Moving the camera reveals ghosting artifacts
- The ghost appears as a residual imprint of the sky/stars
- Most noticeable with moving objects or camera rotation

### Expected Behavior
- Models should cleanly occlude the starry sky
- No residual artifacts when objects move
- Clean depth buffer state between frames

---

## Root Cause Analysis

### 1. Depth Buffer Configuration

**Current Starry Sky Configuration:**
```rust
// In starry_sky_material.rs::specialize()
depth_stencil.depth_write_enabled = false;  // ✓ Correct - sky shouldn't write depth
depth_stencil.depth_compare = CompareFunction::Always;  // ⚠️ PROBLEM
```

**The Issue:**
The starry sky uses `CompareFunction::Always`, which means:
- Sky fragments **always pass** the depth test
- Sky renders **regardless of depth value**
- Sky does NOT write to depth buffer (correct)

However, this creates a problem with the **render order and depth buffer state**:

1. **Opaque Pass**: Models write depth values to depth buffer
2. **Atmosphere Pass**: Reads depth buffer, renders fullscreen quad (no depth write)
3. **Transparent Pass**: Starry sky renders with `depth_compare = Always`

The ghosting occurs because:
- When a model moves, its **old depth values remain** in the depth buffer
- The starry sky with `Always` comparison renders **over everything**
- But the **blending equation** combines sky color with existing framebuffer content
- If the framebuffer wasn't fully cleared or if there's temporal accumulation, ghosts appear

### 2. Alpha Blending Configuration

**Current Configuration:**
```rust
fn alpha_mode(&self) -> AlphaMode {
    AlphaMode::Blend  // Returns Blend
}

// But in specialize():
color_target_state.blend = Some(BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::One,      // ⚠️ ADDITIVE BLENDING
        operation: BlendOperation::Add,
    },
    ...
});
```

**The Conflict:**
- `alpha_mode()` returns `AlphaMode::Blend`
- But `specialize()` overrides with **additive blending** (`SrcAlpha + One`)
- This mismatch can cause Bevy to sort the sky incorrectly in the transparent pass

**Additive Blending Formula:**
```
final_color = source_color * source_alpha + destination_color
```

When the sky renders with additive blending:
- Sky colors **add to** whatever is already in the framebuffer
- If a model was at position X in frame N, its contribution remains
- In frame N+1, when model moves to position Y, position X still has sky contribution
- This creates the "ghost" effect

### 3. Render Phase Sorting

**Bevy's Transparent3d Phase:**
- Objects are sorted **back-to-front** by distance
- Starry sky sphere is at world origin with radius 50,000
- Camera is at ~7,000 units from origin (inside sphere)
- Distance calculation: `distance = camera_to_object_distance`

**Sorting Issue:**
The sky sphere's distance for sorting is calculated from camera to sphere center (origin). This means:
- Sky might sort **before or after** other transparent objects
- If sky sorts before an object, that object should occlude it
- But with `depth_compare = Always`, sky renders regardless
- With additive blending, sky **adds to** the framebuffer even when it shouldn't

### 4. Depth Prepass Interaction

**SSAO Configuration (from lib.rs):**
```rust
ScreenSpaceAmbientOcclusion {
    quality_level: ScreenSpaceAmbientOcclusionQualityLevel::Medium,
    ...
}
```

**SSAO Requirements:**
- Requires `DepthPrepass` component on camera
- Requires `NormalPrepass` component on camera
- Reads depth and normal buffers for occlusion calculation

**Potential Conflict:**
SSAO runs in the prepass and generates occlusion data. If the starry sky:
- Renders in a different phase than expected
- Uses unusual depth comparison
- Contributes to the framebuffer unexpectedly

Then SSAO or other post-processing effects might sample incorrect depth/occlusion data, causing artifacts.

### 5. Temporal Effects (TAA, Motion Vectors)

**Current Configuration:**
```rust
Msaa::Off,  // Required for SSAO and TAA compatibility
```

If TAA (Temporal Anti-Aliasing) is enabled elsewhere:
- TAA accumulates frames over time
- Uses motion vectors to reproject previous frames
- Starry sky with `depth_compare = Always` might not have correct motion vectors
- This causes **temporal ghosting** where sky artifacts persist

---

## Bevy Render Pipeline Analysis

### Render Graph Order (from bevy source)

```
EarlyPrepass (depth/normal buffers for opaque objects)
    ↓
EndPrepasses
    ↓
StartMainPass
    ↓
MainOpaquePass (opaque objects write depth)
    ↓
Atmosphere::RenderSky (fullscreen quad, additive, reads depth)
    ↓
MainTransparentPass (sorted transparent objects, including starry sky)
    ↓
EndMainPass
    ↓
Post-processing (SSAO, Tonemapping, etc.)
```

### The Starry Sky's Position

From `starry_sky_material.rs`:
```rust
fn alpha_mode(&self) -> AlphaMode {
    AlphaMode::Blend  // Places in Transparent3d phase
}
```

This puts the starry sky in `MainTransparentPass`, which:
1. Loads (but doesn't clear) the depth buffer
2. Stores the depth buffer at the end
3. Sorts objects back-to-front
4. Uses standard depth comparison (unless overridden)

### The Conflict

The starry sky overrides depth comparison to `Always`, but:
- It still participates in the transparent pass sorting
- Other transparent objects expect normal depth testing
- The framebuffer accumulation from additive blending persists

---

## Contributing Factors

### Factor 1: Sky Sphere Geometry

**Current Setup:**
- Sphere radius: 50,000 units
- Centered at world origin (0, 0, 0)
- Camera at ~7,000 units from origin

**Issue:**
The sphere is a **3D mesh** in world space, not a true skybox. When the camera moves:
- Different parts of the sphere are at different distances
- Depth values across the sphere vary significantly
- With `depth_compare = Always`, this variation doesn't matter for rendering
- But it DOES matter for sorting in the transparent pass

### Factor 2: Additive Blending with Non-Zero Background

**Additive blending assumes:**
- Background is black (0, 0, 0)
- Adding colors creates the effect

**Reality:**
- Background contains rendered scene (models, terrain, etc.)
- Adding sky colors to scene colors creates incorrect results
- When objects move, the "added" sky colors remain as ghosts

### Factor 3: Alpha Channel Usage

**Shader returns:**
```wgsl
return vec4<f32>(final_color, night_factor);
```

**Issues:**
- `night_factor` ranges from 0.0 (day) to 1.0 (night)
- During transitions, alpha is between 0 and 1
- With additive blending: `src * src_alpha + dst`
- At `alpha = 0.5`, you get: `sky_color * 0.5 + existing_color`
- This partial addition can leave residual colors

### Factor 4: No Depth Write with Always Compare

**Current:**
```rust
depth_write_enabled = false;
depth_compare = CompareFunction::Always;
```

**Consequence:**
- Sky never updates depth buffer
- Sky always renders, even "behind" objects that should occlude it
- Relies entirely on blending to create correct appearance
- Blending with additive operation is not sufficient for proper occlusion

---

## Solutions

### Solution 1: Use Proper Depth Comparison (RECOMMENDED)

**Change:**
```rust
// In specialize():
depth_stencil.depth_write_enabled = false;
depth_stencil.depth_compare = CompareFunction::LessEqual;  // Changed from Always
```

**Why it works:**
- Sky respects depth buffer written by opaque objects
- Models properly occlude the sky
- No ghosting because sky doesn't render where objects are
- Standard behavior for skyboxes

**Trade-offs:**
- Sky sphere must be at correct depth (far plane)
- May need to adjust sphere radius or position
- Could cause issues if sphere intersects with scene geometry

**Implementation:**
```rust
fn specialize(...) {
    // ... existing code ...
    
    if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
        depth_stencil.depth_write_enabled = false;
        depth_stencil.depth_compare = CompareFunction::LessEqual;  // Changed
    }
}
```

---

### Solution 2: Use Skybox Component Instead of Sphere

**Change:** Replace the sphere mesh with Bevy's built-in `Skybox` component.

**Why it works:**
- Skybox is rendered as a **fullscreen quad** in camera space
- Always at infinite distance
- Properly integrated with Bevy's render pipeline
- No depth buffer conflicts

**Implementation:**
```rust
use bevy::pbr::Skybox;

// Replace starry sky sphere with:
commands.spawn((
    Camera,
    Camera3d,
    // ... other camera components ...
    Skybox {
        image: skybox_image_handle,  // Procedurally generated or texture
        intensity: 1.0,
    },
));
```

**Trade-offs:**
- Requires rewriting starry sky as a skybox shader
- Loss of 3D sphere geometry (probably fine for sky)
- More aligned with Bevy best practices

**Extended Implementation:**
Create a custom skybox material that renders the procedural stars:
```rust
// New skybox shader that replaces starry_sky.wgsl
// Uses camera-space direction instead of world-space position
@fragment
fn fragment(in: SkyboxVertexOutput) -> @location(0) vec4<f32> {
    let dir = in.direction;  // Already normalized, camera-relative
    // ... same star generation code ...
}
```

---

### Solution 3: Change Blending Mode

**Change:**
```rust
// In specialize():
color_target_state.blend = Some(BlendState {
    color: BlendComponent {
        src_factor: BlendFactor::SrcAlpha,
        dst_factor: BlendFactor::OneMinusSrcAlpha,  // Changed from One
        operation: BlendOperation::Add,
    },
    // ...
});
```

**Why it works:**
- Standard alpha blending: `src * src_alpha + dst * (1 - src_alpha)`
- When sky is fully opaque (`alpha = 1.0`): shows only sky
- When sky is transparent (`alpha = 0.0`): shows only background
- No accumulation of sky colors when objects move

**Trade-offs:**
- Stars won't have additive glow effect
- May need to adjust star brightness
- Less "dreamy" appearance

**Hybrid Approach:**
Use standard blending for the sky background, additive only for stars:
```wgsl
// In shader:
let background = nebula_background(dir);  // Standard blend
let stars = star_layers(...) * night_factor;  // Could use separate pass with additive
```

---

### Solution 4: Render Sky Before Opaque Pass

**Change:** Move starry sky rendering to run before `MainOpaquePass`.

**Why it works:**
- Sky renders first, establishing background
- Opaque objects write depth and fully occlude sky
- No blending conflicts
- Standard skybox rendering order

**Implementation:**
Create a custom render graph node:
```rust
// In lib.rs or new module:
app.add_render_graph_node::<ViewNodeRunner<StarrySkyNode>>(
    Core3d,
    Node3d::StarrySky,  // Custom node
)
.add_render_graph_edges(
    Core3d,
    (
        Node3d::StartMainPass,
        Node3d::StarrySky,      // Custom
        Node3d::MainOpaquePass,
    ),
);
```

**Trade-offs:**
- More complex render graph setup
- Requires custom node implementation
- Sky must be rendered as fullscreen quad for efficiency

---

### Solution 5: Use Depth Bias to Push Sky Back

**Change:**
```rust
fn depth_bias(&self) -> f32 {
    -1000.0  // Large negative bias to push sky to far plane
}

// And use proper depth comparison:
depth_stencil.depth_compare = CompareFunction::LessEqual;
```

**Why it works:**
- Depth bias modifies depth values during rasterization
- Large negative bias pushes sky fragments to far plane
- Sky renders behind all scene geometry
- Combined with `LessEqual`, sky only shows where no objects are

**Trade-offs:**
- Depth bias can cause artifacts at extreme values
- May not work with all depth buffer configurations
- Less reliable than other solutions

---

### Solution 6: Disable Additive Blending During Day/Transition

**Change:**
```rust
// In specialize(), make blending dynamic based on night_factor:
// (Requires custom pipeline key or material update)

// Or in shader:
if (night_factor < 0.95) {
    // Use standard blending during transitions
    return vec4<f32>(final_color * night_factor, night_factor);
} else {
    // Use additive-like effect only at full night
    // by pre-multiplying colors
    return vec4<f32>(final_color, 1.0);
}
```

**Why it works:**
- Avoids additive blending issues during transitions
- Full night can use special rendering
- Reduces ghosting during most of the day/night cycle

**Trade-offs:**
- More complex shader logic
- May still have issues at full night
- Doesn't address root cause

---

### Solution 7: Clear Color Buffer Before Sky Render

**Change:** Ensure framebuffer is cleared before sky renders.

**Why it works:**
- Additive blending on black background works correctly
- No residual colors from previous frames
- Eliminates ghosting from framebuffer accumulation

**Implementation:**
```rust
// In camera setup:
ClearColorConfig::Color(Color::srgb(0.0, 0.0, 0.0))  // Ensure black clear
```

**Trade-offs:**
- May affect other rendering (UI, overlays)
- Doesn't fix the fundamental depth/blending issue
- Band-aid solution

---

### Solution 8: Two-Pass Sky Rendering

**Change:** Split sky into two materials/passes:

1. **Background Pass** (standard blend, depth test):
   - Nebula, gradient background
   - `depth_compare = LessEqual`
   - `blend = SrcAlpha, OneMinusSrcAlpha`

2. **Stars Pass** (additive blend, no depth write):
   - Stars only
   - `depth_compare = Always` (or LessEqual)
   - `blend = SrcAlpha, One` (additive)

**Why it works:**
- Background properly occluded by depth test
- Stars add glow effect without ghosting
- Separation of concerns

**Implementation:**
```rust
// Two separate entities or two materials on same mesh
// Requires multi-draw or separate mesh instances
```

**Trade-offs:**
- Two render passes (performance cost)
- More complex setup
- Best visual quality

---

## Recommended Solution

### Primary Recommendation: **Solution 1 + Solution 3**

**Combine proper depth comparison with standard blending:**

1. **Change depth comparison to `LessEqual`:**
   ```rust
   depth_stencil.depth_compare = CompareFunction::LessEqual;
   ```

2. **Change blending to standard alpha blend:**
   ```rust
   color_target_state.blend = Some(BlendState {
       color: BlendComponent {
           src_factor: BlendFactor::SrcAlpha,
           dst_factor: BlendFactor::OneMinusSrcAlpha,
           operation: BlendOperation::Add,
       },
       // ...
   });
   ```

3. **Adjust shader to compensate:**
   ```wgsl
   // Increase star brightness to compensate for non-additive blending
   let total_stars = distant_stars * 1.5 + medium_stars * 1.5 + ...;
   ```

**Why this combination:**
- Fixes the root cause (depth buffer interaction)
- Eliminates ghosting completely
- Minimal code changes
- Maintains most of the visual appearance
- Follows Bevy best practices

### Secondary Recommendation: **Solution 2 (Skybox)**

For a long-term, maintainable solution, migrate to Bevy's skybox system:
- Properly integrated with render pipeline
- No depth buffer issues
- Better performance (fullscreen quad vs large sphere)
- Future-proof as Bevy evolves

---

## Testing Recommendations

### Test Case 1: Basic Ghosting Test
1. Set time to night
2. Place a model in front of the sky
3. Move camera left/right
4. **Expected:** No ghosting artifacts
5. **Current:** Ghost imprint remains

### Test Case 2: Object Movement Test
1. Set time to night
2. Animate a model moving across the screen
3. **Expected:** Clean movement, no trails
4. **Current:** Sky ghost trails behind

### Test Case 3: Day/Night Transition Test
1. Cycle through day/night transition
2. Observe sky blending
3. **Expected:** Smooth transition, no artifacts
4. **Current:** May show blending issues

### Test Case 4: Depth Buffer Visualization
1. Add depth buffer visualization shader
2. Observe sky sphere depth values
3. **Expected:** Consistent far-plane depth
4. **Current:** Varying depth across sphere

---

## Performance Considerations

| Solution | Performance Impact | Complexity |
|----------|-------------------|------------|
| 1. Depth Compare Change | None | Low |
| 2. Skybox Migration | Improved | High |
| 3. Blending Change | None | Low |
| 4. Render Order Change | Neutral | High |
| 5. Depth Bias | None | Low |
| 6. Dynamic Blending | None | Medium |
| 7. Clear Buffer | None | Low |
| 8. Two-Pass | -5% (extra pass) | Medium |

---

## Conclusion

The ghosting issue is caused by the interaction between:
1. **`depth_compare = Always`** - Sky renders regardless of depth
2. **Additive blending** - Sky colors accumulate in framebuffer
3. **Transparent pass sorting** - Sky may not sort correctly

The recommended fix is to use **proper depth comparison (`LessEqual`)** combined with **standard alpha blending**. This will ensure models correctly occlude the sky and eliminate ghosting artifacts while maintaining the visual appearance of the starry night sky.

For a long-term solution, consider migrating to Bevy's **skybox system**, which is designed for this exact use case and avoids these depth buffer complications entirely.

---

*Document prepared for ROSE Offline Client development team*  
*Bevy 0.16.1, February 28, 2026*