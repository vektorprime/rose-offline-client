# Damage Digits Invisible Under Camera3d (Text2d Trap)

## Issue
Damage digits never appeared when attacking monsters — neither with the old
custom GPU pipeline nor with a `Text2d` replacement.

## Root cause
Bevy only draws `Text2d`/`Sprite` into the `Transparent2d`/`Opaque2d` render
phases, and those phases are created **exclusively for `Camera2d` views**
(`extract_core_2d_camera_phases` in `bevy_core_pipeline`). This game renders
through a `Camera3d`, so `Text2d` entities are silently skipped every frame.
(This is also why the project has its own `WorldUi` pipeline and previously a
custom digit material instead of using text.)
The old custom pipeline had its own fragility stack: ZMO-gated lifetime
(instant despawn if the motion failed to load), a 1-frame
`PendingDamageDigitMaterial` delay, a degenerate zero-vertex mesh AABB with no
`NoFrustumCulling`, and a vertex-layout mismatch (mesh has POSITION, pipeline
specialized to stride-0).

## Fix
Parent entity + one `Mesh3d` quad child per digit, all Bevy-native:
`StandardMaterial { unlit: true, fog_enabled: false, alpha_mode: Blend,
uv_transform: cell slice }` reusing the original DDS digit strips (24 cached
materials, 1 shared mesh — no per-frame assets). Parent billboards to the main
camera (reflection camera excluded); rise 1.6 m/s for 1.1 s, then despawn
(children cascade). `NotShadowCaster`/`NotShadowReceiver` on quads.

## Files
- `src/resources/damage_digits_spawner.rs` (spawn + quad + materials)
- `src/components/damage_digits.rs` (`DamageNumber` lifetime)
- `src/systems/damage_number_system.rs` (billboard + animate)
- Deleted: `damage_digit_render_system.rs`, `damage_digit_material.rs`,
  `damage_digit_render_data.rs`, `shaders/damage_digit.wgsl`

## Lesson learned
Before using `Text2d`/`Sprite` for in-world 3D labels, check which render
phase they draw into and whether the game's cameras own that phase. For
`Camera3d` world-space quads, use `Mesh3d` + `StandardMaterial`.
