# Camera & Visibility System Architecture

## 1. Overview
The camera system in `rose-offline-client` manages 3D perspective, user interaction (free/orbit modes), and specialized visual effects like underwater rendering. Visibility is handled through a multi-layered approach involving Bevy's built-in ECS components to manage entity and view-specific visibility.

## 2. Camera3d Configuration
Cameras are configured using Bevy's `Camera3d` bundle.
- **PerspectiveProjection**: Controls field of view (FOV), aspect ratio, and near/far clipping planes (`src/lib.rs:1972-1982`: fov PI/4, near 0.1, far 8000.0).
- **MSAA & Clear Color**: The main camera spawns with `Msaa::Off` (`src/lib.rs:1967`; MSAA X1/X2/X4/X8 is opt-in via `apply_msaa_system` in `src/graphics/apply_systems.rs:208-231`) and `clear_color: Custom(srgb(0.0, 0.0, 0.02))` near-black for star visibility (`src/lib.rs:1969`). The water-reflection camera uses `Msaa::Off` + `Custom(BLACK)` (`src/render/water_reflection.rs:151-156).

## 3. Visibility System
Visibility is managed through three primary components plus Bevy's internal systems, ensuring efficient rendering and correct hierarchy propagation:
- **`Visibility`**: The base flag determining if an entity is fundamentally visible.
- **`ViewVisibility`**: Camera-specific visibility, determining if an entity is within the camera's frustum.
- **`InheritedVisibility`**: Propagates visibility changes down the transform hierarchy.
- **`VisibilitySystems`**: Bevy internal systems that update these components based on hierarchy and frustum culling.

## 4. Exposure Control
No `AutoExposure` component exists anywhere in `src/` — brightness is controlled by tonemapping, bloom, environment lighting, and atmosphere instead (`src/lib.rs:1990-1999` explicitly leaves SMAA/SSR/MotionBlur/AutoExposure/CAS off by default).
- **Tonemapping**: `TonyMcMapface` on the main camera (`src/lib.rs:1990`).
- **Bloom**: `Bloom::NATURAL` (`src/lib.rs:1992`).
- **EnvironmentMapLight**: `intensity: 100.0` from `SPECULAR_SPHEREMAP.DDS#cube` (`src/lib.rs:2020-2024`).
- **Atmosphere**: `Atmosphere::earthlike` on the camera, removed at night so stars show (`src/lib.rs:2036-2039`, `toggle_atmosphere_based_on_time`).

## 5. Camera Control Systems
The project implements three control modes:

### Free Camera
Used for debugging, map editing, and free exploration in the viewer modes (zone viewer, model viewer, login screen).
- **Controls**: WASD for movement, Mouse for rotation.
- **Implementation**: `src/systems/free_camera_system.rs`
- **Input**: Uses `MouseMotion` and `KeyCode`.

### Orbit Camera
Used as the main third-person gameplay camera, following a target entity (e.g., the player character, boats, flight movement) with an offset and distance.
- **Controls**: Right-click + Drag to rotate, Mouse Wheel to zoom.
- **Implementation**: `src/systems/orbit_camera_system.rs`
- **Logic**: Uses a `CameraRig` with `YawPitch` and `Position` drivers for smooth movement and collision detection via `bevy_rapier3d`.

### Sail Camera
Boat-follow camera used while sailing.
- **Implementation**: `src/systems/sail_camera_system.rs`

## 6. Underwater Camera Effects
When a camera enters a water volume, specialized post-processing effects are applied.
- **`CameraUnderwaterState`**: A component tracking if the camera is submerged, the water surface Y-level, and the depth.
- **Effects**:
    - **Fog & Color Grading**: Uses Beer-Lambert law for depth-based color absorption (red is absorbed fastest).
    - **Caustics**: Procedural animated caustics are overlaid additively, modulated by depth and the underwater fog density.
- **Implementation**: `src/render/underwater_effect.rs`

## 7. Code Examples

### Orbit Camera Rotation
```rust
// src/systems/orbit_camera_system.rs:235
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
// src/systems/free_camera_system.rs:117
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
// src/render/underwater_effect.rs:426
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
- **Exposure Issues**: If the screen is too bright/dark, check `Tonemapping`, `Bloom`, `EnvironmentMapLight{intensity}`, and whether `Atmosphere` is present (it is removed at night). There is no `AutoExposure` component in this client.

## 9. Source File References
### Bevy Source
- Camera/Visibility: `bevy_camera/src/visibility/`
- Camera Core: `bevy_camera/src/`

### Project Source
- Camera Systems: `src/systems/orbit_camera_system.rs`, `src/systems/free_camera_system.rs`, `src/systems/sail_camera_system.rs`
- Camera Animation: `src/animation/camera_animation.rs` (cinematic ZMO camera); login camera: `src/resources/login_camera_animation.rs`
- Underwater: `src/render/underwater_effect.rs`