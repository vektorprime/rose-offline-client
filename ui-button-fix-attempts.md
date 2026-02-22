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

## Reference
- Working Bevy 0.15.4 implementation: `C:\Users\vicha\RustroverProjects\bevy-0-15-4-rose-offline-client\src`
