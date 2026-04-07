# Skill Effects Not Showing - Comprehensive Analysis

## Issue Description
Skill effects (visual particle and mesh effects) are not appearing when skills are used in the game. The animations play correctly, but no particle effects or colors are visible.

## Investigation Date
2026-04-07

---

## Complete System Architecture Analysis

### Full Effect Flow Pipeline

```
Animation Frame Event (skeletal_animation_system)
    ↓
AnimationFrameEvent message
    ↓
Animation Effect System (animation_effect_system)
    ↓
SpawnEffectEvent message
    ↓
Spawn Effect System (spawn_effect_system)
    ↓
spawn_effect() function (effect_loader.rs)
    ↓
Effect Entity with Particle/Mesh Children
    ↓
Particle Sequence System (particle_sequence_system)
    ↓
ParticleRenderData updated
    ↓
Particle Storage Buffer Update System
    ↓
GPU Storage Buffers updated
    ↓
Particle Shader (particle.wgsl) renders particles
```

### 1. Animation Event Generation - [`skeletal_animation_system`](src/animation/skeletal_animation.rs:37)

**Purpose:** Generates animation frame events when animation frames with event flags are reached.

**Key Code:**
```rust
// skeletal_animation.rs:76-82
animation.iter_animation_events(zmo_asset, |event_id| {
    if let Some(flags) = game_data.animation_event_flags.get(event_id as usize) {
        if !flags.is_empty() {
            animation_frame_events.write(AnimationFrameEvent::new(entity, *flags));
        }
    }
});
```

**Analysis:**
- Events are generated based on `game_data.animation_event_flags`
- If `animation_event_flags` is empty or not populated, no events will be generated
- This is a CRITICAL dependency point

### 2. Animation Effect System - [`animation_effect_system`](src/systems/animation_effect_system.rs:29)

**Purpose:** Processes animation events and spawns effects based on event flags.

**Key Event Flags Processed:**
| Flag | Lines | Purpose |
|------|-------|---------|
| `EFFECT_WEAPON_ATTACK_HIT` | 49-89 | Weapon hit effects |
| `EFFECT_WEAPON_FIRE_BULLET` | 91-173 | Weapon projectile effects |
| `EFFECT_SKILL_FIRE_BULLET` | 175-214 | Skill projectile effects |
| `EFFECT_SKILL_ACTION` | 216-322 | Main skill effect trigger |
| `EFFECT_SKILL_HIT` | 324-362 | Skill hit effect |
| `EFFECT_SKILL_DUMMY_HIT_0/1` | 364-398 | Dummy hit effects |
| `EFFECT_SKILL_CASTING_0-3` | 400-450 | Casting effects |

**Self-Bound Skill Effect Spawning (lines 229-252):**
```rust
SkillType::SelfBound | SkillType::SelfBoundDuration | ...
{
    // Spawn bullet effect if exists
    if let Some(effect_data) = skill_data
        .bullet_effect_id
        .and_then(|id| game_data.effect_database.get_effect(id))
    {
        if let Some(effect_file_id) = effect_data.bullet_effect {
            spawn_effect_events.write(SpawnEffectEvent::OnEntity(
                event.entity,
                Some(skill_data.bullet_link_dummy_bone_id as usize),
                SpawnEffectData::with_file_id(effect_file_id),
            ));
        }
    }

    // Spawn hit effect if exists
    if let Some(hit_effect_file_id) = skill_data.hit_effect_file_id {
        spawn_effect_events.write(SpawnEffectEvent::OnEntity(
            event.entity,
            skill_data.hit_link_dummy_bone_id,
            SpawnEffectData::with_file_id(hit_effect_file_id),
        ));
    }
}
```

**Analysis:**
- Effects are spawned via `SpawnEffectEvent::OnEntity`
- Requires `skill_data.bullet_effect_id` or `skill_data.hit_effect_file_id` to be set
- Requires `game_data.effect_database.get_effect(id)` to return valid effect data
- **CRITICAL:** If `effect_database.get_effect()` returns `None`, no effects will spawn

### 3. Spawn Effect System - [`spawn_effect_system`](src/systems/spawn_effect_system.rs:33)

**Purpose:** Handles effect spawning from `SpawnEffectEvent` messages.

