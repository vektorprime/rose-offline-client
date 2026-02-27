# Bevy 0.16 to 0.18 Upgrade Plan

## Executive Summary

This document outlines a comprehensive plan for upgrading the ROSE Offline Client from Bevy 0.16 to Bevy 0.18. This upgrade spans two major versions and includes significant breaking changes across rendering, ECS, events, animation, and UI systems.

**Estimated Complexity:** High
**Key Risk Areas:** Custom materials/shaders, Event system, Third-party dependencies

---

## Phase 1: Pre-Upgrade Preparation

### 1.1 Third-Party Dependency Compatibility Check

Before starting the upgrade, verify compatibility of all Bevy-related dependencies:

| Dependency | Current Version | Required Action |
|------------|-----------------|-----------------|
| `bevy_egui` | 0.34 | Check for 0.18 compatible version |
| `bevy-inspector-egui` | 0.33 | Check for 0.18 compatible version |
| `bevy_rapier3d` | 0.31 | Check for 0.18 compatible version |

**Action Items:**
- [ ] Check crates.io for latest compatible versions
- [ ] Check each dependency's GitHub issues for Bevy 0.18 compatibility
- [ ] Document any blocking dependencies

### 1.2 Create Backup Branch

```bash
git checkout -b backup/pre-bevy-0.18-upgrade
git push origin backup/pre-bevy-0.18-upgrade
git checkout main
git checkout -b feature/bevy-0.18-upgrade
```

### 1.3 Build and Test Baseline

- [ ] Ensure current code compiles cleanly on Bevy 0.16
- [ ] Run all functionality and document any existing issues
- [ ] Take screenshots of current rendering for comparison

---

## Phase 2: Intermediate Upgrade to Bevy 0.17

It is **strongly recommended** to upgrade to 0.17 first, verify everything works, then upgrade to 0.18. This makes debugging easier.

### 2.1 Cargo.toml Changes for 0.17

```toml
[dependencies.bevy]
version = "0.17"
default-features = false
features = [
    "std",
    "async_executor",
    "bevy_log",
    "bevy_asset",
    "bevy_winit",
    "bevy_core_pipeline",
    "bevy_pbr",
    "bevy_render",
    "bevy_state",
    "multi_threaded",
    "tga",
    "x11",
    "bevy_gizmos",
    # NEW: Required for web builds if targeting wasm
    # "web",
]
```

### 2.2 Critical 0.17 Breaking Changes

#### 2.2.1 Event → Message Rename (HIGH IMPACT)

The biggest change in 0.17 is the renaming of "buffered events" to "messages":

| Old (0.16) | New (0.17) |
|------------|------------|
| `Event` trait (buffered) | `Message` trait |
| `EventWriter<E>` | `MessageWriter<M>` |
| `EventReader<E>` | `MessageReader<M>` |
| `Events<E>` | `Messages<M>` |
| `add_event::<E>()` | `add_message::<M>()` |
| `send_event()` | `write_message()` |

**Files to Update:**
- [`src/events/mod.rs`](src/events/mod.rs) - All event definitions
- [`src/lib.rs`](src/lib.rs:955-984) - All `app.add_event::<T>()` calls
- All systems using `EventWriter`/`EventReader`

**Migration Pattern:**
```rust
// 0.16
#[derive(Event)]
struct MyEvent;

fn my_system(mut writer: EventWriter<MyEvent>) {
    writer.send(MyEvent);
}

// 0.17
#[derive(Message)]
struct MyMessage;

fn my_system(mut writer: MessageWriter<MyMessage>) {
    writer.write(MyMessage);
}
```

**IMPORTANT:** The `Event` trait still exists in 0.17 but is now exclusively for "observable events" used with observers.

#### 2.2.2 Observer/Event API Changes (HIGH IMPACT)

Observer syntax has changed significantly:

```rust
// 0.16
commands.add_observer(|trigger: Trigger<OnAdd, Player>| {
    info!("Spawned player {}", trigger.target());
});

// 0.17
commands.add_observer(|add: On<Add, Player>| {
    info!("Spawned player {}", add.entity);
});
```

