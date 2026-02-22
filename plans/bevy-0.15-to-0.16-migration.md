# Bevy 0.15 to 0.16 Migration Plan

## Executive Summary

This document outlines the migration strategy for upgrading Rose Offline Client from Bevy 0.15 to Bevy 0.16. The migration involves several breaking changes across ECS, rendering, audio, and UI systems.

**Key Impact Areas:**
- **HIGH**: ECS error handling changes (Query::single() returns Result)
- **HIGH**: Handle::weak_from_u128() deprecation (used in material plugins)
- **MEDIUM**: Parent → ChildOf rename affecting hierarchy code
- **MEDIUM**: Audio API changes (if using Bevy audio)
- **LOW**: UI component renames (we use egui primarily)

---

## Phase 1: Preparation

### 1.1 Update Cargo.toml

**File:** [`Cargo.toml`](Cargo.toml:59-77)

```toml
# Before (0.15)
[dependencies.bevy]
version = "0.15"
default-features = false
features = [
    "bevy_asset",
    "bevy_winit",
    # ... other features
]

# After (0.16)
[dependencies.bevy]
version = "0.16"
default-features = false
features = [
    "bevy_asset",
    "bevy_winit",
    "std",              # NEW: Required when default-features = false
    "async_executor",   # NEW: Required when default-features = false
    # ... other features
]
```

**Risk:** Low - Straightforward version bump with required feature additions

### 1.2 Update Edition to 2024 (Optional but Recommended)

**File:** [`Cargo.toml`](Cargo.toml:4)

```toml
# Before
edition = "2021"

# After
edition = "2024"
```

**Note:** Bevy 0.16 uses Rust Edition 2024. While not strictly required, it's recommended for RPIT lifetime changes.

---

## Phase 2: Critical ECS Changes

### 2.1 Query::single() Returns Result

**Impact:** HIGH - Many systems use Query::single()

The following methods now return `Result` instead of panicking:
- `Query::single()`
- `Query::single_mut()`
- `QueryState::single()`
- `QueryState::single_mut()`

**Files to Update:**
- [`src/lib.rs`](src/lib.rs:1377-1392) - Diagnostic systems use `get_single()`
- [`src/render/world_ui.rs`](src/render/world_ui.rs) - Likely uses single() queries
- Various UI systems

**Migration Pattern:**

```rust
// Before (0.15)
fn my_system(query: Single<&Transform, With<Camera>>) {
    let transform = query.into_inner();
}

// After (0.16) - Option 1: Use Result with ? operator
fn my_system(query: Single<&Transform, With<Camera>>) -> Result {
    let transform = query.into_inner();
    Ok(())
}

// After (0.16) - Option 2: Handle explicitly
fn my_system(query: Query<&Transform, With<Camera>>) {
    let Ok(transform) = query.single() else {
        return;
    };
}
```

**Specific Changes in Our Code:**

File: [`src/lib.rs`](src/lib.rs:1377-1392)
```rust
// Before
app.add_systems(Update, |windows: Query<&EguiContext, With<Window>>| {
    if let Ok(_context) = windows.get_single() {
        // ...
    }
});

// After - get_single() is deprecated, use single()
app.add_systems(Update, |windows: Query<&EguiContext, With<Window>>| {
    if let Ok(_context) = windows.single() {
        // ...
    }
});
```

### 2.2 Parent Component Renamed to ChildOf

**Impact:** MEDIUM - Affects hierarchy queries

**Files to Check:**
- [`src/model_loader.rs`](src/model_loader.rs) - Entity hierarchy creation
- [`src/zone_loader.rs`](src/zone_loader.rs) - Zone entity hierarchies
- Animation systems with bone hierarchies

**Migration Pattern:**

```rust
// Before (0.15)
use bevy::prelude::Parent;

fn system(query: Query<&Parent>) {
    let parent = *parent_component;
}

// After (0.16)
use bevy::prelude::ChildOf;

fn system(query: Query<&ChildOf>) {
    let parent = child_of.parent();
}
```

**Despawning Changes:**

