# Black Screen Issue Resolution Plan

## Executive Summary

**Problem:** 540 meshes exist in scene with `Visibility::Visible` set, but Bevy's computed `ViewVisibility` is `false` for all meshes, causing a black screen.

**Root Cause:** Bevy 0.13.2's frustum culling system is incorrectly marking all meshes as not visible to the camera, despite meshes being positioned correctly relative to the camera.

**Key Findings:**
- Camera is properly configured at (5260.33, 8.61, -5372.82) with PerspectiveProjection (FOV: 45°, near: 0.1, far: 10000.0)
- All meshes have `Visibility::Visible`, `ViewVisibility::default()`, `InheritedVisibility::default()`
- Meshes are positioned correctly (81-7519 units from camera, angles 16-44° from camera forward)
- All meshes are in front of camera (dot products 0.72-0.96)
- Terrain and skybox entities have `NoFrustumCulling` component, but object entities (cnst/deco/event/warp) do NOT
- No explicit AABB components found on mesh entities

## Root Cause Analysis

### Hypothesis 1: Missing AABB Components
Bevy 0.13.2 requires AABB (Axis-Aligned Bounding Box) components on mesh entities for frustum culling to work correctly. Without AABBs, Bevy cannot determine if a mesh is within the camera's frustum.

**Evidence:**
- Zone loader spawns entities with `Visibility::Visible` but does NOT add AABB components
- Search of codebase shows no explicit AABB component insertion during mesh spawning
- Bevy's frustum culling system depends on AABBs to compute visibility

### Hypothesis 2: Coordinate System Mismatch
Rose Online uses a different coordinate system than Bevy's default:
- Rose Online: X+ (forward), Y+ (up), Z- (backward)
- Camera at (5260.33, 8.61, -5372.82) looking at (5120.0, 0.0, -5130.0)
- This appears consistent with diagnostic logs showing correct angles and distances

### Hypothesis 3: Bevy 0.13.2 Visibility System Changes
Bevy 0.13.2 introduced changes to the visibility system:
- `ViewVisibility` is now a separate component from `Visibility`
- Frustum culling may have timing/ordering issues with the new system
- `NoFrustumCulling` component exists but may not be working as expected

### Hypothesis 4: Transform Hierarchy Issues
Meshes are spawned as children of zone objects. If:
- Parent transforms are not updated before child visibility is computed
- GlobalTransform is not propagated correctly
- This could cause incorrect world-space positions for frustum culling

### Hypothesis 5: Camera Frustum Configuration
The camera's far plane is 10000.0 units, which should cover all meshes (max distance 7519). However:
- Frustum may be incorrectly calculated
- Aspect ratio (16.0/9.0) may cause frustum distortion
- Near plane (0.1) may clip nearby meshes

## Investigation Steps

### Step 1: Verify AABB Components Exist
**Action:** Query all mesh entities and check for AABB components

**Code Location:** `src/systems/zone_render_validation_system.rs` (enhance existing validation)

**What to Check:**
```rust
// Add AABB component check to zone_render_validation_system
fn check_aabbs(
    meshes: Query<(&Handle<Mesh>, Option<&Aabb>)>,
) {
    let mut without_aabb = 0;
    let mut with_aabb = 0;
    
    for (mesh_handle, aabb) in meshes.iter() {
        if aabb.is_some() {
            with_aabb += 1;
        } else {
            without_aabb += 1;
        }
    }
    
    log::info!("[AABB CHECK] Meshes with AABB: {}, without AABB: {}", 
        with_aabb, without_aabb);
}
```

**Expected Outcome:** Determine if AABBs are missing from all mesh entities

### Step 2: Verify Camera Frustum Configuration
**Action:** Log camera frustum planes and verify they match expected values

**Code Location:** `src/systems/debug_rendering_system.rs` (enhance existing diagnostics)