Lifecycle events renamed:
- `OnAdd` → `Add`
- `OnInsert` → `Insert`
- `OnRemove` → `Remove`
- `OnDespawn` → `Despawn`

#### 2.2.3 Render Crate Reorganization (HIGH IMPACT)

Many types moved to new crates:

| Old Location | New Location |
|--------------|--------------|
| `bevy_render::Camera` | `bevy_camera::Camera` |
| `bevy_render::Mesh` | `bevy_mesh::Mesh` |
| `bevy_render::Image` | `bevy_image::Image` |
| `bevy_core_pipeline::Bloom` | `bevy_post_process::Bloom` |
| `bevy_core_pipeline::Smaa` | `bevy_anti_alias::Smaa` |
| `bevy_core_pipeline::DepthOfField` | `bevy_post_process::DepthOfField` |

**Files to Update:**
- [`src/lib.rs`](src/lib.rs:6-31) - Import statements
- [`src/render/mod.rs`](src/render/mod.rs)
- All material files in [`src/render/`](src/render/)

#### 2.2.4 System Sets Renamed (MEDIUM IMPACT)

All system sets now use `*Systems` suffix:

| Old | New |
|-----|-----|
| `TransformSystem` | `TransformSystems` |
| `RenderSet` | `RenderSystems` |
| `UiSystem` | `UiSystems` |

#### 2.2.5 Window Split into Components

`CursorOptions` is now a separate component:

```rust
// 0.16
fn lock_cursor(window: Single<&mut Window, With<PrimaryWindow>>) {
    window.cursor_options.grab_mode = CursorGrabMode::Locked;
}

// 0.17
fn lock_cursor(cursor: Single<&mut CursorOptions, With<PrimaryWindow>>) {
    cursor.grab_mode = CursorGrabMode::Locked;
}
```

#### 2.2.6 wgpu 25 Bind Group Changes (HIGH IMPACT for Custom Shaders)

Bind group indices have changed:
- `@group(2)` for materials → `@group(#{MATERIAL_BIND_GROUP})`

**All shader files need updating:**
- [`src/render/shaders/*.wgsl`](src/render/shaders/)

```wgsl
// 0.16
@group(2) @binding(0) var<uniform> my_uniform: vec4<f32>;

// 0.17
#import bevy_pbr::forward_io::MATERIAL_BIND_GROUP
@group(MATERIAL_BIND_GROUP) @binding(0) var<uniform> my_uniform: vec4<f32>;
```

#### 2.2.7 Material Plugin Changes

`MaterialPlugin` fields changed:

```rust
// 0.16
MaterialPlugin::<MyMaterial> {
    prepass_enabled: false,
    shadows_enabled: false,
}

// 0.17 - These are now methods on the Material trait
impl Material for MyMaterial {
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }
}
```

**Files to Update:**
- [`src/render/particle_material.rs`](src/render/particle_material.rs:135-139)
- [`src/render/sky_material.rs`](src/render/sky_material.rs)
- All material plugins

#### 2.2.8 SpecializedRenderPipeline Replaced

The `SpecializedRenderPipeline` trait has been replaced with `Specializer`:

**Files to Update:**
- Any custom pipeline specialization in render module

### 2.3 Files Requiring Updates for 0.17

#### Core Files
- [ ] [`Cargo.toml`](Cargo.toml) - Version bump
- [ ] [`src/lib.rs`](src/lib.rs) - Imports, event registration, system set names
- [ ] [`src/main.rs`](src/main.rs) - Minimal changes expected

