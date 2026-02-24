# Wing Shader Troubleshooting Log

## Problem
WGPU crash when typing `/fly` command - shader entry point not found.

## Error
```
wgpu error: Validation Error
Caused by:
  In Device::create_render_pipeline, label = 'alpha_blend_mesh_pipeline'
    Error matching ShaderStages(VERTEX) shader requirements against the pipeline
      Unable to find entry point 'vertex'
```

## Attempts Made

### Attempt 1: Added vertex_shader() method
- **File**: `src/render/wing_material.rs`
- **Change**: Added `fn vertex_shader()` returning the same shader handle
- **Result**: Error changed from "Unable to find entry point 'fragment'" to "Unable to find entry point 'vertex'"

### Attempt 2: Fixed mesh_functions signatures for Bevy 0.16
- **File**: `src/render/shaders/wing_material.wgsl`
- **Changes**:
  - Added `@builtin(instance_index) instance_index: u32` to Vertex struct
  - Changed `get_world_from_local(mesh)` to `get_world_from_local(vertex.instance_index)`
  - Used `mesh_position_local_to_clip()` and `mesh_normal_local_to_world()` helpers
- **Result**: Error persists - "Unable to find entry point 'vertex'"

## Current Status
The shader compiles (cargo build succeeds) but at runtime WGPU cannot find the vertex entry point. This suggests:
1. The shader file might not be loading correctly
2. There might be a shader compilation error that's being silently caught
3. The entry point naming might be wrong

### Attempt 3: Match sky_material.wgsl pattern
- **File**: `src/render/shaders/wing_material.wgsl`
- **Changes**:
  - Removed `instance_index` from Vertex struct
  - Access `mesh.world_from_local[0]` directly like sky_material does
  - Build model matrix manually: `mat4x4<f32>(mesh.world_from_local[0].xyzw, ...)`
- **Result**: Pending user test

## Key Insight
The sky_material.wgsl works and uses `mesh.world_from_local[0]` directly without instance_index. This suggests that for simple meshes without instancing, the mesh binding is accessed differently.

### Attempt 3: Match sky_material.wgsl pattern
- **File**: `src/render/shaders/wing_material.wgsl`
- **Changes**:
  - Removed `instance_index` from Vertex struct
  - Access `mesh.world_from_local[0]` directly like sky_material does
  - Build model matrix manually: `mat4x4<f32>(mesh.world_from_local[0].xyzw, ...)`
- **Result**: Failed - "Unable to find entry point 'vertex'"

### Attempt 4: Use ExtendedMaterial with MaterialExtension
- **Files**: `src/render/wing_material.rs`, `src/render/shaders/wing_material.wgsl`
- **Changes**:
  - Changed from custom Material to ExtendedMaterial<StandardMaterial, WingExtension>
  - MaterialExtension only provides fragment_shader(), uses StandardMaterial vertex
  - Shader uses bevy_pbr imports: pbr_fragment::pbr_input_from_standard_material
  - Uniforms at binding 100 to avoid conflict with base material
- **Result**: Failed - "Unable to find entry point 'fragment'"

## Root Cause Analysis
The shader compiles at build time (cargo build succeeds) but WGPU cannot find entry points at runtime. This suggests:
1. The shader file might not be loading correctly from load_internal_asset!
2. There might be a WGSL compilation error that's silently failing
3. The #import directives might be failing at runtime

## Next Approach
Simplify to use StandardMaterial directly without custom shader for now, to get wings visible.