```rust
// Before (0.15)
commands.entity(parent).despawn_recursive();
commands.entity(parent).despawn_descendants();

// After (0.16)
commands.entity(parent).despawn();  // Now despawns children by default
commands.entity(parent).despawn_related::<Children>();  // Despawn children only
```

### 2.3 EventWriter::send() Renamed to write()

**Impact:** LOW - Simple rename

**Files to Update:**
- All files using `EventWriter::send()` methods

```rust
// Before (0.15)
event_writer.send(MyEvent::new());
event_writer.send_batch(events);

// After (0.16)
event_writer.write(MyEvent::new());
event_writer.write_batch(events);
```

### 2.4 Event: Component Trait Bound Removed

**Impact:** LOW - Events no longer require Component trait

**Files:** [`src/events/`](src/events/) - All event definitions

```rust
// Before (0.15) - Event automatically had Component
#[derive(Event)]
struct MyEvent;

// After (0.16) - If you need Component, add it explicitly
#[derive(Event, Component)]  // Only if you need Component
struct MyEvent;
```

### 2.5 apply_deferred() Changes

**Impact:** LOW - Mostly internal

**File:** [`src/lib.rs`](src/lib.rs:849-857)

```rust
// Before (0.15)
app.add_systems(PostUpdate, apply_deferred);

// After (0.16) - apply_deferred is now a ZST type
use bevy::ecs::schedule::ApplyDeferred;
app.add_systems(PostUpdate, ApplyDeferred);
```

---

## Phase 3: Rendering Changes

### 3.1 Handle::weak_from_u128() Deprecated

**Impact:** HIGH - Used extensively in material plugins

**Files to Update:**
- [`src/render/extension_material_plugin.rs`](src/render/extension_material_plugin.rs:27-37)

```rust
// Before (0.15)
pub const ROSE_OBJECT_EXTENSION_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(0x8a1b2c3d4e5f6789);

// After (0.16)
use bevy::asset::weak_handle;

pub const ROSE_OBJECT_EXTENSION_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("8a1b2c3d-4e5f-6789-abcd-ef1234567890");
```

**All Shader Handles to Update:**
1. `ROSE_OBJECT_EXTENSION_SHADER_HANDLE` (0x8a1b2c3d4e5f6789)
2. `ROSE_TERRAIN_EXTENSION_SHADER_HANDLE` (0x9b2c3d4e5f6a7890)
3. `ROSE_WATER_EXTENSION_SHADER_HANDLE` (0xac3d4e5f6a7b89c1)
4. `ROSE_EFFECT_EXTENSION_SHADER_HANDLE` (0xbd4e5f6a7b8c9d2e)

**UUID Conversion Required:**
Convert hex values to UUID format: `xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx`

### 3.2 MaterialExtension Changes

**Impact:** MEDIUM - Verify MaterialExtension trait still works

**Files:**
- [`src/render/object_material_extension.rs`](src/render/object_material_extension.rs)
- [`src/render/terrain_material_extension.rs`](src/render/terrain_material_extension.rs)
- [`src/render/water_material_extension.rs`](src/render/water_material_extension.rs)
- [`src/render/effect_mesh_extension.rs`](src/render/effect_mesh_extension.rs)

**Key Changes:**
- `AsBindGroup::unprepared_bind_group` is now unconditionally called
- Return `AsBindGroupError::CreateBindGroupDirectly` to fall back to `as_bind_group`

```rust
// In MaterialExtension implementations, ensure this handles the new behavior
impl MaterialExtension for RoseObjectExtension {
    fn fragment_shader() -> ShaderRef {
        // ... existing code
    }
    
    // If you implement unprepared_bind_group, handle the new error case
}
```

### 3.3 GpuImage::size Type Change

**Impact:** LOW - Only if directly accessing GpuImage

```rust
// Before (0.15)
let size: UVec2 = gpu_image.size;

// After (0.16)
let size: Extent3d = gpu_image.size;
let size_2d: UVec2 = gpu_image.size.size_2d();  // Helper method
```

### 3.4 Anti-Aliasing Moved

