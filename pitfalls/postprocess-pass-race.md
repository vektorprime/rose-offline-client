# Post-Process Pass Race: White Flash with SMAA (Fixed 2026-09-30)

## Problem
With only SMAA + Tonemapping enabled, the whole 3D view randomly flashed white for a frame. With Tonemapping off, the 3D view appeared to update very slowly while egui menus stayed responsive. SMAA off: no symptoms.

## Root Cause
In Bevy 0.19, render-graph nodes are systems in the `Core3d` schedule, which runs on the multi-threaded executor. `RenderContext` and `ViewQuery` only take read access, so passes with no ordering between them run **in parallel**. Command buffers are submitted in schedule (topological) order. But `ViewTarget::post_process_write()` flips the shared main-texture ping-pong index in **thread** order, so the two orders can disagree.

`underwater_effect` (ours) and Bevy's `smaa` were both only `.after(tonemapping)`, so they were mutually unordered and both called `post_process_write()` every frame. The underwater pass did this even above water, as a full-screen pass-through copy. When the flip order and the GPU order disagreed, one pass read a texture not yet written this frame:
- Tonemapping on: the pre-tonemap HDR frame (5000-lux sun, no auto exposure) reached the screen, which showed as a white flash.
- Tonemapping off: a stale older frame reached the screen, so the 3D view appeared to crawl. egui was unaffected.

Related latent races of the same kind:
- Bevy leaves `fxaa` and `smaa` mutually unordered.
- bevy_egui 0.40's `egui_pass` is only `.after(EarlyPostProcess).before(upscaling)`. It draws into whichever main texture is current when its system runs.

## Fix
```rust
// src/render/underwater_effect.rs
underwater_effect
    .in_set(Core3dSystems::PostProcess)
    .after(tonemapping)
    .before(fxaa)
    .before(smaa),
// ...and in the pass: `if !underwater_state.is_underwater { return; }`
// BEFORE post_process_write() (no flip, no pass-through copy).

// src/lib.rs (after EguiPlugin)
render_app.configure_sets(
    Core3d,
    Core3dSystems::PostProcess.before(bevy_egui::render::egui_pass),
);
```
`apply_fxaa_system` inserts `Fxaa` only while `smaa_quality == Disabled`.

## Files Modified
- `src/render/underwater_effect.rs`, `src/lib.rs`, `src/graphics/apply_systems.rs`, `system-architecture/Render.md`

## Lesson Learned
1. On Bevy 0.19, every pass that calls `post_process_write()` must be **totally ordered** against every other such pass. `.after(tonemapping)` alone is not enough, because siblings of that constraint run in parallel.
2. A pass that has nothing to do must return **before** `post_process_write()`. Calling it flips the ping-pong index even if nothing is drawn.
3. The symptom depends on which side of tonemapping the race falls. It shows as un-tonemapped frames (white or overexposed) when tonemapping is on, and as stale frames (laggy 3D view with a live UI) when it is off. Neither shows up in CPU-side logs.
4. Earlier "white film/flash" diagnoses from before this fix (2026-09-25 ColorGrading and tonemapping removals) may have partly been this race.
