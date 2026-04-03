# Bevy Transform Features Documentation

Comprehensive documentation for Bevy Transform system (v0.18.1) as used in rose-offline-client.

## Table of Contents

1. [Overview](#overview)
2. [Bevy API References](#bevy-api-references)
3. [Custom Extensions](#custom-extensions)
4. [Code Examples](#code-examples)
5. [Configuration Options](#configuration-options)
6. [Common Patterns](#common-patterns)
7. [Troubleshooting](#troubleshooting)
8. [Source File References](#source-file-references)

---

## Overview

Bevy's transform system provides spatial positioning for entities through two complementary components:

- **`Transform`**: Local-space transform (translation, rotation, scale) relative to parent
- **`GlobalTransform`**: Computed world-space transform

Transforms compose from right to left: `t1 * t2` applies `t2` first, then `t1`.

### Coordinate System

rose-offline-client uses Bevy's coordinate system:
- **X axis**: Right
- **Y axis**: Up
- **Z axis**: Back (negative Z is forward)

Position data from the game server uses a different convention:
- **X**: Right
- **Y**: Forward
- **Z**: Up

Conversion formula:
```rust
// Server Position (centimeters) to Transform (meters)
transform.translation.x = position.x / 100.0;      // right
transform.translation.y = position.z / 100.0;      // up
transform.translation.z = -position.y / 100.0;     // back (negated forward)
```

---

## Bevy API References

### Transform Component

The `Transform` component represents an entity's local-space transform.

### Structure

```rust
pub struct Transform {
    pub translation: Vec3,  // Position in local/parent space
    pub rotation: Quat,     // Quaternion rotation
    pub scale: Vec3,        // Non-uniform scale per axis
}
```

### Key Properties

| Property | Description |
|----------|-------------|
| `translation` | Position relative to parent (or world if no parent). Z-value used for 2D z-ordering. |
| `rotation` | Orientation as quaternion. Identity = no rotation. |
| `scale` | Per-axis scaling. `Vec3::ONE` = no scaling. |

### Important Methods

#### Construction

```rust
// Identity transform
Transform::IDENTITY

// From position
Transform::from_xyz(x, y, z)
Transform::from_translation(Vec3::new(x, y, z))

// From rotation
Transform::from_rotation(Quat)

// From scale
Transform::from_scale(Vec3)

// From matrix
Transform::from_matrix(Mat4)
```

#### Direction Vectors

```rust
// Local axis vectors (returns Dir3 = normalized Vec3)
transform.local_x()     // Local +X direction
transform.local_y()     // Local +Y direction
transform.local_z()     // Local +Z direction

// Convenience methods
transform.right()       // Same as local_x()
transform.left()        // Same as -local_x()
transform.up()          // Same as local_y()
transform.down()        // Same as -local_y()
transform.forward()     // Same as -local_z() (Bevy convention)
transform.back()        // Same as local_z()
```

#### Rotation Methods

```rust
// Rotate by quaternion (parent-relative)
transform.rotate(Quat)

// Rotate around axis (radians)
transform.rotate_axis(Dir3, angle)
transform.rotate_x(angle)
transform.rotate_y(angle)
transform.rotate_z(angle)

// Local-space rotation
transform.rotate_local(Quat)
transform.rotate_local_x(angle)
transform.rotate_local_y(angle)
transform.rotate_local_z(angle)

// Look-at rotation
transform.look_at(target: Vec3, up: Dir3)
transform.look_to(direction: Dir3, up: Dir3)

// Align axes
transform.align(main_axis, main_direction, secondary_axis, secondary_direction)
```

#### Transform Operations

```rust
// Transform a point from local to parent space
transform.transform_point(Vec3)

// Convert to matrix
transform.to_matrix() -> Mat4
transform.compute_affine() -> Affine3A

// Multiply transforms
transform.mul_transform(other_transform)

// Check validity
transform.is_finite() -> bool
```

### Source Reference

- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\components\transform.rs`

---

### GlobalTransform Component

`GlobalTransform` represents the computed world-space transform. It is **read-only** and automatically updated by Bevy's transform propagation systems.

### Structure

```rust
pub struct GlobalTransform(Affine3A);
```

### Key Properties

- Automatically inserted when `Transform` is inserted
- Updated in `PostUpdate` schedule via `TransformSystems::Propagate`
- Contains full world-space affine transformation

### Important Methods

```rust
// Get translation in world space
global_transform.translation() -> Vec3
global_transform.translation_vec3a() -> Vec3A

// Get rotation and scale
global_transform.rotation() -> Quat
global_transform.scale() -> Vec3
global_transform.to_scale_rotation_translation() -> (Vec3, Quat, Vec3)

// Direction vectors in world space
global_transform.right() -> Dir3
global_transform.up() -> Dir3
global_transform.forward() -> Dir3

// Transform points
global_transform.transform_point(Vec3) -> Vec3

// Convert to matrix
global_transform.to_matrix() -> Mat4
global_transform.affine() -> Affine3A
```

### Reparenting

To maintain global position when changing parents:

```rust
// Calculate local transform for new parent
let new_local_transform = child_global_transform.reparented_to(&new_parent_global_transform);
```

### Source Reference

- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\components\global_transform.rs`

---

### Transform Systems

Bevy automatically propagates transforms through entity hierarchies.

### TransformSystems

```rust
pub enum TransformSystems {
    /// Propagates changes to children's GlobalTransform
    Propagate,
}
```

### System Pipeline

Transform propagation runs in `PostUpdate`:

1. **`mark_dirty_trees`**: Marks changed entities and their ancestors
2. **`propagate_parent_transforms`**: Computes GlobalTransform for hierarchy
3. **`sync_simple_transforms`**: Updates root entities without parents

### StaticTransformOptimizations

```rust
pub enum StaticTransformOptimizations {
    Enabled,   // Default - skip unchanged subtrees
    Disabled,  // Always propagate all transforms
}
```

For scenes with many static entities, enabling optimizations significantly improves performance.

### Source Reference

- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\systems.rs`
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\plugins.rs`

---

### Math Types from glam/bevy_math

Bevy uses `glam` for math types, re-exported through `bevy_math`.

### Vector Types

#### Vec3 - 3D Vector

```rust
// Construction
Vec3::ZERO          // (0, 0, 0)
Vec3::ONE           // (1, 1, 1)
Vec3::X             // (1, 0, 0)
Vec3::Y             // (0, 1, 0)
Vec3::Z             // (0, 0, 1)
Vec3::new(x, y, z)
vec3(x, y, z)       // Macro

// Operations
vec.length()                    // Magnitude
vec.length_squared()            // Magnitude² (no sqrt)
vec.normalize()                 // Unit vector (panics if zero)
vec.normalize_or_zero()         // Safe normalize
vec.dot(other)                  // Dot product
vec.cross(other)                // Cross product
vec.project_onto(other)         // Projection
vec.reject_from(other)          // Rejection (perpendicular)

// Utility
vec.is_finite()                 // Check for NaN/Inf
vec.abs_diff_eq(other, epsilon) // Approximate equality
```

#### Vec2 - 2D Vector

```rust
Vec2::ZERO
Vec2::ONE
Vec2::X
Vec2::Y
Vec2::new(x, y)
vec2(x, y)
```

#### Vec4 - 4D Vector

```rust
Vec4::ZERO
Vec4::ONE
Vec4::new(x, y, z, w)
vec4(x, y, z, w)
```

### Quaternion (Quat)

Quaternions represent 3D rotations without gimbal lock.

```rust
// Construction
Quat::IDENTITY                    // No rotation
Quat::from_axis_angle(axis: Dir3, angle: f32)
Quat::from_rotation_x(angle)
Quat::from_rotation_y(angle)
Quat::from_rotation_z(angle)
Quat::from_euler(EulerRot::XYZ, x, y, z)
Quat::from_mat3(&Mat3)
Quat::from_rotation_arc(from: Vec3, to: Vec3)

// Operations
quat * vec3                       // Rotate vector
quat1 * quat2                     // Compose rotations
quat.inverse()                    // Inverse rotation
quat.conjugate()                  // Conjugate
quat.normalize()                  // Normalize (length = 1)

// Interpolation
Quat::lerp(q1, q2, t)             // Linear interpolation
Quat::slerp(q1, q2, t)            // Spherical interpolation (constant speed)
```

#### Euler Angle Conventions

```rust
// Rotation order enum
pub enum EulerRot {
    XYZ,  // Rotate around X, then Y, then Z
    XZY,
    YXZ,
    YZX,
    ZXY,
    ZYX,
}

// Example: 90° around Y axis
let rotation = Quat::from_euler(EulerRot::XYZ, 0.0, std::f32::consts::FRAC_PI_2, 0.0);
```

### Matrix Types

#### Mat4 - 4x4 Matrix

```rust
// Construction
Mat4::IDENTITY
Mat4::from_scale_rotation_translation(scale, rotation, translation)
Mat4::from_cols(col0, col1, col2, col3)

// Operations
mat * vec3                        // Transform point
mat * mat                         // Matrix multiplication
mat.inverse()                     // Inverse matrix
mat.transpose()                   // Transpose
mat.determinant()                 // Determinant

// Decomposition
mat.to_scale_rotation_translation() -> (Vec3, Quat, Vec3)
```

#### Mat3A - 3x3 Aligned Matrix

```rust
Mat3A::IDENTITY
Mat3A::from_cols(col0, col1, col2)
Mat3A::from_quat(quat)
```

### Dir3 - 3D Direction

```rust
// Unit vector type for directions
Dir3::X
Dir3::Y
Dir3::Z
Dir3::NEG_X
Dir3::NEG_Y
Dir3::NEG_Z

// Conversion
let dir: Dir3 = vec3.normalize().try_into().unwrap();
let vec: Vec3 = dir.into();
```

### Source Reference

- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_math\src\lib.rs` (re-exports from glam)

---

## Code Examples

### Character Movement with Collision

Position component stores server coordinates; Transform stores render coordinates:

```rust
// src/systems/collision_system.rs:97
transform.translation.x = position.x / 100.0;      // right
transform.translation.z = -position.y / 100.0;     // back (negated forward)
// Y handled separately with terrain collision
```

### Camera-Relative Movement

Compute movement direction based on camera rotation:

```rust
// src/systems/game_keyboard_input_system.rs:67-71
let camera_rotation = camera_transform.rotation;
let camera_forward = (camera_rotation * -Vec3::Z).with_y(0.0).normalize_or_zero();
let camera_right = (camera_rotation * Vec3::X).with_y(0.0).normalize_or_zero();

// Combine WASD inputs
let mut move_world = Vec3::ZERO;
if keyboard_input.pressed(KeyCode::KeyW) {
    move_world += camera_forward;
}
// ... A/D for left/right
```

### Character Facing Direction

Smooth rotation to face desired direction:

```rust
// src/systems/facing_direction_system.rs:40-43
transform.rotation = Quat::from_axis_angle(
    Vec3::Y,
    facing_direction.actual - std::f32::consts::PI / 2.0,
);
```

### Billboard Objects (Always Face Camera)

```rust
// src/systems/season/fall_system.rs:201-214
// Create a rotation that faces the camera (billboard look-at)
let forward = (camera_position - position).normalize();
let right = forward.cross(Vec3::Y).normalize();
let corrected_up = forward.cross(right);

// Build rotation matrix and convert to quaternion
let look_rotation = Quat::from_mat3(&Mat3::from_cols(right, corrected_up, forward));

// Apply particle's own rotation on top (for visual variety)
transform.rotation = look_rotation * particle_rotation;
```

### Orbit Camera

Uses dolly's CameraRig for smooth orbit controls:

```rust
// src/systems/orbit_camera_system.rs:260-273
let calculated_transform = orbit_camera.rig.update(time.delta().as_secs_f32());
camera_transform.translation = Vec3::new(
    calculated_transform.position.x,
    calculated_transform.position.y,
    calculated_transform.position.z,
);
camera_transform.rotation = Quat::from_xyzw(
    calculated_transform.rotation.v.x,
    calculated_transform.rotation.v.y,
    calculated_transform.rotation.v.z,
    calculated_transform.rotation.s,
);
```

### Transform Animation

Extract scale and translation from GlobalTransform:

```rust
// src/systems/damage_digit_render_system.rs:114
let (scale, _, translation) = global_transform.to_scale_rotation_translation();
```

### Entity Spawning with Transform

```rust
// src/systems/chat_bubble_spawn_system.rs:346-347
commands.spawn((
    Transform::from_translation(Vec3::new(0.0, bubble_height, 0.0)),
    GlobalTransform::default(),
    // ... other components
));
```

### Parent-Child Hierarchies

```rust
// src/systems/bird_system.rs:228-246
// Spawn left wing as child (rotates around body center)
parent.spawn((
    Transform::from_rotation(Quat::from_rotation_z(0.3)), // Slightly spread
    GlobalTransform::default(),
));
```

---

## Custom Extensions

### Rotation Handling

### Coordinate Conventions

| Direction | Bevy Axis | Game Meaning |
|-----------|-----------|--------------|
| Forward   | -Z        | Movement direction |
| Back      | +Z        | Opposite of forward |
| Right     | +X        | Right side |
| Left      | -X        | Left side |
| Up        | +Y        | Vertical up |
| Down      | -Y        | Vertical down |

### Yaw Rotation

Character facing uses Y-axis rotation (yaw):

```rust
// Calculate angle from 2D direction
let angle = direction.y.atan2(direction.x) + std::f32::consts::PI;

// Convert to quaternion
let rotation = Quat::from_axis_angle(Vec3::Y, angle - std::f32::consts::PI / 2.0);
```

### Smooth Rotation

Slerp for smooth interpolation:

```rust
// src/systems/bird_system.rs:648
transform.rotation = transform.rotation.slerp(target_rotation, 2.0 * dt);
```

### Rotation Composition

Rotations compose from right to left:

```rust
// Billboard + sway (src/systems/season/summer_system.rs:315)
transform.rotation = look_rotation * sway_rotation;
```

---

## Configuration Options

### Static Scene Optimization

Enable for scenes with many static entities:

```rust
app.insert_resource(StaticTransformOptimizations::Enabled);
```

---

## Common Patterns

### Transform Propagation Timing

- GlobalTransform updates run in `PostUpdate`
- Changes to Transform have 1-frame lag in GlobalTransform
- For immediate updates, manually compute or use `transform_point()`

### Avoid Unnecessary Transform Access

- Use `GlobalTransform::translation()` for world position
- Use `Transform::translation` for local position
- Avoid `to_scale_rotation_translation()` unless you need all three

### Quaternion Normalization

Quaternions should remain normalized. If accumulating rotations:

```rust
transform.rotation = transform.rotation.normalize();
```

### Use length_squared() for Distance Checks

```rust
// Better - no sqrt
if vector.length_squared() < threshold_squared { }

// Worse - unnecessary sqrt
if vector.length() < threshold { }
```

### Parallel Transform Propagation

Bevy's transform propagation is parallelized for large hierarchies. Deep hierarchies benefit from static optimizations.

---

## Troubleshooting

### Bevy 0.18 Migration Issues

#### Issue 1: Transform Direction Methods Return Dir3 Instead of Vec3

**Problem**: In Bevy 0.18, methods like `transform.forward()`, `transform.right()`, `transform.up()` now return `Dir3` (unit vector type) instead of `Vec3`.

**Symptom**: Type mismatch errors when assigning direction vectors:
```rust
// Bevy 0.17 - worked fine
let forward: Vec3 = transform.forward();  // OK

// Bevy 0.18 - type error
let forward: Vec3 = transform.forward();  // Error: expected Vec3, found Dir3
```

**Solution**: Convert `Dir3` to `Vec3` explicitly:
```rust
// Option 1: Use .into() conversion
let forward: Vec3 = transform.forward().into();

// Option 2: Let type inference handle it
let forward = transform.forward().into();

// Option 3: Use Vec3A for SIMD performance
let forward: Vec3A = transform.forward_vec3a().into();
```

**Affected Code**: `src/systems/game_keyboard_input_system.rs:67-71`
```rust
// Updated for Bevy 0.18
let camera_forward = (camera_rotation * -Vec3::Z).with_y(0.0).normalize_or_zero();
let camera_right = (camera_rotation * Vec3::X).with_y(0.0).normalize_or_zero();
```

---

#### Issue 2: Coordinate System Mismatch with Server Data

**Problem**: Game server uses XY coordinate system (X=right, Y=forward, Z=up) while Bevy uses XYZ (X=right, Y=up, Z=back).

**Symptom**: Characters appear to move sideways or upside down when directly mapping server position to transform.

**Solution**: Apply coordinate transformation:
```rust
// src/systems/collision_system.rs:97-99
transform.translation.x = position.x / 100.0;      // right (same)
transform.translation.y = position.z / 100.0;      // up (was Z)
transform.translation.z = -position.y / 100.0;     // back (was -Y forward)
```

**Key Points**:
- Server uses centimeters, Bevy uses meters (divide by 100)
- Server Y (forward) becomes Bevy -Z (back)
- Server Z (up) becomes Bevy Y (up)

---

#### Issue 3: GlobalTransform Updates Have One-Frame Lag

**Problem**: `GlobalTransform` is updated in `PostUpdate` schedule, so changes to `Transform` don't reflect immediately.

**Symptom**: Entity appears in wrong position when querying `GlobalTransform` in same frame after modifying `Transform`.

**Solution Options**:

**Option 1**: Use `transform_point()` for immediate local-to-world conversion:
```rust
// Instead of waiting for GlobalTransform update
let world_pos = global_transform.transform_point(transform.translation);
```

**Option 2**: Query in next frame using `NextUpdate` or `PostUpdate`:
```rust
fn sync_positions(
    query: Query<(&GlobalTransform, Entity), Changed<Transform>>,
) {
    // Changes from previous frame are now available
}
```

**Option 3**: Manually compute world transform for immediate use:
```rust
let world_transform = parent_global_transform.compute_child_transform(&child_transform);
```

---

#### Issue 4: Quaternion Rotation Order Confusion

**Problem**: Quaternion multiplication is not commutative. `q1 * q2` ≠ `q2 * q1`.

**Symptom**: Billboard objects rotate incorrectly or character faces wrong direction after rotation composition.

**Solution**: Understand rotation order - transformations apply right-to-left:
```rust
// Billboard + sway rotation
// sway_rotation is applied first, then look_rotation
transform.rotation = look_rotation * sway_rotation;

// For local-space rotation
transform.rotation = transform.rotation * local_rotation;
```

**Reference**: `src/systems/season/summer_system.rs:315`

---

#### Issue 5: Non-Uniform Scale Causes Rotation Issues

**Problem**: Non-uniform scale combined with rotation can cause unexpected behavior in transform composition.

**Symptom**: Child entities rotate around wrong axes or appear skewed.

**Solution**: Keep scale uniform for entities with children:
```rust
// Good - uniform scale
transform.scale = Vec3::scale(2.0);

// Problematic for parent entities
transform.scale = Vec3::new(2.0, 1.0, 1.0);  // Avoid for parents
```

**Workaround**: Apply scale as separate child entity if non-uniform scale is required.

---

#### Issue 6: NaN/Inf Values in Transform

**Problem**: Division by zero or invalid math operations can produce NaN/Inf values.

**Symptom**: Entity disappears, camera glitches, or physics breaks.

**Solution**: Validate transform values:
```rust
if !transform.translation.is_finite() {
    transform.translation = Vec3::ZERO;
}

if !transform.rotation.is_finite() {
    transform.rotation = Quat::IDENTITY;
}
```

**Prevention**: Use `normalize_or_zero()` instead of `normalize()`:
```rust
// Safe - returns zero vector if length is zero
let direction = vector.normalize_or_zero();

// Unsafe - panics if length is zero
let direction = vector.normalize();
```

---

#### Issue 7: Euler Angle Gimbal Lock

**Problem**: Using Euler angles for rotation can cause gimbal lock at certain orientations.

**Symptom**: Loss of a degree of freedom, unexpected rotation behavior at 90° pitches.

**Solution**: Use quaternions for all rotation calculations:
```rust
// Good - quaternion-based
let rotation = Quat::from_axis_angle(Vec3::Y, angle);

// Avoid - Euler angles (unless for UI/input)
let rotation = Quat::from_euler(EulerRot::XYZ, x, y, z);
```

**When Euler is OK**: User input, configuration values, debugging. Convert immediately to quaternion for storage.

---

#### Issue 8: Transform Propagation Performance with Large Hierarchies

**Problem**: Deep entity hierarchies can cause performance issues during transform propagation.

**Symptom**: Frame time spikes when updating transforms in large scenes.

**Solution**: Enable static optimization for unchanged subtrees:
```rust
app.insert_resource(StaticTransformOptimizations::Enabled);
```

**Alternative**: Flatten hierarchy where possible, use fewer intermediate parent entities.

---

#### Issue 9: Camera Look-At with Up Vector Singularity

**Problem**: `look_at()` can fail when target is directly above/below the camera.

**Symptom**: Camera rotates unexpectedly when looking straight up or down.

**Solution**: Provide fallback up vector or use custom look-at:
```rust
// Custom implementation with singularity handling
let forward = (target - eye).normalize();
let right = forward.cross(Vec3::Y).normalize_or_zero();
if right.length_squared() < 0.0001 {
    // Singularity - use alternative up vector
    let right = forward.cross(Vec3::X).normalize_or_zero();
}
let up = right.cross(forward);
let rotation = Quat::from_mat3(&Mat3::from_cols(right, up, -forward));
```

---

#### Issue 10: Parent-Child Transform Reparenting

**Problem**: Reparenting entity changes its world position unexpectedly.

**Symptom**: Entity jumps to new location when changing parent.

**Solution**: Calculate new local transform to maintain world position:
```rust
// Get current world transform
let child_world = child_global_transform;
let new_parent_world = new_parent_global_transform;

// Calculate local transform for new parent
let new_local = child_world.reparented_to(&new_parent_world);

// Apply new parent and local transform
commands.entity(child_entity).insert((new_local, new_parent));
```

---

### Common Error Messages

| Error | Cause | Fix |
|-------|-------|-----|
| `expected Vec3, found Dir3` | Bevy 0.18 direction methods return Dir3 | Use `.into()` conversion |
| `GlobalTransform not updated` | Querying in same frame as Transform change | Use `Changed<Transform>` or query in next frame |
| `Quaternion not normalized` | Accumulated floating-point error | Call `.normalize()` periodically |
| `NaN in transform` | Division by zero or invalid math | Use `.is_finite()` checks |
| `Entity not visible after rotation` | Rotation around wrong axis | Verify axis convention (Y-up in Bevy) |

---

## Source File References

### Bevy Transform Crate (v0.18.1)

| File | Path | Description |
|------|------|-------------|
| `transform.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\components\transform.rs` | Transform component definition and methods |
| `global_transform.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\components\global_transform.rs` | GlobalTransform component and reparenting |
| `systems.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\systems.rs` | Transform propagation systems |
| `plugins.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\plugins.rs` | TransformPlugin and TransformSystems enum |
| `commands.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\commands.rs` | Transform-related entity commands |
| `helper.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_transform\src\helper.rs` | Transform helper utilities |

### Bevy Math Crate (v0.18.1)

| File | Path | Description |
|------|------|-------------|
| `lib.rs` | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_math\src\lib.rs` | Re-exports from glam crate |

### rose-offline-client Project

| File | Path | Description |
|------|------|-------------|
| `facing_direction_system.rs` | `src/systems/facing_direction_system.rs:40-43` | Character facing rotation logic |
| `collision_system.rs` | `src/systems/collision_system.rs:97-99` | Server position to Bevy transform conversion |
| `game_keyboard_input_system.rs` | `src/systems/game_keyboard_input_system.rs:67-71` | Camera-relative movement calculation |
| `orbit_camera_system.rs` | `src/systems/orbit_camera_system.rs:260-273` | Orbit camera using dolly CameraRig |
| `fall_system.rs` | `src/systems/season/fall_system.rs:201-214` | Billboard particle rotation |
| `summer_system.rs` | `src/systems/season/summer_system.rs:315` | Billboard with sway rotation composition |
| `bird_system.rs` | `src/systems/bird_system.rs:228-246` | Parent-child wing hierarchy |
| `bird_system.rs` | `src/systems/bird_system.rs:648` | Smooth rotation with slerp |
| `damage_digit_render_system.rs` | `src/systems/damage_digit_render_system.rs:114` | Extract scale/rotation/translation |
| `chat_bubble_spawn_system.rs` | `src/systems/chat_bubble_spawn_system.rs:346-347` | Entity spawning with transform |
| `facing_direction.rs` | `src/components/facing_direction.rs` | FacingDirection component definition |
| `position.rs` | `src/components/position.rs` | Position component (server coordinates) |