#### Rendering System
- [ ] [`src/render/mod.rs`](src/render/mod.rs)
- [ ] [`src/render/particle_material.rs`](src/render/particle_material.rs)
- [ ] [`src/render/terrain_material.rs`](src/render/terrain_material.rs)
- [ ] [`src/render/water_material.rs`](src/render/water_material.rs)
- [ ] [`src/render/sky_material.rs`](src/render/sky_material.rs)
- [ ] [`src/render/cartoon_sky_material.rs`](src/render/cartoon_sky_material.rs)
- [ ] [`src/render/damage_digit_material.rs`](src/render/damage_digit_material.rs)
- [ ] [`src/render/wing_material.rs`](src/render/wing_material.rs)
- [ ] [`src/render/underwater_effect.rs`](src/render/underwater_effect.rs)
- [ ] [`src/render/zone_lighting.rs`](src/render/zone_lighting.rs)
- [ ] [`src/render/trail_effect.rs`](src/render/trail_effect.rs)
- [ ] [`src/render/world_ui.rs`](src/render/world_ui.rs)
- [ ] [`src/render/extension_material_plugin.rs`](src/render/extension_material_plugin.rs)
- [ ] [`src/render/object_material_extension.rs`](src/render/object_material_extension.rs)
- [ ] [`src/render/terrain_material_extension.rs`](src/render/terrain_material_extension.rs)
- [ ] [`src/render/water_material_extension.rs`](src/render/water_material_extension.rs)
- [ ] [`src/render/effect_mesh_extension.rs`](src/render/effect_mesh_extension.rs)
- [ ] [`src/render/skinned_mesh_fix.rs`](src/render/skinned_mesh_fix.rs)

#### All Shader Files
- [ ] [`src/render/shaders/particle.wgsl`](src/render/shaders/particle.wgsl)
- [ ] [`src/render/shaders/terrain_material.wgsl`](src/render/shaders/terrain_material.wgsl)
- [ ] [`src/render/shaders/water_material.wgsl`](src/render/shaders/water_material.wgsl)
- [ ] [`src/render/shaders/sky_material.wgsl`](src/render/shaders/sky_material.wgsl)
- [ ] [`src/render/shaders/cartoon_sky.wgsl`](src/render/shaders/cartoon_sky.wgsl)
- [ ] [`src/render/shaders/damage_digit.wgsl`](src/render/shaders/damage_digit.wgsl)
- [ ] [`src/render/shaders/wing_material.wgsl`](src/render/shaders/wing_material.wgsl)
- [ ] [`src/render/shaders/underwater_effect.wgsl`](src/render/shaders/underwater_effect.wgsl)
- [ ] [`src/render/shaders/zone_lighting.wgsl`](src/render/shaders/zone_lighting.wgsl)
- [ ] [`src/render/shaders/trail_effect.wgsl`](src/render/shaders/trail_effect.wgsl)
- [ ] [`src/render/shaders/world_ui.wgsl`](src/render/shaders/world_ui.wgsl)
- [ ] [`src/render/shaders/rose_object_extension.wgsl`](src/render/shaders/rose_object_extension.wgsl)
- [ ] [`src/render/shaders/rose_terrain_extension.wgsl`](src/render/shaders/rose_terrain_extension.wgsl)
- [ ] [`src/render/shaders/rose_water_extension.wgsl`](src/render/shaders/rose_water_extension.wgsl)
- [ ] [`src/render/shaders/rose_effect_extension.wgsl`](src/render/shaders/rose_effect_extension.wgsl)
- [ ] [`src/render/shaders/post_processing.wgsl`](src/render/shaders/post_processing.wgsl)
- [ ] [`src/render/shaders/particle_prepass.wgsl`](src/render/shaders/particle_prepass.wgsl)

#### Events System
- [ ] [`src/events/mod.rs`](src/events/mod.rs)
- [ ] All event files in [`src/events/`](src/events/)

#### Animation System
- [ ] [`src/animation/mod.rs`](src/animation/mod.rs)
- [ ] [`src/animation/animation_state.rs`](src/animation/animation_state.rs)
- [ ] [`src/animation/skeletal_animation.rs`](src/animation/skeletal_animation.rs)
- [ ] [`src/animation/zmo_asset_loader.rs`](src/animation/zmo_asset_loader.rs)
- [ ] [`src/animation/zmo_asset_loader_fixed.rs`](src/animation/zmo_asset_loader_fixed.rs)