**Event Types Handled:**
- `SpawnEffectEvent::InEntity` - Effect at entity position
- `SpawnEffectEvent::AtEntity` - Effect at another entity's position
- `SpawnEffectEvent::OnEntity` - Effect attached to entity (with optional bone)
- `SpawnEffectEvent::WithTransform` - Effect at specific transform

**Key Code (lines 95-128):**
```rust
SpawnEffectEvent::OnEntity(on_entity, dummy_bone_id, spawn_effect_data) => {
    let mut link_entity = *on_entity;

    if let Some(dummy_bone_id) = dummy_bone_id {
        if let Ok((skinned_mesh, dummy_bone_offset)) = query_skeleton.get(*on_entity) {
            if let Some(joint) = skinned_mesh
                .joints
                .get(dummy_bone_offset.index + dummy_bone_id)
            {
                link_entity = *joint;
            }
        }
    }

    if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data) {
        if let Some(effect_entity) = spawn_effect(...) {
            commands.entity(link_entity).add_child(effect_entity);
        }
    }
}
```

**Analysis:**
- Effect file path is resolved via `get_effect_file_path()`
- If path resolution fails, no effect is spawned
- Effect is attached to entity or bone as child

### 4. Effect Loader - [`spawn_effect`](src/effect_loader.rs:97)

**Purpose:** Loads effect files and creates effect entities with particle and mesh children.

**Effect File Structure (`.eft` files):**
- Contains list of particles (`EftParticle`)
- Contains list of meshes (`EftMesh`)
- Contains optional sound file

**Particle Spawning (lines 388-520):**
```rust
fn spawn_particle(...) -> Option<Entity> {
    let ptl_file = vfs.read_file::<PtlFile, _>(&eft_particle.particle_file).ok()?;

    for sequence in ptl_file.sequences {
        // Create ParticleRenderData
        // Load texture (with NULL fallback)
        // Initialize storage buffers with placeholder data
        // Create particle mesh (6 vertices per particle)
        // Spawn entity with ParticleSequence component
    }
}
```

**Critical Observations:**
1. **Storage buffers initialized with placeholder data:**
   - Positions: `Vec4::ZERO`
   - Sizes: `Vec2::ZERO`
   - Colors: `Vec4::ONE` (white)
   - Textures: `Vec4::ZERO`

2. **Particle mesh created with zero positions:**
   ```rust
   let particle_positions: Vec<[f32; 3]> = vec![[0.0, 0.0, 0.0]; particle_vertex_count];
   ```

3. **Texture fallback for NULL paths:**
   ```rust
   let particle_texture_handle = if particle_texture_path.is_empty() || particle_texture_path == "NULL" {
       log::warn!("[EFFECT LOADER] NULL or empty particle texture path, using fallback");
       asset_server.load::<bevy::prelude::Image>("ETC/SPECULAR_SPHEREMAP.DDS")
   } else {
       asset_server.load::<bevy::prelude::Image>(&particle_texture_path)
   };
   ```

**Analysis:**
- Initial particle data is ALL ZEROS except colors (white)
- Particles start at position (0,0,0) with size (0,0)
- **This means particles are INVISIBLE until particle_sequence_system updates them!**

### 5. Particle Sequence System - [`particle_sequence_system`](src/systems/particle_sequence_system.rs:324)

**Purpose:** Updates particle positions, sizes, colors each frame and spawns new particles.

**Key Operations:**
1. Apply timestep to existing particles (line 349)
2. Apply gravity (lines 350-360)
3. Apply keyframes (line 362)
4. Cleanup dead particles (lines 367-369)
5. Spawn new particles (lines 372-432)
6. Update render data (lines 435-477)

**Particle Spawning (lines 386-432):**
```rust
while particle_sequence.emit_counter > 1.0
    && particle_sequence.particles.len() < particle_sequence.num_particles as usize
{
    // Generate random position within emit radius
    let mut position = Vec3::new(
        rng_gen_range(&mut rng, &particle_sequence.emit_radius_x),
        rng_gen_range(&mut rng, &particle_sequence.emit_radius_y),
        rng_gen_range(&mut rng, &particle_sequence.emit_radius_z),
    );
    
    // Create new particle
    let life = rng_gen_range(&mut rng, &particle_sequence.particle_life);
    let particle_index = particle_sequence.particles.len();
    particle_sequence.particles.push(ActiveParticle::new(life, position, gravity_local, world_direction));
    
    // Apply initial keyframes
    apply_keyframes(&mut rng, &mut particle_sequence, particle_index);
    
    particle_sequence.num_emitted += 1;
    particle_sequence.emit_counter -= 1.0;
}
```

