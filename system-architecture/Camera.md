# Camera & Visibility System Architecture

## 1. Overview
The camera system in `rose-offline-client` manages 3D perspective, user interaction (free/orbit modes), and underwater state tracking. Visibility is handled through a multi-layered approach involving Bevy's built-in ECS components to manage entity and view-specific visibility.

## 2. Camera3d Configuration
Cameras are configured using Bevy's `Camera3d` bundle.
- **PerspectiveProjection**: Controls field of view (FOV), aspect ratio, and near/far clipping planes (`src/lib.rs`: fov PI/4, near 0.1, far 8000.0). Bevy builds an infinite reverse-Z projection: `far` never clips and only sets the `Frustum`'s far plane, which neither CPU nor GPU culling tests. `apply_view_distance_system` (View Distance slider -> `far`) therefore has no effect on culling or rendering; the sky sphere (radius 4000, follows the camera) is never clipped by `far`.
- **MSAA & Clear Color**: The main camera spawns with `Msaa::Off` (`src/lib.rs`). There is no MSAA setting: the camera is deferred (`DeferredPrepass`), and Bevy's `check_msaa` forces `Msaa::Off` on deferred cameras, so the old X2/X4/X8 option and `apply_msaa_system` were removed 2026-09-30. SMAA is the anti-aliasing. It spawns with `clear_color: Custom(srgb(0.0, 0.0, 0.02))` near-black for star visibility (`src/lib.rs:1969`). The water-reflection camera uses `Msaa::Off` + `Custom(BLACK)` (`src/render/water_reflection.rs:151-156).

## 3. Visibility System
Visibility is managed through three primary components plus Bevy's internal systems, ensuring efficient rendering and correct hierarchy propagation:
- **`Visibility`**: The base flag determining if an entity is fundamentally visible.
- **`ViewVisibility`**: Camera-specific visibility, determining if an entity is within the camera's frustum.
- **`InheritedVisibility`**: Propagates visibility changes down the transform hierarchy.
- **`VisibilitySystems`**: Bevy internal systems that update these components based on hierarchy and frustum culling.

Culling summary (details and the per-entity bounds table in [Render.md](Render.md#culling-frustum-occlusion-shadow-cascades)):
- `ViewVisibility` is true when *any* view drew the entity: a camera frustum (`Aabb` vs `Frustum`, near plane only, far plane ignored) or a directional-light shadow cascade. Entities without an `Aabb` are never frustum culled.
- The main camera also has `OcclusionCulling` (GPU, two-phase, works with the deferred prepass in Bevy 0.19.1). It tests the `Aabb` of every mesh, `NoFrustumCulling` ones included, so every mesh needs a finite `Aabb` enclosing what its shader draws (particles, morph animation, skinning and the sky get theirs from `render/culling_bounds.rs`, `DynamicSkinnedMeshBounds` and an explicit sky box).
- The water reflection camera (layer 0, `invert_culling`) gets its `Frustum` written by `sync_reflection_camera` from the mirrored transform (CPU culling and the GPU culling shader both use that component); it has no depth prepass, so no occlusion culling. Without the oblique near clip plane (disabled) it also draws geometry between the mirrored camera and the water surface.
- `ViewVisibility` read before PostUpdate's `CheckVisibility` is last frame's result; systems use it to skip CPU work for undrawn entities (particle buffer uploads, morph material writes).

## 4. Exposure Control
No `AutoExposure` component exists anywhere in `src/` — brightness is controlled by tonemapping, bloom, environment lighting, and atmosphere instead (`src/lib.rs:1990-1999` explicitly leaves SMAA/SSR/MotionBlur/AutoExposure/CAS off by default).
- **Tonemapping**: `TonyMcMapface` on the main camera (`src/lib.rs:1990`).
- **Bloom**: `Bloom::NATURAL` (`src/lib.rs:1992`).
- **EnvironmentMapLight**: `intensity: 100.0` from `SPECULAR_SPHEREMAP.DDS#cube` (`src/lib.rs:2020-2024`).
- **Atmosphere**: camera carries `AtmosphereSettings`; the standalone `Atmosphere` entity is kept spawned at all times (`toggle_atmosphere_based_on_time` — despawning it at night triggered Bevy 0.19.1 bug #24808, see `pitfalls/atmosphere-flash.md`).

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
- **Logic**: Uses a `CameraRig` with `YawPitch` and `Position` drivers for smooth movement and collision detection via `bevy_rapier3d`. The collision shape cast runs every frame (as in the original client); skipping it while the camera was static let the arm extend through walls after the player stopped next to one.

### Sail Camera
Boat-follow camera used while sailing.
- **Implementation**: `src/systems/sail_camera_system.rs`

## 6. Underwater Camera State
There is no underwater screen effect (the fog/tint/caustics post-process was removed). Only the state is tracked:
- **`CameraUnderwaterState`**: A component tracking if the camera is submerged (inside a water volume's XZ bounds and within `WaterSettings::max_depth` below its surface), the water surface Y-level, and the depth. The water reflection camera is disabled while it is set.
- **`UnderwaterVolumes`**: world-space water volumes from `WaterSpawnedEvent`s; volumes whose water entity was despawned (zone change) are dropped.
- Seen from below, the water surface itself shows Snell's window (`water_material.wgsl`).
- **Implementation**: `src/render/underwater_effect.rs` (`UnderwaterStatePlugin`)

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
// src/render/underwater_effect.rs
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
- **Exposure Issues**: If the screen is too bright/dark, check `Tonemapping`, `Bloom`, `EnvironmentMapLight{intensity}`, and the `Atmosphere` entity (always present; must never be despawned at runtime on 0.19.1). There is no `AutoExposure` component in this client.

## 9. Source File References
### Bevy Source
- Camera/Visibility: `bevy_camera/src/visibility/`
- Camera Core: `bevy_camera/src/`

### Project Source
- Camera Systems: `src/systems/orbit_camera_system.rs`, `src/systems/free_camera_system.rs`, `src/systems/sail_camera_system.rs`
- Camera Animation: `src/animation/camera_animation.rs` (cinematic ZMO camera); login camera: `src/resources/login_camera_animation.rs`
- Underwater: `src/render/underwater_effect.rs`