**What to Check:**
```rust
// Add frustum plane logging to camera_configuration_diagnostics
fn log_frustum_planes(
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    for (camera, transform) in cameras.iter() {
        if let Projection::Perspective(perspective) = &camera.projection {
            log::info!("[FRUSTUM PLANES] Camera frustum:");
            log::info!("[FRUSTUM PLANES]   FOV: {} degrees", perspective.fov.to_degrees());
            log::info!("[FRUSTUM PLANES]   Near: {}", perspective.near);
            log::info!("[FRUSTUM PLANES]   Far: {}", perspective.far);
            log::info!("[FRUSTUM PLANES]   Aspect: {}", perspective.aspect_ratio);
            
            // Calculate frustum corners for verification
            let position = transform.translation();
            let forward = transform.forward();
            let right = transform.right();
            let up = transform.up();
            
            log::info!("[FRUSTUM PLANES]   Position: ({:.2}, {:.2}, {:.2})", 
                position.x, position.y, position.z);
            log::info!("[FRUSTUM PLANES]   Forward: ({:.2}, {:.2}, {:.2})", 
                forward.x, forward.y, forward.z);
        }
    }
}
```

**Expected Outcome:** Confirm camera frustum is correctly configured

### Step 3: Test NoFrustumCulling Component
**Action:** Add `NoFrustumCulling` to all mesh entities temporarily to verify this resolves the issue

**Code Location:** `src/zone_loader.rs` (modify spawn_object function)

**What to Change:**
```rust
// In spawn_object function, add NoFrustumCulling to all mesh entities
commands.spawn((
    // ... existing components ...
    NoFrustumCulling,  // ADD THIS
    // ... rest of components
))
```

**Expected Outcome:** If meshes become visible with `NoFrustumCulling`, confirms frustum culling is the issue

### Step 4: Check Transform Hierarchy
**Action:** Verify that GlobalTransform is properly propagated through the entity hierarchy

**Code Location:** Create new diagnostic system `src/systems/transform_hierarchy_diagnostics.rs`

**What to Check:**
```rust
pub fn transform_hierarchy_diagnostics(
    meshes: Query<(&GlobalTransform, &Parent, &ViewVisibility)>,
) {
    log::info!("[TRANSFORM HIERARCHY] Checking transform propagation...");
    
    let mut issues = 0;
    for (global_transform, parent, view_visibility) in meshes.iter() {
        if let Some(parent_entity) = parent.get() {
            // Check if parent has valid GlobalTransform
            // Check if child's world position matches expected
        }
        
        if !view_visibility.get() {
            issues += 1;
        }
    }
    
    log::info!("[TRANSFORM HIERARCHY] Found {} invisible meshes", issues);
}
```

**Expected Outcome:** Identify if transform hierarchy is causing visibility issues

### Step 5: Verify Coordinate System Consistency
**Action:** Add diagnostic to verify mesh positions are in the correct coordinate space

**Code Location:** `src/systems/coordinate_system_diagnostics.rs` (create new system)

**What to Check:**
```rust
pub fn coordinate_system_diagnostics(
    meshes: Query<(&GlobalTransform, &Handle<Mesh>)>,
    camera: Query<(&GlobalTransform, &Camera)>,
) {
    if let Ok((cam_transform, _)) = camera.get_single() {
        let cam_pos = cam_transform.translation();
        
        for (mesh_transform, mesh_handle) in meshes.iter() {
            let mesh_pos = mesh_transform.translation();
            let distance = cam_pos.distance(mesh_pos);
            
            if distance < 10000.0 {
                log::info!("[COORD CHECK] Mesh at ({:.2}, {:.2}, {:.2}) is {} units from camera",
                    mesh_pos.x, mesh_pos.y, mesh_pos.z, distance);
            }
        }
    }
}
```

**Expected Outcome:** Confirm all meshes are within camera's far plane

## Prioritized Resolution Strategy

### Quick Wins (Immediate Fixes)

#### Solution 1: Add NoFrustumCulling to All Mesh Entities
**Priority:** HIGH
**Complexity:** LOW
**Risk:** LOW

**Implementation:**
- Modify `spawn_object()` in `src/zone_loader.rs` to add `NoFrustumCulling` component
- Modify `spawn_terrain()` to add `NoFrustumCulling` component
- Modify `spawn_water()` to add `NoFrustumCulling` component
- Modify `spawn_skybox()` to add `NoFrustumCulling` component (already has it)

**Files to Modify:**
- `src/zone_loader.rs` (lines ~2277, 2051, 1779)

