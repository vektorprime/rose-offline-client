# System Architecture: Rendering Pipeline

## Overview
The ROSE offline client utilizes a high-fidelity PBR (Physically Based Rendering) pipeline with extensive post-processing effects. The rendering architecture is built upon the Bevy engine, leveraging its modular plugin system and `wgpu` abstraction for hardware-accelerated graphics.

The pipeline is designed to handle complex environmental effects including procedural skies, water, clouds, and dynamic particle systems, all while maintaining high performance through deferred rendering and optimized custom materials.

## Render Pipeline Configuration

### Deferred Rendering
The engine utilizes deferred rendering for opaque objects to efficiently manage numerous light sources in the scene. 
- **Method**: `DefaultOpaqueRendererMethod::deferred()` is used to separate geometry processing from lighting calculations.
- **Advantages**: Reduced lighting complexity and support for more dynamic environmental lights (e.g., zone-specific lighting).
- **ROSE object materials are forward.** Every `ExtendedMaterial<StandardMaterial, RoseObjectExtension>` (zone object parts, character/NPC/vehicle/item parts) is built through `rose_object_material()` (`src/render/object_material_extension.rs`), which sets `opaque_render_method: Forward`. The deferred G-buffer has no room for the lightmap, the specular map or the blood overlay, alpha-masked parts were shaded twice (G-buffer + forward), and the water reflection camera (no deferred prepass) skipped deferred opaque parts entirely. Animated zone objects (`RoseEffectExtension`) are forward too. Forward materials still write the depth/normal prepass (SSAO), cast shadows and get fog.

### WGPU Settings and Feature Flags
The rendering backend is powered by `wgpu`. `WgpuSettings` are built by `create_wgpu_settings()` in `src/render/wgpu_settings.rs` (used by `src/lib.rs`):
- Disabled features: `BUFFER_BINDING_ARRAY`, `STORAGE_RESOURCE_BINDING_ARRAY`, and `PARTIALLY_BOUND_BINDING_ARRAY` (bindless paths), while texture binding arrays remain available for `TerrainMaterial`.
- wgpu debug/validation instance flags are off (wgpu turns `DEBUG | VALIDATION | VALIDATION_INDIRECT_CALL` on in debug builds). An adapter probe at startup predicts wgpu's adapter choice; when it is a non-DX12 hardware adapter (Vulkan on Windows) the backend is pinned to it and `VALIDATION_INDIRECT_CALL` is dropped too. On DX12 that flag must stay (wgpu needs it for correct `instance_index`/`vertex_index` with indirect draws). Re-enable per session with `WGPU_VALIDATION=1`, `WGPU_DEBUG=1`, `WGPU_VALIDATION_INDIRECT_CALL=1`.
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
| **RoseObjectExtension** | General object enhancement | Lightmap (pipeline key `has_lightmap` -> `ROSE_OBJECT_LIGHTMAP`; ZMS lightmap UVs are bound from `MESH_ATTRIBUTE_UV_1` at location 3 as `uv_b`; UV = `(uv_b + cell) * (1 / parts_per_row)` with the cell from `MeshTag`; blended as the original MODULATE2X, i.e. x4.5948 on the sRGB-sampled texel), specular map only for ZSC materials with the specular flag (`has_specular` -> `ROSE_OBJECT_SPECULAR`, red channel -> reflectance; others keep the standard reflectance), UV-space blood overlay. No blink state: blinking swaps face meshes (`character_model_blink_system`). |
| **RoseEffectExtension** | VFX meshes and animated zone objects | ZMO morph animation (position, normal, UV) from an animation texture, applied in `rose_effect_mesh.wgsl` in the forward, prepass and deferred vertex stages (`textureLoad` at `vertex_index - first_vertex_index`). `mesh_animation_system` animates every `MeshAnimation` entity (no `EffectMesh` filter). |

Terrain and water are **not** `StandardMaterial` extensions — they use standalone custom `Material` implementations:
- **TerrainMaterial** (`src/render/terrain_material.rs`): up to 100 tile textures in a texture binding array, selected per-vertex via `TERRAIN_MESH_ATTRIBUTE_TILE_INFO` (two layers + rotation), with lightmap support via UV0. One instance per zone, shared by every block (created in `spawn_zone`).
- **WaterMaterial** (`src/render/water_material.rs`): fully procedural shading with a custom `AsBindGroup` that packs per-material values into a storage buffer.

## Post-Processing Effects

The main camera spawns with a fixed default set (`src/lib.rs`, see [graphics-settings.md](graphics-settings.md)); everything else is opt-in via `src/graphics/apply_systems.rs`. No TAA or SSR implementation exists in `src/`.

