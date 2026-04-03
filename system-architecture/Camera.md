# Camera & Visibility System Architecture

## 1. Overview
The camera system in `rose-offline-client` manages 3D perspective, user interaction (free/orbit modes), and specialized visual effects like underwater rendering. Visibility is handled through a multi-layered approach involving Bevy's built-in ECS components to manage entity and view-specific visibility.

## 2. Camera3d Configuration
Cameras are configured using Bevy's `Camera3d` bundle.
- **PerspectiveProjection**: Controls field of view (FOV), aspect ratio, and near/far clipping planes.
- **MSAA & Clear Color**: Multi-Sampling Anti-Aliasing and background clear colors are configured during camera setup to ensure visual fidelity and consistent background rendering.

## 3. Visibility System
Visibility is managed through three primary components to ensure efficient rendering and correct hierarchy propagation:
- **`Visibility`**: The base flag determining if an entity is fundamentally visible.
- **`ViewVisibility`**: Camera-specific visibility, determining if an entity is within the camera's frustum.
- **`InheritedVisibility`**: Propagates visibility changes down the transform hierarchy.
- **`VisibilitySystems`**: Bevy internal systems that update these components based on hierarchy and frustum culling.

## 4. Exposure Control
Exposure is managed via the `Exposure` component, which controls how the camera reacts to light intensity.
- **Auto-exposure**: Integrated via post-processing to dynamically adjust brightness in different lighting environments (e.g., transitioning from dark caves to bright sunlight).

## 5. Camera Control Systems
The project implements two distinct control modes:

### Free Camera
Used primarily for debugging and map editing.
- **Controls**: WASD for movement, Mouse for rotation.
- **Implementation**: `src/systems/free_camera_system.rs`
- **Input**: Uses `MouseMotion` and `KeyCode`.

### Orbit Camera
Used for model viewing and cinematic inspection.
- **Controls**: Right-click + Drag to rotate, Mouse Wheel to zoom.
- **Implementation**: `src/systems/orbit_camera_system.rs`
- **Logic**: Uses a `CameraRig` with `YawPitch` and `Position` drivers for smooth movement and collision detection via `bevy_rapier3d`.

## 6. Underwater Camera Effects
When a camera enters a water volume, specialized post-processing effects are applied.
- **`CameraUnderwaterState`**: A component tracking if the camera is submerged, the water surface Y-level, and the depth.
- **Effects**:
    - **Fog & Color Grading**: Uses Beer-Lambert law for depth-based color absorption (red is absorbed fastest).
    - **FOV Modification**: Slight FOV adjustments can be applied for a "submerged" feel.
- **Implementation**: `src/render/underwater_effect.rs`

## 7. Code Examples

### Orbit Camera Rotation
```rust
// src/systems/orbit_camera_system.rs:233
if right_pressed {
    let sensitivity = 0.1;
    orbit_camera
        .rig
        .driver_mut::<YawPitch>()
        .rotate_yaw_pitch(-sensitivity * drag_delta.x, -sensitivity * drag_delta.y);
}
```

### Free Camera Movement
```rust
// src/systems/free_camera_system.rs:113
for key in keyboard.get_pressed() {
    match key {
        KeyCode::KeyW => move_vec.z -= 1.0,      // Forward
        KeyCode::KeyS => move_vec.z += 1.0,      // Backward
        KeyCode::KeyA => move_vec.x -= 1.0,      // Left
        KeyCode::KeyD => move_vec.x += 1.0,      // Right
        KeyCode::ShiftLeft => speed_boost_multiplier = 4.0,
        _ => {}
    }
}
```

### Underwater State Detection
```rust
// src/render/underwater_effect.rs:422
pub fn detect_underwater_camera(
    mut camera_query: Query<(&GlobalTransform, &mut CameraUnderwaterState), With<Camera>>,
    // ...
) {
    // ... logic to check if camera_position.y < volume.surface_y
}
```

## 8. Troubleshooting
- **Camera not updating**: Check if `egui` is consuming input. Use `egui_ctx.ctx_mut().unwrap().wants_pointer_input()` to gate camera controls.
- **Visibility Flickering**: Ensure `InheritedVisibility` is correctly propagating. Check for conflicting systems modifying `Visibility` or `Transform` in the same frame.
- **Exposure Issues**: If the screen is too bright/dark, verify the `Exposure` component values and ensure the auto-exposure system is running in the correct schedule.

## 9. Source File References
### Bevy Source
- Camera/Visibility: `bevy_render/src/view/visibility/`
- Camera Core: `bevy_camera/src/`

### Project Source
- Camera Systems: `src/systems/camera_system.rs` (if exists), `src/systems/orbit_camera_system.rs`, `src/systems/free_camera_system.rs`
- Underwater: `src/render/underwater_effect.rs`
- Components: `src/components/camera.rs` (if exists)