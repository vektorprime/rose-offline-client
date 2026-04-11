# Skill Effect Gray/Black Box - Issue Tracking

## Issue
Certain skill effects (example: `3DDATA/EFFECT/_MANA_HIT_01.EFT`) render as gray/black quads instead of proper particles.

## Context from logs
- Effect spawn pipeline is active (`spawn_effect_system` events processed)
- Effect file loads successfully (`8 particles, 0 meshes`)
- Particle sequences are spawning particles (`emit_rate=1000`)

## Attempt Log

### Attempt 1 - Pipeline/path verification
- Confirmed spawn path reaches particle rendering systems:
  - `animation_effect_system` -> `spawn_effect_system` -> `effect_loader::spawn_effect` -> `particle_sequence_system`
- Confirmed particles are being emitted at runtime from logs, so issue is not "effect not spawned".
- Result: **Not root cause**.

### Attempt 2 - Compare old working renderer
- Reviewed old Bevy 0.11 client particle pipeline and shader.
- Noted old renderer used custom linear sampler and separate pipeline batching by texture.
- Current renderer uses material-based pipeline with texture loaded by `effect_loader` and sampled in `particle.wgsl`.
- Result: baseline understood.

### Attempt 3 - Texture loading path analysis
- Inspected `src/dds_image_loader.rs` and found parsed DDS formats include `A8`, `L8`, `L8A8`.
- Current `match` did **not** explicitly convert these formats; they fell through to image-crate fallback.
- For ROSE particle textures, alpha-only/luminance DDS data is common for soft masks.
- Hypothesis: alpha/luminance DDS are decoded with incorrect channel mapping, producing opaque gray/black quads.
- Result: **Likely root cause identified**.

### Attempt 4 - Implement DDS conversion fix
- Updated `src/dds_image_loader.rs` format dispatch to explicitly handle:
  - `DdsFormat::A8` via `convert_a8_to_rgba()`
  - `DdsFormat::L8` via `convert_l8_to_rgba()`
  - `DdsFormat::L8A8` via `convert_l8a8_to_rgba()`
- Added conversion functions:
  - `A8` -> RGBA (`255,255,255,alpha`) so particle tinting remains correct
  - `L8` -> RGBA (`luma,luma,luma,255`)
  - `L8A8` -> RGBA (`luma,luma,luma,alpha`)
- Kept linear sampler behavior unchanged.
- Build validation: separate `cargo build` subtask reported **zero errors**.
- Result: **Fix implemented and compiling**.

## Planned Runtime Validation
- Re-test projectile/skill effects that previously showed gray/black quads (e.g. `_MANA_HIT_01.EFT`).
- Confirm particle textures now render with expected alpha/mask behavior.
