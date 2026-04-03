# Animation System Documentation

This document provides comprehensive documentation for the animation system in rose-offline-client, built on Bevy 0.18.1.

## Table of Contents

1. [Overview](#overview)
2. [Bevy API References](#bevy-api-references)
3. [Custom Extensions](#custom-extensions)
4. [Code Examples](#code-examples)
5. [Configuration Options](#configuration-options)
6. [Common Patterns](#common-patterns)
7. [Troubleshooting](#troubleshooting)
8. [Source File References](#source-file-references)

---

## Overview

The animation system in rose-offline-client uses Bevy's ECS architecture to provide skeletal animation, mesh morph animation, camera animation, and transform interpolation. The system is built around the `ZmoAsset` custom asset format, which stores animation data in a compressed binary format.

### Key Features

- **Skeletal Animation**: Bone-based animation for characters and NPCs using `SkinnedMesh`
- **Mesh Morph Animation**: Vertex-based animation for effect meshes using texture storage
- **Camera Animation**: Cinematic camera movements with FOV and projection control
- **Transform Animation**: Simple position/rotation/scale interpolation for objects
- **Event System**: Frame-based animation events for triggering effects and sounds
- **Frame Interpolation**: Smooth interpolation between animation frames
- **Animation Blending**: Weight-based blending between animation states

---

## Bevy API References

### Time Resource

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\time.rs:1-100`

The `Time` resource tracks elapsed time and delta time for animation updates:

```rust
#[derive(Resource, Debug, Copy, Clone)]
pub struct Time<T: Default = ()> {
    context: T,
    wrap_period: Duration,
    delta: Duration,
    delta_secs: f32,
    delta_secs_f64: f64,
    elapsed: Duration,
    elapsed_secs: f32,
    elapsed_secs_f64: f64,
    elapsed_wrapped: Duration,
    elapsed_secs_wrapped: f32,
    elapsed_secs_wrapped_f64: f64,
}
```

**Key Methods** (source: `time.rs:200-350`):

| Method | Description | Source Line |
|--------|-------------|-------------|
| `delta()` | Time since last update as `Duration` | `time.rs:230` |
| `delta_secs()` | Delta time as `f32` seconds | `time.rs:245` |
| `delta_secs_f64()` | Delta time as `f64` seconds | `time.rs:260` |
| `elapsed()` | Total elapsed time as `Duration` | `time.rs:275` |
| `elapsed_secs()` | Total elapsed time as `f32` seconds | `time.rs:290` |
| `elapsed_secs_wrapped()` | Wrapped elapsed time to prevent precision loss | `time.rs:305` |
| `advance_by(delta)` | Advance time by a duration | `time.rs:320` |
| `advance_to(elapsed)` | Advance time to a specific elapsed value | `time.rs:335` |

**Time Types** (source: `time.rs:50-80`, `real.rs:1-50`, `virt.rs:1-80`, `fixed.rs:1-100`):

- `Time<Real>`: Wall-clock time, unaffected by pause/scale (`real.rs`)
- `Time<Virtual>`: Game time, can be paused or scaled (`virt.rs`)
- `Time<Fixed>`: Fixed timestep for physics/deterministic systems (`fixed.rs`)
- `Time<()>`: Generic time (defaults to Virtual except in FixedUpdate)

**Usage in Animation**:

```rust
use bevy::prelude::*;

fn animation_system(time: Res<Time>, mut query: Query<&mut AnimationState>) {
    for mut anim in query.iter_mut() {
        // Use delta_secs for frame-independent animation speed
        let delta = time.delta_secs();
        anim.update(delta);
    }
}
```

### Timer

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\timer.rs:1-150`

Bevy's `Timer` provides countdown and repeating timer functionality:

```rust
pub struct Timer {
    stopwatch: Stopwatch,
    duration: Duration,
    mode: TimerMode,
    finished: bool,
    times_finished_this_tick: u32,
}
```

**Timer Modes** (source: `timer.rs:30-50`):

| Mode | Behavior |
|------|----------|
| `TimerMode::Once` | Runs once and stops at duration |
| `TimerMode::Repeating` | Wraps around and repeats indefinitely |

**Key Methods** (source: `timer.rs:150-400`):

| Method | Description | Source Line |
|--------|-------------|-------------|
| `new(duration, mode)` | Create a new timer | `timer.rs:160` |
| `from_seconds(secs, mode)` | Create timer from seconds | `timer.rs:175` |
| `tick(delta)` | Advance timer by delta | `timer.rs:200` |
| `is_finished()` | Check if timer reached duration | `timer.rs:250` |
| `just_finished()` | True only on the tick that finished | `timer.rs:265` |
| `elapsed()` | Elapsed time as Duration | `timer.rs:280` |
| `elapsed_secs()` | Elapsed time as f32 | `timer.rs:295` |
| `fraction()` | Progress from 0.0 to 1.0 | `timer.rs:310` |
| `fraction_remaining()` | Remaining from 1.0 to 0.0 | `timer.rs:325` |
| `remaining()` | Time remaining as Duration | `timer.rs:340` |
| `pause()` / `unpause()` | Pause/resume timer | `timer.rs:355` |
| `reset()` | Reset timer to zero | `timer.rs:370` |
| `times_finished_this_tick()` | How many times a repeating timer finished | `timer.rs:385` |

**Stopwatch** (source: `stopwatch.rs:1-100`):

Internal timer used by `Timer` for accurate elapsed time tracking.

**Common Conditions** (source: `common_conditions.rs:1-200`):

- `OnStart` - Triggered when timer starts
- `OnComplete` - Triggered when timer completes
- `WhileRunning` - True while timer is running
- `WhilePaused` - True while timer is paused

### Animation Timer Example

```rust
use bevy::time::{Timer, TimerMode};

#[derive(Component)]
struct AnimationCooldown {
    timer: Timer,
}

fn animation_cooldown_system(
    time: Res<Time>,
    mut query: Query<&mut AnimationCooldown>,
) {
    for mut cooldown in query.iter_mut() {
        cooldown.timer.tick(time.delta());
        
        if cooldown.timer.just_finished() {
            // Animation cooldown complete
        }
        
        // Get progress (0.0 to 1.0)
        let progress = cooldown.timer.fraction();
    }
}
```

---

## Custom Extensions

### RoseAnimationPlugin

The main plugin that registers all animation components and systems. Source: `src/animation/mod.rs:1-100`

```rust
#[derive(Default)]
pub struct RoseAnimationPlugin;

impl Plugin for RoseAnimationPlugin {
    fn build(&self, app: &mut App) {
        // Register ZMO asset types
        app.init_asset::<ZmoAsset>()
            .register_type::<ZmoAssetAnimationTexture>()
            .register_type::<ZmoAssetBone>()
            .init_asset_loader::<ZmoAssetLoader>()
            .init_asset_loader::<ZmoTextureAssetLoader>();

        // Register animation event channel
        app.add_message::<AnimationFrameEvent>();

        // Register animation components
        app.register_type::<AnimationState>()
            .register_type::<CameraAnimation>()
            .register_type::<MeshAnimation>()
            .register_type::<SkeletalAnimation>()
            .register_type::<TransformAnimation>();

        // Configure animation system set
        app.configure_sets(
            PostUpdate,
            RoseAnimationSystem.before(TransformSystems::Propagate),
        )
        .add_systems(
            PostUpdate,
            (
                camera_animation_system,
                mesh_animation_system,
                skeletal_animation_system,
                transform_animation_system,
            )
                .in_set(RoseAnimationSystem),
        );
    }
}
```
Source: `src/animation/mod.rs:20-60`

### System Execution Order

Animation systems run in `PostUpdate` before `TransformSystems::Propagate` to ensure:
1. Animation transforms are applied before hierarchy propagation
2. Child entities inherit animated parent transforms correctly

### Animation Components

All animation components wrap `AnimationState` using `Deref`/`DerefMut` for seamless access.

#### AnimationState

Base animation state component used by all animation types. Source: `src/animation/animation_state.rs:1-150`

---

## Animation Components

### AnimationState

Base animation state component used by all animation types. Source: `src/animation/animation_state.rs:20-100`

```rust
#[derive(Reflect, Component)]
pub struct AnimationState {
    /// Currently playing animation asset
    motion: Handle<ZmoAsset>,
    
    /// Speed multiplier for the animation
    animation_speed: f32,
    
    /// Loop count (None = infinite)
    max_loop_count: Option<usize>,
    
    /// Current loop iteration
    current_loop_count: usize,
    
    /// Interpolation weight for animation interval
    interpolate_weight: f32,
    
    /// Whether animation has completed
    completed: bool,
    
    /// Animation start time
    start_time: Option<f64>,
    
    /// Current frame index
    current_frame_index: usize,
    
    /// Next frame index for interpolation
    next_frame_index: usize,
    
    /// Interpolation between current and next frame (0.0 to 1.0)
    current_frame_fract: f32,
    
    /// Last event frame processed
    last_absolute_event_frame: usize,
    
    /// Delay before animation starts
    start_delay: Option<f32>,
}
```

### Constructor Methods

```rust
// Play animation once
impl AnimationState {
    pub fn once(motion: Handle<ZmoAsset>) -> Self
    
    // Play animation with optional loop limit
    pub fn repeat(motion: Handle<ZmoAsset>, limit: Option<usize>) -> Self
}

// Fluent builder pattern
pub fn with_animation_speed(mut self, speed: f32) -> Self
pub fn with_max_loop_count(mut self, count: usize) -> Self
```

### Key Methods

| Method | Description |
|--------|-------------|
| `advance(zmo_asset, time)` | Advance animation, returns true if completed |
| `completed()` | Check if animation finished |
| `current_frame_index()` | Get current frame |
| `next_frame_index()` | Get next frame for interpolation |
| `current_frame_fract()` | Get interpolation weight (0.0 to 1.0) |
| `interpolate_weight()` | Get animation interval blend weight |
| `set_animation_speed(speed)` | Change playback speed |
| `set_max_loop_count(count)` | Change loop limit |
| `set_start_delay(secs)` | Delay animation start |
| `iter_animation_events(handler)` | Process frame events |

### Animation Advancement

```rust
pub fn advance(&mut self, zmo_asset: &ZmoAsset, time: &Time) -> bool {
    if self.completed {
        return true;
    }

    // Handle start delay
    if let Some(start_delay) = self.start_delay.as_mut() {
        *start_delay -= time.delta_secs();
        if *start_delay > 0.0 {
            return false;
        } else {
            self.start_delay = None;
        }
    }

    // Calculate animation progress
    let current_time = time.elapsed_secs_f64();
    let start_time = self.start_time.get_or_insert(current_time);
    
    // Update interpolation weight
    if self.interpolate_weight < 1.0 {
        self.interpolate_weight += time.delta_secs() / zmo_asset.interpolation_interval;
    }

    // Calculate frame number with speed multiplier
    let animation_frame_number =
        (current_time - start_time) * (zmo_asset.fps as f64) * self.animation_speed as f64;

    // Update loop count and completion
    self.current_loop_count = animation_frame_number as usize / zmo_asset.num_frames;
    self.completed = self.current_loop_count >= self.max_loop_count.unwrap_or(usize::MAX);

    // Calculate frame indices for interpolation
    self.current_frame_fract = animation_frame_number.fract() as f32;
    self.current_frame_index = animation_frame_number as usize % zmo_asset.num_frames;
    self.next_frame_index = (self.current_frame_index + 1) % zmo_asset.num_frames;

    self.completed
}
```

---

### SkeletalAnimation

Bone-based animation for skinned meshes. Source: `src/animation/skeletal_animation.rs:1-80`

```rust
#[derive(Component, Reflect, Deref, DerefMut)]
pub struct SkeletalAnimation(AnimationState);
```

#### System Implementation

```rust
pub fn skeletal_animation_system(
    mut query_animations: Query<(Entity, &mut SkeletalAnimation, Option<&SkinnedMesh>)>,
    mut query_transform: Query<&mut Transform>,
    mut animation_frame_events: MessageWriter<AnimationFrameEvent>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    game_data: Res<GameData>,
    time: Res<Time>,
) {
    for (entity, mut skeletal_animation, skinned_mesh) in query_animations.iter_mut() {
        if skeletal_animation.completed() {
            continue;
        }

        let zmo_asset = motion_assets.get(skeletal_animation.motion()).continue_if_none();
        
        // Advance animation state
        let animation = &mut skeletal_animation.0;
        animation.advance(zmo_asset, &time);

        // Process animation events
        animation.iter_animation_events(zmo_asset, |event_id| {
            if let Some(flags) = game_data.animation_event_flags.get(event_id as usize) {
                animation_frame_events.write(AnimationFrameEvent::new(entity, *flags));
            }
        });

        let Some(skinned_mesh) = skinned_mesh else { continue };
        
        // Sample animation data with interpolation
        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();
        let interpolate_weight = animation
            .interpolate_weight()
            .map(|w| (w * FRAC_PI_2).sin());

        // Apply transforms to bones
        for (bone_id, bone_entity) in skinned_mesh.joints.iter().enumerate() {
            let Ok(mut bone_transform) = query_transform.get_mut(*bone_entity) else {
                continue;
            };

            // Interpolate translation
            if let Some(translation) = zmo_asset.sample_translation(
                bone_id, current_frame_fract, current_frame_index, next_frame_index,
            ) {
                bone_transform.translation = if let Some(weight) = interpolate_weight {
                    bone_transform.translation.lerp(translation, weight)
                } else {
                    translation
                };
            }

            // Slerp rotation
            if let Some(rotation) = zmo_asset.sample_rotation(
                bone_id, current_frame_fract, current_frame_index, next_frame_index,
            ) {
                bone_transform.rotation = if let Some(weight) = interpolate_weight {
                    bone_transform.rotation.slerp(rotation, weight)
                } else {
                    rotation
                };
            }
        }
    }
}
```

---

### MeshAnimation

Vertex morph animation for effect meshes using texture-based storage. Source: `src/animation/mesh_animation.rs:1-80`

```rust
#[derive(Component, Reflect, Deref, DerefMut)]
pub struct MeshAnimation(AnimationState);
```

#### System Implementation

```rust
pub fn mesh_animation_system(
    mut query: Query<(
        &mut MeshAnimation,
        Entity,
        Option<&MeshMaterial3d<ExtendedMaterial<..., RoseEffectExtension>>>
    ), With<EffectMesh>>,
    mut effect_mesh_materials: ResMut<Assets<...>>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (mut mesh_animation, entity, material_component) in query.iter_mut() {
        if mesh_animation.completed() {
            continue;
        }

        let zmo_asset = motion_assets.get(mesh_animation.motion()).continue_if_none();
        
        // Advance animation
        let anim_state = &mut mesh_animation.0;
        anim_state.advance(zmo_asset, &time);
        
        // Update material animation uniform
        if let Some(material_handle) = material_component.map(|m| m.0.clone()) {
            if let Some(material) = effect_mesh_materials.get_mut(&material_handle) {
                if material.extension.animation_texture.is_some() {
                    update_effect_mesh_animation_material(
                        &mut material.extension.animation_state, 
                        zmo_asset, 
                        anim_state
                    );
                }
            }
        }
    }
}
```

#### GPU Animation Uniform

```rust
fn update_effect_mesh_animation_material(
    uniform: &mut EffectMeshAnimationUniform,
    zmo_asset: &ZmoAsset,
    anim_state: &AnimationState,
) {
    // Build flags: bits 0-3 = animation type, bits 4-31 = num_frames
    let mut flags: u32 = 0;
    if let Some(texture_data) = &zmo_asset.animation_texture {
        if texture_data.has_position_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_POSITION;
        }
        if texture_data.has_normal_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_NORMAL;
        }
        if texture_data.has_uv1_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_UV;
        }
        if texture_data.has_alpha_channel {
            flags |= EFFECT_MESH_ANIMATION_FLAG_ALPHA;
        }
        uniform.alpha = texture_data.alphas.get(anim_state.current_frame_index()).copied().unwrap_or(1.0);
    }
    flags |= (zmo_asset.num_frames as u32) << 4;
    
    uniform.flags = flags;
    
    // Pack frame indices: lower 16 bits = current, upper 16 bits = next
    let current_frame = anim_state.current_frame_index() as u32 & 0xFFFF;
    let next_frame = anim_state.next_frame_index() as u32 & 0xFFFF;
    uniform.current_next_frame = current_frame | (next_frame << 16);
    
    // Interpolation weight
    uniform.next_weight = anim_state.current_frame_fract();
}
```

---

### CameraAnimation

Cinematic camera animation with FOV control. Source: `src/animation/camera_animation.rs:1-80`

```rust
#[derive(Component, Reflect, Deref, DerefMut)]
pub struct CameraAnimation(AnimationState);
```

#### Channel Mapping

| Bone ID | Purpose |
|---------|---------|
| 0 | Camera eye position |
| 1 | Camera target (look-at) position |
| 2 | Camera up vector |
| 3 | FOV/near/far plane (x=fov, y=far, z=near) |

#### System Implementation

```rust
pub fn camera_animation_system(
    mut query_animations: Query<(
        &mut CameraAnimation,
        Option<&mut Transform>,
        Option<&mut Projection>,
    )>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (mut camera_animation, transform, projection) in query_animations.iter_mut() {
        if camera_animation.completed() {
            continue;
        }

        let zmo_asset = motion_assets.get(camera_animation.motion()).continue_if_none();
        
        let animation = &mut camera_animation.0;
        animation.advance(zmo_asset, &time);

        let (Some(mut transform), Some(mut projection)) = (transform, projection) else {
            continue;
        };
        
        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();
        
        // Sample camera parameters (offset by world center)
        let eye = zmo_asset.sample_translation(
            0, current_frame_fract, current_frame_index, next_frame_index,
        ).map(|e| e + Vec3::new(5200.0, 0.0, -5200.0));
        
        let center = zmo_asset.sample_translation(
            1, current_frame_fract, current_frame_index, next_frame_index,
        ).map(|e| e + Vec3::new(5200.0, 0.0, -5200.0));
        
        let up = zmo_asset.sample_translation(
            2, current_frame_fract, current_frame_index, next_frame_index,
        ).unwrap_or(Vec3::Y);
        
        let fov_near_far = zmo_asset.sample_translation(
            3, current_frame_fract, current_frame_index, next_frame_index,
        );

        // Update camera transform
        if let (Some(eye), Some(center)) = (eye, center) {
            *transform = Transform::from_translation(eye).looking_at(center, up);
        }

        // Update projection
        if let Some(fov_near_far) = fov_near_far {
            if let Projection::Perspective(ref mut perspective) = &mut *projection {
                perspective.fov = (fov_near_far.x * 100.0).to_radians();
                perspective.near = -fov_near_far.z;
                perspective.far = fov_near_far.y * 10.0;
            }
        }
    }
}
```

---

### TransformAnimation

Simple transform interpolation animation. Source: `src/animation/transform_animation.rs:1-80`

```rust
#[derive(Component, Reflect, Deref, DerefMut)]
pub struct TransformAnimation(AnimationState);
```

#### System Implementation

```rust
pub fn transform_animation_system(
    mut query_animations: Query<(&mut TransformAnimation, Option<&mut Transform>)>,
    motion_assets: Res<Assets<ZmoAsset>>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
) {
    for (mut transform_animation, transform) in query_animations.iter_mut() {
        if transform_animation.completed() {
            continue;
        }

        let zmo_asset = motion_assets.get(transform_animation.motion()).continue_if_none();
        
        let animation = &mut transform_animation.0;
        animation.advance(zmo_asset, &time);

        let Some(mut transform) = transform else { continue };
        
        let current_frame_fract = animation.current_frame_fract();
        let current_frame_index = animation.current_frame_index();
        let next_frame_index = animation.next_frame_index();

        // Channel 0 contains translation, rotation, and scale
        if let Some(translation) = zmo_asset.sample_translation(
            0, current_frame_fract, current_frame_index, next_frame_index,
        ) {
            transform.translation = translation;
        }

        if let Some(rotation) = zmo_asset.sample_rotation(
            0, current_frame_fract, current_frame_index, next_frame_index,
        ) {
            transform.rotation = rotation;
        }

        if let Some(scale) = zmo_asset.sample_scale(
            0, current_frame_fract, current_frame_index, next_frame_index,
        ) {
            transform.scale = Vec3::splat(scale);
        }
    }
}
```

---

## ZMO Asset Format

Custom animation file format loader. Source: `src/animation/zmo_asset_loader.rs:1-200`

### ZmoAsset Structure

```rust
#[derive(Reflect)]
pub struct ZmoAsset {
    pub num_frames: usize,
    pub fps: usize,
    pub frame_events: Vec<u16>,
    pub interpolation_interval: f32,
    pub bones: Vec<ZmoAssetBone>,
    pub animation_texture: Option<ZmoAssetAnimationTexture>,
}

#[derive(Reflect, Clone, Default)]
pub struct ZmoAssetBone {
    pub translation: Vec<Vec3>,
    pub rotation: Vec<Quat>,
    pub scale: Vec<f32>,
}

#[derive(Reflect, Clone, Default)]
pub struct ZmoAssetAnimationTexture {
    pub texture: Handle<Image>,
    pub alphas: Vec<f32>,
    pub has_position_channel: bool,
    pub has_normal_channel: bool,
    pub has_alpha_channel: bool,
    pub has_uv1_channel: bool,
}
```

### Asset Loaders

#### ZmoAssetLoader

Loads `.zmo` files for skeletal and transform animations.

```rust
impl AssetLoader for ZmoAssetLoader {
    type Asset = ZmoAsset;
    
    fn extensions(&self) -> &[&str] {
        &["zmo", "ZMO"]
    }
    
    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send {
        async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            
            let zmo = ZmoFile::read((&bytes).into(), &Default::default())?;
            
            // Determine if this is a camera/morph animation (no bone IDs)
            let mut max_bone_id = 0;
            for (bone_id, _) in zmo.channels.iter() {
                max_bone_id = max_bone_id.max(*bone_id);
            }
            let assign_bone_id = max_bone_id == 0 && zmo.channels.len() > 2;
            
            // Parse bone channels
            let mut bones = vec![ZmoAssetBone::default(); (max_bone_id + 1) as usize];
            for (channel_id, (bone_id, channel)) in zmo.channels.iter().enumerate() {
                let bone = if !assign_bone_id {
                    &mut bones[*bone_id as usize]
                } else {
                    &mut bones[channel_id]
                };
                
                match channel {
                    ZmoChannel::Position(positions) => {
                        bone.translation = positions
                            .iter()
                            .map(|p| Vec3::new(p.x, p.z, -p.y) / 100.0)
                            .collect();
                    }
                    ZmoChannel::Rotation(rotations) => {
                        bone.rotation = rotations
                            .iter()
                            .map(|r| Quat::from_xyzw(r.x, r.z, -r.y, r.w))
                            .collect();
                    }
                    ZmoChannel::Scale(scales) => {
                        bone.scale = scales.clone();
                    }
                    _ => {}
                }
            }
            
            Ok(ZmoAsset {
                num_frames: zmo.num_frames,
                fps: zmo.fps,
                bones,
                frame_events: zmo.frame_events,
                interpolation_interval: (zmo.interpolation_interval_ms.unwrap_or(500) as f32 / 1000.0).max(0.0001),
                animation_texture: None,
            })
        }
    }
}
```

#### ZmoTextureAssetLoader

Loads `.zmo_texture` files for mesh morph animations.

```rust
impl AssetLoader for ZmoTextureAssetLoader {
    type Asset = ZmoAsset;
    
    fn extensions(&self) -> &[&str] {
        &["zmo_texture", "ZMO_TEXTURE"]
    }
    
    fn load(...) -> impl Future<...> {
        async move {
            // Parse ZMO file with vertex channels
            let zmo = ZmoFile::read((&bytes).into(), &Default::default())?;
            
            // Pack animation data into RGBA32 texture
            // Layout: x=frame, y=vertex, rgba=(pos.x, pos.y, pos.z, uv.x) or (normal.x, normal.y, normal.z, uv.y)
            let mut image_data = vec![0.0; num_vertices * stride * 16];
            
            for (vertex_id, channel) in zmo.channels.iter() {
                match channel {
                    ZmoChannel::Position(values) => {
                        // Pack position data
                    }
                    ZmoChannel::Normal(values) => {
                        // Pack normal data
                    }
                    ZmoChannel::UV1(values) => {
                        // Pack UV data
                    }
                    ZmoChannel::Alpha(values) => {
                        alphas = values.clone();
                    }
                    _ => {}
                }
            }
            
            // Create texture asset
            let texture_handle = load_context.add_labeled_asset(
                "image".to_string(),
                Image::new(
                    Extent3d { width: stride as u32, height: num_vertices as u32, .. },
                    TextureDimension::D2,
                    image_data,
                    TextureFormat::Rgba32Float,
                    RenderAssetUsages::default(),
                )
            );
            
            Ok(ZmoAsset {
                num_frames: zmo.num_frames,
                fps: zmo.fps,
                frame_events: zmo.frame_events,
                interpolation_interval: ...,
                bones: Vec::new(),
                animation_texture: Some(ZmoAssetAnimationTexture {
                    texture: texture_handle,
                    alphas,
                    has_position_channel,
                    has_normal_channel,
                    has_alpha_channel,
                    has_uv1_channel,
                }),
            })
        }
    }
}
```

### Sampling Methods

```rust
impl ZmoAsset {
    // Get raw frame data
    pub fn get_translation(&self, bone_id: usize, frame_id: usize) -> Option<Vec3>
    pub fn get_rotation(&self, bone_id: usize, frame_id: usize) -> Option<Quat>
    pub fn get_scale(&self, bone_id: usize, frame_id: usize) -> Option<f32>
    pub fn get_frame_event(&self, frame_id: usize) -> Option<NonZeroU16>
    
    // Interpolated sampling
    pub fn sample_translation(
        &self,
        channel_id: usize,
        current_frame_fract: f32,
        current_frame_index: usize,
        next_frame_index: usize,
    ) -> Option<Vec3> {
        let current = self.get_translation(channel_id, current_frame_index)?;
        let next = self.get_translation(channel_id, next_frame_index)?;
        Some(current.lerp(next, current_frame_fract))
    }
    
    pub fn sample_rotation(...) -> Option<Quat> {
        let current = self.get_rotation(channel_id, current_frame_index)?;
        let next = self.get_rotation(channel_id, next_frame_index)?;
        Some(current.slerp(next, current_frame_fract))
    }
    
    pub fn sample_scale(...) -> Option<f32> {
        let current = self.get_scale(channel_id, current_frame_index)?;
        let next = self.get_scale(channel_id, next_frame_index)?;
        Some(current + (next - current) * current_frame_fract)
    }
}
```

---

## Animation Events

Frame-based event system for triggering effects and sounds. Source: `src/animation/animation_state.rs:400-500`

### AnimationFrameEvent

```rust
#[derive(Message)]
pub struct AnimationFrameEvent {
    pub entity: Entity,
    pub flags: AnimationEventFlags,
}

impl AnimationFrameEvent {
    pub fn new(entity: Entity, flags: AnimationEventFlags) -> Self
}
```

### Event Processing in AnimationState

```rust
pub fn iter_animation_events(
    &mut self,
    zmo_asset: &ZmoAsset,
    mut event_handler: impl FnMut(u16),
) {
    let num_frames = zmo_asset.num_frames;
    let current_event_frame = self.current_frame_index + self.current_loop_count * num_frames;

    // Emit events for all frames we've passed since last check
    while self.last_absolute_event_frame <= current_event_frame {
        if let Some(event_id) = zmo_asset.get_frame_event(self.last_absolute_event_frame % num_frames) {
            event_handler(event_id.get());
        }
        self.last_absolute_event_frame += 1;
    }
}
```

### AnimationEventFlags

Event flags define what happens at specific frames. Common flags include:

#### Effect Events
- `EFFECT_WEAPON_ATTACK_HIT` - Trigger hit effect
- `EFFECT_WEAPON_FIRE_BULLET` - Fire projectile
- `EFFECT_SKILL_FIRE_BULLET` - Fire skill projectile
- `EFFECT_SKILL_ACTION` - Execute skill action
- `EFFECT_SKILL_HIT` - Skill hit effect
- `EFFECT_SKILL_CASTING_0-3` - Skill casting effects
- `EFFECT_MOVE_VEHICLE_DUMMY1/2` - Vehicle movement effects

#### Sound Events
- `SOUND_FOOTSTEP` - Footstep sound
- `SOUND_WEAPON_ATTACK_START` - Weapon swing sound
- `SOUND_WEAPON_ATTACK_HIT` - Hit sound
- `SOUND_WEAPON_FIRE_BULLET` - Fire sound
- `SOUND_SKILL_FIRE_BULLET` - Skill fire sound
- `SOUND_SKILL_HIT` - Skill hit sound
- `SOUND_MOVE_VEHICLE_DUMMY1/2` - Vehicle sounds

### Animation Effect System

Source: `src/systems/animation_effect_system.rs:1-100`

```rust
pub fn animation_effect_system(
    mut animation_frame_events: MessageReader<AnimationFrameEvent>,
    mut spawn_effect_events: MessageWriter<SpawnEffectEvent>,
    mut spawn_projectile_events: MessageWriter<SpawnProjectileEvent>,
    mut hit_events: MessageWriter<HitEvent>,
    query_event_entity: Query<EventEntity>,
    game_data: Res<GameData>,
) {
    for event in animation_frame_events.read() {
        let event_entity = query_event_entity.get(event.entity).ok()?;
        let target_entity = event_entity.command.get_target();

        // Handle effect events
        if event.flags.contains(AnimationEventFlags::EFFECT_WEAPON_ATTACK_HIT) {
            // Spawn hit effect
            hit_events.write(HitEvent::with_weapon(...));
        }
        
        if event.flags.contains(AnimationEventFlags::EFFECT_WEAPON_FIRE_BULLET) {
            // Spawn projectile
            spawn_projectile_events.write(SpawnProjectileEvent { ... });
        }
        
        // Handle skill events, vehicle events, etc.
    }
}
```

### Animation Sound System

Source: `src/systems/animation_sound_system.rs:1-100`

```rust
pub fn animation_sound_system(
    mut commands: Commands,
    mut animation_frame_events: MessageReader<AnimationFrameEvent>,
    game_data: Res<GameData>,
    asset_server: Res<AssetServer>,
    // ... other resources
) {
    for event in animation_frame_events.read() {
        // Footstep sounds
        if event.flags.contains(AnimationEventFlags::SOUND_FOOTSTEP) {
            let sound_data = game_data.sounds.get_step_sound(...);
            spawn_sound(...);
        }
        
        // Weapon sounds
        if event.flags.contains(AnimationEventFlags::SOUND_WEAPON_ATTACK_START) {
            let sound_data = get_weapon_attack_sound(...);
            spawn_sound(...);
        }
        
        // Skill sounds
        if event.flags.contains(AnimationEventFlags::SOUND_SKILL_HIT) {
            let sound_data = get_skill_hit_sound(...);
            spawn_sound(...);
        }
    }
}
```

---

## Animation Blending

The animation system supports two types of blending. Source: `src/animation/animation_state.rs:250-350`, `src/animation/skeletal_animation.rs:100-200`

### Frame Interpolation

Linear interpolation between consecutive frames for smooth animation:

```rust
// Translation: linear interpolation
let translation = current.lerp(next, current_frame_fract);

// Rotation: spherical linear interpolation
let rotation = current.slerp(next, current_frame_fract);

// Scale: linear interpolation
let scale = current + (next - current) * current_frame_fract;
```

### Animation Interval Blending

Gradual transition when starting an animation using `interpolation_interval`:

```rust
// In AnimationState::advance
if self.interpolate_weight < 1.0 {
    self.interpolate_weight += time.delta_secs() / zmo_asset.interpolation_interval;
}

// In skeletal_animation_system
let interpolate_weight = animation
    .interpolate_weight()
    .map(|w| (w * FRAC_PI_2).sin()); // Eased transition

if let Some(weight) = interpolate_weight {
    bone_transform.translation = bone_transform.translation.lerp(target, weight);
    bone_transform.rotation = bone_transform.rotation.slerp(target, weight);
} else {
    bone_transform.translation = target;
    bone_transform.rotation = target;
}
```

---

## Animation State Machines

Animation state management is handled through the `Command` component and `command_system`. Source: `src/systems/command_system.rs:1-200`

### Command-Based Animation Selection

```rust
pub fn command_system(
    // ... queries and resources
    mut query_animation: Query<Option<&mut SkeletalAnimation>>,
    // ...
) {
    for (entity, command, character_model, /* ... */) in query.iter_mut() {
        // Get motion handle based on command type
        let motion = match command {
            Command::Move { move_mode, .. } => {
                get_move_animation(*move_mode, character_model, npc_model, vehicle)
            }
            Command::Attack { .. } => {
                get_attack_animation(&mut rng, character_model, npc_model, vehicle)
            }
            Command::Die => get_die_animation(character_model, npc_model),
            Command::Sit => get_sitting_animation(character_model, npc_model),
            Command::Stop => get_stop_animation(character_model, npc_model, vehicle),
            // ...
        };
        
        // Update animation if changed
        if let Some(motion) = motion {
            update_active_motion(
                &mut query_animation,
                entity,
                motion,
                animation_speed,
            );
        }
        
        // Wait for animation completion if required
        let requires_animation_complete = command.requires_animation_complete();
        let active_motion_completed = query_animation
            .get(entity)
            .ok()
            .flatten()
            .map_or(true, |anim| anim.completed());
        
        if !active_motion_completed && requires_animation_complete {
            continue; // Still animating
        }
    }
}
```

### Animation Selection Functions

```rust
fn get_move_animation(
    move_mode: MoveMode,
    character_model: &CharacterModel,
    npc_model: Option<&NpcModel>,
    vehicle: Option<&VehicleModel>,
) -> Option<Handle<ZmoAsset>> {
    match move_mode {
        MoveMode::Foot => {
            // Select walk/run animation based on speed
            if speed > RUN_THRESHOLD {
                character_model.get_motion(CharacterMotionAction::Run)
            } else {
                character_model.get_motion(CharacterMotionAction::Walk)
            }
        }
        MoveMode::Drive => {
            vehicle.and_then(|v| v.get_motion(VehicleMotionAction::Move))
        }
        // ...
    }
}

fn get_attack_animation(
    rng: &mut R,
    character_model: &CharacterModel,
    npc_model: Option<&NpcModel>,
    vehicle: Option<&VehicleModel>,
) -> Option<Handle<ZmoAsset>> {
    // Select random attack animation (1-3)
    let attack_num = rng.gen_range(1..=4);
    character_model.get_motion(match attack_num {
        1 => CharacterMotionAction::Attack1,
        2 => CharacterMotionAction::Attack2,
        3 => CharacterMotionAction::Attack3,
        _ => CharacterMotionAction::Attack1,
    })
}

fn get_die_animation(...) -> Option<Handle<ZmoAsset>>
fn get_sitting_animation(...) -> Option<Handle<ZmoAsset>>
fn get_stop_animation(...) -> Option<Handle<ZmoAsset>>
fn get_pickup_animation(...) -> Option<Handle<ZmoAsset>>
```

### Animation Speed Control

```rust
fn get_move_animation_speed(move_speed: &MoveSpeed) -> f32 {
    // Scale animation speed with movement speed
    move_speed.speed / BASE_WALK_SPEED
}

fn get_attack_animation_speed(ability_values: &AbilityValues) -> f32 {
    // Faster attack speed with higher attack speed stat
    1.0 + (ability_values.attack_speed - 100.0) / 1000.0
}

fn get_vehicle_move_animation_speed(move_speed: &MoveSpeed) -> f32 {
    // Vehicle animation speed scaling
    move_speed.speed / BASE_VEHICLE_SPEED
}
```

---

## Configuration Options

### Animation Speed Control

Animation speed can be controlled per-animation or globally:

| Setting | Default | Description | Source |
|---------|---------|-------------|--------|
| `animation_speed` | `1.0` | Per-animation speed multiplier | `src/animation/animation_state.rs:35` |
| `interpolation_interval` | `0.5` | Blend-in duration in seconds | `src/animation/zmo_asset_loader.rs:150` |

### Loop Configuration

| Setting | Default | Description | Source |
|---------|---------|-------------|--------|
| `max_loop_count` | `None` | Maximum loop iterations (None = infinite) | `src/animation/animation_state.rs:42` |
| `current_loop_count` | `0` | Current loop iteration | `src/animation/animation_state.rs:45` |

### Timing Configuration

| Setting | Default | Description | Source |
|---------|---------|-------------|--------|
| `start_delay` | `None` | Delay before animation starts (seconds) | `src/animation/animation_state.rs:65` |
| `start_time` | `None` | Animation start timestamp | `src/animation/animation_state.rs:58` |

### Frame Interpolation

| Setting | Default | Description | Source |
|---------|---------|-------------|--------|
| `current_frame_fract` | `0.0` | Interpolation weight (0.0 to 1.0) | `src/animation/animation_state.rs:55` |
| `interpolate_weight` | `0.0` | Animation interval blend weight | `src/animation/animation_state.rs:48` |

---

## Common Patterns

### Pattern 1: One-Shot Animation

Play an animation once and handle completion:

```rust
// src/animation/animation_state.rs:100-120
commands.spawn(SkeletalAnimation::once(animation_handle));
```

### Pattern 2: Looping Idle Animation

Continuous looping for idle states:

```rust
// src/animation/animation_state.rs:125-145
commands.spawn(SkeletalAnimation::repeat(idle_handle, None));
```

### Pattern 3: Limited Loop Animation

Play animation a specific number of times:

```rust
// src/animation/animation_state.rs:150-170
commands.spawn(SkeletalAnimation::repeat(wave_handle, Some(3)));
```

### Pattern 4: Speed-Scaled Movement

Scale animation speed with movement speed:

```rust
// src/systems/command_system.rs:300-350
let speed = move_speed.speed / BASE_WALK_SPEED;
SkeletalAnimation::once(walk_handle).with_animation_speed(speed);
```

### Pattern 5: Delayed Animation Start

Start animation after a delay:

```rust
// src/animation/animation_state.rs:175-195
SkeletalAnimation::once(hit_handle).with_start_delay(0.1);
```

### Pattern 6: Event-Driven Effects

Trigger effects at specific animation frames:

```rust
// src/systems/animation_effect_system.rs:50-150
animation.iter_animation_events(zmo_asset, |event_id| {
    // Spawn effect, play sound, etc.
});
```

### Pattern 7: Animation Completion Chain

Chain animations based on completion:

```rust
// src/systems/command_system.rs:400-450
if animation.completed() {
    // Start next animation
}
```

### Pattern 8: Smooth Animation Blending

Blend between animations using interpolation interval:

```rust
// src/animation/skeletal_animation.rs:120-180
let weight = (interpolate_weight * FRAC_PI_2).sin();
bone_transform.translation = current.lerp(target, weight);
```

---

## Code Examples

### Basic Skeletal Animation

```rust
use bevy::prelude::*;
use crate::animation::{SkeletalAnimation, ZmoAsset};

fn spawn_animated_character(commands: &mut Commands, asset_server: &AssetServer) {
    // Load animation asset
    let idle_animation: Handle<ZmoAsset> = asset_server.load("3DDATA/MOTION/CHAR/IDLE.ZMO");
    
    commands.spawn((
        // Animation component - plays once
        SkeletalAnimation::once(idle_animation.clone()),
        
        // Or for looping animation
        // SkeletalAnimation::repeat(idle_animation, None),
        
        // Or with loop limit
        // SkeletalAnimation::repeat(idle_animation, Some(3)),
        
        // With custom speed
        // SkeletalAnimation::once(idle_animation)
        //     .with_animation_speed(1.5),
        
        // Skinned mesh component (required for skeletal animation)
        SkinnedMesh { ... },
        
        Transform::default(),
        GlobalTransform::default(),
        // ... other components
    ));
}
```

### Camera Animation

```rust
use crate::animation::{CameraAnimation, ZmoAsset};

fn spawn_camera_animation(commands: &mut Commands, asset_server: &AssetServer) {
    let camera_anim: Handle<ZmoAsset> = asset_server.load("3DDATA/TITLE/CAMERA01_INTRO01.ZMO");
    
    commands.spawn((
        CameraAnimation::once(camera_anim),
        Camera3d::default(),
        Transform::default(),
        GlobalTransform::default(),
        Projection::Perspective(PerspectiveProjection::default()),
    ));
}
```

### Transform Animation

```rust
use crate::animation::{TransformAnimation, ZmoAsset};

fn spawn_floating_object(commands: &mut Commands, asset_server: &AssetServer) {
    let float_anim: Handle<ZmoAsset> = asset_server.load("animations/float.ZMO");
    
    commands.spawn((
        // Loop forever
        TransformAnimation::repeat(float_anim, None),
        Transform::from_translation(Vec3::new(0.0, 5.0, 0.0)),
        GlobalTransform::default(),
        // ... mesh and material
    ));
}
```

### Mesh Morph Animation

```rust
use crate::animation::{MeshAnimation, ZmoAsset};

fn spawn_morph_effect(commands: &mut Commands, asset_server: &AssetServer) {
    let morph_anim: Handle<ZmoAsset> = asset_server.load("effects/wave.ZMO_TEXTURE");
    
    commands.spawn((
        MeshAnimation::once(morph_anim)
            .with_start_delay(0.5), // Start after 0.5 seconds
        EffectMesh,
        MeshMaterial3d(material_handle),
        // ... mesh and transform
    ));
}
```

### Animation Event Handling

```rust
use crate::animation::AnimationFrameEvent;
use rose_data::AnimationEventFlags;

fn on_animation_event(
    mut events: EventReader<AnimationFrameEvent>,
    mut commands: Commands,
) {
    for event in events.read() {
        if event.flags.contains(AnimationEventFlags::EFFECT_WEAPON_FIRE_BULLET) {
            // Spawn projectile at animation frame
            commands.spawn(Projectile {
                source: event.entity,
                // ...
            });
        }
        
        if event.flags.contains(AnimationEventFlags::SOUND_FOOTSTEP) {
            // Play footstep sound
            play_footstep_sound(event.entity);
        }
    }
}
```

### Animation Completion Detection

```rust
use crate::animation::SkeletalAnimation;

fn on_animation_complete(
    mut query: Query<(Entity, &SkeletalAnimation), Changed<SkeletalAnimation>>,
    mut commands: Commands,
) {
    for (entity, animation) in query.iter_mut() {
        if animation.completed() {
            // Animation finished - trigger next action
            commands.entity(entity).insert(NextState::Ready);
        }
    }
}
```

### Dynamic Animation Control

```rust
use crate::animation::SkeletalAnimation;

fn control_animation(
    mut query: Query<&mut SkeletalAnimation>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for mut anim in query.iter_mut() {
        // Change speed based on input
        if keyboard.pressed(KeyCode::ShiftLeft) {
            anim.set_animation_speed(2.0); // Run faster
        } else {
            anim.set_animation_speed(1.0); // Normal speed
        }
        
        // Change loop count dynamically
        if keyboard.just_pressed(KeyCode::Space) {
            anim.set_max_loop_count(Some(1)); // Play once then stop
        }
    }
}
```

---

## Troubleshooting

### Bevy 0.18 Migration Issues

#### Issue 1: Time Resource Type Changes

**Problem**: After migrating to Bevy 0.18, animation systems receive incorrect delta time values or time doesn't advance properly.

**Root Cause**: Bevy 0.18 changed how `Time` generic types work. The default `Time<()>` now behaves differently in different system schedules.

**Solution**: Explicitly specify the time type needed:

```rust
// Before (Bevy 0.11-0.14)
fn animation_system(time: Res<Time>) {
    let delta = time.delta_secs();
}

// After (Bevy 0.18) - Use Time<Virtual> for game time
fn animation_system(time: Res<Time<Virtual>>) {
    let delta = time.delta_secs();
}

// Or use Time<Real> for wall-clock time (unaffected by pause/scale)
fn animation_system(time: Res<Time<Real>>) {
    let delta = time.delta_secs();
}
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\time.rs:50-80`, `virt.rs:1-50`

---

#### Issue 2: Timer::just_finished() Returns False Positives

**Problem**: Timer's `just_finished()` returns true multiple times or doesn't trigger at all after migration.

**Root Cause**: Bevy 0.18 changed timer tick behavior. Timers now need to be explicitly ticked with the correct delta time type.

**Solution**: Ensure timer is ticked with `Time::delta()` not `Time::delta_secs()`:

```rust
// Incorrect
timer.tick(time.delta_secs()); // Type mismatch!

// Correct
timer.tick(time.delta()); // Returns Duration
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\timer.rs:200-250`

---

#### Issue 3: Animation Frame Rate Inconsistency

**Problem**: Animations play at different speeds on different hardware after migration to Bevy 0.18.

**Root Cause**: Using `Fixed` timestep for animation updates instead of variable timestep.

**Solution**: Use `Time<Virtual>` or `Time<Real>` for animations, reserve `Time<Fixed>` for physics:

```rust
// Animation systems - use variable time
fn skeletal_animation_system(time: Res<Time<Virtual>>, ...) {
    let delta = time.delta_secs(); // Variable, frame-dependent
}

// Physics systems - use fixed time  
fn physics_system(time: Res<Time<Fixed>>, ...) {
    let delta = time.delta_secs(); // Fixed, deterministic
}
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\fixed.rs:1-100`

---

#### Issue 4: Wrap Period Causing Time Resets

**Problem**: Animation elapsed time suddenly resets after running for extended periods.

**Root Cause**: Bevy's wrap period prevents floating-point precision loss but can cause unexpected resets if not handled.

**Solution**: Use `elapsed_secs_wrapped()` for long-running animations:

```rust
// For short animations (< 1 hour)
let elapsed = time.elapsed_secs();

// For long-running animations (cutscenes, etc.)
let elapsed = time.elapsed_secs_wrapped();
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\time.rs:300-320`

---

#### Issue 5: Timer Mode Changes in 0.18

**Problem**: Repeating timers don't wrap correctly or `times_finished_this_tick()` returns unexpected values.

**Root Cause**: Bevy 0.18 changed how repeating timers handle multiple completions in one frame.

**Solution**: Check `times_finished_this_tick()` for repeating timers:

```rust
timer.tick(time.delta());

if timer.mode() == TimerMode::Repeating {
    let completions = timer.times_finished_this_tick();
    for _ in 0..completions {
        // Handle each completion
    }
} else if timer.just_finished() {
    // Handle one-time completion
}
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\timer.rs:380-400`

---

#### Issue 6: System Scheduling and Time Updates

**Problem**: Animation systems run before time is updated, receiving stale delta values.

**Root Cause**: Bevy 0.18 changed the default system execution order. Time updates happen in `First` schedule.

**Solution**: Ensure animation systems run in `PostUpdate` after time has been updated:

```rust
app.add_systems(
    PostUpdate, // Not Update!
    skeletal_animation_system.in_set(RoseAnimationSystem),
);
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\lib.rs:100-150`

---

#### Issue 7: Virtual Time Pause Affecting Animations

**Problem**: All animations freeze when virtual time is paused, including UI animations that should continue.

**Root Cause**: Using `Time<Virtual>` for all animations when some should continue during pause.

**Solution**: Use `Time<Real>` for UI and menu animations:

```rust
// Game animations - pause with game
fn game_animation_system(time: Res<Time<Virtual>>, ...) { }

// UI animations - always run
fn ui_animation_system(time: Res<Time<Real>>, ...) { }
```

Source: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\real.rs:1-50`, `virt.rs:50-100`

---

#### Issue 8: Delta Time Spikes Causing Animation Jumps

**Problem**: Animations jump forward when frame rate drops or garbage collection occurs.

**Root Cause**: No delta time clamping, large delta values passed to animation systems.

**Solution**: Clamp delta time in animation systems:

```rust
const MAX_DELTA_TIME: f32 = 0.1; // 100ms max

fn skeletal_animation_system(time: Res<Time>, mut query: Query<&mut SkeletalAnimation>) {
    let delta = time.delta_secs().min(MAX_DELTA_TIME);
    
    for mut anim in query.iter_mut() {
        anim.advance(delta);
    }
}
```

---

#### Issue 9: Interpolation Weight Not Resetting

**Problem**: Animation blend-in doesn't work when switching to the same animation.

**Root Cause**: `interpolate_weight` not reset when motion handle changes.

**Solution**: Reset interpolation weight when changing animations:

```rust
// In animation state update
if new_motion != current_motion {
    animation.interpolate_weight = 0.0;
    animation.current_frame_fract = 0.0;
}
```

Source: `src/animation/animation_state.rs:250-280`

---

#### Issue 10: Frame Event Duplication

**Problem**: Animation frame events fire multiple times for the same frame.

**Root Cause**: `last_absolute_event_frame` not properly tracked across loop boundaries.

**Solution**: Ensure event frame tracking uses absolute frame count:

```rust
// In AnimationState::iter_animation_events
let current_event_frame = self.current_frame_index + self.current_loop_count * num_frames;

while self.last_absolute_event_frame <= current_event_frame {
    // Process event once per frame
    self.last_absolute_event_frame += 1;
}
```

Source: `src/animation/animation_state.rs:400-450`

---

### Common Animation Issues

#### Issue 11: Bones Not Animating

**Symptom**: Skinned mesh doesn't animate, bones stay in bind pose.

**Debug Steps**:
1. Verify `SkinnedMesh` component has `joints` populated
2. Check ZMO asset loaded successfully (not `None` in assets)
3. Ensure bone entity IDs in `SkinnedMesh::joints` match animation bone IDs

**Source**: `src/animation/skeletal_animation.rs:80-120`

---

#### Issue 12: Animation Playing Backwards

**Symptom**: Animation plays in reverse direction.

**Cause**: Negative animation speed or Y-axis coordinate system mismatch.

**Fix**:
```rust
// Ensure positive speed
anim.set_animation_speed(speed.abs());

// Check ZMO loader Y-axis conversion
// src/animation/zmo_asset_loader.rs:180-200
```

---

#### Issue 13: Camera Animation FOV Not Updating

**Symptom**: Camera position animates but FOV stays constant.

**Cause**: Projection component not mutable or FOV channel (bone 3) missing in ZMO.

**Fix**: Ensure query includes `&mut Projection`:
```rust
Query<(
    &mut CameraAnimation,
    Option<&mut Transform>,
    Option<&mut Projection>, // Required for FOV
)>
```

Source: `src/animation/camera_animation.rs:60-100`

---

#### Issue 14: Mesh Morph Animation Texture Not Updating

**Symptom**: Effect mesh doesn't morph, stays static.

**Debug Steps**:
1. Verify `ZmoTextureAssetLoader` registered (not `ZmoAssetLoader`)
2. Check animation texture channels match mesh vertex count
3. Ensure material has `animation_texture` set in extension

**Source**: `src/animation/mesh_animation.rs:80-150`, `src/animation/zmo_asset_loader.rs:300-400`

---

#### Issue 15: Animation Events Not Firing

**Symptom**: Footstep sounds, hit effects don't trigger.

**Debug Steps**:
1. Verify `AnimationFrameEvent` message channel registered in plugin
2. Check `GameData::animation_event_flags` populated
3. Ensure event system runs after animation systems

**Source**: `src/animation/mod.rs:30-50`, `src/systems/animation_effect_system.rs:1-50`

---

## Source File References

### Bevy Source Files (v0.18.1)

| Component | File Path |
|-----------|-----------|
| Time Resource | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\time.rs` |
| Timer | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\timer.rs` |
| Stopwatch | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\stopwatch.rs` |
| Real Time | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\real.rs` |
| Virtual Time | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\virt.rs` |
| Fixed Time | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\fixed.rs` |
| Common Conditions | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\common_conditions.rs` |
| Delayed Commands | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\delayed_commands.rs` |
| Time Plugin | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_time\src\lib.rs` |

### Project Source Files

| Component | File Path |
|-----------|-----------|
| Animation Plugin | `src/animation/mod.rs` |
| Animation State | `src/animation/animation_state.rs` |
| Skeletal Animation | `src/animation/skeletal_animation.rs` |
| Mesh Animation | `src/animation/mesh_animation.rs` |
| Camera Animation | `src/animation/camera_animation.rs` |
| Transform Animation | `src/animation/transform_animation.rs` |
| ZMO Asset Loader | `src/animation/zmo_asset_loader.rs` |
| ZMO Texture Loader | `src/animation/zmo_asset_loader.rs` (ZmoTextureAssetLoader) |
| Animation Effect System | `src/systems/animation_effect_system.rs` |
| Animation Sound System | `src/systems/animation_sound_system.rs` |
| Command System | `src/systems/command_system.rs` |
| Model Loader | `src/model_loader.rs` |
