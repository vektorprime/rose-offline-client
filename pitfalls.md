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

---

## Terrain Adherence Bug - Player Could Not Descend Below Spawn Height (Fixed 2026-02-18)

### Problem
Player character could ascend terrain slopes but could not descend below the elevation coordinate it spawned at. The spawn height was effectively treated as a minimum floor. Additionally, NPCs and monsters were spawning at extremely high elevation (y=9702m) instead of at terrain level.

### Root Causes
1. **Zone Asset Timing**: In `zone_loader.rs`, the zone asset was being sent via `ZoneLoadedFromVfsEvent` BEFORE being added to the `Assets<ZoneLoaderAsset>` collection. This meant `collision_player_system` couldn't access terrain height data when trying to compute ground position.

2. **NPC/Monster Spawn Height**: Server sends `position.z = 0.00` for NPCs/monsters, and spawn code was using arbitrary `+ 10000.0` offset instead of querying terrain height from the zone heightmap.

3. **Bevy Bundle Tuple Limit**: Bevy's `Bundle` trait implementation has a maximum of ~15 components per tuple. Spawn handlers exceeded this limit, causing compilation errors.

### Solution
1. **Zone Asset Timing**: Moved `zone_loader_assets.add(zone_asset)` to occur BEFORE sending `ZoneLoadedFromVfsEvent`.

2. **Terrain Height Helper**: Added `get_spawn_height_from_world()` function that accesses `CurrentZone` and `Assets<ZoneLoaderAsset>` from the World to query terrain height from the zone heightmap.

3. **Deferred Spawning with Split Inserts**: Used `commands.queue()` with closures to spawn entities, splitting component inserts into two phases to avoid tuple size limits:
```rust
// Phase 1: Spawn with core components (~13)
let entity = world.spawn((...core components...)).id();

// Phase 2: Add remaining components
world.entity_mut(entity).insert((...remaining components...));
```

### Files Modified
- `src/zone_loader.rs` (lines 1177-1183) - Zone asset added to Assets before event sent
- `src/systems/game_connection_system.rs` - Added terrain height helper, split spawn handlers
- `src/systems/collision_system.rs` - Re-enabled `collision_height_only_system`

### Key Code Pattern for Deferred Spawning with World Access
```rust
commands.queue(move |world: &mut World| {
    // Get terrain height at spawn position
    let spawn_y = get_spawn_height_from_world(world, position.x, position.y);
    
    // Spawn with core components first
    let entity = world.spawn((
        // ... core components (up to ~14)
    )).id();

    // Add remaining components in a second insert
    world.entity_mut(entity).insert((
        // ... remaining components
        Transform::from_xyz(position.x / 100.0, spawn_y, -position.y / 100.0),
    ));
});
```

### Lesson Learned
1. When using `commands.queue()` with closures, you can access World resources directly via `world.get_resource::<T>()`
2. Bevy's Bundle trait implementation limits tuples to ~15 components - split large spawns into multiple `insert()` calls
3. Zone/level data must be added to Assets collection BEFORE events trigger systems that depend on that data
4. Server position data (especially z/height) may be unreliable - use client-side terrain heightmap for ground placement

---

## Custom Terrain Material with Texture Arrays (Fixed 2026-02-18)

### Problem
Terrain was rendering with only a single texture instead of supporting multiple tile textures with proper blending. The custom `TerrainMaterial` implementation from Bevy 0.11 was broken due to significant API changes in Bevy 0.15.

### Root Causes
1. **AsBindGroup API Changes**: Bevy 0.15 changed `AsBindGroup` trait to require:
   - `as_bind_group()` method instead of the old pattern
   - `unprepared_bind_group()` method (required even if you override `as_bind_group`)
   - `bind_group_layout_entries()` returns `Vec<BindGroupLayoutEntry>` instead of creating layout directly

2. **Material Bind Group Index**: Materials now use bind group index 2 (changed from index 1 in Bevy 0.11)

3. **Shader Import Changes**:
   - `mesh.model` no longer exists - use `get_world_from_local(instance_index)` instead
   - Must add `@builtin(instance_index) instance_index: u32` to vertex struct
   - Import path changed: `#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}`

4. **Texture View Type Mismatch**: Bevy's `TextureView` wraps `wgpu::TextureView` - must use `&*view` (deref) to get raw wgpu type for `BindingResource::TextureViewArray`

5. **Asset Registration**: Must use `app.init_asset::<TerrainMaterial>()` with `AssetApp` trait in scope

