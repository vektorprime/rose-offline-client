# Shader/Bind Group Migration Analysis for Bevy 0.17-0.18

## Executive Summary

This analysis identifies all shader and bind group changes needed for upgrading ROSE Offline Client from Bevy 0.16 to Bevy 0.17-0.18.

### Critical Changes in Bevy 0.17 (wgpu 25)
- Material bind groups changed from `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})`
- The `MATERIAL_BIND_GROUP` shader def was added for backwards compatibility

### Additional Changes in Bevy 0.18
- `RenderPipelineDescriptor` now holds `BindGroupLayoutDescriptor` instead of `BindGroupLayout`
- `TrackedRenderPass::set_index_buffer` no longer takes buffer offset parameter

---

## Shaders Requiring @group(2) Migration

### 1. particle.wgsl
**Location:** [`src/render/shaders/particle.wgsl`](src/render/shaders/particle.wgsl)

**Current Bindings (Lines 9-40):**
```wgsl
@group(2) @binding(0) var<storage, read> positions: array<vec4<f32>>;
@group(2) @binding(1) var<storage, read> sizes: array<vec2<f32>>;
@group(2) @binding(2) var<storage, read> colors: array<vec4<f32>>;
@group(2) @binding(3) var<storage, read> textures: array<vec4<f32>>;
@group(2) @binding(4) var base_color_texture: texture_2d<f32>;
@group(2) @binding(5) var base_color_sampler: sampler;
@group(2) @binding(6) var<uniform> blend_op: u32;
@group(2) @binding(7) var<uniform> src_blend_factor: u32;
@group(2) @binding(8) var<uniform> dst_blend_factor: u32;
@group(2) @binding(9) var<uniform> billboard_type: u32;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Uses Storage Buffers:** Yes (bindings 0-3) - may have additional complications

**Rust Material:** [`ParticleMaterial`](src/render/particle_material.rs:19) in [`particle_material.rs`](src/render/particle_material.rs)

---

### 2. particle_prepass.wgsl
**Location:** [`src/render/shaders/particle_prepass.wgsl`](src/render/shaders/particle_prepass.wgsl)

**Current Bindings (Lines 10-39):**
```wgsl
@group(2) @binding(0) var<storage, read> positions: array<vec4<f32>>;
@group(2) @binding(1) var<storage, read> sizes: array<vec2<f32>>;
@group(2) @binding(2) var<storage, read> colors: array<vec4<f32>>;
@group(2) @binding(3) var<storage, read> textures: array<vec4<f32>>;
@group(2) @binding(4) var base_color_texture: texture_2d<f32>;
@group(2) @binding(5) var base_color_sampler: sampler;
@group(2) @binding(6) var<uniform> blend_op: u32;
@group(2) @binding(7) var<uniform> src_blend_factor: u32;
@group(2) @binding(8) var<uniform> dst_blend_factor: u32;
@group(2) @binding(9) var<uniform> billboard_type: u32;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Uses Storage Buffers:** Yes (bindings 0-3) - may have additional complications

**Rust Material:** Same as particle.wgsl - [`ParticleMaterial`](src/render/particle_material.rs:19)

---

