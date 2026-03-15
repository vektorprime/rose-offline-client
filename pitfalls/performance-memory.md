# Performance and Memory Pitfalls

This document records performance and memory-related issues encountered during development.

---

## GPU Storage Buffer Memory Leak (Fixed 2026-03-03)

### Problem
Graphics would slow down over time due to a slow memory leak in GPU storage buffers.

### Root Cause
Multiple locations were creating new GPU storage buffers every frame without removing old ones:
1. `src/systems/particle_sequence_system.rs` - Created 4 new buffers per particle per frame
2. `src/systems/damage_digit_render_system.rs` - Created 3 new buffers per frame

The `Assets<ShaderStorageBuffer>::add()` method creates a NEW asset handle each call. When updating materials every frame, the old buffer handles were being overwritten but the underlying GPU resources were never removed from the Assets storage.

### Symptoms
- GPU memory grows linearly with particle effects active
- Frame rate degrades over time (minutes of gameplay)
- `Assets<ShaderStorageBuffer>` count grows unbounded

### Fix
Before creating new storage buffers, store the old handles and remove them after:
```rust
// Store old buffer handles
let old_positions = mat.positions.clone();
// ... create new buffers ...
// Remove old buffers to prevent memory leak
storage_buffers.remove(&old_positions);
```

### Files Modified
- `src/systems/particle_sequence_system.rs` - Added buffer cleanup
- `src/systems/damage_digit_render_system.rs` - Added buffer cleanup

### Lesson Learned
When using `Assets::add()` in a system that runs every frame, you MUST track and remove old assets or they will leak indefinitely. This is especially critical for GPU resources like `ShaderStorageBuffer`.

### Related Patterns to Watch For
- Any `storage_buffers.add()` in update systems
- Any `assets.add()` called every frame
- Unbounded Vec/HashSet growth in long-running systems
