# Bevy 0.16 Feature Investigation: Occlusion Culling and GPU-Driven Rendering

This document summarizes the investigation into the configuration and status of Bevy 0.16 features, specifically **Occlusion Culling** and **GPU-Driven Rendering**, within the `rose-offline-client` project.

## 1. Bevy 0.16.1 Source Code Review

Based on the review of Bevy 0.16.1 source code (specifically `bevy_pbr` and `bevy_render` crates):

### GPU-Driven Rendering (GPU Mesh Preprocessing)
- **Implementation**: Located in `crates/bevy_pbr/src/render/gpu_preprocess.rs`.
- **Mechanism**: Uses compute shaders (`mesh_preprocess.wgsl`, `build_indirect_params.wgsl`) to perform frustum culling, occlusion culling, and transform calculations on the GPU.
- **Indirect Drawing**: If supported by hardware, Bevy 0.16 uses indirect draw calls (`multi_draw_indirect` or `multi_draw_indirect_count`) to submit batches to the GPU.
- **Activation**: Enabled by default in `PbrPlugin` via the `use_gpu_instance_buffer_builder` field (defaults to `true`). It can be disabled by adding the `NoIndirectDrawing` component to a camera.

### Occlusion Culling
- **Implementation**: Integrated into the GPU mesh preprocessing pipeline.
- **Mechanism**: A two-phase approach using a depth pyramid (`ViewDepthPyramid`) generated from a depth prepass.
- **Activation**: Requires the following on the `Camera` entity:
    1. `OcclusionCulling` component (from `bevy_render::experimental::occlusion_culling`).
    2. `DepthPrepass` component (from `bevy_core_pipeline::prepass`).
- **Hardware Support**: Requires compute shader support and specific WGPU features.

## 2. Project Configuration Analysis (`rose-offline-client`)

### Occlusion Culling Configuration
The investigation confirmed that Occlusion Culling is explicitly configured in `src/lib.rs`:

```rust
// src/lib.rs:1848
// Prepasses for depth (required for some effects and GPU occlusion culling)
DepthPrepass,
// GPU Occlusion Culling - Bevy 0.16 experimental feature
// Culls objects hidden behind other objects to improve performance
OcclusionCulling,
```

The main camera is spawned with both `DepthPrepass` and `OcclusionCulling` components, satisfying the requirements for Bevy's experimental occlusion culling.

### GPU-Driven Rendering Configuration
GPU-driven rendering is enabled through the default plugin configuration:

- **Plugin Registration**: In `src/lib.rs` (within `run_client`), `PbrPlugin::default()` is added as part of the `DefaultPlugins` setup.
- **Default Behavior**: Since `PbrPlugin::default()` has `use_gpu_instance_buffer_builder` set to `true`, GPU mesh preprocessing is active.
- **Indirect Drawing**: No `NoIndirectDrawing` components were found in the codebase, meaning indirect drawing is utilized where supported by the hardware.

### Diagnostic Confirmation
The file `src/diagnostics/render_diagnostics.rs` provides further evidence that these features are active. It contains specific logic to debug crashes occurring within the Bevy GPU preprocessing internal code:

```rust
// src/diagnostics/render_diagnostics.rs:9
//! - Called from: `bevy_pbr::render::gpu_preprocess::impl$2::run`
```

This indicates that the `gpu_preprocess` node is part of the render graph and is executing during the frame.

## 3. Conclusion

Both **Occlusion Culling** and **GPU-Driven Rendering** are properly configured and active in the `rose-offline-client` project:

1.  **Occlusion Culling** is explicitly enabled on the main camera with the required `DepthPrepass`.
2.  **GPU-Driven Rendering** is enabled via the default `PbrPlugin` configuration and is actively running as confirmed by the presence of related diagnostic code.

These features leverage Bevy 0.16's modern rendering pipeline to improve performance by shifting culling and draw call generation to the GPU.

## 4. Recommended Bevy 0.16 Features for Quality Enhancement

The following features are available in Bevy 0.16 but are not currently implemented in the project. They are relatively easy to add and would significantly enhance visual quality:

### 1. Screen Space Reflections (SSR)
Adds real-time reflections to surfaces based on screen-space data.
- **Impact**: Enhances water, metallic armor, and shiny surfaces.
- **Implementation**: Add `ScreenSpaceReflections` component to the camera.

### 2. Temporal Anti-Aliasing (TAA)
Provides superior edge smoothing and reduces shimmering on fine details during movement.
- **Impact**: Cleaner image, especially for vegetation and fine meshes.
- **Implementation**: Add `TemporalAntiAlias` component to the camera (replaces SMAA).

### 3. Motion Blur
Adds cinematic blur to moving objects and camera rotations.
- **Impact**: Smoother perceived motion, especially at lower frame rates.
- **Implementation**: Add `MotionBlur` component to the camera.

### 4. Auto Exposure
Dynamically adjusts scene brightness based on luminance.
- **Impact**: Realistic eye-adaptation when moving between dark and bright areas.
- **Implementation**: Add `AutoExposure` component to the camera.

### 5. Contrast Adaptive Sharpening (CAS)
Restores sharpness without introducing artifacts.
- **Impact**: Excellent companion for TAA to maintain image clarity.
- **Implementation**: Add `ContrastAdaptiveSharpening` component to the camera.

### 6. Deferred Rendering
Changes the lighting pass to be independent of scene complexity.
- **Impact**: Significant performance boost if many point/spot lights are used.
- **Implementation**: Set `DefaultOpaqueRendererMethod` resource to `OpaqueRendererMethod::Deferred`.