**Code Changes:**
```rust
// In spawn_object (around line 2277)
commands.spawn((
    object_type(ZoneObjectId { /* ... */ }),
    object_transform,
    GlobalTransform::default(),
    Visibility::Visible,
    ViewVisibility::default(),
    InheritedVisibility::default(),
    NoFrustumCulling,  // ADD THIS LINE
    RigidBody::Fixed,
    // ... rest of components
))

// In spawn_terrain (around line 1967)
commands.spawn((
    ZoneObject::Terrain(ZoneObjectTerrain { /* ... */ }),
    meshes.add(mesh),
    material_handle,
    terrain_transform,
    GlobalTransform::default(),
    Visibility::Visible,
    ViewVisibility::default(),
    InheritedVisibility::default(),
    NoFrustumCulling,  // ADD THIS LINE
    NotShadowCaster,
    RigidBody::Fixed,
    Collider::trimesh(collider_verts, collider_indices),
    CollisionGroups::new(/* ... */),
))

// In spawn_water (around line 2051)
commands.spawn((
    ZoneObject::Water,
    meshes.add(mesh),
    water_material.clone(),
    Transform::default(),
    GlobalTransform::default(),
    Visibility::Visible,
    ViewVisibility::default(),
    InheritedVisibility::default(),
    NoFrustumCulling,  // ADD THIS LINE
    NotShadowCaster,
    NotShadowReceiver,
    RigidBody::Fixed,
    Collider::trimesh(collider_verts, collider_indices),
    CollisionGroups::new(/* ... */),
))
```

**Expected Outcome:** All meshes bypass frustum culling and should render

**Verification:**
- Run application and check diagnostic logs
- Verify that `visible_meshes` count increases from 0 to >0
- Check that meshes appear on screen

**Side Effects:**
- May reduce performance (all meshes always rendered)
- May render meshes that should be culled (behind camera, outside frustum)
- Acceptable for debugging and as temporary fix

#### Solution 2: Manually Set ViewVisibility to True
**Priority:** HIGH
**Complexity:** LOW
**Risk:** VERY LOW

**Implementation:**
- Create a system that forces `ViewVisibility` to `true` for all mesh entities
- This bypasses Bevy's frustum culling entirely

**Files to Create:**
- `src/systems/force_visibility_system.rs` (new file)

**Code:**
```rust
use bevy::prelude::*;

/// System to force all mesh entities to be visible
/// This is a diagnostic/development workaround, not a production solution
pub fn force_visibility_system(
    mut meshes: Query<&mut ViewVisibility>,
) {
    for mut view_visibility in meshes.iter_mut() {
        view_visibility.set();
    }
}
```

**Integration:**
Add to `src/lib.rs`:
```rust
app.add_systems(Update, force_visibility_system);
```

**Expected Outcome:** All meshes have `ViewVisibility::true`, should render

**Side Effects:**
- Completely disables frustum culling
- Performance impact (all meshes always rendered)
- Not suitable for production use

**Verification:**
- Run application and check diagnostic logs
- Verify `visible_meshes` count equals total mesh count

#### Solution 3: Adjust Camera Far Plane
**Priority:** MEDIUM
**Complexity:** LOW
**Risk:** LOW

**Implementation:**
- Increase camera far plane from 10000.0 to 20000.0 or higher
- This ensures frustum covers all meshes even with coordinate system differences

**Files to Modify:**
- `src/lib.rs` (line 1416)

**Code Changes:**
```rust
// Change from:
Projection::from(PerspectiveProjection {
    fov: std::f32::consts::PI / 4.0,
    near: 0.1,
    far: 10000.0,
    aspect_ratio: 16.0 / 9.0,
}),

// To:
Projection::from(PerspectiveProjection {
    fov: std::f32::consts::PI / 4.0,
    near: 0.1,
    far: 20000.0,  // INCREASED
    aspect_ratio: 16.0 / 9.0,
}),
```

**Expected Outcome:** Frustum covers larger area, should include all meshes

**Side Effects:**
- May reduce depth precision for distant objects
- May include objects that should be culled

**Verification:**
- Run application and verify meshes at maximum distance are visible
- Check diagnostic logs for frustum coverage

### Medium-Term Solutions

#### Solution 4: Add AABB Components to Mesh Entities
**Priority:** HIGH
**Complexity:** MEDIUM
**Risk:** MEDIUM

