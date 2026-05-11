# Cloud Outline + Gray Shadow Fix Tracking

## Issue
Clouds need a cartoonish outline and should not receive dark shadowing that turns them gray.

## Affected Systems
- `src/render/volumetric_cloud.rs`
- `src/render/shaders/volumetric_cloud.wgsl`
- Bevy material shadow/prepass behavior (`bevy_pbr` `Material` trait)

## Validation Notes
- Confirmed in Bevy 0.18.1 `Material` trait defaults:
  - `enable_prepass()` defaults to `true`
  - `enable_shadows()` defaults to `true`
- Confirmed render prep reads `M::enable_shadows()` and `M::enable_prepass()` for pipeline draw functions.
- Current project already has `enable_shadows() -> false` and `enable_prepass() -> false` on `VolumetricCloudMaterial`.

## Attempt Log
1. Baseline inspection completed.
   - Result: Shader already has rim-light term but not a crisp cartoon outline pass.
   - Result: Clouds still likely darken from directional shading balance, not from shadow-map sampling path.
2. Planned implementation:
   - Add explicit toon outline/dark edge banding based on view-normal rim.
   - Flatten/whiten lighting response to remove gray cast from directional dark side.
   - Keep shadow/prepass disabled at material level.
