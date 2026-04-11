# Pitfalls and Lessons Learned

This folder contains documentation of issues encountered during development and their solutions, organized by component. These records help avoid similar problems in the future.

## Quick Navigation by Component

| File | Description |
|------|-------------|
| [skill-bar-ui.md](skill-bar-ui.md) | UI drag and drop, hotbar system issues |
| [networking.md](networking.md) | Network thread, respawn, and connection issues |
| [rendering-camera.md](rendering-camera.md) | Depth of field, shadows, SSAO, TAA, camera setup |
| [materials-transparency.md](materials-transparency.md) | Alpha modes, custom materials, texture arrays |
| [lighting.md](lighting.md) | Ambient light, photometric units |
| [terrain-physics.md](terrain-physics.md) | Terrain adherence, spawn height, bundle limits |
| [water-system.md](water-system.md) | Water materials, fish spawning, shader migration |
| [zone-loading.md](zone-loading.md) | Asset tracking, state initialization, skybox loading |
| [model-viewer.md](model-viewer.md) | Bundle duplicates, runtime panics |
| [performance-memory.md](performance-memory.md) | GPU memory leaks, buffer management |
| [blood-effects.md](blood-effects.md) | Terrain blood decal visibility, orientation, and wound overlay tuning |

## Quick Navigation by Bevy Version

### Bevy 0.14+ Changes
- [AmbientLight brightness now uses photometric units](lighting.md) - Values like `0.3` are now far too low

### Bevy 0.15 Changes
- [Custom material AsBindGroup API changes](materials-transparency.md) - New method signatures required
- [Shader time access changed from `view.time` to `globals.time`](water-system.md)
- [ExtendedMaterial alpha_discard must be called explicitly](rendering-camera.md)

### Bevy 0.16 Changes
- [AsBindGroupError::CreateBindGroupDirectly required for custom materials](water-system.md)
- [insert_state() vs init_state() for initial state values](zone-loading.md)

## Common Patterns to Watch For

### System Ordering
When multiple systems share mutable resource state, ensure proper ordering so "consumer" systems run before "cleanup" systems. See [skill-bar-ui.md](skill-bar-ui.md) for an example.

### Asset Tracking
Assets loaded via `asset_server.load()` should have their handles tracked if other systems need to wait for them. See [zone-loading.md](zone-loading.md) for an example.

### Every-Frame Allocations
Systems that run every frame should NOT create new assets without removing old ones. See [performance-memory.md](performance-memory.md) for an example.

### Parent-Child Transforms
Entities spawned within a transformed parent (like a zone) must be parented to inherit the transform. See [water-system.md](water-system.md) for an example.

## Contributing

When you fix an issue AND the user confirms it's resolved, add a new entry to the appropriate file in this folder. If no file exists for that component, create one. Keep entries concise but include:
1. Problem description
2. Root cause analysis
3. Solution with code example
4. Files modified
5. Lesson learned