**Implementation:**
- Automatically generate and add AABB components when spawning meshes
- Use Bevy's built-in AABB computation from mesh vertices
- This is the proper long-term solution

**Files to Modify:**
- `src/zone_loader.rs` (modify spawn_object, spawn_terrain, spawn_water)
- Create new AABB computation utility in `src/utils/aabb_utils.rs` (new file)

**Code Changes:**

Create `src/utils/aabb_utils.rs`:
```rust
use bevy::prelude::*;
use bevy::render::mesh::Mesh;

/// Utility to compute AABB from mesh
pub fn compute_mesh_aabb(mesh: &Mesh) -> Option<Aabb> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?;
    
    // Get all vertex positions
    let vertex_positions: Vec<Vec3> = match positions {
        VertexAttributeValues::Float32x3(v) => v.clone(),
        _ => return None,
    };
    
    if vertex_positions.is_empty() {
        return None;
    }
    
    // Compute min and max
    let mut min = vertex_positions[0];
    let mut max = vertex_positions[0];
    
    for pos in &vertex_positions[1..] {
        min = min.min(*pos);
        max = max.max(*pos);
    }
    
    Some(Aabb {
        center: (min + max) / 2.0,
        half_extents: (max - min) / 2.0,
    })
}
```

Modify `src/zone_loader.rs` in `spawn_object`:
```rust
// After spawning mesh entity
if let Some(mesh) = meshes.get(mesh_handle) {
    if let Some(aabb) = compute_mesh_aabb(mesh) {
        commands.entity(part_entity).insert(aabb);
    }
}
```

**Expected Outcome:** All meshes have AABB components for proper frustum culling

**Side Effects:**
- Slight performance overhead from AABB computation
- More memory usage per entity
- Proper frustum culling restored

**Verification:**
- Run AABB check diagnostic (Solution 1) - should show all meshes with AABB
- Verify frustum culling works correctly (meshes behind camera not visible)

#### Solution 5: Verify and Fix Transform Propagation
**Priority:** MEDIUM
**Complexity:** MEDIUM
**Risk:** MEDIUM

**Implementation:**
- Ensure GlobalTransform is properly updated before visibility computation
- Add diagnostic to track transform propagation timing
- May need to adjust system ordering

**Files to Modify:**
- `src/lib.rs` (adjust system ordering)
- `src/systems/transform_propagation_diagnostics.rs` (new file)

**Code Changes:**

In `src/lib.rs`, ensure transform system runs before visibility systems:
```rust
// Current ordering:
app.add_systems(
    Update,
    (
        zone_loader_system,
        zone_loaded_from_vfs_system,
        game_zone_change_system
        // ... other systems
    ),
);

// Change to ensure transform propagation before visibility:
app.configure_sets(
    Update,
    (
        GameSystemSets::UpdateCamera,
        TransformSystem::TransformPropagate,  // ADD THIS
    ).before(GameSystemSets::UpdateCamera),
);
```

Create `src/systems/transform_propagation_diagnostics.rs`:
```rust
use bevy::prelude::*;

pub fn transform_propagation_diagnostics(
    transforms: Query<&GlobalTransform>,
    parents: Query<&Parent>,
) {
    log::info!("[TRANSFORM PROP] Checking transform propagation...");
    
    for (global_transform, parent) in transforms.iter().zip(parents.iter()) {
        if let Some(parent_entity) = parent.get() {
            log::info!("[TRANSFORM PROP] Entity {:?} has parent {:?}", 
                parent_entity, parent_entity);
        }
    }
}
```

**Expected Outcome:** Transforms propagate correctly before visibility computation

**Side Effects:**
- May require system ordering adjustments
- Could affect other systems that depend on transform timing

**Verification:**
- Run transform propagation diagnostic
- Check that child entities have correct world positions

#### Solution 6: Adjust Camera Near Plane and FOV
**Priority:** LOW
**Complexity:** LOW
**Risk:** LOW

**Implementation:**
- Fine-tune camera near plane and FOV for better frustum culling
- Based on diagnostic data showing mesh distances (81-7519 units)

**Files to Modify:**
- `src/lib.rs` (camera setup)