**Impact:** LOW - If using SMAA/TAA

```rust
// Before (0.15)
use bevy::core_pipeline::smaa::SmaaSettings;

// After (0.16)
use bevy::anti_aliasing::SmaaSettings;
```

### 3.5 Projection Changes

**Impact:** LOW - PerspectiveProjection no longer a component

**File:** [`src/lib.rs`](src/lib.rs:1555-1560)

```rust
// Before (0.15)
Projection::Perspective(PerspectiveProjection {
    fov: std::f32::consts::PI / 4.0,
    near: 0.1,
    far: 50000.0,
    aspect_ratio: 16.0 / 9.0,
})

// After (0.16) - Same code works, but custom projections changed
// If using custom projections, use Projection::custom()
```

---

## Phase 4: Audio Changes

### 4.1 Note: We Use Custom Audio (Oddio)

**Impact:** MINIMAL - We use a custom audio plugin with `oddio` and `cpal`

**File:** [`src/audio/mod.rs`](src/audio/mod.rs)

Our custom `OddioPlugin` doesn't use Bevy's built-in audio, so most audio changes don't affect us. However, if we add Bevy audio later:

```rust
// Bevy 0.16 Audio Changes (for reference)

// Volume is now an enum
let volume = Volume::Linear(1.0);  // Instead of Volume(1.0)
let volume = Volume::Decibels(0.0);

// toggle() renamed
sink.toggle_playback();  // Instead of toggle()

// set_volume now requires &mut
fn system(mut sink: Single<&mut AudioSink>) {
    sink.set_volume(Volume::Linear(0.5));
}
```

---

## Phase 5: UI Changes

### 5.1 UiImage Renamed to ImageNode

**Impact:** LOW - We primarily use egui

**Files to Check:**
- [`src/ui/`](src/ui/) directory for any Bevy UI usage

```rust
// Before (0.15)
use bevy::ui::UiImage;

// After (0.16)
use bevy::ui::ImageNode;
```

### 5.2 TargetCamera Renamed

**Impact:** LOW - If using multiple cameras with UI

```rust
// Before (0.15)
use bevy::ui::TargetCamera;

// After (0.16)
use bevy::ui::UiTargetCamera;
```

---

## Phase 6: Utility/Import Changes

### 6.1 bevy_utils Refactored

**Impact:** MEDIUM - Many items moved to bevy_platform

**Common Changes:**

```rust
// Before (0.15)
use bevy_utils::HashMap;
use bevy_utils::HashSet;
use bevy_utils::Instant;

// After (0.16)
use bevy::platform_support::collections::HashMap;
use bevy::platform_support::collections::HashSet;
use bevy::platform_support::time::Instant;

// Or use standard library
use std::collections::HashMap;
use std::collections::HashSet;
use std::time::Instant;
```

### 6.2 NonSendMarker Moved

**Impact:** LOW - If using NonSendMarker

```rust
// Before (0.15)
use bevy::core::NonSendMarker;

// After (0.16)
use bevy::ecs::system::NonSendMarker;
```

### 6.3 Name Component Moved

**Impact:** LOW - If using Name component directly

```rust
// Before (0.15)
use bevy::core::Name;

// After (0.16)
use bevy::ecs::name::Name;
```

---

## Phase 7: Third-Party Crate Updates

### 7.1 bevy_egui Update Required

**Current Version:** 0.32
**Required:** Check for 0.16 compatible version

```toml
# Check crates.io for bevy_egui version compatible with Bevy 0.16
[dependencies]
bevy_egui = "0.XX"  # Update to 0.16 compatible version
```

### 7.2 bevy-inspector-egui Update Required

**Current Version:** 0.29
**Required:** Check for 0.16 compatible version

### 7.3 bevy_rapier3d Update Required

**Current Version:** 0.28
**Required:** Check for 0.16 compatible version

---

## Migration Order

### Recommended Sequence

1. **Update Cargo.toml** (Phase 1)
   - Update Bevy version
   - Add `std` and `async_executor` features
   - Update third-party crate versions

