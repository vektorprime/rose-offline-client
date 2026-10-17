# Cel Shading Pitfalls

## Outlines Never Drawn: WGSL Reserved Word (Fixed 2026-10-17)

### Problem
The cel outline pass (`src/render/cel_outline.rs`) was wired up and the camera had `CelOutline`, but no outlines appeared. There was no visible error in game; the toon lighting worked.

### Root Cause
`shaders/cel_outline.wgsl` used `class` as a struct field name. `class` is a WGSL reserved word, so naga rejected the shader and the pipeline was never created. The render system does `let Some(pipeline) = pipeline_cache.get_render_pipeline(..) else { return; }`, so the pass silently drew nothing. Shader errors only appear at runtime (pipeline creation), never in `cargo build`.

### Fix
Renamed the field to `outline_class`.

### Lesson Learned
1. WGSL reserves many common words that are not keywords: `class`, `target`, `filter`, `module`, `type`, `self`, `meta`, `pass`, `set`, `get`, `new`, ... Check the WGSL reserved-word list when naming shader identifiers.
2. A custom pass whose pipeline fails to compile is skipped silently. When a pass "does nothing", check the log for naga / pipeline errors first.
