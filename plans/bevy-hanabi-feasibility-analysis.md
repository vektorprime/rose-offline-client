# Bevy Hanabi Particle System Feasibility Analysis

## Executive Summary

**Recommendation: FEASIBLE with moderate effort**

Bevy Hanabi is a viable replacement for the current custom particle system and would provide significantly higher quality effects. The `bevy_hanabi` version 0.16 is fully compatible with Bevy 0.16.1 used in this project.

---

## Current Particle System Analysis

### Architecture Overview

The project uses a **CPU-based custom particle system** with the following components:

```
┌─────────────────────────────────────────────────────────────────┐
│                    Current Particle System                       │
├─────────────────────────────────────────────────────────────────┤
│  ParticleSequence Component                                      │
│  ├── ActiveParticle Vec - CPU-side particle storage             │
│  ├── Keyframe-based animation system                            │
│  ├── Texture atlas support with animation                       │
│  └── World/Local coordinate systems                             │
├─────────────────────────────────────────────────────────────────┤
│  ParticleMaterial - Custom WGSL shader                          │
│  ├── Storage buffers for positions, sizes, colors, textures     │
│  ├── Billboard rendering with 3 modes                           │
│  └── Premultiplied alpha blending                               │
├─────────────────────────────────────────────────────────────────┤
│  Particle Systems Using This:                                    │
│  ├── Game effects from PTL/EFT files                            │
│  ├── Weather particles for seasons - snow, rain, leaves         │
│  ├── Dirt dash particles - running dust                         │
│  ├── Wind effect particles - flying streaks                     │
│  └── Blood effects - spatter and gashes                         │
└─────────────────────────────────────────────────────────────────┘
```

### Key Files

| File | Purpose |
|------|---------|
| [`src/components/particle_sequence.rs`](src/components/particle_sequence.rs) | Particle data structures and keyframe logic |
| [`src/systems/particle_sequence_system.rs`](src/systems/particle_sequence_system.rs) | CPU-side particle update system |
| [`src/render/particle_material.rs`](src/render/particle_material.rs) | Custom material and plugin |
| [`src/render/shaders/particle.wgsl`](src/render/shaders/particle.wgsl) | GPU shader for billboard rendering |
| [`src/effect_loader.rs`](src/effect_loader.rs) | Loads PTL/EFT effect files |

### Current Capabilities

- **Spawn Control**: Rate-based, burst, loop count
- **Position**: Random within XYZ radius ranges
- **Velocity**: Keyframe-based with step interpolation
- **Color**: RGBA with keyframe animation and fade
- **Size**: XY with keyframe animation
- **Rotation**: Degrees with step interpolation
- **Texture Atlas**: Animated texture indices
- **Billboard Modes**: Full, Y-axis only, None
- **Coordinate Systems**: World and Local

### Current Limitations

1. **CPU-bound**: All particle updates run on CPU, limiting particle count
2. **No GPU Simulation**: Cannot leverage compute shaders
3. **Limited Physics**: Basic gravity only, no collisions
4. **No Advanced Effects**: No trails, ribbons, or mesh particles
5. **Manual Buffer Management**: Storage buffers updated manually each frame

---

## Bevy Hanabi Analysis

### Version Compatibility

| bevy_hanabi | Bevy |
|-------------|------|
| 0.18 | 0.18 |
| **0.16** | **0.16** ✓ |
| 0.14-0.15 | 0.15 |

**Current project uses Bevy 0.16 → bevy_hanabi 0.16 is a direct match!**

### Feature Comparison

| Feature | Current System | Bevy Hanabi |
|---------|---------------|-------------|
| GPU Compute Shaders | ❌ | ✅ |
| Particle Count | ~1000s | ~100,000s |
| 2D Camera Support | ✅ | ✅ |
| 3D Camera Support | ✅ | ✅ |
| Spawn Rate | ✅ | ✅ |
| Burst Spawn | ✅ | ✅ |
| Position Shapes | Basic sphere | Cube, Circle, Sphere, Cone, Plane, Mesh |
| Velocity Shapes | Random ranges | Circle, Sphere, Tangent |
| Color Over Lifetime | ✅ Keyframes | ✅ Gradients |
| Size Over Lifetime | ✅ Keyframes | ✅ |
| Rotation | ✅ | ✅ |
| Texture Atlas | ✅ | ✅ |
| Billboarding | ✅ 3 modes | ✅ Multiple modes |
| **Trails/Ribbons** | ❌ | ✅ |
| **Collisions** | ❌ | ✅ Plane, Cube, Sphere, Depth Buffer |
| **Force Fields** | ❌ | ✅ |
| **Mesh Particles** | ❌ | ✅ |
| **Velocity Stretching** | ❌ | ✅ |
| **HDR/Bloom** | ❌ | ✅ |
| **Multiple Viewports** | ❌ | ✅ |
| WebGPU/WASM | ❌ | ✅ via WebGPU |

### Hanabi Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Bevy Hanabi System                            │
├─────────────────────────────────────────────────────────────────┤
│  EffectAsset                                                     │
│  ├── Module - Expression system for GPU calculations            │
│  ├── SpawnerSettings - Rate, burst, repeat configurations       │
│  ├── Init Modifiers - Position, velocity, color, size, lifetime │
│  ├── Update Modifiers - Forces, collisions, lifetime            │
│  └── Render Modifiers - Color/size over lifetime, trails        │
├─────────────────────────────────────────────────────────────────┤
│  GPU Compute Pipeline                                            │
│  ├── Particle simulation on GPU                                 │
│  ├── Minimal CPU intervention                                   │
│  └── Efficient batch rendering                                  │
├─────────────────────────────────────────────────────────────────┤
│  ParticleEffect Component                                        │
│  └── Instance of an EffectAsset with transform                  │
└─────────────────────────────────────────────────────────────────┘
```

---

## Integration Requirements

### 1. Cargo.toml Addition

```toml
[dependencies]
bevy_hanabi = "0.16"
```

For 3D-only mode with smaller compile:
```toml
bevy_hanabi = { version = "0.16", default-features = false, features = ["3d", "serde"] }
```

### 2. Plugin Registration

```rust
use bevy_hanabi::prelude::*;

