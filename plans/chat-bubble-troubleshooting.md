# Chat Bubble Troubleshooting Log

## Problem
Chat bubbles and name tags are not visible in the game despite successful compilation and no runtime errors.

## Current Status: FIX APPLIED - AWAITING VERIFICATION

### Latest Investigation Date: 2026-03-02

## What's Working (Based on Logs)
1. **Spawning**: Chat bubble entities are spawned successfully with correct hierarchy
2. **Extraction**: `[WORLD_UI_EXTRACT] visible=3, hidden=124, missing_image=0`
3. **Queueing**: `[WORLD_UI_QUEUE] extracted=3, gpu_missing=0, frustum_culled=0, screen_culled=0, queued=3`
4. **Projection**: `clip_pos` values are within valid NDC range (z between 0 and 1)
5. **Vertex Buffer**: Vertices are being created and written to GPU

## Fixes Applied

### Fix 1: Visibility Components
**Issue**: Entities were spawned with manually inserted `InheritedVisibility::default()` and `ViewVisibility::default()` which default to `HIDDEN`.

**Fix Applied**: Removed manual insertion of these components from:
- `src/systems/chat_bubble_spawn_system.rs` - Parent bubble, background rect, text rect
- `src/systems/name_tag_system.rs` - Parent name tag and all child rect entities

**Status**: Applied

### Fix 2: Shader Clip Space Calculation
**Issue**: Vertex shader was dividing by `w` to get NDC, adding offset, but not multiplying back by `w` before output.

**Fix Applied**:
```wgsl
// Before (buggy):
let screen_pos = (clip_pos.xy / clip_pos.w) + ndc_offset;
out.clip_position = vec4<f32>(screen_pos, clip_pos.z, clip_pos.w);

// After (fixed):
let ndc_pos = clip_pos.xy / clip_pos.w + ndc_offset;
out.clip_position = vec4<f32>(ndc_pos * clip_pos.w, clip_pos.z, clip_pos.w);
```

**File**: `src/render/shaders/world_ui.wgsl`

**Status**: Applied

### Fix 3: Visibility Inheritance
**Issue**: Child entities were using `Visibility::default()` instead of `Visibility::Inherited`.

**Fix Applied**: Changed `Visibility::default()` to `Visibility::Inherited` for child rect entities.

**Status**: Applied

### Fix 4: CRITICAL - SetWorldUiViewBindGroup Render Command Types (NEW)
**Issue**: The `SetWorldUiViewBindGroup` render command had completely broken type definitions:
- `ViewQuery = Option<Read<ViewUniformOffset>>` - Should NOT be wrapped in `Option`
- `ItemQuery` had **30 nested `Option`s** wrapping `()` - Should just be `()`

This was the root cause of rendering failure. The render command's type system was so broken that it likely never properly matched or executed.

**Fix Applied**:
```rust
// Before (broken):
type ViewQuery = Option<Read<ViewUniformOffset>>;
type ItemQuery = Option<Option<Option<...30 times...<()>>>>;

// After (fixed):
type ViewQuery = Read<ViewUniformOffset>;
type ItemQuery = ();
```

**File**: `src/render/world_ui.rs` lines 444-466

**Status**: Applied - Build successful, awaiting runtime verification

## Current Investigation Areas

### Area 1: Render Pipeline Configuration
The render pipeline may not be properly configured:
- Pipeline specialization may have issues
- Bind groups may not be set correctly
- Zone lighting bind group (group 2) may be missing or incorrect

### Area 2: Transparent3d Phase Sorting
- Distance calculation uses `-999999.0` offset to render on top
- Transparent3d sorts back-to-front (ascending order)
- Items with smaller distance render first (behind), larger distance render last (in front)
- The negative offset should make them render first, not last

### Area 3: Depth Buffer Configuration
- Pipeline uses `depth_compare: CompareFunction::Greater` (reverse Z)
- `depth_write_enabled: false`
- May conflict with main scene depth buffer

### Area 4: Vertex Buffer State
- Vertices are written to GPU with `write_buffer()`
- Buffer may not be properly bound during draw call
- Instance count is `0..1` (one instance per batch)

## Debug Logs Analysis