### Solution
1. Implemented `AsBindGroup` with overridden `as_bind_group()` to create texture array bind groups
2. Updated shader to use `@group(2)` for material bindings
3. Updated shader to use `get_world_from_local(instance_index)` instead of `mesh.model`
4. Used `&*texture_view` deref pattern to get raw wgpu types

### Key Code Patterns for Bevy 0.15 Custom Materials

**Rust - AsBindGroup implementation:**
```rust
impl AsBindGroup for TerrainMaterial {
    type Data = TerrainMaterialKey;
    type Param = (SRes<RenderAssets<GpuImage>>, SRes<FallbackImage>);

    fn as_bind_group(
        &self,
        layout: &BindGroupLayout,
        render_device: &RenderDevice,
        (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
    ) -> Result<PreparedBindGroup<Self::Data>, AsBindGroupError> {
        use std::ops::Deref;
        
        // Collect textures and deref to wgpu types
        let fallback_view = &*fallback_image.d2.texture_view;
        let mut textures: Vec<&_> = vec![fallback_view; MAX_TEXTURES];
        for (id, image) in images.into_iter().enumerate() {
            textures[id] = &*image.texture_view;  // Deref!
        }
        
        let bind_group = render_device.create_bind_group(Self::label(), layout, &entries);
        Ok(PreparedBindGroup { bindings: vec![], bind_group, data: key })
    }

    fn unprepared_bind_group(...) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
        Err(AsBindGroupError::RetryNextUpdate)  // Stub - we override as_bind_group
    }

    fn bind_group_layout_entries(_: &RenderDevice) -> Vec<BindGroupLayoutEntry> {
        vec![
            BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::FRAGMENT,
                ty: BindingType::Texture { sample_type: TextureSampleType::Float { filterable: true }, view_dimension: TextureViewDimension::D2, multisampled: false },
                count: NonZeroU32::new(MAX_TEXTURES as u32),  // Texture array!
            },
            // ... sampler binding
        ]
    }
}
```

**WGSL - Vertex shader with instance_index:**
```wgsl
#import bevy_pbr::mesh_functions::{get_world_from_local, mesh_position_local_to_clip}

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    // ...
}

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    let world_from_local = get_world_from_local(vertex.instance_index);
    out.clip_position = mesh_position_local_to_clip(world_from_local, vec4<f32>(vertex.position, 1.0));
    return out;
}
```

**WGSL - Material bind group at index 2:**
```wgsl
@group(2) @binding(0)
var tile_array_texture: binding_array<texture_2d<f32>, 100>;
@group(2) @binding(1)
var tile_array_sampler: sampler;
```

### Files Modified
- `src/render/terrain_material.rs` - Full AsBindGroup and Material implementation
- `src/render/mod.rs` - TERRAIN_MESH_ATTRIBUTE_TILE_INFO definition
- `src/render/shaders/terrain_material.wgsl` - Updated for Bevy 0.15 API
- `src/zone_loader.rs` - spawn_terrain() uses TerrainMaterial

### Lesson Learned
1. Bevy 0.15 materials use bind group index 2 (not 1)
2. Must implement both `as_bind_group()` AND `unprepared_bind_group()` (latter can be stub)
3. Use `get_world_from_local(instance_index)` instead of `mesh.model` in shaders
4. Bevy wrapper types (TextureView, Sampler, etc.) must be dereferenced with `&*` to get raw wgpu types
5. Import `AssetApp` trait for `app.init_asset::<T>()` to work
6. Texture arrays require `count: NonZeroU32::new(N)` in bind group layout

---

## MaterialExtension Shaders Cannot Access Custom Bind Groups (2026-02-18)

### Problem
Attempted to import `zone_lighting` module in `rose_object_extension.wgsl` to apply zone ambient color. This caused runtime crash:
```
Error matching ShaderStages(FRAGMENT) shader requirements against the pipeline
Shader global ResourceBinding { group: 3, binding: 0 } is not available in the pipeline layout
```

### Root Cause
`MaterialExtension` shaders use Bevy's standard `MaterialPlugin` pipeline layout, which only includes:
- Group 0: View uniforms
- Group 1: Mesh uniforms  
- Group 2: Material uniforms

The zone lighting bind group at group 3 is NOT included in the standard pipeline layout. Only custom materials with specialized pipeline setup (like `world_ui.rs`) can access additional bind groups.

### Solution
Do NOT import modules with `@group(3)+` bind groups in `MaterialExtension` shaders. Instead:
1. Use only the extension's own bindings (100+) at group 2
2. Pass any needed data through the extension's uniform/texture bindings
3. Or create a fully custom Material with specialized pipeline layout

### Files Modified
- `src/render/shaders/rose_object_extension.wgsl` - Removed zone_lighting import

