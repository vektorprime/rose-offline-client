# UI Button Click Fix Attempts

## Problem
UI buttons render but don't respond to hover or click events in bevy_egui 0.34 with Bevy 0.16.

## Attempt 1: Multi-Pass Mode Migration (FAILED)
**Date:** 2026-02-22

### Changes Made:
1. Enabled multi-pass mode: `enable_multipass_for_primary_context: true`
2. Migrated all UI systems from `Update` with `.after(bevy_egui::EguiPreUpdateSet::InitContexts)` to `bevy_egui::EguiContextPass`

### Result:
- Code compiles successfully
- **Buttons still don't respond to hover/click**

### Root Cause Analysis:
The `EguiWantsInput` resource is populated in `PostUpdate` by `write_egui_wants_input_system`, which runs AFTER `EguiContextPass`. This means the `egui_wants_any_pointer_input` run condition uses stale data from the previous frame.

## Attempt 2: Revert to Single-Pass Mode (CURRENT)
**Date:** 2026-02-22

### Changes Made:
1. Disabled multi-pass mode: `enable_multipass_for_primary_context: false`
2. Reverted all UI systems back to `Update` with `.after(bevy_egui::EguiPreUpdateSet::InitContexts)`
3. This matches the working Bevy 0.15.4 approach

### Files Modified:
- `src/lib.rs`: EguiPlugin configuration and all system registrations
- `src/systems/debug_inspector_system.rs`: System registration

### Result:
- Code compiles successfully
- **NEEDS TESTING** - Run game to verify if buttons work

## Attempt 3: Character Clicking Issue Investigation (2026-02-22)

### Problem:
UI buttons now work, but clicking on 3D character models in character select screen doesn't work.

### Root Cause Analysis:
Comparing `src/systems/character_select_system.rs` between versions revealed **CRITICAL BUG**:

**Bevy 0.15.4 (WORKING)** - Line 434-437:
```rust
QueryFilter::new().groups(CollisionGroups::new(
    COLLISION_FILTER_CLICKABLE,              // membership
    COLLISION_GROUP_CHARACTER | COLLISION_GROUP_PLAYER, // filter
))
```

**Bevy 0.16.1 (BROKEN)** - Line 465-468:
```rust
QueryFilter::new().groups(CollisionGroups::new(
    COLLISION_GROUP_PLAYER,      // membership - WRONG!
    COLLISION_FILTER_CLICKABLE,  // filter - WRONG!
))
```

### The Problem:
The `CollisionGroups::new(memberships, filters)` parameters are **SWAPPED**!

In bevy_rapier3d, for a raycast to hit:
1. Ray's memberships must intersect with target's filters
2. Ray's filters must intersect with target's memberships

Character models have:
- Membership: `COLLISION_GROUP_CHARACTER` (bit10)
- Filter: `COLLISION_FILTER_CLICKABLE` (bit18)

Broken version:
- Ray membership (bit9) ∩ character filter (bit18) = **EMPTY** → NO HIT

### Secondary Issues:
1. egui check changed from `wants_pointer_input()` to `is_pointer_over_area() || is_using_pointer()`
2. bevy_rapier3d API changed: `ReadDefaultRapierContext` → `ReadRapierContext.single()`

### Fix Required:
In `src/systems/character_select_system.rs` line 465-468, swap the CollisionGroups parameters:
```rust
let query_filter = QueryFilter::new().groups(CollisionGroups::new(
    COLLISION_FILTER_CLICKABLE,  // membership - ray is a click ray
    COLLISION_GROUP_CHARACTER | COLLISION_GROUP_PLAYER, // filter - can hit characters
));
```

## Attempt 4: Water Not Loading/Rendering Issue (2026-02-22)

### Problem:
Water in the game is not loading at all after upgrading from Bevy 0.15.4 to 0.16.1.

### Root Cause Analysis:
Comparing `src/render/water_material.rs` between versions and checking the Bevy 0.16 migration guide revealed **CRITICAL API CHANGE**:

**Migration Guide (line 1729-1730):**
> "Bevy will now unconditionally call `AsBindGroup::unprepared_bind_group` for your materials, so you must no longer panic in that function. Instead, return the new `AsBindGroupError::CreateBindGroupDirectly` error, and Bevy will fall back to calling `AsBindGroup::as_bind_group` as before."

**Current Code (Bevy 0.16.1) - Line 317-326:**
```rust
fn unprepared_bind_group(
    &self,
    _layout: &BindGroupLayout,
    _render_device: &RenderDevice,
    _param: &mut SystemParamItem<'_, '_, Self::Param>,
    _bindless: bool,  // NEW PARAMETER in 0.16
) -> Result<UnpreparedBindGroup<Self::Data>, AsBindGroupError> {
    // This should never be called since we override as_bind_group
    Err(AsBindGroupError::RetryNextUpdate)  // <-- WRONG!
}
```

### The Problem:
1. In Bevy 0.16, `unprepared_bind_group()` is **always called first**, even when you override `as_bind_group()`
2. Returning `RetryNextUpdate` causes Bevy to keep retrying every frame, never falling back to `as_bind_group()`
3. The water material's custom bind group (with texture arrays) is never created
4. Water meshes have no valid material bind group → water doesn't render

### Secondary Changes Already Applied (CORRECT):
- `bind_group_layout_entries()` signature now has `bindless: bool` parameter ✓
- `PreparedBindGroup.bindings` now uses `BindingResources(vec![])` instead of `vec![]` ✓
- Shader handle uses new `weak_handle!` macro instead of `Handle::weak_from_u128()` ✓

### Fix Required:
In `src/render/water_material.rs` line 325, change:
```rust
Err(AsBindGroupError::RetryNextUpdate)
```
to:
```rust
Err(AsBindGroupError::CreateBindGroupDirectly)
```

This tells Bevy "I implement `as_bind_group()` directly, call that instead."

### Files to Modify:
- `src/render/water_material.rs` - Line 325

### Reference:
- Bevy 0.16 migration guide: `bevy-0.15-to-0.16-migration-guide.md` line 1729-1730
- Bevy 0.16.1 source: `bevy-collection/bevy-0.16.1/crates/bevy_render/src/render_resource/bind_group.rs`
- Example fix: `bevy-collection/bevy-0.16.1/examples/shader/texture_binding_array.rs` line 153

## Reference
- Working Bevy 0.15.4 implementation: `C:\Users\vicha\RustroverProjects\bevy-0-15-4-rose-offline-client\src`