App::default()
    .add_plugins(DefaultPlugins)
    .add_plugins(HanabiPlugin)
    // ...
```

### 3. Effect Asset Creation Pattern

```rust
fn create_effect(mut effects: ResMut<Assets<EffectAsset>>) {
    let mut module = Module::default();
    
    // Define spawner
    let spawner = SpawnerSettings::rate(5.0.into());
    
    // Create effect
    let effect = EffectAsset::new(32768, spawner, module)
        .init(SetPositionSphereModifier {
            center: module.lit(Vec3::ZERO),
            radius: module.lit(2.0),
            dimension: ShapeDimension::Surface,
        })
        .init(SetVelocitySphereModifier {
            center: module.lit(Vec3::ZERO),
            speed: module.lit(6.0),
        })
        .init(SetAttributeModifier::new(
            Attribute::LIFETIME,
            module.lit(10.0),
        ))
        .update(AccelModifier::new(module.lit(Vec3::new(0.0, -3.0, 0.0))))
        .render(ColorOverLifetimeModifier {
            gradient: {
                let mut g = Gradient::new();
                g.add_key(0.0, Vec4::new(1.0, 0.0, 0.0, 1.0));
                g.add_key(1.0, Vec4::splat(0.0));
                g
            },
            ..default()
        });
    
    let handle = effects.add(effect);
}
```

### 4. Migration Strategy

#### Phase 1: Parallel Implementation
- Add Hanabi plugin alongside existing system
- Create new Hanabi effects for NEW particle types
- Keep existing system for backward compatibility

#### Phase 2: Effect Translation Layer
- Create translator from PTL keyframes to Hanabi modifiers
- Map current keyframe types to Hanabi expressions

| PTL Keyframe | Hanabi Equivalent |
|--------------|-------------------|
| Size | `SetSizeModifier` + `SizeOverLifetimeModifier` |
| Color | `SetColorModifier` + `ColorOverLifetimeModifier` |
| Velocity | `SetVelocitySphereModifier` / `SetVelocityCircleModifier` |
| TextureIndex | `TextureAtlasModifier` |
| Rotation | `SetRotationModifier` + `OrientModifier` |

#### Phase 3: System-by-System Migration
1. **Weather particles** - Easiest, self-contained
2. **Dirt dash** - Simple, good test case
3. **Wind effects** - Moderate complexity
4. **Game effects** - Most complex, requires PTL translation
5. **Blood effects** - Can use Hanabi collisions

#### Phase 4: Cleanup
- Remove old particle system code
- Remove custom WGSL shader
- Remove storage buffer management

---

## Potential Challenges

### 1. PTL File Format Translation

The Rose Online PTL format uses a keyframe system that differs from Hanabi's modifier approach:

**Current approach**:
```
Keyframe at time T:
  - Set size to random value in range
  - Fade to next keyframe's value over time
```

**Hanabi approach**:
```
Init modifier: Set initial size
Update modifier: Change size over lifetime
```

**Solution**: Create a translation layer that converts keyframe sequences to lifetime-based modifiers.

### 2. Texture Atlas Animation

Current system animates texture atlas index via keyframes. Hanabi supports texture atlases but may need custom modifier for frame-by-frame animation.

### 3. World vs Local Coordinates

Current system has `PtlUpdateCoords::World` and `PtlUpdateCoords::Local`. Hanabi supports both via `SimulationSpace`.

### 4. Learning Curve

Hanabi's expression API (`module.lit()`, `module.add()`, etc.) has a learning curve but is well-documented.

---

## Performance Comparison

| Metric | Current System | Bevy Hanabi |
|--------|---------------|-------------|
| Max Particles | ~5,000-10,000 | ~100,000+ |
| CPU Usage | High | Minimal |
| GPU Usage | Low | High via compute |
| Draw Calls | Per-effect | Batched |
| Memory | CPU + GPU | Primarily GPU |

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| Breaking existing effects | Medium | High | Parallel implementation |
| PTL translation complexity | Medium | Medium | Incremental migration |
| Performance regression | Low | High | Profile before/after |
| Hanabi bugs | Low | Medium | Active community, MIT license |

---

## Recommendations

### Immediate Actions

1. **Add bevy_hanabi 0.16 to Cargo.toml**
2. **Create a test branch** with Hanabi plugin
3. **Implement one simple effect** (e.g., dirt dash) as proof of concept

### Migration Priority

1. **Weather System** - Self-contained, easy to test
2. **Dirt Dash** - Simple, visible improvement
3. **Wind Effects** - Moderate, flying is key gameplay
4. **Blood Effects** - Can leverage collision features
5. **Game Effects** - Most complex, save for last

### Long-term Strategy

- Keep PTL loader for asset compatibility
- Build Hanabi effect cache from PTL data
- Consider effect hot-reloading for development

---

## Conclusion

Bevy Hanabi is **highly recommended** as a replacement for the current particle system. The benefits significantly outweigh the migration effort:

✅ **Direct Bevy 0.16 compatibility**
✅ **GPU-accelerated for massive particle counts**
✅ **Advanced features: trails, collisions, force fields**
✅ **Active development and community**
✅ **MIT/Apache dual licensing**

The migration can be done incrementally without breaking existing functionality, making this a low-risk, high-reward improvement for the game's visual quality.
