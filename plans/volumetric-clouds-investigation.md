# Volumetric Clouds Investigation Log

## Issue
Investigate why volumetric clouds are not visible in-game.

## Scope
- Rendering setup for atmosphere, fog volumes, and cloud material/shader path
- Scene entities/resources that gate cloud visibility
- Bevy 0.18.1 behavior for relevant volumetric features

## Attempt Log

### 2026-04-11 - Session Start
- Reviewed prior knowledge in `pitfalls/` and `system-architecture/`.
- Noted existing architecture docs focus on atmosphere/fog/light shafts and star sky.
- Next: locate concrete cloud implementation in source and trace spawn/update path.

### 2026-04-11 - Spawn Distribution Change
- Updated volumetric cloud spawning to center on map middle `(5120, -5120)` and spread across the whole map extent.
- Updated default cloud spawn radius to map half-extent (`5120`) for full-map coverage.
- Updated cloud spawn height to fixed midpoint between configured min/max heights.

### 2026-04-11 - Visual Look Improvement (Whiter + More Variation)
- Increased baseline cloud brightness in [`VolumetricCloudSettings::default()`](src/render/volumetric_cloud.rs:75) by raising `brightness` to `2.9`.
- Added per-cloud non-uniform axis scaling in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:289) to reduce repeated "same circle" silhouettes:
  - wider/larger horizontal variability
  - flatter-vs-taller profile variability
  - depth variability
- Added subtle per-cloud orientation randomization in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:289) using yaw/pitch/roll to further break repetition.
- Added stable per-cloud shader seed from cloud origin in [`cloud_density()`](src/render/shaders/volumetric_cloud.wgsl:121) to vary:
  - noise frequencies
  - noise layer weights
  - density threshold
  - shell thickness profile
- Pushed final shading toward cleaner white cumulus in [`cloud_lighting()`](src/render/shaders/volumetric_cloud.wgsl:191) by increasing white bias and enforcing a higher lit floor to reduce gray cast.

### 2026-04-11 - Instant Apply from Settings UI (No Zone Reload)
- Added runtime structural sync system in [`sync_volumetric_cloud_structure_system()`](src/render/volumetric_cloud.rs:452) and registered it in [`VolumetricCloudPlugin`](src/render/volumetric_cloud.rs:31).
- Structural cloud settings are now tracked via [`VolumetricCloudStructuralSettings`](src/render/volumetric_cloud.rs:57).
- When structural settings change, cloud instances are respawned immediately by invoking [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:289), so updates are visible instantly without zone load.
- Material-only settings continue to update live every frame through [`update_volumetric_cloud_material_system()`](src/render/volumetric_cloud.rs:422) and [`update_volumetric_cloud_lighting_system()`](src/render/volumetric_cloud.rs:445), avoiding unnecessary respawns.
- Updated cloud settings UI note to reflect instant-apply behavior in [`ui_settings_system.rs`](src/ui/ui_settings_system.rs:812).

### 2026-04-11 - Clustered Cloud Grouping + Whiter Look
- Added cluster-size settings to volumetric cloud config:
  - [`cluster_size_min`](src/render/volumetric_cloud.rs:84) (default `2`)
  - [`cluster_size_max`](src/render/volumetric_cloud.rs:84) (default `5`)
- Updated cloud spawn logic in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:315) to generate clouds in clusters instead of isolated single blobs:
  - each cluster spawns `cluster_size_min..=cluster_size_max` cloud blobs
  - final cluster logic guarantees no single-blob leftovers
  - per-cluster center + spread is used so blobs naturally group together
- Added settings UI sliders for cluster size in [`ui_settings_system.rs`](src/ui/ui_settings_system.rs:694), with min/max clamping.
- Cluster settings are structural, so runtime changes trigger immediate respawn through [`sync_volumetric_cloud_structure_system()`](src/render/volumetric_cloud.rs:453).
- Tuned shading to be significantly whiter in [`cloud_lighting()`](src/render/shaders/volumetric_cloud.wgsl:182):
  - reduced ambient gray influence
  - increased white base and lit floor
  - stronger white bias blend
  - increased minimum TOD multiplier in [`fragment()`](src/render/shaders/volumetric_cloud.wgsl:236)

