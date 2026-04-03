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
The rendering backend is powered by `wgpu`. Configuration includes specific feature flags to ensure compatibility across different hardware while enabling advanced features like:
- Storage buffer support for particle data.
- Specialized vertex buffer layouts for procedural geometry.
- Reverse-Z depth buffering for improved precision.

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
- **Underwater Effects**: Integrated with `RoseWaterExtension` and `UnderwaterEffectPlugin` to provide fog and color blending when the camera is submerged.
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
- **Animation**: Wind-driven movement via UV offset/translation in the shader.
- **Reference**: `src/render/cloud_material.rs`

## ExtendedMaterial Extensions

The following extensions allow the `StandardMaterial` to be augmented with ROSE-specific features:

| Extension | Purpose | Key Features |
| :--- | :--- | :--- |
| **RoseObjectExtension** | General object enhancement | Lightmap support, specular maps, and blink state for characters. |
| **RoseTerrainExtension** | Terrain rendering | Multiple texture splatting (up to 4), detail textures, and tile-based selection. |
| **RoseWaterExtension** | Water augmentation | UV animation for wave movement and specialized water textures. |
| **RoseEffectExtension** | VFX mesh rendering | Frame-based animation using texture atlases and interpolation. |

## Post-Processing Effects

The rendering pipeline includes a comprehensive suite of post-processing effects applied after the main opaque and transparent passes:
- **Bloom**: Simulates light bleeding from bright sources.
- **Depth of Field (DoF)**: Provides cinematic focus effects.
- **Motion Blur**: Smoothes high-speed movement.
- **Auto Exposure**: Adjusts brightness based on scene luminance.
- **SMAA**: Subpixel Morphological Anti-Aliasing for smoother edges.
- **SSAO**: Screen Space Ambient Occlusion for enhanced depth perception.
- **SSR**: Screen Space Reflections for realistic surface reflections.

## Code Examples

### Particle Material Bind Group Layout
```rust
// src/render/particle_material.rs:20
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
// src/render/water_material.rs:181
fn as_bind_group(
    &self,
    layout_descriptor: &BindGroupLayoutDescriptor,
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    _param: &mut Self::Param,
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

### Bevvy Source
- **PBR**: `C:\Users\vicha\RustroverProjects\bevvy-collection\bevvy-0.18.1\crates\bev_pbr\src\`
- **Post-Processing**: `C:\Users\vicha\RustroverProjects\bevvy-collection\bevvy-0.18.1\crates\bev_post_process\src\`

### Project Source
- **Core Render Logic**: `src/render/mod.rs`
- **Material Definitions**: `src/render/materials/*.rs` (Note: located in `src/render/` directly in this project)
- **Extensions**: `src/render/*_extension.rs`