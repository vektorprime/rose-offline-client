# Atmosphere Flash (Fixed 2026-09-15)

## Problem
Random 1-2 frame milky cyan/blue flashes covering the entire 3D view (terrain, models, even the sky — only UI unaffected), at random intervals, mostly at night. Appeared after the Bevy 0.18.1 -> 0.19.1 upgrade. All CPU-side data (lights, fog, uniforms) was logged and constant during flashes; no wgpu errors.

## Root Cause
Bevy 0.19.1 bug (bevy#24808, fixed upstream by PR #24884 which is NOT in 0.19.1): when no `Atmosphere` entity exists, `extract_atmosphere` removes only `ExtractedAtmosphere` + `GpuAtmosphereSettings` from the view but leaves stale `AtmosphereBindGroups` + `DynamicUniformIndex<GpuAtmosphereSettings>` behind. Our `toggle_atmosphere_based_on_time` despawned the atmosphere entity at night, so `render_sky` (a no-depth-attachment fullscreen dual-source-blend pass running between the opaque and transparent 3D passes) kept drawing every night with a bind group pointing at a recycled depth texture and the last daytime sky-view LUT. Whenever the aliased depth texture read as "sky", the daytime LUT was splashed over the whole viewport — the flash. Each despawn also caused a lag spike (matches the dt spikes seen near flashes).

## Solution
Never despawn the `Atmosphere` entity. `toggle_atmosphere_based_on_time` now keeps it permanently spawned (self-healing re-spawn if missing). With the entity alive, bind groups are rebuilt from live data every frame so the stale state cannot form. Stars remain visible because the starry-sky dome uses `AlphaMode::Add` (Transparent3d, drawn after the atmosphere pass) and the night sky LUT is dark (no `SunDisk` component on our lights, sun illuminance 0 at night).

## Files Modified
- `src/render/starry_sky_material.rs` — `toggle_atmosphere_based_on_time` no longer time-gates despawn

## Lesson Learned
On Bevy 0.19.0/0.19.1, do NOT remove `AtmosphereSettings` from a camera or despawn all `Atmosphere` entities at runtime — stale render-world atmosphere components keep `render_sky` drawing garbage overlays. Keep the atmosphere entity alive; if an upstream fix (#24884) gets backported or we upgrade past 0.19.1, the despawn-on-night design could be reinstated.