**Code Changes:**
```rust
// Experiment with different values:
Projection::from(PerspectiveProjection {
    fov: std::f32::consts::PI / 3.5,  // Wider FOV (51 degrees)
    near: 0.5,  // Further near plane
    far: 20000.0,
    aspect_ratio: 16.0 / 9.0,
}),
```

**Expected Outcome:** Better frustum coverage for nearby meshes

**Side Effects:**
- Changes visual perspective
- May affect gameplay feel

**Verification:**
- Test with different FOV and near plane values
- Monitor which meshes become visible

### Long-Term Solutions

#### Solution 7: Implement Custom Visibility System
**Priority:** MEDIUM
**Complexity:** HIGH
**Risk:** MEDIUM

**Implementation:**
- Create custom frustum culling system optimized for Rose Online's coordinate system
- Replace Bevy's default visibility computation
- More control over visibility determination

**Files to Create:**
- `src/systems/custom_visibility_system.rs` (new file)
- `src/render/custom_frustum.rs` (new file)

**Code Changes:**

Create `src/systems/custom_visibility_system.rs`:
```rust
use bevy::prelude::*;
use bevy::render::view::Visibility;

/// Custom visibility system for Rose Online
/// Handles coordinate system differences and provides more control
pub fn custom_visibility_system(
    mut meshes: Query<(&mut ViewVisibility, &GlobalTransform)>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    if let Ok((camera, camera_transform)) = cameras.get_single() {
        let camera_pos = camera_transform.translation();
        let camera_forward = camera_transform.forward();
        
        for (mut view_visibility, mesh_transform) in meshes.iter_mut() {
            let mesh_pos = mesh_transform.translation();
            let to_mesh = mesh_pos - camera_pos;
            let distance = to_mesh.length();
            
            // Simple frustum test: check if in front of camera
            let in_front = camera_forward.dot(to_mesh.normalize()) > 0.0;
            let in_range = distance < camera.far; // Need to access far plane
            
            // Custom visibility logic
            *view_visibility = if in_front && in_range {
                Visibility::Visible
            } else {
                Visibility::Hidden
            };
        }
    }
}
```

**Integration:**
Add to `src/lib.rs`:
```rust
app.add_systems(Update, custom_visibility_system);
```

**Expected Outcome:** Custom visibility logic based on Rose Online coordinate system

**Side Effects:**
- Replaces Bevy's visibility system
- Requires maintenance and testing
- May have edge cases not handled by Bevy

**Verification:**
- Test with various camera positions and mesh configurations
- Compare with Bevy's default visibility system

#### Solution 8: Optimize Mesh Spawning
**Priority:** LOW
**Complexity:** MEDIUM
**Risk:** MEDIUM

**Implementation:**
- Batch mesh spawning to reduce entity count
- Use instancing for repeated meshes
- Optimize AABB computation

**Files to Modify:**
- `src/zone_loader.rs` (spawning logic)
- `src/utils/mesh_utils.rs` (new file)

**Expected Outcome:** Better performance with proper visibility

**Side Effects:**
- More complex spawning logic
- Requires testing and validation

**Verification:**
- Monitor spawn times
- Measure performance improvements

## Implementation Order

### Phase 1: Diagnostics (Immediate)
1. Implement AABB check diagnostic (Solution 1, Step 1)
2. Implement frustum plane logging (Solution 2, Step 2)
3. Implement transform hierarchy diagnostic (Solution 4, Step 3)
4. Implement coordinate system diagnostic (Solution 5, Step 4)

### Phase 2: Quick Fixes (High Priority)
5. Add NoFrustumCulling to all mesh entities (Solution 1)
6. Test and verify NoFrustumCulling resolves issue

### Phase 3: Medium-Term Solutions
7. Add AABB components to mesh entities (Solution 4)
8. Verify and fix transform propagation (Solution 5)
9. Adjust camera configuration if needed (Solution 3, 6)

### Phase 4: Long-Term Solutions (Optional)
10. Implement custom visibility system (Solution 7)
11. Optimize mesh spawning (Solution 8)

## Fallback Options

