# Rendering Extraction Failure - Root Cause Analysis & Resolution Plan (UPDATED)

## Problem Statement
Despite implementing fixes for ViewVisibility issues, application still shows:
- `[RENDER WORLD] 0 entities extracted to render world`
- `[RENDER PHASE] Transparent3d render phase: 0 items`
- `[RENDER PHASE] CRITICAL: No items in Transparent3d render queue!`

## Key Findings from Bevy 0.14.2 Research

### Finding #1: ViewVisibility is Computed Component
From Bevy documentation:
> "ViewVisibility: Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering. Each frame, this will be reset to false."

**Implications**:
- `ViewVisibility` is NOT a component you set manually
- It's computed by Bevy's visibility system each frame
- It's reset to `false` at the start of each frame
- The visibility system (VisibilityPropagate, CheckVisibility) computes its value

### Finding #2: Bevy 0.14.2 Component Requirements
From Bevy 0.14 migration guide:
> "The Visibility component now requires InheritedVisibility and ViewVisibility, meaning that you can now just require Visibility if you want your..."

**Required Components for Visibility**:
1. `Visibility` - The desired visibility state (Visible, Inherited, Hidden)
2. `InheritedVisibility` - Computed inherited visibility from parent
3. `ViewVisibility` - Computed final visibility (reset to false each frame, then computed)

## Root Cause Analysis

### Issue #1: Camera ViewVisibility Initialization (CONFIRMED CRITICAL)
**Location**: [`src/lib.rs:1639`](src/lib.rs:1639)

```rust
ViewVisibility::default(),  // ← Returns FALSE in Bevy 0.14.2!
```

**Problem**: While `ViewVisibility` is computed each frame, the initial value of `default()` (which is `false`) may interfere with the visibility system's computation.

**Impact**: The camera's initial `ViewVisibility::false` may prevent the visibility system from properly computing entity visibility for extraction.

**Fix**: Remove `ViewVisibility::default()` from camera spawn entirely. Let Bevy's visibility system compute it automatically.

### Issue #2: Entities Missing ViewVisibility Component (LIKELY)
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

**Problem**: Entities are spawned WITHOUT `ViewVisibility` component.

**Impact**: While Bevy's visibility system should add `ViewVisibility` during computation, if the extraction system requires the component to exist, entities won't be extracted.

**Fix**: Add `ViewVisibility::default()` to entity spawn. This gives the visibility system a component to update.

### Issue #3: Uncertainty About Visibility System Behavior
**Problem**: There's uncertainty about:
1. Whether `ViewVisibility` should be explicitly added to entities
2. How Bevy 0.14.2's visibility computation differs from 0.13
3. Whether removing `ViewVisibility::default()` from entities was correct

**Resolution**: Based on research findings:
- `ViewVisibility` is computed each frame, so initial value doesn't matter much
- But extraction system likely requires component to exist
- Best practice: Add `ViewVisibility::default()` to entity spawns to ensure component exists

## Resolution Plan (UPDATED)

### Step 1: Remove Camera ViewVisibility (HIGHEST PRIORITY)
**File**: [`src/lib.rs`](src/lib.rs)
**Location**: Line 1639
**Action**: REMOVE `ViewVisibility::default()` from camera spawn

```rust
// BEFORE:
ViewVisibility::default(),

// AFTER:
// (remove this line entirely)
```

**Rationale**: Since `ViewVisibility` is computed each frame, we should NOT set it manually. Let Bevy's visibility system compute it automatically.

### Step 2: Add ViewVisibility to Entity Spawning
**Files to Check**:
- [`src/model_loader.rs`](src/model_loader.rs) - spawn_model function (line 1222)
- [`src/zone_loader.rs`](src/zone_loader.rs) - All entity spawning locations
- [`src/effect_loader.rs`](src/effect_loader.rs) - Particle spawning
- [`src/resources/damage_digits_spawner.rs`](src/resources/damage_digits_spawner.rs) - Damage digit spawning

**Action**: For each entity spawn location, ADD `ViewVisibility::default()` to the spawn tuple.

**Example for model_loader.rs**:
```rust
// BEFORE:
let mut entity_commands = commands.spawn((
    mesh,
    material,
    Transform::default(),
    GlobalTransform::default(),
    Visibility::Inherited,
    InheritedVisibility::default(),
    Aabb::default(),
));

// AFTER:
let mut entity_commands = commands.spawn((
    mesh,
    material,
    Transform::default(),
    GlobalTransform::default(),
    Visibility::Inherited,
    InheritedVisibility::default(),
    ViewVisibility::default(),  // ← ADD THIS LINE
    Aabb::default(),
));
```

**Rationale**: Adding `ViewVisibility::default()` ensures the component exists so Bevy's visibility system can update it each frame.

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
- Verify visibility systems are running (VisibilityPropagate, CheckVisibility, CalculateBounds)

### Step 4: Validate Rendering Pipeline
**Action**: Once entities are being extracted, verify:
1. Meshes are visible in scene
2. Materials are rendering correctly
3. Lighting is working (if applicable)
4. No other rendering issues exist

## Implementation Order

1. **Remove camera ViewVisibility** (highest priority - confirmed root cause)
2. **Add ViewVisibility to entity spawning** (ensures component exists for extraction)
3. **Test and validate** with diagnostic logs
4. **Iterate** based on results

## Alternative Approaches

If the above fixes don't resolve the issue:

### Approach A: Verify Visibility System Execution
Check if Bevy's visibility systems are actually running:
- Add diagnostic logging to VisibilityPropagate system
- Add diagnostic logging to CheckVisibility system
- Add diagnostic logging to CalculateBounds system
- Verify these systems are in the correct schedule

### Approach B: Force Zone Visibility
If entities are children of zone entity and zone entity is not visible:
- Ensure zone entity has `Visibility::Visible`
- Verify zone entity's `ViewVisibility` is being computed correctly
- Check parent-child visibility propagation

### Approach C: Disable Frustum Culling
If frustum culling is incorrectly culling visible entities:
- Add `NoFrustumCulling` component to entities
- Or adjust camera frustum parameters (near/far planes)

### Approach D: Debug Extraction System
Add detailed logging to Bevy's extraction system to see:
- Which entities are being considered for extraction
- Why entities are being skipped
- What components are missing

## Diagnostic Verification

After implementing fixes, verify with these diagnostic systems:

1. **Main World Visibility**: Check if entities have `ViewVisibility=true`
2. **Render World Extraction**: Check if entities are being extracted (> 0)
3. **Render Phase**: Check if Transparent3d queue has items (> 0)
4. **Camera-Entity Distance**: Verify entities are within camera range
5. **Material Plugin Verification**: Confirm materials are being extracted
6. **Visibility System Execution**: Verify VisibilityPropagate, CheckVisibility, CalculateBounds are running

## Success Criteria

The issue is resolved when:
- ✅ Diagnostic logs show entities being extracted to Render World
- ✅ Render phase queues have items
- ✅ Visual rendering works (meshes visible on screen)
- ✅ No black screen
- ✅ All visibility systems are running correctly

## Key Insights from Research

1. **ViewVisibility is computed, not set**: Don't try to manually set it to `true` or `false`. Let Bevy compute it.
2. **ViewVisibility is reset each frame**: It starts as `false` each frame, then the visibility system computes the correct value.
3. **Component must exist**: Even though it's computed, the component must exist on the entity for the extraction system to find it.
4. **Camera shouldn't have ViewVisibility**: Since it's computed and the camera doesn't need to be "visible to itself", we shouldn't set it manually.
