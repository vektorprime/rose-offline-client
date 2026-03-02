# Name Tag and Chat Bubble System Fix

## Summary

This document describes the analysis and fixes applied to resolve issues with the name tag and chat bubble systems not working properly.

## Root Cause Analysis

### Issue 1: Pending Name Tag Data Loss (FIXED)

**Location:** [`src/systems/name_tag_system.rs`](../src/systems/name_tag_system.rs)

**Problem:** When `create_nametag_data()` returned `None` (typically because egui font textures weren't ready yet), the pending name tag data was removed from the cache but never re-inserted. This caused entities to never get name tags.

**Original Code:**
```rust
} else if let Some(pending_name_tag_data) = name_tag_cache.pending.remove(&object.entity) {
    if let Some(name_tag_data) = create_nametag_data(
        window_entity,
        &mut egui_context,
        &egui_managed_textures,
        &mut images,
        pending_name_tag_data,  // Ownership transferred here
    ) {
        // Success path
    } else {
        // BUG: pending_name_tag_data was moved and lost!
        continue;  // Data lost forever
    }
}
```

**Fix Applied:**
1. Added `#[derive(Clone)]` to `NameTagPendingData` struct (line 46)
2. Clone the data before passing to `create_nametag_data()`
3. Re-insert the original data back into pending cache if creation fails

```rust
} else if let Some(pending_name_tag_data) = name_tag_cache.pending.remove(&object.entity) {
    if let Some(name_tag_data) = create_nametag_data(
        window_entity,
        &mut egui_context,
        &egui_managed_textures,
        &mut images,
        pending_name_tag_data.clone(),  // Clone instead of move
    ) {
        // Success path
    } else {
        // FIX: Re-insert pending data to try again next frame instead of losing it
        name_tag_cache.pending.insert(object.entity, pending_name_tag_data);
        continue;
    }
}
```

### Issue 2: Chat Bubble Font Texture Timing (FIXED)

**Location:** [`src/systems/chat_bubble_spawn_system.rs`](../src/systems/chat_bubble_spawn_system.rs)

**Problem:** The chat bubble spawn system did not have any retry mechanism for when egui font textures weren't ready. If a chat bubble event was processed before the font textures were uploaded to GPU, the bubble would be created with missing/corrupted text.

**Fix Applied:**
1. Created `PendingChatBubble` struct to hold bubble data
2. Created `ChatBubblePendingCache` with `Local` storage for persistence across frames
3. Added `all_textures_ready` check during font texture iteration
4. Re-insert pending bubbles to cache if textures aren't ready

```rust
// New pending bubble structures
struct PendingChatBubble {
    target_entity: Entity,
    text: String,
    duration: f32,
    color: Color,
    galley: Arc<egui::Galley>,
}

#[derive(Default)]
pub struct ChatBubblePendingCache {
    pending: Vec<PendingChatBubble>,
}

// In the system:
let mut all_textures_ready = true;
for row in galley.rows.iter() {
    // ... process row ...
    if let Some(managed_texture) = egui_managed_textures.0.get(&(window_entity, font_texture_id)) {
        font_source_textures.push(&managed_texture.color_image);
    } else {
        all_textures_ready = false;  // Mark as not ready
    }
}

// Retry next frame if not ready
if !all_textures_ready {
    pending_cache.pending.push(PendingChatBubble { ... });
    continue;
}
```

## Bevy 0.16.1 Visibility System Analysis

### Key Findings from Bevy Source Code

Reviewed `crates/bevy_render/src/view/visibility/mod.rs`:

1. **VisibilityClass Component**: Bevy 0.16.1 introduced `VisibilityClass` as a required component for visibility checking. The `check_visibility` system requires this component.

2. **Current Implementation**: Our `WorldUiRect` entities already have:
   - `Visibility` 
   - `InheritedVisibility`
   - `ViewVisibility`
   - `NoFrustumCulling`

3. **VisibilityClass Not Required**: After analysis, `VisibilityClass` is primarily used for automatic visibility system integration (meshes, sprites). Our custom `WorldUiRect` render pipeline extracts entities directly using `InheritedVisibility`, so `VisibilityClass` is not strictly required.

4. **NoFrustumCulling**: This component is correctly used to bypass frustum culling for billboard UI elements.

## Files Modified

1. **`src/systems/name_tag_system.rs`**
   - Line 46: Added `#[derive(Clone)]` to `NameTagPendingData`
   - Line 462: Changed `pending_name_tag_data` to `pending_name_tag_data.clone()`
   - Lines 468-471: Added re-insertion of pending data on failure

2. **`src/systems/chat_bubble_spawn_system.rs`**
   - Lines 38-51: Added `PendingChatBubble` and `ChatBubblePendingCache` structs
   - Line 64: Added `mut pending_cache: Local<ChatBubblePendingCache>` parameter
   - Lines 72-114: Converted events to pending bubbles and merged with existing pending
   - Lines 167, 203-205: Added `all_textures_ready` tracking
   - Lines 208-219: Added retry logic for when textures aren't ready

## Testing Recommendations

1. **Name Tags**: Verify that name tags appear on all entities after loading into a zone
2. **Chat Bubbles**: Trigger NPC dialogue and verify bubbles appear with correct text
3. **Timing**: Test on slower systems where font texture upload might be delayed
4. **Multiple Entities**: Test with many entities on screen to verify no name tags are lost

## Related Documentation

- [Bevy 0.15 to 0.16 Migration Guide](../bevy-0.15-to-0.16-migration-guide.md)
- [Chat Bubble Architecture](./chat-bubble-architecture.md)
- [Chat Bubble Troubleshooting](./chat-bubble-troubleshooting.md)

---

## Latest Fixes (2026-03-02)

### Root Cause Analysis
After comprehensive analysis of Bevy 0.16.1 source code and comparison with the `custom_phase_item.rs` example, two critical issues were identified:

1. **Missing VisibilityClass Component Hook**: The `WorldUiRect` component was missing the required `#[component(on_add = view::add_visibility_class::<WorldUiRect>)]` attribute that Bevy 0.16.1 requires for visibility system integration.

2. **Wrong Visibility Check in Extraction**: The `extract_world_ui_rects` system was using `InheritedVisibility` instead of `ViewVisibility`, which prevented proper view-based culling.

### Changes Made to `src/render/world_ui.rs`

1. **Added imports** (line 31):
   - Added `view` module import
   - Added `VisibilityClass` import

2. **Added VisibilityClass component hook** (lines 87-90):
   ```rust
   #[derive(Component, Clone)]
   #[require(VisibilityClass)]
   #[component(on_add = view::add_visibility_class::<WorldUiRect>)]
   pub struct WorldUiRect {
   ```

3. **Changed visibility check** (lines 122-135):
   - Changed from `InheritedVisibility` to `ViewVisibility`
   - Updated the query and visibility check accordingly

### Status
- Code compiles successfully
- Awaiting user testing to confirm fix

## Final Resolution (2026-03-02)

### Root Causes Identified

After comprehensive analysis of Bevy 0.16.1 source code and debugging, two root causes were identified:

1. **Shader Bind Group Conflict** (CRITICAL)
   - Location: `src/render/shaders/world_ui.wgsl`
   - Problem: The `#import rose_client::zone_lighting` directive imported a `zone_lighting` variable at `@group(3)`, but the shader also declared `zone_lighting` at `@group(2)`, creating a bind group layout conflict.
   - Fix: Removed the `#import` and defined `ZoneLightingData` struct locally in the shader.

2. **UV Coordinate Inversion** (CRITICAL)
   - Location: `src/render/world_ui.rs:656-661`
   - Problem: UV Y coordinates were inverted - top of screen quad mapped to bottom of texture and vice versa.
   - Fix: Corrected the UV mapping to properly align screen coordinates with texture coordinates.

### Fixes Applied

| Fix | File | Necessity | Status |
|-----|------|-----------|--------|
| Shader bind group conflict | `world_ui.wgsl` | ✅ NECESSARY | Applied |
| UV coordinate inversion | `world_ui.rs` | ✅ NECESSARY | Applied |
| VisibilityClass component hook | `world_ui.rs` | ⚠️ Speculative | Applied (good practice) |
| ViewVisibility extraction | `world_ui.rs` | ⚠️ Optimization | Applied (improvement) |

### What Was NOT the Issue

The following were attempted but were not root causes:
- NameTagPendingData clone/re-insert logic (addressed timing, not rendering)
- ChatBubblePendingCache retry logic (addressed timing, not rendering)
- VisibilityClass requirement (custom pipelines extract directly)

### Status: ✅ RESOLVED

Name tags and chat bubbles are now rendering correctly with text.
