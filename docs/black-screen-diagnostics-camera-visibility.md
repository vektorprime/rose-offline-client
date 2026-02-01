# ECS Camera Component Validation & Visibility Diagnostics

> **Version**: 1.0 | **Bevy Version**: 0.13.2 | **Project**: Rose Online Client  
> **Purpose**: Supplementary diagnostic documentation for camera, visibility, and transform hierarchy validation
> **Parent Document**: [`black-screen-diagnostic-protocol.md`](docs/black-screen-diagnostic-protocol.md)

---

## Table of Contents

1. [Camera Component Validation](#1-camera-component-validation)
2. [Projection Matrix Validation](#2-projection-matrix-validation)
3. [Camera Target Validation](#3-camera-target-validation)
4. [Visibility System Diagnostics](#4-visibility-system-diagnostics)
5. [Frustum Culling Diagnostics](#5-frustum-culling-diagnostics)
6. [Lighting Bundle Completeness](#6-lighting-bundle-completeness)
7. [Transform Hierarchy Validation](#7-transform-hierarchy-validation)
8. [Diagnostic Queries Reference](#8-diagnostic-queries-reference)

---

## 1. Camera Component Validation

### 1.1 Camera Entity Existence Check

In Bevy 0.13.2, a camera entity is required for any rendering to occur. Without at least one active camera, all rendering systems will bypass rendering entirely.

#### Required Camera Components

| Component | Import Path | Purpose | Required |
|-----------|-------------|---------|----------|
| `Camera` | `bevy::render::camera::Camera` | Core camera configuration | Yes |
| `CameraRenderGraph` | `bevy::render::camera::CameraRenderGraph` | Render graph association | Yes |
| `Transform` | `bevy::transform::components::Transform` | Local transform | Yes |
| `GlobalTransform` | `bevy::transform::components::GlobalTransform` | World-space transform | Computed |
| `Projection` | `bevy::render::camera::Projection` | Projection matrix (enum wrapper) | Yes |
| `Frustum` | `bevy::render::primitives::Frustum` | View frustum for culling | Computed |

#### Camera Ordering (`CameraOrder`)

Bevy 0.13.2 uses the `Order` component to determine camera priority:

```rust
use bevy::render::camera::Camera;
use bevy::render::view::VisibleEntities;

// Camera order component (higher = higher priority)
#[derive(Component, Default)]
pub struct Order(pub isize);

// Default camera order values
const CAMERA_3D_ORDER: isize = 100;
const CAMERA_2D_ORDER: isize = 200;
```

| Property | Type | Description |
|----------|------|-------------|
| `Order` | `isize` | Camera priority (higher renders on top) |
| `is_active` | `bool` | Whether camera renders to target |
| `viewport` | `Option<Viewport>` | Optional viewport rect |

#### Active Camera Determination

Bevy 0.13.2 determines the active camera using this algorithm:

```rust
// Pseudocode of Bevy's camera selection logic:
fn determine_active_cameras(
    cameras: Query<(&Camera, &Order)>
) -> Vec<Entity> {
    cameras
        .iter()
        .filter(|(cam, _)| cam.is_active)
        .filter(|(cam, _)| cam.target.is_some())
        .filter(|(cam, _)| cam.viewport.is_some())
        .sorted_by_key(|(_, order)| order.0)
        .map(|(entity, _)| entity)
        .collect()
}
```

**Troubleshooting Active Camera Issues**:

| Symptom | Diagnostic Query | Resolution |
|---------|------------------|------------|
| "No active cameras" | `query_camera_validation` | Spawn camera with `is_active=true` |
| Multiple cameras fighting | Check `Order` values | Ensure unique orders or disable extras |
| Camera not rendering | Check `Camera::target` | Ensure target window exists |
| Order ignored | Verify `Order` component | Insert `Order(100)` component |

#### Diagnostic System: Camera Existence

```rust
use bevy::prelude::*;
use bevy::render::camera::{Camera, CameraRenderGraph};

/// System to validate camera entity existence and configuration
fn camera_entity_validation_system(
    cameras: Query<(Entity, &Camera, Option<&Order>), With<CameraRenderGraph>>,
    mut diagnostics: ResMut<CameraDiagnostics>,
) {
    let camera_count = cameras.iter().count();
    let active_count = cameras.iter().filter(|(_, cam, _)| cam.is_active).count();
    
    diagnostics.camera_count = camera_count;
    diagnostics.active_camera_count = active_count;
    
    if camera_count == 0 {
        warn!("[CAMERA DIAGNOSTIC] No camera entities found with CameraRenderGraph");
        return;
    }
    
    if active_count == 0 {
        warn!("[CAMERA DIAGNOSTIC] Found {} cameras, but none are active", camera_count);
    }
    
    for (entity, camera, order) in cameras.iter() {
        let order_val = order.map(|o| o.0).unwrap_or(0);
        debug!(
            "[CAMERA DIAGNOSTIC] Camera {:?} - Active: {}, Order: {}",
            entity, camera.is_active, order_val
        );
        
        // Validate camera has target
        if camera.target.is_none() {
            warn!("[CAMERA DIAGNOSTIC] Camera {:?} has no render target", entity);
        }
    }
}
```

---

## 2. Projection Matrix Validation

### 2.1 Projection Types in Bevy 0.13.2

Bevy 0.13.2 uses an enum-based projection system:

```rust
use bevy::render::camera::Projection;
use bevy::render::camera::{PerspectiveProjection, OrthographicProjection};

#[derive(Component, Clone)]
pub enum Projection {
    Perspective(PerspectiveProjection),
    Orthographic(OrthographicProjection),
}
```

### 2.2 Perspective Projection Diagnostics

**Component**: `PerspectiveProjection`
**Import**: `bevy::render::camera::PerspectiveProjection`

```rust
pub struct PerspectiveProjection {
    pub fov: f32,           // Field of view in radians
    pub aspect_ratio: f32,  // Width / Height
    pub near: f32,          // Near clipping plane
    pub far: f32,           // Far clipping plane
}
```

**Validation Requirements**:

| Property | Valid Range | Failure Symptoms | Resolution |
|----------|-------------|------------------|------------|
| `fov` | 0.01 to π | Extreme distortion, fisheye effect | Clamp to 0.785 (45°) to 1.571 (90°) |
| `aspect_ratio` | > 0.01 | Stretched/squashed rendering | Calculate from window: `width/height` |
| `near` | 0.001 to 1000 | Z-fighting, clipping issues | Use 0.1 for Rose Online |
| `far` | > `near` | Distant objects culled | Use 10000.0 to 50000.0 for zones |
| `near` < `far` | Must be true | Projection matrix fails | Ensure near < far |

#### Perspective Projection Diagnostic System

```rust
fn perspective_projection_validation_system(
    cameras: Query<(Entity, &Camera, &Projection, &Transform)>,
) {
    for (entity, camera, projection, transform) in cameras.iter() {
        if !camera.is_active {
            continue;
        }
        
        if let Projection::Perspective(persp) = projection {
            // FOV validation
            if persp.fov <= 0.0 || persp.fov >= std::f32::consts::PI {
                warn!(
                    "[PROJECTION DIAGNOSTIC] Camera {:?} has invalid FOV: {}",
                    entity, persp.fov
                );
            }
            
            // Aspect ratio validation
            if persp.aspect_ratio <= 0.0 {
                warn!(
                    "[PROJECTION DIAGNOSTIC] Camera {:?} has invalid aspect ratio: {}",
                    entity, persp.aspect_ratio
                );
            }
            
            // Near/far plane validation
            if persp.near <= 0.0 {
                warn!(
                    "[PROJECTION DIAGNOSTIC] Camera {:?} near plane <= 0: {}",
                    entity, persp.near
                );
            }
            
            if persp.far <= persp.near {
                warn!(
                    "[PROJECTION DIAGNOSTIC] Camera {:?} far <= near: far={}, near={}",
                    entity, persp.far, persp.near
                );
            }
            
            // NaN/Inf detection
            if persp.fov.is_nan() || persp.fov.is_infinite() {
                error!(
                    "[PROJECTION DIAGNOSTIC] Camera {:?} FOV is NaN/Inf!",
                    entity
                );
            }
            
            // Rose Online specific: check far plane is adequate
            if persp.far < 1000.0 {
                warn!(
                    "[PROJECTION DIAGNOSTIC] Camera {:?} far plane may be too low for zones: {}",
                    entity, persp.far
                );
            }
        }
    }
}
```

### 2.3 Orthographic Projection Diagnostics

**Component**: `OrthographicProjection`
**Import**: `bevy::render::camera::OrthographicProjection`

```rust
pub struct OrthographicProjection {
    pub near: f32,
    pub far: f32,
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub window_origin: WindowOrigin,
    pub scaling_mode: ScalingMode,
}
```

**Validation Requirements**:

| Property | Valid Range | Failure Symptoms |
|----------|-------------|------------------|
| `left` < `right` | Required | Inverted geometry |
| `bottom` < `top` | Required | Inverted geometry |
| All bounds | Finite values | Projection failure |

### 2.4 Custom Projection Validation

For custom projections, validate the computed matrix:

```rust
fn custom_projection_validation_system(
    cameras: Query<(Entity, &Camera, &GlobalTransform)>,
) {
    for (entity, camera, global_transform) in cameras.iter() {
        if let Some(view_proj) = camera.computed.world_to_ndc_matrix() {
            let matrix = view_proj.to_cols_array_2d();
            
            // Check for NaN/Inf in matrix
            for row in &matrix {
                for &val in row {
                    if val.is_nan() {
                        error!(
                            "[PROJECTION DIAGNOSTIC] Camera {:?} has NaN in projection matrix!",
                            entity
                        );
                    }
                    if val.is_infinite() {
                        warn!(
                            "[PROJECTION DIAGNOSTIC] Camera {:?} has Inf in projection matrix",
                            entity
                        );
                    }
                }
            }
        } else {
            warn!(
                "[PROJECTION DIAGNOSTIC] Camera {:?} has no computed projection matrix",
                entity
            );
        }
    }
}
```

---

## 3. Camera Target Validation

### 3.1 Render Target Types

Bevy 0.13.2 supports two camera target types:

```rust
pub enum RenderTarget {
    /// Render to a window surface
    Window(WindowRef),
    /// Render to an image texture
    Image(Handle<Image>),
}

pub enum WindowRef {
    /// Primary window
    Primary,
    /// Specific window entity
    Entity(Entity),
}
```

### 3.2 Window Entity Reference Validation

```rust
use bevy::window::Window;

fn camera_target_window_validation_system(
    cameras: Query<(Entity, &Camera)>,
    windows: Query<(Entity, &Window)>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
) {
    for (cam_entity, camera) in cameras.iter() {
        match &camera.target {
            RenderTarget::Window(window_ref) => {
                match window_ref {
                    WindowRef::Primary => {
                        match primary_window.get_single() {
                            Ok(_) => {}, // Primary window exists
                            Err(_) => {
                                error!(
                                    "[TARGET DIAGNOSTIC] Camera {:?} targets Primary window, but none exists!",
                                    cam_entity
                                );
                            }
                        }
                    }
                    WindowRef::Entity(window_entity) => {
                        if windows.get(*window_entity).is_err() {
                            error!(
                                "[TARGET DIAGNOSTIC] Camera {:?} targets non-existent window entity {:?}",
                                cam_entity, window_entity
                            );
                        }
                    }
                }
            }
            RenderTarget::Image(image_handle) => {
                // Image target validation handled in texture section
                if image_handle.id() == Handle::default().id() {
                    warn!(
                        "[TARGET DIAGNOSTIC] Camera {:?} targets default image handle",
                        cam_entity
                    );
                }
            }
            _ => {}
        }
    }
}
```

### 3.3 Target Resolution Matching

```rust
fn camera_resolution_validation_system(
    cameras: Query<(Entity, &Camera), With<Camera2d>>, // or Camera3d
    windows: Query<&Window>,
    images: Res<Assets<Image>>,
) {
    for (cam_entity, camera) in cameras.iter() {
        let target_size = match &camera.target {
            RenderTarget::Window(window_ref) => {
                // Get window resolution
                Some(Vec2::new(window.width() as f32, window.height() as f32))
            }
            RenderTarget::Image(handle) => {
                images.get(handle).map(|img| {
                    Vec2::new(img.width() as f32, img.height() as f32)
                })
            }
            _ => None,
        };
        
        if let Some(size) = target_size {
            // Check for zero-size targets
            if size.x <= 0.0 || size.y <= 0.0 {
                error!(
                    "[TARGET DIAGNOSTIC] Camera {:?} has zero-size target: {:?}",
                    cam_entity, size
                );
            }
            
            // Check for extreme resolutions
            if size.x > 16384.0 || size.y > 16384.0 {
                warn!(
                    "[TARGET DIAGNOSTIC] Camera {:?} has extreme resolution: {:?}",
                    cam_entity, size
                );
            }
        }
    }
}
```

### 3.4 Clear Color Configuration

In Bevy 0.13.2, clear color is configured via the `ClearColor` resource:

```rust
use bevy::core_pipeline::clear_color::ClearColor;

fn clear_color_validation_system(
    clear_color: Res<ClearColor>,
    cameras: Query<&Camera>,
) {
    // Validate clear color is reasonable
    let color = clear_color.0;
    
    // Check for NaN colors
    if color.r().is_nan() || color.g().is_nan() || color.b().is_nan() || color.a().is_nan() {
        error!("[CLEAR COLOR DIAGNOSTIC] ClearColor contains NaN values!");
    }
    
    // Check for HDR values if not using HDR camera
    let max_component = color.r().max(color.g()).max(color.b());
    if max_component > 1.0 {
        info!(
            "[CLEAR COLOR DIAGNOSTIC] ClearColor has HDR values (max={})",
            max_component
        );
    }
}
```

**Clear Color Options**:

| Configuration | Effect | Use Case |
|--------------|--------|----------|
| `Color::BLACK` | Solid black | Testing, dark scenes |
| `Color::rgb(0.1, 0.1, 0.15)` | Night sky | Night environments |
| `Color::rgb(0.5, 0.7, 1.0)` | Sky blue | Day environments |

---

## 4. Visibility System Diagnostics

### 4.1 Three-Tier Visibility System

Bevy 0.13.2 uses a sophisticated three-tier visibility system:

```rust
use bevy::render::view::Visibility;
use bevy::render::view::InheritedVisibility;
use bevy::render::view::ViewVisibility;
```

#### Tier 1: `Visibility` (User-Controlled)

**Import**: `bevy::render::view::Visibility`
**Purpose**: User-controlled visibility state

```rust
pub enum Visibility {
    /// Entity is visible (unless parent is hidden)
    Visible,
    /// Entity is hidden regardless of parent
    Hidden,
    /// Entity inherits visibility from parent
    Inherited,
}

impl Default for Visibility {
    fn default() -> Self { Visibility::Visible }
}
```

**Troubleshooting**:

| Value | Description | When to Use |
|-------|-------------|-------------|
| `Visibility::Visible` | Force visible | Entity should always render |
| `Visibility::Hidden` | Force hidden | Entity should never render |
| `Visibility::Inherited` | Use parent state | Child entities, default behavior |

#### Tier 2: `InheritedVisibility` (Parent-Propagated)

**Import**: `bevy::render::view::InheritedVisibility`
**Purpose**: Computed visibility from hierarchy

```rust
#[derive(Component, Default, Clone)]
pub struct InheritedVisibility(pub bool);

impl InheritedVisibility {
    pub fn get(&self) -> bool { self.0 }
}
```

**Propagation Rules**:
- If parent has `Visibility::Hidden` → child `InheritedVisibility(false)`
- If parent has `Visibility::Visible` → child `InheritedVisibility(true)`
- If parent has `Visibility::Inherited` → propagates grandparents' state

#### Tier 3: `ViewVisibility` (Frustum-Culled)

**Import**: `bevy::render::view::ViewVisibility`
**Purpose**: Frustum culling result

```rust
#[derive(Component, Default, Clone)]
pub struct ViewVisibility(pub bool);

impl ViewVisibility {
    pub fn get(&self) -> bool { self.0 }
}
```

**Final visibility = `InheritedVisibility && ViewVisibility`**

### 4.2 Visibility Propagation Validation

```rust
fn visibility_propagation_diagnostics_system(
    query: Query<(Entity, &Visibility, &InheritedVisibility), With<Children>>,
    children_query: Query<&Children>,
    children_visibility: Query<(Entity, &Visibility, &InheritedVisibility)>,
) {
    for (parent_entity, parent_vis, parent_inherited) in query.iter() {
        // Check if this entity has children
        if let Ok(children) = children_query.get(parent_entity) {
            for child in children.iter() {
                if let Ok((child_entity, child_vis, child_inherited)) = 
                    children_visibility.get(*child) {
                    
                    // Validate propagation logic
                    let expected_inherited = match parent_vis {
                        Visibility::Hidden => false,
                        Visibility::Visible => true,
                        Visibility::Inherited => parent_inherited.get(),
                    };
                    
                    if child_inherited.get() != expected_inherited {
                        warn!(
                            "[VISIBILITY PROPAGATION] Entity {:?} inherited={}, expected={}",
                            child_entity, child_inherited.get(), expected_inherited
                        );
                    }
                }
            }
        }
    }
}
```

### 4.3 VisibilitySystems Set Ordering

Bevy 0.13.2 uses system sets for visibility computation:

```rust
use bevy::render::view::VisibilitySystems;

// System set for visibility computation
#[derive(SystemSet, Clone, Hash, Debug, PartialEq, Eq)]
pub enum VisibilitySystems {
    /// Label for the system sets `VisibilityPropagate`
    VisibilityPropagate,
    /// Label for the system set containing all visibility check systems
    CheckVisibility,
    /// Label for the system set calculating bounds for entities
    CalculateBounds,
}
```

**Execution Order**:

```rust
// The correct ordering is:
// 1. Propagate parent visibility to children
// 2. Calculate AABB bounds if needed
// 3. Check visibility against camera frustums

.systems(PostUpdate, (
    // Propagation must happen first
    propagate_visibility_system.in_set(VisibilitySystems::VisibilityPropagate),
    // Then calculate/update bounds
    calculate_bounds_system.in_set(VisibilitySystems::CalculateBounds),
    // Finally check visibility
    check_visibility_system.in_set(VisibilitySystems::CheckVisibility)
        .after(VisibilitySystems::VisibilityPropagate)
        .after(VisibilitySystems::CalculateBounds),
))
```

**Validation Checklist**:

- [ ] `VisibilitySystems::VisibilityPropagate` runs before `CheckVisibility`
- [ ] All entities with `Visibility` component get `InheritedVisibility` updated
- [ ] No system orders itself after `VisibilitySystems::CheckVisibility` and mutates visibility

### 4.4 Visibility Diagnostic System

```rust
fn comprehensive_visibility_diagnostics_system(
    entities: Query<(Entity, Option<&Name>, &Visibility, &InheritedVisibility, &ViewVisibility)>,
    cameras: Query<&Camera>,
) {
    let has_active_camera = cameras.iter().any(|c| c.is_active);
    
    if !has_active_camera {
        warn!("[VISIBILITY DIAGNOSTIC] No active cameras - visibility checks are meaningless!");
        return;
    }
    
    let mut visible_count = 0;
    let mut hidden_count = 0;
    let mut culled_count = 0;
    
    for (entity, name, vis, inherited, view) in entities.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        let is_user_visible = matches!(vis, Visibility::Visible);
        let is_inherited_visible = inherited.get();
        let is_view_visible = view.get();
        
        let final_visible = is_inherited_visible && is_view_visible;
        
        if final_visible {
            visible_count += 1;
        } else {
            hidden_count += 1;
            
            if is_inherited_visible && !is_view_visible {
                culled_count += 1;
                debug!(
                    "[VISIBILITY DIAGNOSTIC] Entity {:?} ({}) culled by frustum: user={:?}, inherited={}, view={}",
                    entity, name_str, vis, is_inherited_visible, is_view_visible
                );
            }
        }
    }
    
    info!(
        "[VISIBILITY SUMMARY] Visible: {}, Hidden: {}, Frustum Culled: {}",
        visible_count, hidden_count, culled_count
    );
}
```

---

## 5. Frustum Culling Diagnostics

### 5.1 Frustum Component

**Component**: `Frustum`
**Import**: `bevy::render::primitives::Frustum`
**Purpose**: Defines the camera's view frustum for culling

```rust
pub struct Frustum {
    pub half_spaces: [HalfSpace; 6], // Left, Right, Top, Bottom, Near, Far
}

pub struct HalfSpace {
    pub normal_d: Vec4, // Normal vector + distance from origin
}
```

**The Frustum is automatically computed from the camera's projection matrix.**

### 5.2 AABB (Axis-Aligned Bounding Box) Validation

**Component**: `Aabb`
**Import**: `bevy::render::primitives::Aabb`
**Purpose**: Bounding volume for mesh culling

```rust
pub struct Aabb {
    pub center: Vec3A,
    pub half_extents: Vec3A,
}
```

**Validation Requirements**:

| Property | Valid Range | Failure Mode |
|----------|-------------|--------------|
| `half_extents.x` | > 0 | Zero-width mesh always culled |
| `half_extents.y` | > 0 | Zero-height mesh always culled |
| `half_extents.z` | > 0 | Zero-depth mesh always culled |
| Center | Finite | NaN center causes culling |

#### AABB Diagnostic System

```rust
fn aabb_validation_system(
    query: Query<(Entity, Option<&Name>, Option<&Aabb>), With<Mesh>>,
) {
    let mut with_aabb = 0;
    let mut without_aabb = 0;
    let mut invalid_aabb = 0;
    
    for (entity, name, aabb) in query.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        match aabb {
            Some(aabb) => {
                with_aabb += 1;
                
                // Validate AABB dimensions
                if aabb.half_extents.x <= 0.0 || 
                   aabb.half_extents.y <= 0.0 || 
                   aabb.half_extents.z <= 0.0 {
                    warn!(
                        "[AABB DIAGNOSTIC] Entity {:?} ({}) has invalid AABB extents: {:?}",
                        entity, name_str, aabb.half_extents
                    );
                    invalid_aabb += 1;
                }
                
                // Check for NaN
                if aabb.center.is_nan().any() {
                    error!(
                        "[AABB DIAGNOSTIC] Entity {:?} ({}) has NaN in AABB center!",
                        entity, name_str
                    );
                    invalid_aabb += 1;
                }
                
                if aabb.half_extents.is_nan().any() {
                    error!(
                        "[AABB DIAGNOSTIC] Entity {:?} ({}) has NaN in AABB extents!",
                        entity, name_str
                    );
                    invalid_aabb += 1;
                }
            }
            None => {
                without_aabb += 1;
                warn!(
                    "[AABB DIAGNOSTIC] Entity {:?} ({}) has Mesh but no AABB component!",
                    entity, name_str
                );
            }
        }
    }
    
    info!(
        "[AABB SUMMARY] Total meshes: {}, With AABB: {}, Without: {}, Invalid: {}",
        with_aabb + without_aabb, with_aabb, without_aabb, invalid_aabb
    );
}
```

### 5.3 NoFrustumCulling Marker

**Component**: `NoFrustumCulling`
**Import**: `bevy::render::view::NoFrustumCulling`
**Purpose**: Skip frustum culling for entity

```rust
/// Add this component to skip frustum culling
#[derive(Component, Default)]
pub struct NoFrustumCulling;
```

**Use Cases**:
- Large terrain tiles that always need rendering
- Skybox/sphere that surrounds camera
- UI elements in world space
- Debugging geometry

### 5.4 Distance Culling Configuration

While Bevy 0.13.2 doesn't have built-in distance culling, you can implement it:

```rust
#[derive(Component)]
pub struct DistanceCulling {
    pub max_distance: f32,
}

fn distance_culling_system(
    mut query: Query<(Entity, &GlobalTransform, &DistanceCulling, &mut ViewVisibility)>,
    camera: Query<&GlobalTransform, With<Camera>>,
) {
    let Ok(cam_transform) = camera.get_single() else { return };
    let cam_pos = cam_transform.translation();
    
    for (entity, transform, culling, mut view_vis) in query.iter_mut() {
        let distance = transform.translation().distance(cam_pos);
        
        if distance > culling.max_distance {
            view_vis.0 = false;
            debug!("[DISTANCE CULL] Hiding entity {:?} at distance {}", entity, distance);
        }
    }
}
```

---

## 6. Lighting Bundle Completeness

### 6.1 Light Component Types

Bevy 0.13.2 provides three light types:

```rust
use bevy::pbr::{PointLight, DirectionalLight, SpotLight};
```

#### Point Light

```rust
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,      // Luminous intensity in candela
    pub range: f32,          // Cutoff distance
    pub radius: f32,         // Light source radius
    pub shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
}
```

**Validation**:

| Property | Recommendation | Invalid Value |
|----------|-----------------|---------------|
| `range` | > 0.0 | 0 or negative |
| `intensity` | 1000.0 - 100000.0 | 0 (invisible) |
| `radius` | ≥ 0.0 | Negative |

#### Directional Light

```rust
pub struct DirectionalLight {
    pub illuminance: f32,           // Lux
    pub shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub cascade_shadow_config: CascadeShadowConfig,
}
```

**Validation Requirements**:
- Must have a `Transform` component to determine light direction
- Direction is `-transform.forward()`

### 6.2 Shadow Caster Validation

**Component**: `NotShadowCaster`
**Import**: `bevy::pbr::NotShadowCaster`
**Purpose**: Exclude entity from shadow casting

```rust
/// Marker to skip shadow casting
#[derive(Component, Default)]
pub struct NotShadowCaster;

/// Marker to skip shadow receiving
#[derive(Component, Default)]
pub struct NotShadowReceiver;
```

**Shadow Diagnostic System**:

```rust
fn shadow_validation_system(
    query: Query<(Entity, &PointLight, &Transform)>,
    shadow Casters: Query<(Entity, &GlobalTransform), Without<NotShadowCaster>>,
) {
    for (light_entity, light, transform) in query.iter() {
        if light.shadows_enabled {
            // Check for valid shadow configuration
            if light.shadow_depth_bias < 0.0 {
                warn!(
                    "[SHADOW DIAGNOSTIC] Light {:?} has negative depth bias: {}",
                    light_entity, light.shadow_depth_bias
                );
            }
            
            // Verify light transform
            if transform.translation.is_nan().any() {
                error!(
                    "[SHADOW DIAGNOSTIC] Light {:?} has NaN transform!",
                    light_entity
                );
            }
        }
    }
}
```

### 6.3 Environment Map Validation

**Resource**: `AmbientLight`
**Import**: `bevy::pbr::AmbientLight`

```rust
fn environment_validation_system(
    ambient: Res<AmbientLight>,
) {
    // Check ambient light is reasonable
    if ambient.brightness < 0.0 {
        warn!(
            "[ENVIRONMENT DIAGNOSTIC] Ambient light has negative brightness: {}",
            ambient.brightness
        );
    }
    
    if ambient.brightness > 2.0 {
        warn!(
            "[ENVIRONMENT DIAGNOSTIC] Ambient light may be too bright: {}",
            ambient.brightness
        );
    }
}
```

---

## 7. Transform Hierarchy Validation

### 7.1 Transform vs GlobalTransform

| Component | Computed By | Purpose | Mutable |
|-----------|-------------|---------|---------|
| `Transform` | User/Systems | Local transform relative to parent | Yes |
| `GlobalTransform` | `TransformSystem` | World-space transform | No (Read-only) |

**Import Paths**:
- `Transform`: `bevy::transform::components::Transform`
- `GlobalTransform`: `bevy::transform::components::GlobalTransform`

### 7.2 Parent/Children Hierarchy Integrity

**Components**:
- `Parent`: `bevy::hierarchy::Parent`
- `Children`: `bevy::hierarchy::Children`

```rust
fn hierarchy_validation_system(
    query: Query<(Entity, Option<&Parent>, Option<&Children>)>,
    transform_query: Query<(Entity, &Transform, &GlobalTransform)>,
) {
    for (entity, parent, children) in query.iter() {
        // Validate parent exists
        if let Some(parent) = parent {
            if query.get(parent.get()).is_err() {
                error!(
                    "[HIERARCHY DIAGNOSTIC] Entity {:?} has orphaned Parent reference to {:?}",
                    entity, parent.get()
                );
            }
        }
        
        // Validate children exist
        if let Some(children) = children {
            for child in children.iter() {
                if query.get(*child).is_err() {
                    error!(
                        "[HIERARCHY DIAGNOSTIC] Entity {:?} has orphaned Children reference to {:?}",
                        entity, child
                    );
                }
            }
        }
    }
}
```

### 7.3 PropagateTransforms System Execution

**System Set**: `TransformSystem::TransformPropagate`

```rust
use bevy::transform::TransformSystem;

// Ensure your systems run after transform propagation
.systems(PostUpdate, (
    my_system.after(TransformSystem::TransformPropagate),
))
```

**Validation**:

```rust
fn transform_propagation_timing_validation(
    query: Query<(Entity, &Transform, &GlobalTransform), Changed<GlobalTransform>>,
) {
    let changed_count = query.iter().count();
    
    if changed_count == 0 {
        // This might be normal, or might indicate transform system didn't run
        trace!("[TRANSFORM DIAGNOSTIC] No GlobalTransforms changed this frame");
    } else {
        trace!(
            "[TRANSFORM DIAGNOSTIC] {} GlobalTransforms updated this frame",
            changed_count
        );
    }
}
```

### 7.4 NaN Transform Detection

```rust
fn nan_transform_validation_system(
    query: Query<(Entity, Option<&Name>, &Transform, &GlobalTransform)>,
) {
    let mut nan_count = 0;
    
    for (entity, name, local, global) in query.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        // Check local transform
        if local.translation.is_nan().any() {
            error!(
                "[TRANSFORM NAN] Entity {:?} ({}) has NaN in Transform.translation: {:?}",
                entity, name_str, local.translation
            );
            nan_count += 1;
        }
        
        if local.scale.is_nan().any() || local.scale.x == 0.0 || local.scale.y == 0.0 || local.scale.z == 0.0 {
            error!(
                "[TRANSFORM SCALE] Entity {:?} ({}) has invalid scale: {:?}",
                entity, name_str, local.scale
            );
        }
        
        // Check global transform
        if global.translation().is_nan().any() {
            error!(
                "[GLOBAL TRANSFORM NAN] Entity {:?} ({}) has NaN in GlobalTransform!",
                entity, name_str
            );
            nan_count += 1;
        }
        
        // Check for non-finite matrices
        let matrix = global.compute_matrix();
        if matrix.x_axis.is_nan().any() || 
           matrix.y_axis.is_nan().any() || 
           matrix.z_axis.is_nan().any() || 
           matrix.w_axis.is_nan().any() {
            error!(
                "[MATRIX NAN] Entity {:?} ({}) has NaN in transform matrix!",
                entity, name_str
            );
            nan_count += 1;
        }
    }
    
    if nan_count > 0 {
        warn!("[TRANSFORM DIAGNOSTIC] Found {} entities with NaN transforms", nan_count);
    }
}
```

---

## 8. Diagnostic Queries Reference

### 8.1 Finding All Cameras and Their Properties

```rust
use bevy::prelude::*;
use bevy::render::camera::{Camera, Projection, CameraRenderGraph};

fn find_all_cameras_system(
    cameras: Query<(Entity, &Camera, &Projection, &GlobalTransform, Option<&Order>)>,
) {
    info!("=== CAMERAS IN SCENE ===");
    
    for (entity, camera, projection, global, order) in cameras.iter() {
        let order_val = order.map(|o| o.0).unwrap_or(0);
        let pos = global.translation();
        
        info!("Camera entity: {:?}", entity);
        info!("  Active: {}", camera.is_active);
        info!("  Order: {}", order_val);
        info!("  Position: [{:.2}, {:.2}, {:.2}]", pos.x, pos.y, pos.z);
        info!("  Target: {:?}", camera.target);
        info!("  Viewport: {:?}", camera.viewport);
        
        match projection {
            Projection::Perspective(p) => {
                info!("  Type: Perspective");
                info!("    FOV: {:.2} rad ({:.1} deg)", p.fov, p.fov.to_degrees());
                info!("    Aspect: {:.3}", p.aspect_ratio);
                info!("    Near/Far: {:.2} / {:.2}", p.near, p.far);
            }
            Projection::Ortho(o) => {
                info!("  Type: Orthographic");
                info!("    LR BT: [{:.1}, {:.1}, {:.1}, {:.1}]", o.left, o.right, o.bottom, o.top);
                info!("    Near/Far: {:.2} / {:.2}", o.near, o.far);
            }
        }
        
        info!("---");
    }
}
```

### 8.2 Finding Entities with Visibility Issues

```rust
fn find_visibility_issues_system(
    entities: Query<(Entity, Option<&Name>, &Visibility, &InheritedVisibility, Option<&ViewVisibility>)>,
) {
    info!("=== VISIBILITY ISSUES ===");
    
    for (entity, name, visibility, inherited, view) in entities.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        let mut issues = Vec::new();
        
        // Check visibility state combinations
        match visibility {
            Visibility::Hidden => {
                issues.push("forced_hidden");
            }
            Visibility::Inherited if !inherited.get() => {
                issues.push("inherited_hidden");
            }
            _ => {}
        }
        
        // Check if visible but not view visible (culled)
        if let Some(view) = view {
            if visibility == Visibility::Visible && !view.get() {
                issues.push("visible_but_culled");
            }
        }
        
        // Check for missing ViewVisibility
        if view.is_none() {
            issues.push("missing_view_visibility");
        }
        
        if !issues.is_empty() {
            warn!(
                "Entity {:?} ({}): Issues: {:?}",
                entity, name_str, issues
            );
        }
    }
}
```

### 8.3 Finding Entities with Invalid Transforms

```rust
fn find_invalid_transforms_system(
    entities: Query<(Entity, Option<&Name>, &Transform, &GlobalTransform)>,
) {
    let mut issues = Vec::new();
    
    for (entity, name, local, global) in entities.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        
        // Check for NaN
        if local.translation.is_nan().any() {
            issues.push((entity, name_str, "NaN in translation"));
        }
        
        // Check for zero scale
        if local.scale.x == 0.0 || local.scale.y == 0.0 || local.scale.z == 0.0 {
            issues.push((entity, name_str, "Zero scale"));
        }
        
        // Check for negative scale (may cause face culling issues)
        if local.scale.x < 0.0 || local.scale.y < 0.0 || local.scale.z < 0.0 {
            issues.push((entity, name_str, "Negative scale"));
        }
        
        // Check GlobalTransform is reasonable
        if global.translation().is_nan().any() {
            issues.push((entity, name_str, "NaN in GlobalTransform"));
        }
    }
    
    if !issues.is_empty() {
        error!("=== TRANSFORM ISSUES ===");
        for (entity, name, issue) in issues {
            error!("Entity {:?} ({}): {}", entity, name, issue);
        }
    }
}
```

### 8.4 Finding Renderable Entities Without Proper Bundles

```rust
fn find_incomplete_renderable_entities_system(
    meshes: Query<(Entity, Option<&Name>, &Handle<Mesh>), Without<GlobalTransform>>,
    materials: Query<(Entity, Option<&Name>, &Handle<StandardMaterial>), Without<Mesh>>,
    no_visibility: Query<(Entity, Option<&Name>, &Handle<Mesh>), Without<Visibility>>,
    no_aabb: Query<(Entity, Option<&Name>, &Handle<Mesh>), Without<Aabb>>,
) {
    info!("=== RENDERABLE ENTITY VALIDATION ===");
    
    // Find meshes without GlobalTransform
    for (entity, name, _) in meshes.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        warn!(
            "Entity {:?} ({}) has Mesh but no GlobalTransform - won't render!",
            entity, name_str
        );
    }
    
    // Find materials without meshes
    for (entity, name, _) in materials.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        warn!(
            "Entity {:?} ({}) has Material but no Mesh - won't render!",
            entity, name_str
        );
    }
    
    // Find meshes without Visibility
    for (entity, name, _) in no_visibility.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        info!(
            "Entity {:?} ({}) missing Visibility - will be Visible by default",
            entity, name_str
        );
    }
    
    // Find meshes without AABB (can't be frustum culled)
    for (entity, name, _) in no_aabb.iter() {
        let name_str = name.map(|n| n.as_str()).unwrap_or("unnamed");
        warn!(
            "Entity {:?} ({}) has Mesh but no AABB - will always be culled!",
            entity, name_str
        );
    }
}
```

### 8.5 Complete Camera Analysis Query

```rust
fn comprehensive_camera_analysis_system(
    cameras: Query<(Entity, &Camera, &Projection, &GlobalTransform, Option<&Order>)>,
    query_frustum: Query<&Frustum>,
    windows: Query<&Window>,
    images: Res<Assets<Image>>,
) {
    info!("=== COMPREHENSIVE CAMERA ANALYSIS ===");
    
    for (entity, camera, projection, global, order) in cameras.iter() {
        info!("\nCamera Entity: {:?}", entity);
        
        // Basic info
        info!("  is_active: {}", camera.is_active);
        info!("  order: {:?}", order.map(|o| o.0));
        info!("  hdr: {:?}", camera.hdr);
        
        // Transform info
        let pos = global.translation();
        let forward = global.forward();
        let up = global.up();
        info!("  Position: [{:.2}, {:.2}, {:.2}]", pos.x, pos.y, pos.z);
        info!("  Forward:  [{:.2}, {:.2}, {:.2}]", forward.x, forward.y, forward.z);
        info!("  Up:       [{:.2}, {:.2}, {:.2}]", up.x, up.y, up.z);
        
        // Projection details
        match projection {
            Projection::Perspective(p) => {
                info!("  Projection: Perspective");
                info!("    FOV: {} rad ({} deg)", p.fov, p.fov.to_degrees());
                info!("    Aspect: {}", p.aspect_ratio);
                info!("    Near: {} Far: {}", p.near, p.far);
            }
            Projection::Ortho(o) => {
                info!("  Projection: Orthographic");
                info!("    Scale mode: {:?}", o.scaling_mode);
                info!("    Near: {} Far: {}", o.near, o.far);
            }
        }
        
        // Target info
        match &camera.target {
            RenderTarget::Window(window_ref) => {
                match window_ref {
                    WindowRef::Primary => info!("  Target: Primary Window"),
                    WindowRef::Entity(e) => info!("  Target: Window entity {:?}", e),
                }
            }
            RenderTarget::Image(handle) => {
                if let Some(img) = images.get(handle) {
                    info!("  Target: Image {}x{}", img.width(), img.height());
                } else {
                    warn!("  Target: Image (NOT LOADED)");
                }
            }
            _ => info!("  Target: {:?}", camera.target),
        }
        
        // Viewport
        if let Some(viewport) = &camera.viewport {
            info!("  Viewport: {:?}, size: {:?}", viewport, viewport.physical_size);
        } else {
            info!("  Viewport: Full target");
        }
        
        // Check for frustum
        if let Ok(frustum) = query_frustum.get(entity) {
            info!("  Frustum: Present (6 half-spaces)");
        } else {
            warn!("  Frustum: MISSING (will cause culling issues)");
        }
        
        // Computed values
        info!("  Computed: {:?}", camera.computed);
    }
    
    info!("\n--- END CAMERA ANALYSIS ---\n");
}
```

---

## Quick Reference: Bevy 0.13.2 Import Paths

| Component/Resource | Import Path |
|-------------------|-------------|
| `Camera` | `bevy::render::camera::Camera` |
| `Camera2d` | `bevy::core_pipeline::core_2d::Camera2d` |
| `Camera3d` | `bevy::core_pipeline::core_3d::Camera3d` |
| `Projection` | `bevy::render::camera::Projection` |
| `PerspectiveProjection` | `bevy::render::camera::PerspectiveProjection` |
| `OrthographicProjection` | `bevy::render::camera::OrthographicProjection` |
| `CameraRenderGraph` | `bevy::render::camera::CameraRenderGraph` |
| `Order` | `bevy::render::camera::CameraOrder` |
| `Transform` | `bevy::transform::components::Transform` |
| `GlobalTransform` | `bevy::transform::components::GlobalTransform` |
| `Visibility` | `bevy::render::view::Visibility` |
| `InheritedVisibility` | `bevy::render::view::InheritedVisibility` |
| `ViewVisibility` | `bevy::render::view::ViewVisibility` |
| `NoFrustumCulling` | `bevy::render::view::NoFrustumCulling` |
| `Frustum` | `bevy::render::primitives::Frustum` |
| `Aabb` | `bevy::render::primitives::Aabb` |
| `VisibleEntities` | `bevy::render::view::VisibleEntities` |
| `PointLight` | `bevy::pbr::PointLight` |
| `DirectionalLight` | `bevy::pbr::DirectionalLight` |
| `AmbientLight` | `bevy::pbr::AmbientLight` |
| `NotShadowCaster` | `bevy::pbr::NotShadowCaster` |
| `NotShadowReceiver` | `bevy::pbr::NotShadowReceiver` |
| `ClearColor` | `bevy::core_pipeline::clear_color::ClearColor` |
| `TransformSystem` | `bevy::transform::TransformSystem` |
| `VisibilitySystems` | `bevy::render::view::VisibilitySystems` |
| `Window` | `bevy::window::Window` |
| `PrimaryWindow` | `bevy::window::PrimaryWindow` |
| `Parent` | `bevy::hierarchy::Parent` |
| `Children` | `bevy::hierarchy::Children` |

---

*Document Version: 1.0*  
*Bevy Version: 0.13.2*  
*Compatible with: Rose Online Client codebase*