**Analysis:**
- Particles are spawned based on `emit_rate` and `emit_counter`
- New particles get initial values from `apply_keyframes()`
- **If `emit_rate` is 0 or `num_particles` is 0, no particles will be spawned!**

### 6. Particle Storage Buffer Update System - [`particle_storage_buffer_update_system`](src/systems/particle_sequence_system.rs:489)

**Purpose:** Copies CPU particle data to GPU storage buffers for rendering.

**Key Code (lines 536-577):**
```rust
if let Some(existing_material_handle) = material_handle {
    if let Some(mat) = materials.get_mut(&existing_material_handle.0) {
        let should_recreate_buffers = true;
        
        if should_recreate_buffers {
            // Create new buffers with updated data
            mat.positions = storage_buffers.add(
                ShaderStorageBuffer::from(render_data.positions.clone())
            );
            mat.sizes = storage_buffers.add(
                ShaderStorageBuffer::from(render_data.sizes.clone())
            );
            mat.colors = storage_buffers.add(
                ShaderStorageBuffer::from(render_data.colors.clone())
            );
            mat.textures = storage_buffers.add(
                ShaderStorageBuffer::from(render_data.textures.clone())
            );
            
            // Remove old buffers to prevent memory leak
            storage_buffers.remove(&old_positions);
            // ... etc
        }
        
        // Update blend settings
        mat.blend_op = render_data.blend_op as u32;
        mat.src_blend_factor = render_data.src_blend_factor as u32;
        mat.dst_blend_factor = render_data.dst_blend_factor as u32;
        mat.billboard_type = render_data.billboard_type as u32;
    }
}
```

**Analysis:**
- This system is CRITICAL for particle rendering
- Without this, storage buffers contain only placeholder data (zeros)
- **If this system doesn't run, particles will be invisible!**

### 7. Particle Material and Shader - [`particle_material.rs`](src/render/particle_material.rs:1) and [`particle.wgsl`](src/render/shaders/particle.wgsl:1)

**Material Structure:**
```rust
pub struct ParticleMaterial {
    #[storage(0, read_only)] pub positions: Handle<ShaderStorageBuffer>,
    #[storage(1, read_only)] pub sizes: Handle<ShaderStorageBuffer>,
    #[storage(2, read_only)] pub colors: Handle<ShaderStorageBuffer>,
    #[storage(3, read_only)] pub textures: Handle<ShaderStorageBuffer>,
    #[texture(4)] #[sampler(5)] pub texture: Handle<Image>,
    #[uniform(6)] pub blend_op: u32,
    #[uniform(7)] pub src_blend_factor: u32,
    #[uniform(8)] pub dst_blend_factor: u32,
    #[uniform(9)] pub billboard_type: u32,
    pub alpha_mode: AlphaMode,
}
```

**Vertex Shader (particle.wgsl:52-124):**
```wgsl
@vertex
fn vertex(model: VertexInput) -> VertexOutput {
    let vert_idx = model.vertex_idx % 6u;
    let particle_idx = model.vertex_idx / 6u;
    
    // Get particle data from storage buffers
    let particle_position = positions[particle_idx].xyz;
    let theta = positions[particle_idx].w;
    let size = sizes[particle_idx];
    let color = colors[particle_idx];
    let texture_uv = textures[particle_idx];
    
    // Build billboard quad
    let world_position = particle_position + (camera_right * ...) + (camera_up * ...);
    
    out.position = view.clip_from_world * vec4<f32>(world_position, 1.0);
    out.color = colors[particle_idx];
    // ... UV calculation
}
```

**Fragment Shader (particle.wgsl:126-157):**
```wgsl
@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color = textureSample(base_color_texture, base_color_sampler, in.uv);
    let result = in.color * texture_color;
    
    // Discard transparent pixels
    if (result.a < 0.01) {
        discard;
    }
    
    // Handle blend modes
    // ...
}
```

