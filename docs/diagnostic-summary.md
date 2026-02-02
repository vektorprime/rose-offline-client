# Bevy 0.14.2 Black Screen Diagnostic Summary

## Diagnostic Systems Added

### Phase 0: Standard Cube Test
- Location: `src/lib.rs` function `spawn_test_cube`
- Purpose: Isolates custom material issues vs fundamental rendering issues
- Expected: Red cube should appear if core rendering works

### Phase 1: Camera Setup Verification
- Location: `src/lib.rs` function `diagnose_camera_system`
- Checks: Camera3d marker, all visibility components, is_active status
- Expected log: `[CAMERA DIAGNOSTIC] Camera Entity(...): is_active=true, ...`

### Phase 2: Render Extraction Diagnostics
- Location: `src/lib.rs` function `diagnose_main_world_meshes`
- Resource: `RenderExtractionDiagnostics` in `src/resources/debug_render.rs`
- Checks: Main World mesh count, visibility states
- Expected log: `[MAIN WORLD] Total meshes with Visibility: N`

### Phase 3: Material Plugin Registration
- Location: All material plugin build() functions
- Checks: MaterialPlugin order, Material trait implementations
- Expected log: `[MATERIAL PLUGIN] XxxMaterial plugin built`

### Phase 4: WGSL Shader Compatibility
- Files checked: 12 shader files in `src/render/shaders/`
- Issues fixed: Struct alignment, binding syntax
- All shaders now compatible with naga 0.14.2

### Phase 5: Render Graph Execution
- Location: `src/lib.rs` function `diagnose_mesh_materials`
- Checks: Mesh material assignments
- Expected log: `[MATERIAL DIAGNOSTIC] Meshes with StandardMaterial: N`

### Phase 6: GPU Buffer Upload
- Location: `src/lib.rs` functions `diagnose_gpu_mesh_upload`, `diagnose_asset_loading`
- Checks: Mesh asset loading, GPU readiness
- Expected log: `[GPU MESH] Total mesh entities: X, Loaded mesh assets: Y`

## Log Filter for Debugging

To see all diagnostic output, run with:
```
RUST_LOG=info cargo run
```

Or in code, the LogPlugin is already configured with:
```rust
filter: "wgpu=error,bevy_render=debug,bevy_pbr=debug"
```

## Decision Tree

```
1. Does red test cube render?
   YES → Custom material/shader issue → Check Phase 4
   NO  → Continue to 2

2. Does camera diagnostic show active camera?
   NO  → Fix camera setup (Phase 1)
   YES → Continue to 3

3. Do main world meshes exist?
   NO  → Entity spawning issue
   YES → Continue to 4

4. Are material plugins logging "built"?
   NO  → Check plugin registration (Phase 3)
   YES → Continue to 5

5. Are meshes GPU-ready?
   NO  → Asset loading issue (Phase 6)
   YES → Continue to 6

6. All checks pass but still black?
   → Use RenderDoc for GPU-level debugging
```

## RenderDoc Setup

1. Download RenderDoc from https://renderdoc.org/
2. Launch RenderDoc
3. File → Launch Application
4. Set Executable Path to your built binary: `target/debug/rose-offline-client.exe`
5. Set Working Directory to project root
6. Click Launch
7. Press F12 to capture a frame
8. Analyze the capture

### What to Check in RenderDoc

- **Event Browser**: Look for Draw calls (glDraw*, vkCmdDraw*)
- **Pipeline State**: Verify correct shaders are bound
- **Mesh Viewer**: Check vertex positions are valid (not NaN)
- **Texture Viewer**: Verify textures are loaded and bound
- **VS Output**: Check vertex shader output positions
