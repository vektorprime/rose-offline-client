# Rendering Extraction Failure - Root Cause Analysis & Resolution Plan

## Problem Statement
Despite implementing fixes for ViewVisibility issues, the application still shows:
- `[RENDER WORLD] 0 entities extracted to render world`
- `[RENDER PHASE] Transparent3d render phase: 0 items`
- `[RENDER PHASE] CRITICAL: No items in Transparent3d render queue!`

## Root Cause Analysis

### Issue #1: Camera ViewVisibility is FALSE (Critical)
**Location**: [`src/lib.rs:1639`](src/lib.rs:1639)

```rust
ViewVisibility::default(),  // ← Returns FALSE in Bevy 0.14.2!
```

**Problem**: In Bevy 0.14.2, `ViewVisibility::default()` returns `false`. The camera is spawned with `ViewVisibility::default()`, which means the camera itself is marked as not visible to itself.

**Impact**: When the camera has `ViewVisibility::false`, Bevy's visibility system may not compute visibility for entities that should be visible to this camera, preventing extraction.

**Fix**: Change camera spawn to use `ViewVisibility::VISIBLE` or remove `ViewVisibility::default()` entirely and let Bevy compute it.

### Issue #2: Entities May Need ViewVisibility Component
**Location**: [`src/model_loader.rs:1215-1223`](src/model_loader.rs:1215-1223) (spawn_model function)

```rust
let mut entity_commands = commands.spawn((
    mesh,
    material,
    Transform::default(),
    GlobalTransform::default(),
    Visibility::Inherited,
    InheritedVisibility::default(),
    Aabb::default(),
));
```

**Problem**: Entities are spawned WITHOUT `ViewVisibility` component. While Bevy's visibility system should compute `ViewVisibility`, the extraction system may require the component to exist.

**Impact**: Entities may not be extracted if the extraction system expects `ViewVisibility` component to exist.

**Fix**: Add `ViewVisibility::default()` to entity spawn, OR verify that Bevy 0.14.2 automatically adds it during visibility computation.

### Issue #3: Unclear Bevy 0.14.2 Visibility Behavior
**Problem**: There's uncertainty about:
1. Whether `ViewVisibility` should be explicitly added to entities
2. Whether `ViewVisibility::default()` should be used or avoided
3. How Bevy 0.14.2's visibility system differs from 0.13

**Impact**: Without clear understanding, we may be applying incorrect fixes.

**Resolution**: Need to consult Bevy 0.14.2 documentation and migration guide.

## Resolution Plan

### Step 1: Fix Camera ViewVisibility
**File**: [`src/lib.rs`](src/lib.rs)
**Location**: Line 1639
**Action**: Change `ViewVisibility::default()` to `ViewVisibility::VISIBLE`

```rust
// BEFORE:
ViewVisibility::default(),

// AFTER:
ViewVisibility::VISIBLE,
```

**Rationale**: The camera MUST be visible to itself for entities to be extracted. Using `VISIBLE` ensures the camera can see entities.

### Step 2: Verify Entity ViewVisibility Requirements
**Files to Check**:
- [`src/model_loader.rs`](src/model_loader.rs) - spawn_model function (line 1215)
- [`src/zone_loader.rs`](src/zone_loader.rs) - All entity spawning locations
- [`src/effect_loader.rs`](src/effect_loader.rs) - Particle spawning
- [`src/resources/damage_digits_spawner.rs`](src/resources/damage_digits_spawner.rs) - Damage digit spawning

**Action**: For each entity spawn location, determine if `ViewVisibility` component should be added.

**Options**:
1. **Add ViewVisibility::default()** - Explicitly add the component and let Bevy compute its value
2. **Don't add ViewVisibility** - Rely on Bevy to add it automatically during visibility computation

**Recommendation**: Try Option 1 first (add `ViewVisibility::default()`), as it's more explicit and matches Bevy's expected component structure.

### Step 3: Test Entity Extraction
**Action**: After applying fixes, run the application and check diagnostic logs.

**Expected Results**:
- `[RENDER WORLD]` should show > 0 entities extracted
- `[RENDER PHASE]` should show items in Transparent3d queue
- Black screen should be resolved

**If Still Failing**:
- Check diagnostic logs for visibility component states
- Verify entities have `Visibility`, `InheritedVisibility`, and `ViewVisibility`
- Check if camera is actually active and positioned correctly

### Step 4: Validate Rendering Pipeline
**Action**: Once entities are being extracted, verify:
1. Meshes are visible in scene
2. Materials are rendering correctly
3. Lighting is working (if applicable)
4. No other rendering issues exist

## Implementation Order

1. **Fix camera ViewVisibility** (highest priority - most likely root cause)
2. **Add ViewVisibility to entity spawning** (if needed)
3. **Test and validate** with diagnostic logs
4. **Iterate** based on results

## Alternative Approaches

If the above fixes don't resolve the issue:

### Approach A: Use Bevy's Built-in Visibility System
Instead of manually managing visibility components, rely entirely on Bevy's built-in systems:
- Remove all explicit `ViewVisibility` from spawning
- Ensure entities have `Visibility` and `InheritedVisibility`
- Let Bevy's `VisibilityPropagate` and `CheckVisibility` systems handle the rest

### Approach B: Force Zone Visibility
If entities are children of zone entity and zone entity is not visible:
- Ensure zone entity has `Visibility::Visible`
- Verify zone entity's `ViewVisibility` is true
- Check parent-child visibility propagation

### Approach C: Disable Frustum Culling
If frustum culling is incorrectly culling visible entities:
- Add `NoFrustumCulling` component to entities
- Or adjust camera frustum parameters

## Diagnostic Verification

After implementing fixes, verify with these diagnostic systems:

1. **Main World Visibility**: Check if entities have `ViewVisibility=true`
2. **Render World Extraction**: Check if entities are being extracted (> 0)
3. **Render Phase**: Check if Transparent3d queue has items (> 0)
4. **Camera-Entity Distance**: Verify entities are within camera range
5. **Material Plugin Verification**: Confirm materials are being extracted

## Success Criteria

The issue is resolved when:
- ✅ Diagnostic logs show entities being extracted to Render World
- ✅ Render phase queues have items
- ✅ Visual rendering works (meshes visible on screen)
- ✅ No black screen