### Lesson Learned
`ExtendedMaterial<StandardMaterial, T>` shaders cannot access bind groups beyond the standard PBR pipeline layout (groups 0-2). Custom bind groups require a full custom `Material` implementation with specialized pipeline setup, not just a `MaterialExtension`.

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

---

## Water Material Not Rendering (Fixed 2026-02-19)

### Problem
Water planes were not visible in the game after porting from Bevy 0.11 to Bevy 0.15.

### Root Cause
The water shader (`water_material.wgsl`) was using `view.time` for animation, but in Bevy 0.15.4, the `View` struct no longer has a `time` field. Time is now stored in a separate `Globals` struct accessed via `globals.time`.

**Before (Bevy 0.11):**
```wgsl
#import bevy_pbr::mesh_view_bindings view
// ...
let time = view.time * 10.0;
```

**After (Bevy 0.15.4):**
```wgsl
#import bevy_pbr::mesh_view_bindings::{view, globals}
// ...
let time = globals.time * 10.0;
```

### Solution
1. Updated shader import to include `globals` from `mesh_view_bindings`
2. Changed `view.time` to `globals.time`
3. Changed `view.inverse_view` to `view.view_from_world` (the correct field name in Bevy 0.15.4)

### Files Modified
- `src/render/shaders/water_material.wgsl` - Updated shader imports and time access

### Key Changes in Bevy 0.15.4 WGSL API
| Bevy 0.11 | Bevy 0.15.4 |
|-----------|-------------|
| `view.time` | `globals.time` |
| `view.inverse_view` | `view.view_from_world` |
| `#import bevy_pbr::mesh_view_bindings view` | `#import bevy_pbr::mesh_view_bindings::{view, globals}` |

### Lesson Learned
When porting custom shaders between Bevy versions, check the WGSL struct definitions in the Bevy source code:
- `crates/bevy_render/src/view/view.wgsl` - View struct definition
- `crates/bevy_render/src/globals.wgsl` - Globals struct definition
- `crates/bevy_pbr/src/render/mesh_view_bindings.wgsl` - Available bindings

---

## Water Not Rendering After Bevy 0.16.1 Migration (Fixed 2026-02-22)

### Problem
Water was not loading/rendering at all in the game after upgrading from Bevy 0.15.4 to Bevy 0.16.1.

### Root Cause
Breaking API change in Bevy 0.16's `AsBindGroup` trait. The migration guide states:
> "Bevy will now unconditionally call `AsBindGroup::unprepared_bind_group` for your materials, so you must no longer panic in that function. Instead, return the new `AsBindGroupError::CreateBindGroupDirectly` error, and Bevy will fall back to calling `AsBindGroup::as_bind_group` as before."

### Solution
Changed the return value in `unprepared_bind_group()` at [`src/render/water_material.rs:325`](src/render/water_material.rs:325):

```rust
// Before (broken - infinite retry loop):
Err(AsBindGroupError::RetryNextUpdate)

// After (fixed):
Err(AsBindGroupError::CreateBindGroupDirectly)
```

### Why It Works
`CreateBindGroupDirectly` tells Bevy "I implement `as_bind_group()` directly, call that instead." This allows the water material's custom bind group creation with texture arrays to work properly.

### Files Modified
- `src/render/water_material.rs` (line 325) - Changed `RetryNextUpdate` to `CreateBindGroupDirectly`

### Lesson Learned
When implementing custom materials with `AsBindGroup::as_bind_group()` override in Bevy 0.16+:
1. Always return `Err(AsBindGroupError::CreateBindGroupDirectly)` from `unprepared_bind_group()` - this signals Bevy to use your custom `as_bind_group()` implementation
2. Never return `RetryNextUpdate` from `unprepared_bind_group()` in Bevy 0.16+ - it causes an infinite retry loop since Bevy now calls this method unconditionally

---

## Fish Not Appearing in Water (Fixed 2026-02-19)

### Problem
Fish were not appearing in water areas despite the fish spawning system being implemented and events being sent correctly.

### Root Cause
Fish entities were spawning at local water coordinates but were **not parented to the zone entity**. Since zones have a transform offset of `(5200.0, 0.0, -5200.0)`, the fish were appearing at incorrect world positions.

For example:
- Fish local position: `(410.0, -8.0, 0.0)`
- Expected world position: `(5610.0, -8.0, 0.0)` (local + zone offset)
- Actual world position: `(410.0, -8.0, 0.0)` (no parent, so no transform inheritance)

### Solution
1. Added `zone_entity: Entity` field to `WaterSpawnedEvent` struct
2. Updated `spawn_fish_in_water()` function to accept `zone_entity` parameter
3. Added parenting: `commands.entity(zone_entity).add_child(fish_entity);`
4. Updated `zone_loader.rs` to pass `zone_entity` when sending the event