Default-on at startup:
- **Tonemapping** (`TonyMcMapface`): RESTORED 2026-09-25. The white "film"/"flash" was traced to ColorGrading exposure (Brightness slider + TOD tint) landing on clamped HDR, NOT to the tonemap pass — so the whole color-grading path (`ColorGrading` camera component, brightness/contrast/saturation/gamma fields, `apply_color_grading_system`, `TimeOfDayGrading` + its writer in zone_time_system.rs) was DELETED and the filmic curve is the sole tone path. CORRECTION (2026-09-30): the random white *flashes* with SMAA on, and the "3D view crawls with tonemapping off" symptom, were both the post-process pass race described below, not ColorGrading or menu cost. Racy frames showed the pre-tonemap HDR image when tonemapping was on, and a stale frame when it was off. See [pitfalls/postprocess-pass-race.md](../pitfalls/postprocess-pass-race.md).
- **AutoExposure** (Histogram, custom compensation curve): normalizes sun-lux/atmosphere output toward `GraphicsSettings::auto_exposure_target_ev` (default -1.3 EV; Bevy's default curve targets 0 EV = very bright), with 50% adaptation in dark scenes so night stays darker than day. `AutoExposurePlugin` is added manually (Bevy 0.19.1's `PostProcessPlugin` does not register it). See [graphics-settings.md](graphics-settings.md) "Auto Exposure target".
- **Bloom** (`Bloom { intensity: 0.15, ..NATURAL }`): light bleeding from bright sources.
- **SMAA** (`Smaa`, Ultra preset): default anti-aliasing (`Msaa::Off`); quality switchable via `apply_smaa_system`.
- **SSAO** (`ScreenSpaceAmbientOcclusion`, Medium default): contact depth. Ultra is only `SsaoQuality::Ultra`.
- **Shadow filtering** (`ShadowFilteringMethod::Gaussian`).
- **Prepasses**: `DepthPrepass` + `DeferredPrepass` (the latter is required for deferred; without it Bevy 0.18.1 panics in `queue_prepass_material_meshes`), plus `OcclusionCulling`.

Opt-in via graphics settings (NOT on by default):
- **Depth of Field** (`DepthOfField`): Gaussian; inserted on demand.
- **Motion Blur** (`apply_motion_blur_system`): inserted/removed on demand; stripped from the water-reflection view.

Not implemented:
- **SSR**: no implementation; explicitly left off.
- **TAA**: no `TemporalAntiAliasing` component in `src/` (only mentioned in the `Msaa::Off` compatibility comment).

### Post-process pass ordering invariant (Bevy 0.19)

Render-graph nodes are plain systems in the `Core3d` schedule (multi-threaded executor). `RenderContext`/`ViewQuery` are read-only, so **unordered passes run in parallel**. Command buffers are submitted in schedule (topological) order, but `ViewTarget::post_process_write()` flips the shared main-texture ping-pong index in *thread* order. Two unordered passes that both call `post_process_write()` therefore randomly read a texture that has not been written yet this frame. With tonemapping on, the pre-tonemap HDR frame reaches the screen (white flash). With tonemapping off, a stale frame reaches it (3D view appears to crawl while egui stays live).

Rules:
- Every custom pass that calls `post_process_write()` must be totally ordered against the others. `underwater_effect` is `.after(tonemapping).before(fxaa).before(smaa)` and returns before flipping when not underwater (`src/render/underwater_effect.rs`).
- Bevy leaves `fxaa` and `smaa` mutually unordered, so `apply_fxaa_system` only inserts `Fxaa` while SMAA is Disabled.
- `bevy_egui::render::egui_pass` is pinned after `Core3dSystems::PostProcess` (`src/lib.rs`, next to the `EguiPlugin` setup). By default it is only `.after(EarlyPostProcess)`.

### Material update invariants (Bevy 0.19.1)

- `Assets::iter_mut()` queues `AssetEvent::Modified` for **every** asset it visits, written or not, and `Assets::get_mut()` does so on any `DerefMut`. Each `Modified` material is re-extracted, its bind group rebuilt, and every mesh using it re-specialized. Find stale assets with `iter()`/`get()` and call `get_mut()` only when a value differs (see `update_terrain_lighting_system`, `apply_water_settings`, `sync_material_overlay` in `blood_overlay_system.rs`).
- A custom material that overrides `as_bind_group` and returns `CreateBindGroupDirectly` from `unprepared_bind_group` **leaks its previous bind group on every modification**: `prepare_asset` inserts a new allocator slot without freeing the old one (`bevy_pbr` `material.rs`). Return the bindings from `unprepared_bind_group` instead (`OwnedBindingResource::Buffer/TextureView/Sampler`); that path frees the old slot. Cloud, starry sky, volumetric cloud and water materials do this. `TerrainMaterial` must stay on the direct path (texture-view array), so it is shared per zone and only written on change.
- A repainted `Image` (same descriptor, `COPY_DST`) is written into the existing GPU texture, so materials that bind it need no re-prepare.
- `sync_volumetric_fog_step_count` (`zone_lighting.rs`, PostUpdate) runs `VolumetricFog` at 1 step while every `FogVolume` has zero density (output is identical, only the ambient term remains) and at 64 otherwise. The camera's `VolumetricFog::ambient_intensity` is 0: Bevy adds that ambient as `exp(-depth * (absorption + scattering)) * ambient` regardless of density, which put a white veil on nearby models that grew as the camera zoomed in.

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