**Analysis:**
- Shader reads particle data from storage buffers
- If storage buffers contain zeros, particles will be at position (0,0,0) with size (0,0)
- **This confirms particles need storage buffer updates to be visible!**

---

## Identified Potential Root Causes

### Root Cause 1: Animation Event Flags Not Populated
**Location:** `game_data.animation_event_flags`

**Symptom:** No animation events generated, no effects spawned.

**Check:** Verify `animation_event_flags` is populated in `GameData`.

### Root Cause 2: Effect Database Not Loaded
**Location:** `game_data.effect_database`

**Symptom:** `get_effect()` returns `None`, no effects spawned.

**Check:** Verify effect database is loaded and contains valid effects.

### Root Cause 3: Effect File Paths Invalid
**Location:** `.eft` file paths in effect database

**Symptom:** `vfs.read_file::<EftFile, _>()` fails, no effect created.

**Check:** Verify effect files exist in VFS and can be read.

### Root Cause 4: Particle Sequence Not Emitting
**Location:** `ParticleSequence.emit_rate`, `ParticleSequence.num_particles`

**Symptom:** No particles spawned, storage buffers remain empty.

**Check:** Verify `emit_rate > 0` and `num_particles > 0` in `.ptl` files.

### Root Cause 5: Storage Buffer Update System Not Running
**Location:** `particle_storage_buffer_update_system`

**Symptom:** Storage buffers contain placeholder data (zeros), particles invisible.

**Check:** Verify system is registered and running in correct schedule.

### Root Cause 6: Particle Material Not Specialized Correctly
**Location:** `ParticleMaterial::specialize()`

**Symptom:** Render pipeline fails, particles not rendered.

**Check:** Verify material specialization succeeds without errors.

---

## Debugging Plan

### Phase 1: Verify Animation Events
1. Add logging to `skeletal_animation_system` to confirm events are generated
2. Add logging to `animation_effect_system` to confirm events are processed
3. Check `game_data.animation_event_flags` population

### Phase 2: Verify Effect Spawning
1. Add logging to `spawn_effect_system` to confirm effects are spawned
2. Add logging to `spawn_effect()` to confirm effect files are loaded
3. Check effect database contents

### Phase 3: Verify Particle System
1. Add logging to `particle_sequence_system` to confirm particles are emitted
2. Add logging to `particle_storage_buffer_update_system` to confirm buffers are updated
3. Check particle sequence parameters (emit_rate, num_particles)

### Phase 4: Verify Rendering
1. Add logging to `ParticleMaterial::specialize()` to confirm pipeline creation
2. Check for shader compilation errors
3. Verify texture loading

---

## Analysis Progress (2026-04-07)

### Completed Analysis
- [x] Analyzed `animation_effect_system.rs` - understands skill effect triggers
- [x] Analyzed `spawn_effect_system.rs` - understands effect spawning from events
- [x] Analyzed `effect_loader.rs` - understands effect file loading and entity creation
- [x] Analyzed `particle_sequence_system.rs` - understands particle updates and spawning
- [x] Analyzed `particle_material.rs` - understands particle material structure
- [x] Analyzed `particle.wgsl` - understands particle shader
- [x] Analyzed `effect_system.rs` - understands effect cleanup
- [x] Analyzed `skeletal_animation.rs` - understands animation event generation
- [x] Analyzed `particle_render_data.rs` - understands particle render data structure
- [x] Checked effect database loading in GameData

### Key Findings

#### 1. Effect Database Loading
The effect database is loaded in [`lib.rs:1785-1786`](src/lib.rs:1785):
```rust
effect_database: rose_data_irose::get_effect_database(&vfs_resource.vfs)
    .expect("Failed to load effect database"),
```

This loads from the VFS (Virtual Filesystem). If the VFS doesn't contain the effect files, the database will be empty or incomplete.

#### 2. Animation Event Flags
Animation event flags are loaded in [`lib.rs:1780`](src/lib.rs:1780):
```rust
animation_event_flags: rose_data_irose::get_animation_event_flags(),
```

If this is empty, no animation events will be generated, and no effects will spawn.

#### 3. Particle Initialization Issue (CRITICAL FINDING)

