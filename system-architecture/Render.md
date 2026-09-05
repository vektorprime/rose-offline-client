# System Architecture: Rendering Pipeline

## Overview
The ROSE offline client utilizes a high-fidelity PBR (Physically Based Rendering) pipeline with extensive post-processing effects. The rendering architecture is built upon the Bevy engine, leveraging its modular plugin system and `wgpu` abstraction for hardware-accelerated graphics.

The pipeline is designed to handle complex environmental effects including procedural skies, water, clouds, and dynamic particle systems, all while maintaining high performance through deferred rendering and optimized custom materials.

## Render Pipeline Configuration

### Deferred Rendering
The engine utilizes deferred rendering for opaque objects to efficiently manage numerous light sources in the scene. 
- **Method**: `DefaultOpaqueRendererMethod::deferred()` is used to separate geometry processing from lighting calculations.
- **Advantages**: Reduced lighting complexity and support for more dynamic environmental lights (e.g., zone-specific lighting).

### WGPU Settings and Feature Flags
The rendering backend is powered by `wgpu`. The client configures `WgpuSettings` in `src/lib.rs` to disable problematic bindless features for stability across hardware, while relying on core wgpu features for the custom pipeline:
- Disabled features: `BUFFER_BINDING_ARRAY`, `STORAGE_RESOURCE_BINDING_ARRAY`, and `PARTIALLY_BOUND_BINDING_ARRAY` (bindless paths), while texture binding arrays remain available for `TerrainMaterial`.
- Read-only storage buffers carry per-particle and per-material data to the GPU (particles, damage digits, water, terrain lighting).
- Specialized vertex buffer layouts for procedural geometry are configured per material in each material's `specialize()`.
- Reverse-Z depth buffering (Bevy default) is used; sky and cloud materials combine it with `CompareFunction::GreaterEqual`.

## Custom Materials

### ParticleMaterial
A GPU-driven particle system that bypasses traditional CPU-side mesh updates.
- **Architecture**: Uses a storage buffer architecture to pass particle properties directly to the GPU.
- **Data Buffers**:
  - `positions`: `Handle<ShaderStorageBuffer>` (Binding 0)
  - `sizes`: `Handle<ShaderStorageBuffer>` (Binding 1)
  - `colors`: `Handle<ShaderStorageBuffer>` (Binding 2)
  - `textures`: `Handle<ShaderStorageBuffer>` (Binding 3)
- **Reference**: `src/render/particle_material.rs`

### WaterMaterial
A fully procedural water rendering solution that does not rely on external textures for its core appearance.
- **Features**: Supports animated waves, foam intensity, refraction, and subsurface scattering (SSS).
- **Underwater Effects**: Integrated with `UnderwaterEffectPlugin` to provide volumetric fog and color blending when the camera is submerged.
- **Reference**: `src/render/water_material.rs`

### DamageDigitMaterial
Specialized material for rendering high-performance 3D text for combat feedback.
- **Geometry**: Uses procedural geometry generation in the vertex shader via `@builtin(vertex_index)`.
- **Data**: Leverages storage buffers for positions, sizes, and UVs to minimize draw calls.
- **Reference**: `src/render/damage_digit_material.rs`

### StarrySkyMaterial
A procedural sky system that renders a star field and moon.
- **Implementation**: Renders an inverted sphere mesh at a large radius.
- **Logic**: Uses a `night_factor` (driven by the zone time system) to fade stars in/out and manages moon phases and direction.
- **Reference**: `src/render/starry_sky_material.rs`

### CloudMaterial
Procedural cloud generation using fBm noise.
- **Visuals**: Supports coverage, density, softness, and time-of-day lighting integration.
- **Animation**: Wind-driven movement via a time-based offset of the noise sampling position in the shader.
- **Reference**: `src/render/cloud_material.rs`

## ExtendedMaterial Extensions

The following extensions allow the `StandardMaterial` to be augmented with ROSE-specific features:

| Extension | Purpose | Key Features |
| :--- | :--- | :--- |
| **RoseObjectExtension** | General object enhancement | Lightmap support, specular maps, and blink state for characters. |
| **RoseEffectExtension** | VFX mesh rendering | Frame-based animation using texture atlases and interpolation. |