#### Other Systems
- [ ] [`src/audio/mod.rs`](src/audio/mod.rs)
- [ ] [`src/zone_loader.rs`](src/zone_loader.rs)
- [ ] [`src/model_loader.rs`](src/model_loader.rs)
- [ ] [`src/dds_image_loader.rs`](src/dds_image_loader.rs)
- [ ] [`src/zms_asset_loader.rs`](src/zms_asset_loader.rs)
- [ ] [`src/vfs_asset_io.rs`](src/vfs_asset_io.rs)
- [ ] [`src/effect_loader.rs`](src/effect_loader.rs)

---

## Phase 3: Upgrade to Bevy 0.18

After successfully upgrading to 0.17 and verifying functionality, proceed to 0.18.

### 3.1 Cargo.toml Changes for 0.18

```toml
[dependencies.bevy]
version = "0.18"
default-features = false
features = [
    "std",
    "async_executor",
    "bevy_log",
    "bevy_asset",
    "bevy_winit",
    "bevy_core_pipeline",
    "bevy_pbr",
    "bevy_render",
    "bevy_state",
    "multi_threaded",
    "tga",
    "x11",
    "bevy_gizmos",
]
```

### 3.2 Critical 0.18 Breaking Changes

#### 3.2.1 RenderTarget is Now a Component

```rust
// 0.17
commands.spawn((
    Camera3d::default(),
    Camera {
        target: RenderTarget::Image(image_handle.into()),
        ..default()
    },
));

// 0.18
commands.spawn((
    Camera3d::default(),
    RenderTarget::Image(image_handle.into()),
));
```

**Files to Update:**
- [`src/lib.rs`](src/lib.rs:1753-1786) - Camera spawning code

#### 3.2.2 AnimationTarget Split

```rust
// 0.17
entity.insert(AnimationTarget { id: AnimationTargetId(id), player: player_entity });

// 0.18
entity.insert((AnimationTargetId(id), AnimatedBy(player_entity)));
```

**Files to Update:**
- [`src/animation/`](src/animation/) - Animation systems

#### 3.2.3 AmbientLight Split

```rust
// 0.17 - Resource
app.insert_resource(AmbientLight { ... });

// 0.18 - Use GlobalAmbientLight for resource
app.insert_resource(GlobalAmbientLight { ... });

// AmbientLight is now a component for per-camera override
```

**Files to Update:**
- [`src/lib.rs`](src/lib.rs) - If using AmbientLight resource
- [`src/render/zone_lighting.rs`](src/render/zone_lighting.rs)

#### 3.2.4 Material enable_prepass/enable_shadows Methods

```rust
// 0.17 - MaterialPlugin fields
MaterialPlugin::<MyMaterial> {
    prepass_enabled: false,
    shadows_enabled: false,
}

// 0.18 - Material trait methods
impl Material for MyMaterial {
    fn enable_prepass() -> bool { false }
    fn enable_shadows() -> bool { false }
}
```

#### 3.2.5 BorderRect Changes

```rust
// 0.17
BorderRect {
    left: 10.0,
    right: 10.0,
    top: 5.0,
    bottom: 5.0,
}

// 0.18
BorderRect {
    min_inset: Vec2::new(10.0, 5.0),  // left, bottom
    max_inset: Vec2::new(10.0, 5.0),  // right, top
}
```

#### 3.2.6 Same State Transitions

```rust
// 0.17 - Setting same state does nothing
next_state.set(State::Menu);

// 0.18 - Setting same state triggers transitions
next_state.set(State::Menu);
// Use set_if_neq for old behavior
next_state.set_if_neq(State::Menu);
```

#### 3.2.7 BindGroupLayoutDescriptor Changes

```rust
// 0.17
let bind_group_layout = render_device.create_bind_group_layout(...);

// 0.18
let bind_group_layout = BindGroupLayoutDescriptor::new(...);
```

**Files to Update:**
- All custom material implementations

#### 3.2.8 Entity API Changes

Entity index methods renamed:
- `Entity::row()` → `Entity::index()`
- `Entity::from_row()` → `Entity::from_index()`
- `EntityRow` → `EntityIndex`

#### 3.2.9 Internal Component Removed

The `Internal` component has been removed. Remove any `Allow<Internal>` filters from queries.