In [`effect_loader.rs:447-457`](src/effect_loader.rs:447), storage buffers are initialized with placeholder data:
```rust
// Initialize storage buffers with placeholder data to avoid zero-size buffer error
let num_particles = sequence.num_particles as usize;
let positions_data: Vec<bevy::math::Vec4> = vec![bevy::math::Vec4::ZERO; num_particles];
let sizes_data: Vec<bevy::math::Vec2> = vec![bevy::math::Vec2::ZERO; num_particles];
let colors_data: Vec<bevy::math::Vec4> = vec![bevy::math::Vec4::ONE; num_particles];
let textures_data: Vec<bevy::math::Vec4> = vec![bevy::math::Vec4::ZERO; num_particles];
```

**This means:**
- Initial particle positions are ALL ZERO (0, 0, 0, 0)
- Initial particle sizes are ALL ZERO (0, 0)
- Initial particle colors are WHITE (1, 1, 1, 1)
- Initial particle UVs are ALL ZERO (0, 0, 0, 0)

**Particles are INVISIBLE until `particle_sequence_system` updates them!**

#### 4. Particle Spawning Logic

In [`particle_sequence_system.rs:386-432`](src/systems/particle_sequence_system.rs:386), particles are spawned based on:
- `emit_counter` must be > 1.0
- `particles.len()` must be < `num_particles`

The `emit_counter` increases by:
```rust
particle_sequence.emit_counter += delta_time * rng_gen_range(&mut rng, &particle_sequence.emit_rate);
```

**If `emit_rate` is 0 or very small, particles won't spawn quickly!**

#### 5. Storage Buffer Update System

In [`particle_storage_buffer_update_system.rs:509-577`](src/systems/particle_sequence_system.rs:509), the system updates GPU buffers:
```rust
for (entity, render_data, material_handle) in query.iter() {
    if render_data.positions.is_empty() {
        continue;  // Skip if no particles!
    }
    // ... update buffers
}
```

**If `render_data.positions` is empty, buffers are NOT updated!**

---

## Most Likely Root Causes (Ranked by Probability)

### Root Cause #1: Animation Event Flags Not Populated (HIGH PROBABILITY)
**Symptom:** No animation events generated, no effects spawned.

**Why:** The `animation_event_flags` vector in `GameData` might be empty or not properly populated. Without event flags, `skeletal_animation_system` won't generate `AnimationFrameEvent` messages.

**How to Verify:**
1. Add logging to check `game_data.animation_event_flags.len()`
2. Check if `rose_data_irose::get_animation_event_flags()` returns data

### Root Cause #2: Effect Files Not in VFS (HIGH PROBABILITY)
**Symptom:** Effect database is empty or effect file paths are invalid.

**Why:** The VFS might not contain the `.eft` effect files, or the paths in the effect database don't match the actual file locations.

**How to Verify:**
1. Use the debug effect list UI to see if effects are listed
2. Try spawning an effect manually via the debug UI
3. Check if `.eft` files exist in the game data archive

### Root Cause #3: Particle Emit Rate is Zero (MEDIUM PROBABILITY)
**Symptom:** Effects spawn but no particles appear.

**Why:** The `.ptl` particle files might have `emit_rate = 0` or `num_particles = 0`, preventing particle spawning.

**How to Verify:**
1. Add logging to `particle_sequence_system` to check `emit_rate` and `num_particles`
2. Check if `emit_counter` increases over time

### Root Cause #4: Storage Buffer Update System Not Running (MEDIUM PROBABILITY)
**Symptom:** Particles are spawned but not rendered.

**Why:** The `particle_storage_buffer_update_system` might not be registered or running in the correct schedule.

**How to Verify:**
1. Check if the system is added to the app in `lib.rs`
2. Add logging to confirm the system runs

### Root Cause #5: Particle Material Shader Issues (LOW PROBABILITY)
**Symptom:** Particles should render but don't appear.

**Why:** The particle shader might have compilation errors or the material specialization might fail.

**How to Verify:**
1. Check for shader compilation errors in the log
2. Add logging to `ParticleMaterial::specialize()`

---

## Recommended Debugging Steps

