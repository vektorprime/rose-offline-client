# Graphics Settings Apply Chain

Runtime graphics toggles live in `src/graphics/`: `graphics_settings.rs` (enums/resources/defaults) + `apply_systems.rs` (insert/remove Bevy components). Settings UI is the Seasons-adjacent pages in `src/ui/ui_settings_system.rs`.

## Defaults vs opt-in

- Camera spawn defaults (`src/lib.rs:1990-2061`): `Tonemapping::TonyMcMapface`, `Bloom::NATURAL`, `DepthOfField` (Gaussian), `ShadowFilteringMethod::Gaussian`, `ScreenSpaceAmbientOcclusion` Medium, `VolumetricFog{steps: 64}`, `Msaa::Off`, `DirectionalLightShadowMap{size: 2048}`. SMAA/SSR/MotionBlur/AutoExposure/CAS are NOT spawned (`src/lib.rs:1995-1999`).
- TAA, SSR, AutoExposure have no implementation in `src/`; do not document them as available.

## Apply systems (`src/graphics/apply_systems.rs`)

- `apply_shadow_quality_system` (`:68`): `ShadowQuality::{Off,Low,Medium(default),High,Ultra}` → cascades 0/1/2/3/4, map size 0/1024/2048/2048/4096, distance 0/50/100/200/400; recomputes `CascadeShadowConfig{Bounds}`; skips `MoonLight`.
- `apply_tonemapping_system` (`:124`), `apply_bloom_system` (`:155`), `apply_shadow_filtering_system` (`:190`), `apply_msaa_system` (`:209`, X1=Off/X2/X4/X8), `apply_ssao_system` (`:236`), `apply_smaa_system` (`:274`, Disabled/Low/Medium/High/Ultra), `apply_motion_blur_system` (`:318`), `apply_dof_enabled_system` (`:354`), `apply_fxaa_system` (`:379`), `apply_view_distance_system` (`:408`), `apply_texture_quality_system` (`:430`), `apply_ambient_light_system` (`:450`, `80.0 * multiplier` into `GlobalAmbientLight`).
- Round-trip truth for shadows/volumetrics/post defaults: [zone_lighting.md](zone_lighting.md), [Render.md](Render.md).