### 3. terrain_material.wgsl
**Location:** [`src/render/shaders/terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl)

**Current Bindings (Lines 33-36):**
```wgsl
@group(2) @binding(0) var tile_array_texture: binding_array<texture_2d<f32>, 100>;
@group(2) @binding(1) var tile_array_sampler: sampler;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Uses Binding Arrays:** Yes (texture array with 100 elements)

**Rust Material:** [`TerrainMaterial`](src/render/terrain_material.rs:66) in [`terrain_material.rs`](src/render/terrain_material.rs)
- Uses custom [`as_bind_group()`](src/render/terrain_material.rs:147) implementation
- Uses custom [`bind_group_layout_entries()`](src/render/terrain_material.rs:218) implementation

---

### 4. water_material.wgsl
**Location:** [`src/render/shaders/water_material.wgsl`](src/render/shaders/water_material.wgsl)

**Current Bindings (Lines 38-74):**
```wgsl
@group(2) @binding(0) var water_array_texture: binding_array<texture_2d<f32>, 25>;
@group(2) @binding(1) var water_array_sampler: sampler;
@group(2) @binding(2) var<uniform> light_direction: vec4<f32>;
@group(2) @binding(3) var<uniform> ambient_color: vec4<f32>;
@group(2) @binding(4) var<uniform> diffuse_color: vec4<f32>;
@group(2) @binding(6) var<uniform> fog_uniforms: FogUniforms;
@group(2) @binding(5) var<uniform> water_settings: WaterSettingsUniform;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Uses Binding Arrays:** Yes (texture array with 25 elements)

**Rust Material:** [`WaterMaterial`](src/render/water_material.rs:82) in [`water_material.rs`](src/render/water_material.rs)
- Uses custom [`as_bind_group()`](src/render/water_material.rs:211) implementation
- Uses custom [`bind_group_layout_entries()`](src/render/water_material.rs:364) implementation

---

### 5. sky_material.wgsl
**Location:** [`src/render/shaders/sky_material.wgsl`](src/render/shaders/sky_material.wgsl)

**Current Bindings (Lines 39-48):**
```wgsl
@group(2) @binding(0) var sky_texture_day: texture_2d<f32>;
@group(2) @binding(1) var sky_sampler_day: sampler;
@group(2) @binding(2) var sky_texture_night: texture_2d<f32>;
@group(2) @binding(3) var sky_sampler_night: sampler;
@group(2) @binding(4) var<uniform> day_weight: f32;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings

**Rust Material:** [`SkyMaterial`](src/render/sky_material.rs:44) in [`sky_material.rs`](src/render/sky_material.rs)
- Uses derive [`AsBindGroup`](src/render/sky_material.rs:44) - automatic handling

---

### 6. cartoon_sky.wgsl
**Location:** [`src/render/shaders/cartoon_sky.wgsl`](src/render/shaders/cartoon_sky.wgsl)

**Current Bindings (Lines 75-76):**
```wgsl
@group(2) @binding(0) var<uniform> sky: SkyUniforms;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings

**Rust Material:** [`CartoonSkyMaterial`](src/render/cartoon_sky_material.rs:158) in [`cartoon_sky_material.rs`](src/render/cartoon_sky_material.rs)
- Uses derive [`AsBindGroup`](src/render/cartoon_sky_material.rs:158) - automatic handling

---

### 7. damage_digit.wgsl
**Location:** [`src/render/shaders/damage_digit.wgsl`](src/render/shaders/damage_digit.wgsl)

**Current Bindings (Lines 13-22):**
```wgsl
@group(2) @binding(0) var<storage, read> positions: PositionBuffer;
@group(2) @binding(1) var<storage, read> sizes: SizeBuffer;
@group(2) @binding(2) var<storage, read> uvs: UvBuffer;
@group(2) @binding(3) var base_color_texture: texture_2d<f32>;
@group(2) @binding(4) var base_color_sampler: sampler;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Uses Storage Buffers:** Yes (bindings 0-2) - may have additional complications

**Rust Material:** [`DamageDigitMaterial`](src/render/damage_digit_material.rs:11) in [`damage_digit_material.rs`](src/render/damage_digit_material.rs)

---

### 8. wing_material.wgsl
**Location:** [`src/render/shaders/wing_material.wgsl`](src/render/shaders/wing_material.wgsl)

**Current Bindings (Lines 37-38):**
```wgsl
@group(2) @binding(100) var<uniform> wing_extension: WingExtension;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Note:** Uses high binding index (100) for material extension

**Rust Material:** Currently using [`StandardMaterial`](src/render/wing_material.rs:25) directly - shader may not be actively used

---

### 9. world_ui.wgsl
**Location:** [`src/render/shaders/world_ui.wgsl`](src/render/shaders/world_ui.wgsl)

**Current Bindings (Lines 17-18):**
```wgsl
#ifdef ZONE_LIGHTING_GROUP_2
#import rose_client::zone_lighting
@group(2) @binding(0) var<uniform> zone_lighting: ZoneLightingData;
#endif
```

**Migration Required:**
- Conditional compilation with `ZONE_LIGHTING_GROUP_2` shader def
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` if used as material bind group
- **Note:** This is a special case - zone lighting may need separate handling

**Rust Material:** Custom pipeline in [`world_ui.rs`](src/render/world_ui.rs)
- Uses custom [`WorldUiPipeline`](src/render/world_ui.rs:184) with manual bind group layout creation

---

### 10. rose_object_extension.wgsl
**Location:** [`src/render/shaders/rose_object_extension.wgsl`](src/render/shaders/rose_object_extension.wgsl)

**Current Bindings (Lines 26-41):**
```wgsl
@group(2) @binding(100) var<uniform> lightmap_params: vec4<f32>;
@group(2) @binding(101) var lightmap_texture: texture_2d<f32>;
@group(2) @binding(102) var lightmap_sampler: sampler;
@group(2) @binding(103) var specular_texture: texture_2d<f32>;
@group(2) @binding(104) var specular_sampler: sampler;
```

**Migration Required:**
- Change `@group(2)` to `@group(#{MATERIAL_BIND_GROUP})` for all bindings
- **Note:** Uses high binding indices (100-104) for material extension

**Rust Material:** [`RoseObjectExtension`](src/render/extension_material_plugin.rs:40) - ExtendedMaterial extension

---

### 11. zone_lighting.wgsl
**Location:** [`src/render/shaders/zone_lighting.wgsl`](src/render/shaders/zone_lighting.wgsl)

**Current Bindings (Lines 29-30):**
```wgsl
@group(3) @binding(0) var<uniform> zone_lighting: ZoneLightingData;
```

**Migration Required:**
- **Uses @group(3), NOT @group(2)** - This is NOT a material bind group
- Zone lighting uses its own bind group at group 3
- No changes needed for material bind group migration
- May need verification that group 3 is still valid in Bevy 0.17+

---

## Shaders NOT Requiring @group(2) Migration

The following shaders do not use `@group(2)` and should not require changes:

| Shader | Bind Groups Used | Notes |
|--------|-----------------|-------|
| [`trail_effect.wgsl`](src/render/shaders/trail_effect.wgsl) | @group(0), @group(1) | No material bind group |
| [`underwater_effect.wgsl`](src/render/shaders/underwater_effect.wgsl) | @group(0) | Post-process effect, uses fullscreen shader |
| [`post_processing.wgsl`](src/render/shaders/post_processing.wgsl) | @group(0), @group(1) | Post-process effect |
| [`rose_terrain_extension.wgsl`](src/render/shaders/rose_terrain_extension.wgsl) | None declared | Uses bevy_pbr imports only |
| [`rose_water_extension.wgsl`](src/render/shaders/rose_water_extension.wgsl) | None declared | Uses bevy_pbr imports only |
| [`rose_effect_extension.wgsl`](src/render/shaders/rose_effect_extension.wgsl) | None declared | Uses bevy_pbr imports only |

---

## Rust Material Files Analysis

### Files with Custom `as_bind_group()` Implementation

These files manually create bind groups and may need additional updates:

#### 1. terrain_material.rs
**Location:** [`src/render/terrain_material.rs`](src/render/terrain_material.rs)

**Key Functions:**
- [`as_bind_group()`](src/render/terrain_material.rs:147) - Creates bind group with texture array
- [`bind_group_layout_entries()`](src/render/terrain_material.rs:218) - Defines layout entries

**Bevy 0.18 Impact:**
- `RenderPipelineDescriptor.layout` change from `Vec<BindGroupLayout>` to `Vec<BindGroupLayoutDescriptor>` may affect pipeline creation
- Current code uses `BindGroupLayout` directly - verify compatibility

#### 2. water_material.rs
**Location:** [`src/render/water_material.rs`](src/render/water_material.rs)

**Key Functions:**
- [`as_bind_group()`](src/render/water_material.rs:211) - Creates bind group with texture array and uniforms
- [`bind_group_layout_entries()`](src/render/water_material.rs:364) - Defines layout entries

**Bevy 0.18 Impact:**
- Same as terrain_material.rs - verify `BindGroupLayout` vs `BindGroupLayoutDescriptor` compatibility

#### 3. world_ui.rs
**Location:** [`src/render/world_ui.rs`](src/render/world_ui.rs)

**Key Functions:**
- [`FromWorld`](src/render/world_ui.rs:298) implementation creates bind group layouts
- [`specialize()`](src/render/world_ui.rs:195) creates `RenderPipelineDescriptor`

**Bevy 0.18 Impact:**
- Direct `RenderPipelineDescriptor` construction at line 196-294
- Uses `Vec<BindGroupLayout>` in `layout` field - needs update to `Vec<BindGroupLayoutDescriptor>`

---

### Files with `RenderPipelineDescriptor` Usage

These files construct or modify `RenderPipelineDescriptor` and need review for Bevy 0.18:

| File | Function | Line | Usage Type |
|------|----------------|------------|
| [`particle_material.rs`](src/render/particle_material.rs) | [`specialize()`](src/render/particle_material.rs:70) | 72 | Modifies vertex buffers |
| [`terrain_material.rs`](src/render/terrain_material.rs) | [`specialize()`](src/render/terrain_material.rs:99) | 101 | Modifies vertex buffers, blend state |
| [`water_material.rs`](src/render/water_material.rs) | [`specialize()`](src/render/water_material.rs:153) | 155 | Modifies depth stencil, primitive, vertex buffers, blend |
| [`sky_material.rs`](src/render/sky_material.rs) | [`specialize()`](src/render/sky_material.rs:85) | 87 | Modifies depth stencil, vertex buffers |
| [`cartoon_sky_material.rs`](src/render/cartoon_sky_material.rs) | [`specialize()`](src/render/cartoon_sky_material.rs:305) | 307 | Modifies depth stencil, primitive, vertex buffers |
| [`underwater_effect.rs`](src/render/underwater_effect.rs) | [`specialize()`](src/render/underwater_effect.rs:363) | 363 | Creates full descriptor |
| [`world_ui.rs`](src/render/world_ui.rs) | [`specialize()`](src/render/world_ui.rs:195) | 195 | Creates full descriptor |

---

### Files with `set_index_buffer` Usage

**Finding:** No direct usage of `set_index_buffer` was found in the material implementation files reviewed. The `TrackedRenderPass::set_index_buffer` change in Bevy 0.18 should not directly affect these materials.

**Note:** The [`world_ui.rs`](src/render/world_ui.rs) file uses `set_vertex_buffer` at line 437:
```rust
pass.set_vertex_buffer(0, sprite_meta.vertices.buffer().unwrap().slice(..));
```
This is not affected by the `set_index_buffer` change.

---

## Import Statements Analysis

Shaders using `#import` statements that may need verification:

| Shader | Import | Notes |
|--------|--------|-------|
| [`particle.wgsl`](src/render/shaders/particle.wgsl:4) | `bevy_render::view::View` | Standard Bevy import |
| [`particle.wgsl`](src/render/shaders/particle.wgsl:5) | `bevy_pbr::mesh_bindings::mesh` | Standard Bevy import |
| [`terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl:9) | `bevy_pbr::mesh_functions` | Standard Bevy import |
| [`terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl:10) | `bevy_pbr::mesh_view_bindings view` | Standard Bevy import |
| [`water_material.wgsl`](src/render/shaders/water_material.wgsl:17) | `bevy_pbr::mesh_functions` | Standard Bevy import |
| [`water_material.wgsl`](src/render/shaders/water_material.wgsl:18) | `bevy_pbr::mesh_view_bindings::{view, globals}` | Standard Bevy import |
| [`world_ui.wgsl`](src/render/shaders/world_ui.wgsl:16) | `rose_client::zone_lighting` | **Custom import** - verify path |
| [`underwater_effect.wgsl`](src/render/shaders/underwater_effect.wgsl:8) | `bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput` | Standard Bevy import |
| [`zone_lighting.wgsl`](src/render/shaders/zone_lighting.wgsl:4) | `#define_import_path rose_client::zone_lighting` | **Custom module definition** |
| [`rose_object_extension.wgsl`](src/render/shaders/rose_object_extension.wgsl:10) | `bevy_pbr::*` | Standard Bevy imports |

---

## Summary of Required Changes

### Shader Files (11 files needing @group(2) → #{MATERIAL_BIND_GROUP} migration)

| Priority | File | Bindings | Storage Buffers | Binding Arrays |
|----------|------|----------|-----------------|----------------|
| HIGH | [`particle.wgsl`](src/render/shaders/particle.wgsl) | 10 (0-9) | Yes (0-3) | No |
| HIGH | [`particle_prepass.wgsl`](src/render/shaders/particle_prepass.wgsl) | 10 (0-9) | Yes (0-3) | No |
| HIGH | [`damage_digit.wgsl`](src/render/shaders/damage_digit.wgsl) | 5 (0-4) | Yes (0-2) | No |
| MEDIUM | [`terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl) | 2 (0-1) | No | Yes (100 textures) |
| MEDIUM | [`water_material.wgsl`](src/render/shaders/water_material.wgsl) | 7 (0-6) | No | Yes (25 textures) |
| MEDIUM | [`sky_material.wgsl`](src/render/shaders/sky_material.wgsl) | 5 (0-4) | No | No |
| MEDIUM | [`cartoon_sky.wgsl`](src/render/shaders/cartoon_sky.wgsl) | 1 (0) | No | No |
| LOW | [`wing_material.wgsl`](src/render/shaders/wing_material.wgsl) | 1 (100) | No | No |
| LOW | [`rose_object_extension.wgsl`](src/render/shaders/rose_object_extension.wgsl) | 5 (100-104) | No | No |
| SPECIAL | [`world_ui.wgsl`](src/render/shaders/world_ui.wgsl) | 1 (0) | No | No |
| NONE | [`zone_lighting.wgsl`](src/render/shaders/zone_lighting.wgsl) | @group(3) | No | No |

### Rust Material Files (7 files needing review)

| Priority | File | Change Type |
|----------|------|-------------|
| HIGH | [`world_ui.rs`](src/render/world_ui.rs) | RenderPipelineDescriptor.layout type change |
| MEDIUM | [`terrain_material.rs`](src/render/terrain_material.rs) | Verify BindGroupLayout compatibility |
| MEDIUM | [`water_material.rs`](src/render/water_material.rs) | Verify BindGroupLayout compatibility |
| LOW | [`particle_material.rs`](src/render/particle_material.rs) | Specialize function review |
| LOW | [`sky_material.rs`](src/render/sky_material.rs) | Specialize function review |
| LOW | [`cartoon_sky_material.rs`](src/render/cartoon_sky_material.rs) | Specialize function review |
| LOW | [`underwater_effect.rs`](src/render/underwater_effect.rs) | Specialize function review |

---

## Migration Checklist

### Phase 1: Shader Updates
- [ ] Update [`particle.wgsl`](src/render/shaders/particle.wgsl) - all @group(2) bindings
- [ ] Update [`particle_prepass.wgsl`](src/render/shaders/particle_prepass.wgsl) - all @group(2) bindings
- [ ] Update [`terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl) - all @group(2) bindings
- [ ] Update [`water_material.wgsl`](src/render/shaders/water_material.wgsl) - all @group(2) bindings
- [ ] Update [`sky_material.wgsl`](src/render/shaders/sky_material.wgsl) - all @group(2) bindings
- [ ] Update [`cartoon_sky.wgsl`](src/render/shaders/cartoon_sky.wgsl) - all @group(2) bindings
- [ ] Update [`damage_digit.wgsl`](src/render/shaders/damage_digit.wgsl) - all @group(2) bindings
- [ ] Update [`wing_material.wgsl`](src/render/shaders/wing_material.wgsl) - all @group(2) bindings
- [ ] Update [`world_ui.wgsl`](src/render/shaders/world_ui.wgsl) - conditional @group(2) binding
- [ ] Update [`rose_object_extension.wgsl`](src/render/shaders/rose_object_extension.wgsl) - all @group(2) bindings

### Phase 2: Rust Material Updates
- [ ] Review [`world_ui.rs`](src/render/world_ui.rs) for RenderPipelineDescriptor.layout change
- [ ] Review [`terrain_material.rs`](src/render/terrain_material.rs) for BindGroupLayout compatibility
- [ ] Review [`water_material.rs`](src/render/water_material.rs) for BindGroupLayout compatibility
- [ ] Review all specialize() implementations for descriptor changes

### Phase 3: Testing
- [ ] Test particle rendering
- [ ] Test terrain rendering
- [ ] Test water rendering
- [ ] Test sky rendering (both texture-based and cartoon)
- [ ] Test damage digit display
- [ ] Test wing rendering
- [ ] Test world UI elements
- [ ] Test zone lighting integration

---

## Notes and Caveats

1. **Storage Buffers:** Shaders using storage buffers (particle, damage_digit) may have additional complications since storage buffers have specific alignment and usage requirements that may have changed in wgpu 25.

2. **Binding Arrays:** Terrain (100 textures) and water (25 textures) use `binding_array` which is a wgpu feature. Verify this is still supported with the new bind group indexing.

3. **High Binding Indices:** The wing_material.wgsl and rose_object_extension.wgsl use binding indices starting at 100. Ensure this is compatible with the new system.

4. **Zone Lighting:** The zone_lighting.wgsl uses @group(3) which is separate from material bind groups. This should not need changes but should be verified to work correctly with the new bind group layout.

5. **Conditional Compilation:** The world_ui.wgsl uses `#ifdef ZONE_LIGHTING_GROUP_2` for conditional zone lighting inclusion. Verify shader defs are still processed correctly.

6. **Custom Pipelines:** The world_ui.rs creates its own render pipeline manually. This will need careful review for the `BindGroupLayout` to `BindGroupLayoutDescriptor` change in Bevy 0.18.