### Step 1: Add Logging to Animation Effect System
Add debug logging to [`animation_effect_system.rs:216-224`](src/systems/animation_effect_system.rs:216):
```rust
if event.flags.contains(AnimationEventFlags::EFFECT_SKILL_ACTION) {
    log::info!("[ANIMATION EFFECT] EFFECT_SKILL_ACTION triggered for entity {:?}", event.entity);
    
    if let Some(skill_data) = event_entity
        .command
        .get_skill_id()
        .and_then(|skill_id| game_data.skills.get_skill(skill_id))
    {
        log::info!("[ANIMATION EFFECT] Skill data found: id={}, bullet_effect_id={:?}, hit_effect_file_id={:?}", 
            skill_data.id, skill_data.bullet_effect_id, skill_data.hit_effect_file_id);
        // ... rest of code
    } else {
        log::warn!("[ANIMATION EFFECT] No skill data found for entity {:?}", event.entity);
    }
}
```

### Step 2: Add Logging to Spawn Effect System
Add debug logging to [`spawn_effect_system.rs:47-67`](src/systems/spawn_effect_system.rs:47):
```rust
for event in events.read() {
    log::info!("[SPAWN EFFECT] Processing event: {:?}", event);
    
    match event {
        SpawnEffectEvent::InEntity(effect_entity, spawn_effect_data) => {
            if let Some(effect_file_path) = get_effect_file_path(spawn_effect_data, &game_data) {
                log::info!("[SPAWN EFFECT] Spawning effect from path: {}", effect_file_path.path().to_string_lossy());
                // ... rest of code
            } else {
                log::warn!("[SPAWN EFFECT] No effect file path found for {:?}", spawn_effect_data);
            }
        }
        // ... other match arms
    }
}
```

### Step 3: Add Logging to Effect Loader
Add debug logging to [`effect_loader.rs:111-125`](src/effect_loader.rs:111):
```rust
let path_str = effect_path.path().to_string_lossy().into_owned();
log::info!("[EFFECT LOADER] Loading effect: {}", path_str);

let eft_file = if let Some(cache) = effect_cache {
    if let Some(cached) = cache.get(&path_str) {
        log::info!("[EFFECT LOADER] Effect loaded from cache: {} particles, {} meshes", 
            cached.particles.len(), cached.meshes.len());
        cached
    } else {
        // Load from disk and cache
        let loaded = Arc::new(vfs.read_file::<EftFile, _>(&effect_path).ok()?);
        log::info!("[EFFECT LOADER] Effect loaded from disk: {} particles, {} meshes", 
            loaded.particles.len(), loaded.meshes.len());
        cache.insert_arc(path_str, Arc::clone(&loaded));
        loaded
    }
} else {
    // No cache available, load directly
    let loaded = Arc::new(vfs.read_file::<EftFile, _>(&effect_path).ok()?);
    log::info!("[EFFECT LOADER] Effect loaded (no cache): {} particles, {} meshes", 
        loaded.particles.len(), loaded.meshes.len());
    loaded
};
```

### Step 4: Add Logging to Particle Sequence System
Add debug logging to [`particle_sequence_system.rs:337-345`](src/systems/particle_sequence_system.rs:337):
```rust
for (entity, global_transform, mut particle_sequence, mut particle_render_data) in query.iter_mut() {
    if particle_sequence.start_delay > 0.0 {
        particle_sequence.start_delay -= delta_time;
        if particle_sequence.start_delay > 0.0 {
            continue;
        }
        particle_sequence.start_delay = 0.0;
    }
    
    log::debug!("[PARTICLE SEQUENCE] Entity {:?}: emit_rate={:?}, num_particles={}, emit_counter={}, particles.len={}", 
        entity, particle_sequence.emit_rate, particle_sequence.num_particles, 
        particle_sequence.emit_counter, particle_sequence.particles.len());
    
    // ... rest of code
}
```

### Step 5: Check Debug Effect List UI
1. Run the game
2. Open the debug menu (usually F1 or a specific key)
3. Navigate to "Effect List"
4. Check if effects are listed
5. Try spawning an effect manually by clicking "View"

---

## Next Steps

1. **Run the game with debug logging enabled** - Set log level to `debug` or `info` to see the logging output
2. **Check the debug effect list UI** - Verify effects are loaded in the database
3. **Try spawning effects manually** - Use the debug UI to spawn effects and see if they appear
4. **Analyze the logs** - Look for errors or warnings related to effect loading
5. **Identify the root cause** - Based on the logs, determine which root cause is the issue
6. **Implement the fix** - Fix the identified issue
7. **Test the fix** - Verify skill effects now appear when skills are used