### Fallback 1: Disable Frustum Culling Globally
**Implementation:**
```rust
// In src/lib.rs, modify PbrPlugin initialization
app.add_plugins((
    bevy::pbr::PbrPlugin {
        prepass_enabled: false,
        add_default_deferred_lighting_plugin: true,
        // DISABLE FRUSTUM CULLING
        // This is a debug option, NOT for production
    }),
));
```

**Risk:** High performance impact

### Fallback 2: Use Orthographic Camera
**Implementation:**
```rust
// In src/lib.rs, change camera projection
Projection::from(OrthographicProjection {
    far: 10000.0,
    near: 0.1,
    ..Default::default()
}),
```

**Risk:** Different visual perspective, may not be suitable

### Fallback 3: Manual Frustum Culling Bypass
**Implementation:**
- Create UI option to toggle frustum culling
- Allow runtime debugging of visibility issues

**Risk:** Requires UI development

## Verification Steps

### After Each Solution:
1. Run application and observe screen
2. Check diagnostic logs for visibility changes
3. Verify mesh count matches expected
4. Test camera movement and verify meshes remain visible
5. Monitor performance impact

### Success Criteria:
- `visible_meshes` count in diagnostic logs > 0
- Meshes visible on screen match camera view direction
- No performance degradation
- Stable rendering across camera movements

## Mermaid Diagram: Visibility System Flow

```mermaid
graph TD
    A[Camera Entity] -->|Perspective Projection|
    B -->|Frustum Calculation|
    C -->|Frustum Test|
    
    D[Mesh Entity] -->|Has Visibility Component|
    D -->|Has AABB Component|
    D -->|Has NoFrustumCulling|
    
    E[Visibility Computation] -->|Bevy Frustum Culling|
    E -->|Custom Visibility Logic|
    E -->|Force Visible|
    
    F[ViewVisibility Result] -->|true or false|
    
    C -->|Mesh in Frustum| -->|true|
    D -->|Mesh in Frustum| -->|true|
    E -->|Override Result| -->|true|
    
    style A fill:#f9f,stroke:#333
    style D fill:#bbf,stroke:#333
    style E fill:#ff9,stroke:#333
    style F fill:#9f9,stroke:#333
```

## Expected Outcomes

### Best Case:
- Solution 1 or 2 resolves issue immediately
- Minimal code changes
- Low risk of side effects
- Performance acceptable

### Likely Case:
- Solution 4 (AABB components) is required
- May need combination of solutions
- More complex implementation

### Worst Case:
- All quick and medium solutions fail
- Requires long-term solution (Solution 7 or 8)
- Significant refactoring required

## Risk Assessment

| Solution | Risk Level | Performance Impact | Reversibility |
|----------|-------------|-------------------|--------------|
| NoFrustumCulling | Low | Negative (all meshes render) | Easy |
| Force Visible | Very Low | Very Negative (no culling) | Easy |
| Adjust Far Plane | Low | Neutral | Easy |
| AABB Components | Medium | Slight Positive (proper culling) | Medium |
| Custom Visibility | Medium | Neutral | Medium |
| Disable Frustum | High | Very Negative | Easy |
| Orthographic Camera | High | Neutral | Easy |

## Notes

### Bevy 0.13.2 Specific Considerations:
- `ViewVisibility` is now computed separately from `Visibility` component
- Frustum culling happens in the render thread, not main thread
- `NoFrustumCulling` component should work but may have edge cases
- Transform propagation timing is critical for visibility computation

### Rose Online Specific Considerations:
- Coordinate system: X+ (forward), Y+ (up), Z- (backward)
- Large world coordinates (5000+ units)
- Camera should be positioned to see gameplay area (5120, 0, -5120)
- Zone loading is async, may cause timing issues with visibility

### Recommended Approach:
1. Start with Solution 1 (NoFrustumCulling) - quickest to test
2. If Solution 1 works, investigate why frustum culling is failing
3. If Solution 1 doesn't work, try Solution 2 (Force Visible)
4. Implement Solution 4 (AABB components) as proper fix
5. Use other solutions as needed based on findings

## Conclusion

The black screen issue is most likely caused by Bevy 0.13.2's frustum culling system incorrectly determining that meshes are not visible to the camera. The recommended approach is to start with quick fixes (Solution 1 or 2) to verify this hypothesis, then implement the proper fix (Solution 4: AABB components) for long-term stability.
