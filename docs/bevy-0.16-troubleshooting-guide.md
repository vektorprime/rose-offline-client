# Bevy 0.16.1 Troubleshooting Guide for Engineers

This guide provides a comprehensive list of Bevy 0.16.1 source files to review when troubleshooting specific categories of issues.

## 1. Rendering Visibility and Culling
**Symptoms**: Models not appearing, flickering, or disappearing at certain angles.

- **`crates/bevy_render/src/view/visibility/mod.rs`**: The core visibility system. Check how `ViewVisibility` and `InheritedVisibility` are calculated.
- **`crates/bevy_render/src/primitives/mod.rs`**: Defines `Aabb` and `Frustum`. Check if your model's AABB is correctly calculated.
- **`crates/bevy_render/src/view/visibility/visibility_class.rs`**: New in 0.16.1. Defines the `VisibilityClass` required for standard visibility system integration.
- **`crates/bevy_pbr/src/render/mesh.rs`**: Handles the extraction and queuing of 3D meshes.

## 2. Render Phases and Sorting
**Symptoms**: Transparent objects rendering in the wrong order, UI appearing behind geometry, or objects not being queued for rendering.

- **`crates/bevy_render/src/render_phase/mod.rs`**: Defines `BinnedRenderPhase` (opaque) and `SortedRenderPhase` (transparent).
- **`crates/bevy_core_pipeline/src/core_3d/mod.rs`**: Defines the standard 3D phases: `Opaque3d`, `AlphaMask3d`, `Transparent3d`, and `Transmissive3d`.
- **`crates/bevy_render/src/render_phase/rangefinder.rs`**: Math for calculating view-space Z distance used for sorting.

## 3. Materials and Shaders
**Symptoms**: "Gray boxes", incorrect colors, broken transparency, or shader compilation errors.

- **`crates/bevy_pbr/src/material.rs`**: The `Material` trait and `MaterialPlugin`. Review `specialize` and `alpha_mode` logic.
- **`crates/bevy_pbr/src/render/pbr_functions.wgsl`**: The main PBR lighting and post-processing functions.
- **`crates/bevy_pbr/src/render/view_transformations.wgsl`**: Critical math for coordinate space conversions (World -> View -> Clip -> NDC).
- **`crates/bevy_pbr/src/render/mesh_functions.wgsl`**: Functions for local-to-world transformations and skinning.

## 4. Depth and Coordinate Systems
**Symptoms**: Objects appearing "inside out", depth testing failing, or UI elements flipped/misplaced.

- **`crates/bevy_render/src/render_resource/mod.rs`**: Defines `CompareFunction`. Remember that Bevy uses **Reversed-Z** (1.0 is near, 0.0 is far).
- **`crates/bevy_pbr/src/render/view_transformations.wgsl`**: Review how NDC coordinates are handled. Note that Screen Y increases DOWN, while NDC Y increases UP.

## 5. Skinned Meshes and Animation
**Symptoms**: Characters appearing as "spaghetti", animations not playing, or crashes when spawning animated entities.

- **`crates/bevy_render/src/mesh/skinning.rs`**: Core skinning logic and `SkinnedMesh` component.
- **`crates/bevy_pbr/src/render/mesh_functions.wgsl`**: Review the `skinning` function to see how joint matrices are applied.
- **`crates/bevy_animation/src/lib.rs`**: The animation player and clip sampling logic.

## 6. UI and Text
**Symptoms**: Text not rendering, blurry text, or UI layout issues.

- **`crates/bevy_ui/src/render/mod.rs`**: The UI rendering pipeline.
- **`crates/bevy_text/src/render/mod.rs`**: How text is extracted and rendered.
- **`crates/bevy_text/src/glyph_brush.rs`**: Integration with the glyph layout engine.

## 7. Performance and Batching
**Symptoms**: Low FPS with many objects, high draw call counts.

- **`crates/bevy_render/src/batching/mod.rs`**: Logic for combining multiple instances into a single draw call.
- **`crates/bevy_render/src/render_phase/mod.rs`**: Review `RenderBin` and how `BinnedRenderPhase` groups items by pipeline and material.

---

### Pro-Tip for Bevy 0.16.1:
Always check if your custom components require `VisibilityClass` or specific `#[require(...)]` attributes to participate in the standard render graph. Bevy 0.16 has moved towards more explicit component requirements for performance.