---

## Related Files

| File | Purpose | Key Functions |
|------|---------|---------------|
| [`src/systems/animation_effect_system.rs`](src/systems/animation_effect_system.rs) | Processes animation events and spawns effects | `animation_effect_system()` |
| [`src/systems/spawn_effect_system.rs`](src/systems/spawn_effect_system.rs) | Handles effect spawning from events | `spawn_effect_system()` |
| [`src/effect_loader.rs`](src/effect_loader.rs) | Loads and creates effect entities | `spawn_effect()`, `spawn_particle()`, `spawn_mesh()` |
| [`src/systems/particle_sequence_system.rs`](src/systems/particle_sequence_system.rs) | Updates particle effects | `particle_sequence_system()`, `particle_storage_buffer_update_system()` |
| [`src/render/particle_material.rs`](src/render/particle_material.rs) | Particle material and shader | `ParticleMaterial`, `ParticleMaterialPlugin` |
| [`src/render/shaders/particle.wgsl`](src/render/shaders/particle.wgsl) | Particle shader | vertex(), fragment() |
| [`src/render/particle_render_data.rs`](src/render/particle_render_data.rs) | Particle render data structure | `ParticleRenderData` |
| [`src/systems/effect_system.rs`](src/systems/effect_system.rs) | Cleans up finished effects | `effect_system()` |
| [`src/animation/skeletal_animation.rs`](src/animation/skeletal_animation.rs) | Generates animation frame events | `skeletal_animation_system()` |
| [`src/ui/ui_debug_effect_list.rs`](src/ui/ui_debug_effect_list.rs) | Debug UI for effect list | `ui_debug_effect_list_system()` |

---

## Debug Logging Implementation (2026-04-07)

The following debug logging has been added to trace the effect spawning flow:

### 1. Animation Effect System (`src/systems/animation_effect_system.rs`)

**Lines 216-253**: Added logging for `EFFECT_SKILL_ACTION` triggers and skill data resolution.

```rust
// When EFFECT_SKILL_ACTION is triggered
log::info!("[ANIMATION EFFECT] EFFECT_SKILL_ACTION triggered for entity: {}, skill_id: {}", entity, skill_id);

// When skill data is resolved
log::info!("[ANIMATION EFFECT] Skill data resolved for skill_id {}: bullet_effect_id={:?}, hit_effect_id={:?}", 
    skill_id, skill_data.bullet_effect_id, skill_data.hit_effect_id);

// When spawning bullet effect
log::info!("[ANIMATION EFFECT] Spawning bullet effect: effect_id={}, effect_file_id={:?} for entity: {}", 
    skill_data.bullet_effect_id, effect_data.bullet_effect, entity);
```

**What to look for in logs:**
- `[ANIMATION EFFECT] EFFECT_SKILL_ACTION triggered` - Animation events are being generated
- `[ANIMATION EFFECT] Skill data resolved` - Skill data is being found in the database
- `[ANIMATION EFFECT] Spawning bullet effect` - Effect spawning is being triggered

### 2. Spawn Effect System (`src/systems/spawn_effect_system.rs`)

**Lines 47-70**: Added logging for effect spawning events.

```rust
// When processing spawn effect events
log::info!("[SPAWN EFFECT SYSTEM] Processing SpawnEffectEvent: effect_file_id={}", event.effect_file_id);

// When effect file path is resolved
log::info!("[SPAWN EFFECT SYSTEM] Effect file path: {}", effect_file_path);

// When spawning effect on entity
log::info!("[SPAWN EFFECT SYSTEM] Spawning effect on entity: {}", entity);
```

**What to look for in logs:**
- `[SPAWN EFFECT SYSTEM] Processing SpawnEffectEvent` - Spawn events are being processed
- `[SPAWN EFFECT SYSTEM] Effect file path` - Effect file paths are being resolved
- `[SPAWN EFFECT SYSTEM] Spawning effect on entity` - Effects are being spawned

### 3. Effect Loader (`src/effect_loader.rs`)

**Lines 110-125**: Added logging for effect file loading and cache usage.

