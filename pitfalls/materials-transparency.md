# Materials and Transparency Pitfalls

This document records material and transparency-related issues encountered during development.

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
Error matching ShaderStages(FRAGMENT) shader requirements against the pipeline layout
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