2. **Fix Compilation Errors** (Phase 2-3)
   - Handle::weak_from_u128() → weak_handle!()
   - Query::single() Result handling
   - Parent → ChildOf changes
   - apply_deferred changes

3. **Fix Deprecation Warnings** (Phase 2-6)
   - EventWriter::send() → write()
   - Import path updates
   - Type renames

4. **Testing** (Phase 8)
   - Run application
   - Test all major features
   - Verify rendering works
   - Verify audio works

---

## Risk Assessment

| Risk Area | Severity | Likelihood | Mitigation |
|-----------|----------|------------|------------|
| Query::single() panics | High | High | Search and replace all occurrences |
| Shader handle migration | High | High | Convert UUIDs carefully |
| Third-party crate incompatibility | High | Medium | Check crate compatibility before starting |
| MaterialExtension changes | Medium | Medium | Test all custom materials |
| Hierarchy/Parent changes | Medium | Low | Search for Parent usage |
| Import path changes | Low | High | Compiler will catch these |

---

## Testing Checklist

After migration, verify:

- [ ] Application compiles without errors
- [ ] Application starts without panics
- [ ] Zone loading works
- [ ] Character rendering works
- [ ] Terrain rendering works
- [ ] Water rendering works
- [ ] Particle effects work
- [ ] Animation system works
- [ ] Audio playback works
- [ ] UI (egui) works
- [ ] Physics (Rapier) works
- [ ] Network functionality works

---

## Rollback Plan

If migration fails:

1. Revert Cargo.toml to Bevy 0.15
2. Revert third-party crate versions
3. Use `git checkout` to restore changed files
4. Document issues encountered for future attempt

---

## References

- [Bevy 0.15 to 0.16 Migration Guide](https://bevy.org/learn/migration-guides/0-15-to-0-16/)
- [Bevy 0.16 Release Notes](https://bevy.org/news/)
- [bevy_egui on crates.io](https://crates.io/crates/bevy_egui)
- [bevy_rapier3d on crates.io](https://crates.io/crates/bevy_rapier3d)

---

## Appendix A: Files Requiring Changes

### High Priority (Breaking Changes)

| File | Change Type | Phase |
|------|-------------|-------|
| `Cargo.toml` | Version update | 1 |
| `src/render/extension_material_plugin.rs` | weak_handle! macro | 3 |
| `src/lib.rs` | Query handling, apply_deferred | 2 |

### Medium Priority (May Break)

| File | Change Type | Phase |
|------|-------------|-------|
| `src/model_loader.rs` | Parent → ChildOf | 2 |
| `src/zone_loader.rs` | Parent → ChildOf | 2 |
| `src/render/*.rs` | MaterialExtension verify | 3 |
| `src/events/*.rs` | Event trait bound | 2 |

### Low Priority (Deprecations)

| File | Change Type | Phase |
|------|-------------|-------|
| All files with EventWriter | send() → write() | 2 |
| Files using bevy_utils | Import updates | 6 |

---

## Appendix B: UUID Conversion Table

Convert shader handle hex values to UUID format:

| Old Hex Value | New UUID Format |
|---------------|-----------------|
| `0x8a1b2c3d4e5f6789` | `8a1b2c3d-4e5f-6789-????-????????????` |
| `0x9b2c3d4e5f6a7890` | `9b2c3d4e-5f6a-7890-????-????????????` |
| `0xac3d4e5f6a7b89c1` | `ac3d4e5f-6a7b-89c1-????-????????????` |
| `0xbd4e5f6a7b8c9d2e` | `bd4e5f6a-7b8c-9d2e-????-????????????` |

**Note:** These need proper UUID v4 format. Consider generating new UUIDs with `uuid::Uuid::new_v4()`.

---

## Appendix C: Quick Reference Commands

```bash
# Search for Query::single usage
rg "\.single\(\)" --type rust

# Search for Parent usage
rg "Parent" --type rust src/

# Search for EventWriter::send usage
rg "\.send\(" --type rust src/

# Search for weak_from_u128
rg "weak_from_u128" --type rust

# Build and check for errors
cargo build 2>&1 | head -100
```
