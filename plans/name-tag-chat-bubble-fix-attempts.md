# Name Tag and Chat Bubble Fix Attempts

## Date: 2026-03-02

## Initial Problem Description
Name tags and chat bubbles are not rendering despite the render pipeline appearing to work correctly. Analysis shows:
- Extraction succeeds (entities are extracted to render world)
- Queueing succeeds (phase items are queued)
- Vertex buffer creation succeeds
- But nothing appears on screen

## Identified Issues from Analysis

1. **Missing VisibilityClass Component**: In Bevy 0.16.1, entities with visibility need `VisibilityClass::default()` to be properly classified in the render pipeline.

2. **Potential Render Command Execution**: Need to verify that `SetZoneLightingBindGroup` render command is actually executing.

## Files Involved
- `src/systems/chat_bubble_spawn_system.rs` - Chat bubble entity spawning
- `src/systems/name_tag_system.rs` - Name tag entity spawning  
- `src/render/world_ui.rs` - World UI render commands

## Attempt Log

### Attempt 1: Add VisibilityClass Component
**Date**: 2026-03-02
**Status**: Implemented - Pending Testing

**Changes Made**:
- [x] Added `VisibilityClass::default()` to chat bubble entities
- [x] Added `VisibilityClass::default()` to name tag entities
- [x] Added diagnostic logging to `SetZoneLightingBindGroup`

**Detailed Changes**:

1. **`src/systems/chat_bubble_spawn_system.rs`**:
   - Line 12: Added import for `VisibilityClass` from `bevy::render::view`
   - Line 445: Added `VisibilityClass::default()` to parent bubble entity spawn
   - Line 473: Added `VisibilityClass::default()` to background rect entity spawn
   - Line 501: Added `VisibilityClass::default()` to text rect entity spawn

2. **`src/systems/name_tag_system.rs`**:
   - Line 16: Added import for `VisibilityClass` from `bevy::render::view`
   - Line 510: Added `VisibilityClass::default()` to name tag parent entity spawn
   - Line 663: Added `VisibilityClass::default()` to name tag name rect spawns
   - Line 676: Added `VisibilityClass::default()` to target mark spawns
   - Line 689: Added `VisibilityClass::default()` to healthbar background spawns
   - Line 706: Added `VisibilityClass::default()` to healthbar foreground spawns

3. **`src/render/zone_lighting.rs`**:
   - Line 647: Added `log::info!` diagnostic logging to `SetZoneLightingBindGroup::render()` to verify execution

**Results**:
_Pending testing - User needs to compile and run the game to verify if name tags and chat bubbles now render correctly_

---

## Notes
- VisibilityClass is from `bevy_render::view::VisibilityClass`
- This component was introduced/changed in Bevy 0.16 to help classify entities in the visibility system

---

### Attempt 2: Bevy 0.18 Name Tag Regression (Font Atlas Extraction)
**Date**: 2026-03-24
**Status**: Implemented - Pending Testing

**Observed Symptom**:
- Name tags fail after Bevy 0.16 -> 0.18 upgrade.
- Chat bubbles already include a Bevy 0.39-compatible glyph extraction path.

**Hypothesis**:
1. Name tags still used old direct RGBA glyph copy from egui atlas.
2. In `bevy_egui 0.39`, atlas channel packing can differ; direct RGBA assumptions may produce transparent/incorrect glyphs.
3. Name tag font upload forcing used a single shared egui `Area` id, which is less robust when many entities are pending.

**Changes Made** (`src/systems/name_tag_system.rs`):
- Updated glyph copy to robust coverage extraction:
  - `coverage = max(r, g, b, a)`
  - write output as white RGB + coverage alpha
  - tint remains controlled by `WorldUiRect.color`
- Changed forced egui upload id to be entity-specific:
  - from `Id::new("nametag_font_upload")`
  - to `Id::new(("nametag_font_upload", object.entity))`

**Expected Effect**:
- Name tag text texture generation should match the now-working chat bubble strategy on Bevy 0.18 / bevy_egui 0.39.
- Reduced risk of missing glyph uploads when many tags are generated in the same frame.
