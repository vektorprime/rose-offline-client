# Phase 4.3 - Irradiance Volumes Evaluation Report

## Executive Summary

**Decision: NOT FEASIBLE without new assets**

Irradiance volumes and reflection probes require pre-baked cubemap/3D texture data that does not exist in the original ROSE Online game assets. The game was designed with lightmap-based lighting, not irradiance volumes.

---

## Analysis

### 1. Current Lighting System

The project currently uses:

| Feature | Implementation | Location |
|---------|---------------|----------|
| Directional Light | `DirectionalLight` with cascade shadows | [`zone_lighting.rs:101-115`](../src/render/zone_lighting.rs:101) |
| Ambient Light | `AmbientLight` resource | [`zone_lighting.rs:121-124`](../src/render/zone_lighting.rs:121) |
| Volumetric Fog | `FogVolume` + `VolumetricLight` | [`zone_lighting.rs:151-162`](../src/render/zone_lighting.rs:151) |
| Lightmaps | `.LIT` files per block | [`zone_loader.rs:894-916`](../src/zone_loader.rs:894) |
| Custom Lighting Uniforms | `ZoneLighting` resource | [`zone_lighting.rs:222-252`](../src/render/zone_lighting.rs:222) |

### 2. Bevy 0.15 Light Probe Features

Bevy 0.15 provides two light probe types:

#### IrradianceVolume
```rust
// From bevy_pbr/src/light_probe/irradiance_volume.rs
#[derive(Component)]
pub struct IrradianceVolume {
    /// 3D texture with ambient cubes in specific format
    pub voxels: Handle<Image>,
    pub intensity: f32,
}
```

**Requirements:**
- Pre-baked 3D texture in specific format (Rx, 2Ry, 3Rz dimensions)
- Baked using Blender Eevee + `export-blender-gi` tool
- Stores Half-Life 2 style ambient cubes (6 directional colors per voxel)

#### EnvironmentMapLight
```rust
// From bevy_pbr/src/light_probe/environment_map.rs
#[derive(Component)]
pub struct EnvironmentMapLight {
    pub diffuse_map: Handle<Image>,   // Diffuse cubemap
    pub specular_map: Handle<Image>,  // Specular cubemap (mipmapped)
    pub intensity: f32,
    pub rotation: Quat,
}
```

**Requirements:**
- Two pre-filtered cubemap textures
- Diffuse map: Lambertian distribution
- Specular map: GGX distribution with mipmaps
- Can be generated with [glTF IBL Sampler](https://github.com/KhronosGroup/glTF-IBL-Sampler)

### 3. Asset Availability Analysis

| Asset Type | Exists in Game | Notes |
|------------|----------------|-------|
| Lightmaps (.LIT) | ✅ Yes | Per-block lightmap data for static geometry |
| Cubemap textures | ❌ No | Game uses skybox meshes, not cubemaps |
| Irradiance volumes | ❌ No | Not part of original engine design |
| Specular spheremap | ⚠️ Partial | `SPECULAR_SPHEREMAP.DDS` exists but is for material specular, not IBL |

### 4. Why Implementation Is Not Feasible

#### Irradiance Volumes
1. **No pre-baked data** - The original ROSE Online engine did not use irradiance volumes
2. **Complex baking pipeline** - Would require:
   - Export all zone geometry to Blender
   - Set up Eevee irradiance volumes
   - Bake and export using `bevy-baked-gi` tools
   - Create 3D textures in specific format
3. **Per-zone work** - Each of the 50+ zones would need custom baking
4. **Format complexity** - Bevy expects ambient cubes in a specific 3D texture layout

#### Environment Maps (Reflection Probes)
1. **No zone-specific cubemaps** - Would need per-environment captures
2. **Generic cubemaps mismatch** - Using generic outdoor cubemaps wouldn't match zone aesthetics
3. **Labor intensive** - Creating cubemaps for each zone requires:
   - Capturing 6 views per location
   - Pre-filtering with glTF IBL Sampler
   - Multiple probes per zone for indoor/outdoor areas

---

## Recommendations

### Short Term: Document Limitation
Mark this task as **not feasible without new assets** and document what would be needed.

### Medium Term: Consider Alternatives

1. **Enhanced Ambient Light**
   - Already implemented via `ZoneLighting` resource
   - Could add hemisphere lighting for sky/ground color blend

2. **Light Probes from Lightmaps**
   - Theoretical: Could sample lightmap data to generate approximate irradiance
   - Complex: Would require custom tooling
   - Low priority: Lightmaps already provide static GI

3. **Generic Sky Environment Map**
   - Use a generic outdoor environment map attached to camera
   - Would improve metallic reflections without zone-specific data
   - Could be implemented if visual improvement justifies effort

### Long Term: Asset Creation Pipeline
If irradiance volumes are desired, would need:
1. Create Blender add-on to import ROSE zone geometry
2. Set up Eevee irradiance volume baking
3. Create export pipeline for Bevy format
4. Bake all 50+ zones
5. Integrate loading into zone_loader.rs

---

## Technical Reference

### Bevy 0.15 Irradiance Volume Usage (if assets existed)
```rust
// This would work IF we had pre-baked assets
commands.spawn((
    IrradianceVolume {
        voxels: asset_server.load("zones/31/irradiance.ktx2"),
        intensity: 1.0,
    },
    Transform::from_scale(Vec3::splat(1000.0)), // Cover large area
    Visibility::default(),
));
```

### Bevy 0.15 Environment Map Usage (if assets existed)
```rust
// This would work IF we had pre-baked cubemaps
commands.spawn((
    EnvironmentMapLight {
        diffuse_map: asset_server.load("environment/sky_diffuse.ktx2"),
        specular_map: asset_server.load("environment/sky_specular.ktx2"),
        intensity: 1000.0,
    },
    Transform::default(),
    Visibility::default(),
));
```

---

## Conclusion

**Irradiance volumes are not feasible for this project** because:

1. The original game assets do not include pre-baked irradiance data
2. Creating such data would require significant tooling and manual work
3. The existing lightmap system already provides static bounced light
4. The visual improvement would not justify the effort required

**Recommendation:** Close this task as "not feasible without assets" and focus on other lighting improvements that can work with existing data.

---

## Related Files

- [`src/render/zone_lighting.rs`](../src/render/zone_lighting.rs) - Current lighting implementation
- [`src/zone_loader.rs`](../src/zone_loader.rs) - Lightmap loading (lines 894-916)
- [`src/render/object_material_extension.rs`](../src/render/object_material_extension.rs) - Lightmap shader integration
- [Bevy Irradiance Volume Source](../../C:/Users/vicha/RustroverProjects/bevy-collection/bevy-0.15.4/crates/bevy_pbr/src/light_probe/irradiance_volume.rs)
- [Bevy Environment Map Source](../../C:/Users/vicha/RustroverProjects/bevy-collection/bevy-0.15.4/crates/bevy_pbr/src/light_probe/environment_map.rs)