### Code Changes
```rust
// WaterSpawnedEvent - added zone_entity field
pub struct WaterSpawnedEvent {
    pub water_entity: Entity,
    pub zone_entity: Entity,  // NEW: Required for transform inheritance
    pub water_center: Vec3,
    pub water_half_extents: Vec2,
}

// spawn_fish_in_water - parent fish to zone
fn spawn_fish_in_water(
    water_entity: Entity,
    zone_entity: Entity,  // NEW parameter
    water_center: Vec3,
    // ...
) {
    // ... spawn fish_entity ...
    
    // Parent fish to zone entity so it inherits zone transform
    commands.entity(zone_entity).add_child(fish_entity);
}

// zone_loader.rs - pass zone_entity in event
water_spawned_events.send(WaterSpawnedEvent {
    water_entity,
    zone_entity,  // NEW: Pass the zone entity
    water_center,
    water_half_extents,
});
```

### Files Modified
- `src/components/fish.rs` - Added `zone_entity` field to `WaterSpawnedEvent`
- `src/systems/fish_system.rs` - Updated `spawn_fish_in_water()` to parent fish to zone
- `src/zone_loader.rs` - Pass `zone_entity` when sending `WaterSpawnedEvent`
- `src/ui/ui_settings_system.rs` - Added Fish settings tab to Settings UI

### Lesson Learned
When spawning entities that should appear within a transformed parent (like a zone with offset):
1. **Always parent child entities to the zone** - Without parenting, children won't inherit the parent's transform
2. **Zone offset matters** - Zones are positioned at `(5200.0, 0.0, -5200.0)` to center them in the world
3. **Event data must include parent reference** - Events that trigger entity spawning should include the parent entity reference
4. **Debug with world positions** - When debugging visibility issues, check both local and world positions to identify transform inheritance problems

---

## Game Freezes on Window Close Instead of Exiting (Fixed 2026-02-22)

### Problem
When the user closes the game window (clicks the X button), the game freezes instead of exiting cleanly. The process hangs indefinitely and must be force-killed.

### Root Cause
The network thread function `run_network_thread()` in [`src/resources/network_thread.rs`](src/resources/network_thread.rs) had an **extra outer `loop`** that prevented the thread from ever exiting:

```rust
// BROKEN CODE - outer loop causes infinite loop on exit
pub fn run_network_thread(mut control_rx: ...) {
    loop {  // <-- BUG: This outer loop should NOT exist!
        tokio::runtime::Builder::new_current_thread()
            .block_on(async {
                loop {  // <-- Inner loop
                    match control_rx.recv().await {
                        Some(NetworkThreadMessage::Exit) => return,  // Only exits inner block!
                        None => return,  // Only exits inner block!
                        // ...
                    }
                }
            })
        // After return from async block, outer loop continues!
    }
}
```

**What happens on exit:**
1. Window close → Bevy sends `AppExit` event
2. [`lib.rs:1425`](src/lib.rs:1425) sends `NetworkThreadMessage::Exit` to network thread
3. [`lib.rs:1426`](src/lib.rs:1426) calls `network_thread.join()` to wait for thread
4. Network thread receives `Exit` message
5. `return` only exits the inner `block_on` async block, NOT the function
6. Outer `loop` continues, creates new tokio runtime
7. Channel is closed (sender dropped), so `recv()` returns `None` immediately
8. **Infinite busy loop** - thread never exits, `join()` blocks forever = **FREEZE**

### Solution
Remove the outer `loop`. The function should only have the inner async loop:

```rust
// FIXED CODE - no outer loop, thread exits properly
pub fn run_network_thread(mut control_rx: ...) {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            loop {
                match control_rx.recv().await {
                    Some(NetworkThreadMessage::RunProtocolClient(client)) => { /* ... */ }
                    Some(NetworkThreadMessage::Exit) => return,  // Now exits the function!
                    None => return,  // Now exits the function!
                }
            }
        })
}
```

### Files Modified
- `src/resources/network_thread.rs` - Removed outer `loop` from `run_network_thread()`

### Lesson Learned
When implementing threaded background tasks that should exit on a signal:
1. **Be careful with nested loops** - A `return` inside an async block only exits that block, not the outer function
2. **Test exit paths** - Always verify background threads actually terminate when sent an exit signal
3. **Avoid unnecessary nesting** - The outer `loop` was unnecessary; the tokio runtime only needs to be created once
4. **Channel closure handling** - When the sender is dropped, `recv()` returns `None`, which should also trigger exit