Terrain and water are **not** `StandardMaterial` extensions — they use standalone custom `Material` implementations:
- **TerrainMaterial** (`src/render/terrain_material.rs`): up to 100 tile textures in a texture binding array, selected per-vertex via `TERRAIN_MESH_ATTRIBUTE_TILE_INFO` (two layers + rotation), with lightmap support via UV0.
- **WaterMaterial** (`src/render/water_material.rs`): fully procedural shading with a custom `AsBindGroup` that packs per-material values into a storage buffer.

## Post-Processing Effects

The main camera spawns with a fixed default set (`src/lib.rs:1990-2008`); everything else is opt-in via `src/graphics/apply_systems.rs`. No TAA, SSR, or AutoExposure component is spawned anywhere in `src/` (`src/lib.rs:1995-1999`).

Default-on at startup:
- **Bloom** (`Bloom::NATURAL`): light bleeding from bright sources.
- **Depth of Field (DoF)** (`DepthOfField` Gaussian): cinematic focus effects.
- **Tonemapping** (`TonyMcMapface`): filmic HDR mapping.
- **SSAO** (`ScreenSpaceAmbientOcclusion`, Medium default): contact depth. Ultra is only `SsaoQuality::Ultra`.
- **Shadow filtering** (`ShadowFilteringMethod::Gaussian`).
- **Prepasses**: `DepthPrepass` + `DeferredPrepass` (the latter is required for deferred; without it Bevy 0.18.1 panics in `queue_prepass_material_meshes`), plus `OcclusionCulling`.

Opt-in via graphics settings (NOT on by default):
- **SMAA** (`apply_smaa_system`): Disabled/Low/Medium/High/Ultra.
- **Motion Blur** (`apply_motion_blur_system`): inserted/removed on demand; stripped from the water-reflection view.

Not implemented:
- **SSR**: no implementation; explicitly left off.
- **Auto Exposure**: no `AutoExposure` component in `src/`.
- **TAA**: no `TemporalAntiAliasing` component in `src/` (only mentioned in the `Msaa::Off` compatibility comment).

## Code Examples

### Particle Material Bind Group Layout
```rust
// src/render/particle_material.rs:17
#[derive(Asset, TypePath, AsBindGroup, Clone)]
pub struct ParticleMaterial {
    #[storage(0, read_only)]
    pub positions: Handle<ShaderStorageBuffer>,
    #[storage(1, read_only)]
    pub sizes: Handle<ShaderStorageBuffer>,
    #[storage(2, read_only)]
    pub colors: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)]
    pub textures: Handle<ShaderStorageBuffer>,
    // ...
}
```

### Water Material Custom AsBindGroup
```rust
// src/render/water_material.rs:193
fn as_bind_group(
    &self,
    layout_descriptor: &BindGroupLayoutDescriptor,
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    (image_assets, fallback_image): &mut SystemParamItem<'_, '_, Self::Param>,
) -> Result<PreparedBindGroup, AsBindGroupError> {
    // Packs per-material values into a single storage buffer for efficiency
    let water_material_data = [ ... ]; 
    // ...
}
```

## Troubleshooting

### Material Rendering Issues
- **Bind Group Mismatch**: Ensure that the `AsBindGroup` derive macro in Rust matches the `@binding(n)` declarations in the corresponding `.wgsl` shader.
- **Storage Buffer Errors**: Particle and DamageDigit materials require valid `ShaderStorageBuffer` assets. Check if buffers are properly initialized in `Assets<ShaderStorageBuffer>`.

### Post-Processing Artifacts
- **Ghosting/Flicker**: Ensure that `AlphaMode::Blend` is used correctly for transparent elements to prevent accumulation errors in the post-processing buffers.
- **Depth Fighting**: For sky/cloud materials, check `depth_compare` settings (e.g., using `GreaterEqual` with Reverse-Z).

### Shader Compilation Failures
- **Missing Defines**: Extensions like `RoseEffectExtension` rely on shader defines (e.g., `HAS_ANIMATION_TEXTURE`). Ensure these are pushed to the `RenderPipelineDescriptor` during specialization.
- **Pathing**: Verify that `load_internal_asset!` paths correctly point to the `shaders/` directory.

## Source File References

### Bevy Source
- **PBR**: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_pbr\src\`
- **Post-Processing**: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_post_process\src\`

### Project Source
- **Core Render Logic**: `src/render/mod.rs`
- **Material Definitions**: `src/render/*_material.rs`
- **Extensions**: `src/render/*_extension.rs`