From latest test:
```
[CHAT_BUBBLE_DEBUG] Successfully created chat bubble! Parent: 2391v2, Children: bg=9043v14, text=9680v2
[WORLD_UI_QUEUE] Projection: world_pos=Vec3(5495.425, 1.8891735, -5484.916), clip_pos=Vec3(-7.9234596e-5, 0.5079396, 0.08113623), order=9
[WORLD_UI_QUEUE] Projection: world_pos=Vec3(5495.425, 1.8891735, -5484.916), clip_pos=Vec3(-7.9234596e-5, 0.5079396, 0.08113623), order=10
[WORLD_UI_QUEUE] extracted=3, gpu_missing=0, frustum_culled=0, screen_culled=0, queued=3
[WORLD_UI_EXTRACT] total=127, visible=3, hidden=124, missing_image=0
```

**Observations**:
- `clip_pos.z` is `0.08113623` (within 0..1 range, valid)
- `clip_pos.x` is nearly 0 (centered horizontally)
- `clip_pos.y` is `0.5079396` (slightly above center)
- 3 items queued (name tag + 2 chat bubble parts)

## Next Steps to Investigate

1. **Verify pipeline is being cached correctly**
   - Check if `CachedRenderPipelineId` is valid
   - Add logging to pipeline specialization

2. **Verify bind groups are being set**
   - Add logging to `SetWorldUiViewBindGroup`
   - Add logging to `SetWorldUiMaterialBindGroup`
   - Verify zone lighting bind group exists

3. **Check draw call execution**
   - Add logging to `DrawWorldUiBatch::render`
   - Verify vertex buffer is bound correctly
   - Verify vertex range is correct

4. **Test without zone lighting**
   - Remove `ZONE_LIGHTING_GROUP_2` shader def
   - Create simplified shader without fog

5. **Compare with working Bevy examples**
   - Review `custom_shader_instancing` example
   - Check how they add to Transparent3d phase

## Reference Files

### Key Source Files
- `src/systems/chat_bubble_spawn_system.rs` - Spawns chat bubble entities
- `src/systems/name_tag_system.rs` - Spawns name tag entities
- `src/render/world_ui.rs` - Render pipeline and queue system
- `src/render/shaders/world_ui.wgsl` - Vertex and fragment shaders
- `src/components/chat_bubble.rs` - Chat bubble components

### Bevy 0.16.1 Reference
- `crates/bevy_core_pipeline/src/core_3d/mod.rs` - Transparent3d definition
- `crates/bevy_render/src/view/view.wgsl` - View uniform structure
- `crates/bevy_render/src/view/mod.rs` - View uniform Rust structure

## Historical Issues (Resolved)

### Issue 1: Frame Timing
**Symptom**: Queue showed `extracted=1` while extract showed `visible=3`
**Cause**: Logs were from different frames
**Resolution**: This was a logging artifact, actual data was correct

### Issue 2: Font Texture Not Ready
**Symptom**: Chat bubbles weren't spawning on first attempt
**Cause**: egui font textures weren't loaded yet
**Resolution**: Added pending cache mechanism to retry on next frame

## Component Hierarchy

```
Target Entity (character/monster)
└── ChatBubbleEntity (parent)
    ├── ChatBubble (component for lifetime)
    ├── Visibility::Inherited
    ├── Transform (at model_height + offset)
    ├── Background Rect Entity
    │   ├── ChatBubbleBackground (marker)
    │   ├── WorldUiRect (render data)
    │   └── Visibility::Inherited
    └── Text Rect Entity
        ├── ChatBubbleText (marker)
        ├── WorldUiRect (render data)
        └── Visibility::Inherited
```

## Render Pipeline Flow

```
ExtractSchedule
└── extract_world_ui_rects
    └── Queries WorldUiRect entities
    └── Checks InheritedVisibility
    └── Populates ExtractedWorldUi resource

Render Schedule
└── queue_world_ui_meshes
    └── For each view:
        └── Sort rects by distance
        └── Create vertex data
        └── Create bind groups
        └── Add to Transparent3d phase

MainTransparentPass3dNode
└── Renders Transparent3d phase items
    └── SetItemPipeline
    └── SetWorldUiViewBindGroup<0>
    └── SetWorldUiMaterialBindGroup<1>
    └── SetZoneLightingBindGroup<2>
    └── DrawWorldUiBatch
```
