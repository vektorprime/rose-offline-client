# Model Viewer Pitfalls

This document records model viewer-related issues encountered during development.

---

## Model Viewer Crash - Duplicate Component in Bundle (Fixed 2026-02-24)

### Problem
Opening the Model Viewer tab inside the Zone Viewer caused a crash with error:
```
Bundle has duplicate components: bevy_render::view::visibility::InheritedVisibility
```

### Root Cause
In [`model_viewer_system.rs`](src/systems/model_viewer_system.rs:285), the character spawn bundle had `InheritedVisibility::default()` listed **twice** (lines 293 and 295):

```rust
// BROKEN CODE - duplicate InheritedVisibility
let entity = commands.spawn((
    ClientEntityName { ... },
    character_info,
    equipment,
    Visibility::default(),
    InheritedVisibility::default(),  // First instance
    ViewVisibility::default(),
    InheritedVisibility::default(),  // DUPLICATE!
    GlobalTransform::default(),
    Transform::default(),
)).id();
```

This was likely a copy-paste error during code changes.

### Solution
Remove the duplicate `InheritedVisibility::default()` from the bundle.

### Files Modified
- `src/systems/model_viewer_system.rs` - Removed duplicate `InheritedVisibility::default()` from character spawn bundle

### Lesson Learned
1. **Bundles cannot have duplicate component types** - Bevy will panic at runtime if a bundle contains the same component type twice
2. **Copy-paste errors can create subtle duplicates** - Always review bundle definitions after copy-pasting
3. **Compiler doesn't catch this** - This is a runtime error, not a compile-time error, since the bundle is a tuple type