```rust
// When loading effect file
log::info!("[EFFECT LOADER] Loading effect: {}", path_str);

// When effect is loaded from cache
log::info!("[EFFECT LOADER] Effect loaded from cache: {} particles, {} meshes", 
    cached.particles.len(), cached.meshes.len());

// When effect is loaded from disk
log::info!("[EFFECT LOADER] Effect loaded from disk: {} particles, {} meshes", 
    loaded.particles.len(), loaded.meshes.len());
```

**What to look for in logs:**
- `[EFFECT LOADER] Loading effect` - Effect files are being loaded
- `[EFFECT LOADER] Effect loaded from cache/disk` - Effect files are successfully parsed
- Particle and mesh counts indicate the effect has content

### 4. Particle Sequence System (`src/systems/particle_sequence_system.rs`)

**Lines 345-347**: Added logging for particle sequence start.

```rust
log::info!("[PARTICLE SEQUENCE] Starting particle sequence: {} particles, emit_rate={}", 
    particle_sequence.particles.len(), particle_sequence.emit_rate);
```

**Lines 388-395**: Added logging for particle spawning.

```rust
log::info!("[PARTICLE SEQUENCE] Spawning particle: {} -> {} particles, emit_rate={}, num_particles={}", 
    particle_sequence.particles.len(), particle_sequence.particles.len() + 1,
    particle_sequence.emit_rate, particle_sequence.num_particles);
```

**What to look for in logs:**
- `[PARTICLE SEQUENCE] Starting particle sequence` - Particle sequences are starting
- `[PARTICLE SEQUENCE] Spawning particle` - Particles are being spawned
- `emit_rate` should be > 0 for particles to spawn
- `num_particles` should be > 0 for particles to spawn

---

## Analysis Progress (Updated 2026-04-07)

### Completed Analysis
- [x] Analyzed `animation_effect_system.rs` - understands skill effect triggers
- [x] Analyzed `spawn_effect_system.rs` - understands effect spawning from events
- [x] Analyzed `effect_loader.rs` - understands effect file loading and entity creation
- [x] Analyzed `particle_sequence_system.rs` - understands particle updates and spawning
- [x] Analyzed `particle_material.rs` - understands particle material structure
- [x] Analyzed `particle.wgsl` - understands particle shader
- [x] Analyzed `effect_system.rs` - understands effect cleanup
- [x] Analyzed `skeletal_animation.rs` - understands animation event generation
- [x] Analyzed `particle_render_data.rs` - understands particle render data structure
- [x] Checked effect database loading in GameData
- [x] Added debug logging to `animation_effect_system.rs`
- [x] Added debug logging to `spawn_effect_system.rs`
- [x] Added debug logging to `effect_loader.rs`
- [x] Added debug logging to `particle_sequence_system.rs`

### Pending Tasks
- [ ] Run game with debug logging and analyze output
- [ ] Identify root cause based on logs
- [ ] Implement fix
- [ ] Test fix and verify effects appear

---

## Next Steps

1. **Run the game with debug logging enabled**
   - Set log level to `info` or `debug` in the game's logging configuration
   - Use a skill that should spawn effects (e.g., a basic attack or spell)
   - Observe the logging output to trace the effect flow

2. **Check the debug effect list UI**
   - Open debug menu (F1 or specific key)
   - Navigate to "Effect List"
   - Verify effects are listed in the database
   - Try spawning an effect manually by clicking "View"

3. **Analyze the logs**
   - Look for `[ANIMATION EFFECT]` messages to confirm events are triggered
   - Look for `[SPAWN EFFECT SYSTEM]` messages to confirm effects are spawned
   - Look for `[EFFECT LOADER]` messages to confirm files are loaded
   - Look for `[PARTICLE SEQUENCE]` messages to confirm particles are spawning
   - Identify where the flow breaks

4. **Identify the root cause**
   - Based on logs, determine which root cause is the issue
   - Check `game_data.animation_event_flags.len()` if no animation events
   - Check effect database contents if no effects spawn
   - Check particle sequence parameters if effects spawn but no particles

5. **Implement the fix**
   - Fix the identified issue based on root cause analysis
   - May involve: populating animation flags, fixing VFS paths, adjusting particle parameters, or ensuring systems are registered

6. **Test the fix**
   - Verify skill effects now appear when skills are used
   - Confirm particles are visible and animated correctly
