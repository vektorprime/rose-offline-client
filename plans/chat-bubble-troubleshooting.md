# Chat Bubble Troubleshooting Log

## Problem
Chat bubbles are not visible in the game despite successful compilation and no runtime errors.

## What's Working (Based on Logs)
1. **Spawning**: `[CHAT_BUBBLE] Spawning bubble for entity ... with height ...`
2. **Extraction**: `[WORLD_UI_EXTRACT] visible=3, hidden=214, missing_image=0`
3. **Queueing**: `[WORLD_UI_QUEUE] extracted=3, gpu_missing=0, frustum_culled=0, screen_culled=0, queued=3`
4. **Projection**: `clip_pos` values are now within NDC range (-1 to 1)

## Fixes Attempted

### 1. B0001 Query Conflict Error (FIXED)
- **Issue**: Two mutable queries for `WorldUiRect` caused Bevy error B0001
- **Fix**: Added `Without<>` constraints to make queries disjoint
- **File**: `src/systems/chat_bubble_update_system.rs`

### 2. Missing ModelHeight Component (FIXED)
- **Issue**: `ModelHeight` was required but not always present on entities
- **Fix**: Made `ModelHeight` optional with default fallback of 2.0
- **File**: `src/systems/chat_bubble_spawn_system.rs`

### 3. Missing ClientEntityName on Monsters (FIXED)
- **Issue**: `ClientEntityName` was required for monsters but not always present
- **Fix**: Made `ClientEntityName` optional with "Monster" fallback
- **File**: `src/systems/monster_chatter_system.rs`

### 4. Weak Image Handles (FIXED)
- **Issue**: Using `clone_weak()` caused images to be deallocated immediately
- **Fix**: Changed to strong handles by transferring ownership
- **File**: `src/systems/chat_bubble_spawn_system.rs`

### 5. Y Coordinate Inversion in Screen Culling (FIXED)
- **Issue**: Screen culling was rejecting valid bubbles due to inverted Y
- **Original**: `(clip_pos.y + 1.0) / 2.0 * view_height`
- **Fixed**: `(1.0 - clip_pos.y) / 2.0 * view_height`
- **File**: `src/render/world_ui.rs`

### 6. Excessive Vertical Offset (FIXED)
- **Issue**: `CHAT_BUBBLE_VERTICAL_OFFSET = 20.0` placed bubbles too high
- **Fix**: Reduced to `2.0`
- **File**: `src/systems/chat_bubble_spawn_system.rs`

### 7. Missing GPU Buffer Write (FIXED)
- **Issue**: Vertex buffer was populated but never written to GPU
- **Fix**: Added `world_ui_meta.vertices.write_buffer(&render_device, &render_queue);`
- **File**: `src/render/world_ui.rs`

### 8. Shader World Space Transform (FIXED)
- **Issue**: Shader used `clip_from_view` directly on world-space coordinates
- **Fix**: Added `view_from_world` transform first
- **File**: `src/render/shaders/world_ui.wgsl`

## Current Status
All fixes applied, build succeeds, extraction and queue work correctly, but bubbles still not visible.

## Remaining Investigation Areas
1. **Shader binding issues** - Zone lighting bind group may not be set correctly
2. **Pipeline specialization** - May not be creating valid pipeline
3. **Render command execution** - Draw commands may not be executing
4. **Depth buffer issues** - Depth compare may be culling everything
5. **Vertex data format** - May not match shader expectations
6. **View uniform binding** - May not be available in render phase

## Next Steps
1. Check if name tags (other WorldUiRects) are visible - they use the same system
2. Add debug logging to render commands to verify they're executing
3. Check if pipeline is being created and cached correctly
4. Verify bind groups are being created and bound correctly
5. Check depth/stencil state configuration
