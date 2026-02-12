# Pitfalls and Lessons Learned

This document records issues encountered during development and their solutions, to help avoid similar problems in the future.

---

## Tree/Grass Transparency Not Working (Fixed 2026-02-12)

### Problem
Trees and grass textures were not showing transparency in their leaves. The texture alpha channel was being ignored.

### Root Cause
When creating Bevy materials in `model_loader.rs` and `zone_loader.rs`, the code always used `AlphaMode::Opaque` regardless of the ZSC material's `alpha_enabled` and `alpha_test` properties.

### Solution
Both files were updated to properly set `alpha_mode` based on ZSC material properties:
- `AlphaMode::Mask(threshold)` when `alpha_enabled` with `alpha_test` threshold
- `AlphaMode::Blend` when `alpha_enabled` without threshold
- `AlphaMode::Opaque` when alpha is disabled

### Files Modified
- `src/model_loader.rs` (lines ~1357-1375)
- `src/zone_loader.rs` (lines ~2649-2669)

### Lesson Learned
When working with Bevy's StandardMaterial or custom materials, always explicitly set `alpha_mode` based on the source material's transparency properties. The default `AlphaMode::Opaque` will ignore any alpha channel in textures.

