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

### 9. Missing ViewVisibility on Parent Entity (FIXED - 2026-02-26)
- **Issue**: The `ChatBubbleEntity` parent entity was missing `ViewVisibility::default()` component
- **Comparison**: The working `name_tag_system.rs` includes `ViewVisibility::default()` on the parent entity
- **Root Cause**: In Bevy 0.16, the visibility propagation system requires all three visibility components
  (`Visibility`, `InheritedVisibility`, `ViewVisibility`) for proper visibility computation. Without
  `ViewVisibility` on the parent, the children's inherited visibility was not correctly computed for rendering.
- **Fix**: Added `ViewVisibility::default()` to the parent entity spawn in `chat_bubble_spawn_system.rs`
- **File**: `src/systems/chat_bubble_spawn_system.rs` line 321

## Resolution
The chat bubble visibility issue was caused by a missing `ViewVisibility::default()` component on the
parent `ChatBubbleEntity`. This is a Bevy 0.16 requirement - all entities in the visibility hierarchy
must have all three visibility components for proper rendering.

## Key Lesson
When spawning entities with visibility in Bevy 0.16, always include all three visibility components:
1. `Visibility` (or `Visibility::Inherited`)
2. `InheritedVisibility::default()`
3. `ViewVisibility::default()`

This is especially important for parent entities that have child entities with renderable components.