### 3.3 Files Requiring Updates for 0.18

All files from 0.17 migration plus:

- [ ] Camera spawning code in [`src/lib.rs`](src/lib.rs)
- [ ] Animation target code in [`src/animation/`](src/animation/)
- [ ] Any BorderRect usage
- [ ] Any Entity index/row usage
- [ ] BindGroupLayout creation in materials

---

## Phase 4: Third-Party Dependency Updates

### 4.1 bevy_egui Update

Check for Bevy 0.18 compatible version and update Cargo.toml:
```toml
bevy_egui = "0.XX"  # Check crates.io for latest compatible
```

### 4.2 bevy-inspector-egui Update

```toml
bevy-inspector-egui = "0.XX"  # Check crates.io
```

### 4.3 bevy_rapier3d Update

```toml
bevy_rapier3d = "0.XX"  # Check crates.io
```

**Note:** Rapier physics may have its own breaking changes.

---

## Phase 5: Testing & Validation

### 5.1 Compilation Test

After each phase:
1. Run `cargo check` to identify compilation errors
2. Fix all errors before proceeding
3. Run `cargo build` for full build test

### 5.2 Runtime Testing Checklist

#### Core Functionality
- [ ] Game launches without crashes
- [ ] Main menu displays correctly
- [ ] Login functionality works
- [ ] Character selection works
- [ ] Zone loading works

#### Rendering
- [ ] Terrain renders correctly
- [ ] Water renders with animations
- [ ] Sky renders correctly
- [ ] Character models render
- [ ] Particle effects render
- [ ] Shadows work correctly
- [ ] Post-processing effects work
- [ ] Underwater effect works

#### Animation
- [ ] Character animations play
- [ ] Skeletal animation works
- [ ] Camera animations work

#### Audio
- [ ] Background music plays
- [ ] Sound effects play
- [ ] Spatial audio works

#### UI
- [ ] All dialogs display correctly
- [ ] Egui windows work
- [ ] Tooltips appear

#### Physics
- [ ] Collision detection works
- [ ] Player movement is correct

### 5.3 Performance Comparison

- [ ] Compare frame rates before/after
- [ ] Check for any new stutters
- [ ] Monitor memory usage

---

## Risk Assessment

### High Risk Areas

1. **Custom Materials/Shaders** - Bind group changes may require significant shader rewrites
2. **Event System** - Complete rename to Message system affects many files
3. **Third-party Dependencies** - May not have 0.18 compatible versions yet

### Medium Risk Areas

1. **Animation System** - AnimationTarget split requires code changes
2. **Camera System** - RenderTarget component split
3. **System Set Names** - Many renames throughout codebase

### Low Risk Areas

1. **Core ECS** - Mostly compatible with additions
2. **Basic Transforms** - Largely unchanged
3. **Asset System** - Mostly compatible

---

## Rollback Plan

If critical issues are encountered:

1. **Git Revert:**
   ```bash
   git checkout main
   git branch -D feature/bevy-0.18-upgrade
   git checkout -b feature/bevy-0.18-upgrade-retry
   ```

2. **Restore Cargo.toml:**
   ```bash
   git checkout main -- Cargo.toml
   git checkout main -- Cargo.lock
   ```

3. **Document Issues:**
   - Create GitHub issue documenting blocking problems
   - Note which dependencies are incompatible

---

## Timeline Recommendation

| Phase | Description | Complexity |
|-------|-------------|------------|
| 1 | Preparation | Low |
| 2 | Upgrade to 0.17 | High |
| 2.5 | Test 0.17 | Medium |
| 3 | Upgrade to 0.18 | Medium-High |
| 3.5 | Test 0.18 | Medium |
| 4 | Dependencies | Medium |
| 5 | Final Testing | Medium |

---

## Notes

- **Do not skip 0.17** - Upgrading directly from 0.16 to 0.18 will make debugging significantly harder
- **Commit frequently** - Make small, atomic commits during migration
- **Test incrementally** - Don't try to fix everything at once
- **Keep migration guides open** - Reference the official guides constantly