## Findings
- Volumetric clouds are spawned around world origin in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:290): X/Z are randomized in `[-cloud_spawn_radius, +cloud_spawn_radius]` (default ±3000) and Y in [300, 700], see [`VolumetricCloudSettings::default()`](src/render/volumetric_cloud.rs:69) and spawn position math in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:340).
- Main gameplay camera/world center is around `(5120, 100, -5120)`, logged in [`setup_camera()`](src/lib.rs:2000), and other world-space systems (fog volume) are centered at `(5120, 0, -5120)` in [`zone_lighting.rs`](src/render/zone_lighting.rs:197). This places clouds far from the playable center.
- Material pipeline depth testing for clouds uses [`CompareFunction::Less`](src/render/volumetric_cloud.rs:280), which is incompatible with Bevy 0.18 reverse-Z defaults.
- Bevy 0.18 uses reverse-Z depth (camera docs: [`Camera3dDepthLoadOp`](../bevy-collection/bevy-0.18.1/crates/bevy_camera/src/components.rs:62)), and the standard 3D mesh pipeline compares depth using [`CompareFunction::GreaterEqual`](../bevy-collection/bevy-0.18.1/crates/bevy_pbr/src/render/mesh.rs:3249).

## Root Cause
1. **Primary visibility issue**: cloud spawn distribution is centered on origin instead of the actual zone center near `(5120, -5120)`.
2. **Pipeline visibility issue**: cloud material uses non-reverse-Z depth comparison (`Less`), so fragments are likely rejected in most view/depth scenarios.

## Fix Plan
1. Rebase cloud spawn positions to the active zone center (or camera XY center at spawn time) instead of `(0,0)`.
2. Change volumetric cloud depth compare to reverse-Z compatible mode (`GreaterEqual`) in material specialization.
3. Optional robustness: add a follow-center system (similar to old cloud layer approach) or periodic cloud field recentering for large zones.
4. Validate in-game with cloud debug logs and settings page controls.

## Implementation Applied (2026-04-11)
- Spawn center is now map middle `(5120, -5120)` via constants in [`volumetric_cloud.rs`](src/render/volumetric_cloud.rs:25).
- Spawn spread now covers full map by default (`cloud_spawn_radius = 5120`) in [`VolumetricCloudSettings::default()`](src/render/volumetric_cloud.rs:75).
- Runtime spawn radius now guarantees full-map distribution using `max(configured_radius, 5120)` in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:327).
- Cloud spawn height is now fixed to midpoint between min and max: `min + (max - min) * 0.5` in [`spawn_volumetric_clouds()`](src/render/volumetric_cloud.rs:321).
- Verified compile status: build subtask reported no errors.

## Additional Root-Cause Analysis (Active Path + Bevy 0.18.1)
- Confirmed active path uses volumetric cloud module, not decommissioned 2D module:
  - [`VolumetricCloudPlugin`](src/lib.rs:1016) is registered.
  - [`spawn_volumetric_clouds()`](src/lib.rs:1484) is called on startup.
  - Old 2D path is commented out at [`src/lib.rs`](src/lib.rs:1013) and [`src/lib.rs`](src/lib.rs:1483).
- Confirmed feature usage against Bevy 0.18.1 source:
  - Material alpha behavior from [`Material::alpha_mode()`](../bevy-collection/bevy-0.18.1/crates/bevy_pbr/src/material.rs:158).
  - Alpha pipeline key mapping in [`alpha_mode_pipeline_key()`](../bevy-collection/bevy-0.18.1/crates/bevy_pbr/src/material.rs:604).
  - `AlphaMode` semantics in [`bevy_material::alpha::AlphaMode`](../bevy-collection/bevy-0.18.1/crates/bevy_material/src/alpha.rs:9).
  - Mesh component usage in [`Mesh3d`](../bevy-collection/bevy-0.18.1/crates/bevy_mesh/src/components.rs:98).
  - Shader import/transform helpers in [`mesh_functions.wgsl`](../bevy-collection/bevy-0.18.1/crates/bevy_pbr/src/render/mesh_functions.wgsl:17).
- Critical rendering bug identified in shader model:
  - Previous density model multiplied by center-weighted radial falloff (`1 - smoothstep(0.3, 1.0, radius)`), but rendered geometry is only the sphere surface (`local_pos` near radius ~1), causing near-zero density and effective full discard.
  - Fixed by switching to shell-preserving density around sphere surface in [`cloud_density()`](src/render/shaders/volumetric_cloud.wgsl:116), using `shell_density = smoothstep(1.05, 0.95, radius)`.
  - Also raised discard threshold to keep large contiguous fluffy silhouettes in [`fragment()`](src/render/shaders/volumetric_cloud.wgsl:217).
