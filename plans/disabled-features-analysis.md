# Disabled Features Analysis for Re-implementation

This document analyzes three disabled features in the Rose Online client that need to be re-implemented for Bevy 0.16 compatibility.

## Executive Summary

| Feature | Location | Status | Primary Issue |
|---------|----------|--------|---------------|
| Trail Effect Rendering | [`src/render/trail_effect.rs`](../src/render/trail_effect.rs) | Disabled | Rendering pipeline needs Bevy 0.16 update |
| Gem Effects | [`src/model_loader.rs:581-637`](../src/model_loader.rs:581) | Commented Out | Material system incompatibility |
| Zone Object Effect Spawning | [`src/zone_loader.rs:3045-3076`](../src/zone_loader.rs:3045) | Commented Out | Material system incompatibility |

---

## 1. Trail Effect Rendering

### Current State

**File**: [`src/render/trail_effect.rs`](../src/render/trail_effect.rs)

The trail effect system is **temporarily disabled** with this comment:
```rust
// Trail effect rendering temporarily disabled for Bevy 0.14 migration
// The component definitions are kept for API compatibility
```

### Component Definitions

```rust
// src/render/trail_effect.rs:12-19
#[derive(Component)]
pub struct TrailEffect {
    pub colour: Color,
    pub duration: f32, // Seconds as f32
    pub start_offset: Vec3,
    pub end_offset: Vec3,
    pub trail_texture: Handle<bevy::prelude::Image>,
    pub distance_per_point: f32,
}
```

```rust
// src/render/trail_effect.rs:28-34
#[derive(Component)]
pub struct TrailEffectPositionHistory {
    history: VecDeque<TrailEffectPoint>,
    catmull_points: [TrailEffectPoint; 4],
    trail_length_excess: f32,
    last_temp_points: usize,
}
```

### Plugin Implementation (Disabled)

```rust
// src/render/trail_effect.rs:47-54
pub struct TrailEffectRenderPlugin;

impl bevy::app::Plugin for TrailEffectRenderPlugin {
    fn build(&self, _app: &mut bevy::app::App) {
        // Trail effect rendering temporarily disabled for Bevy 0.14 migration
        // The component definitions are kept for API compatibility
    }
}
```

### Disabled Systems

1. **`initialise_trail_effects`** (line 56-71) - Adds `TrailEffectPositionHistory` to entities with `TrailEffect`
2. **`update_trail_effects`** (line 73-84) - Updates trail position history and rendering

### Usage Sites

Trail effects are actively spawned in [`src/model_loader.rs`](../src/model_loader.rs):

- **Line 542-578**: [`spawn_weapon_trail()`](../src/model_loader.rs:542) - Creates trail entities
- **Line 639-720**: [`spawn_character_weapon_trail()`](../src/model_loader.rs:639) - Spawns trails for character weapons

```rust
// src/model_loader.rs:542-578 (abbreviated)
fn spawn_weapon_trail(
    &self,
    commands: &mut Commands,
    model_list: &ZscFile,
    model_id: usize,
    base_effect_index: usize,
    colour: Color,
    duration: f32,
) -> Option<Entity> {
    let object = model_list.objects.get(model_id)?;
    let start_position = object.effects.get(base_effect_index)?.position;
    let end_position = object.effects.get(base_effect_index + 1)?.position;
    Some(
        commands
            .spawn((
                TrailEffect {
                    colour,
                    duration,
                    start_offset: Vec3::new(...),
                    end_offset: Vec3::new(...),
                    trail_texture: self.trail_effect_image.clone_weak(),
                    distance_per_point: 10.0 / 100.0,
                },
                Transform::default(),
                GlobalTransform::default(),
                Visibility::default(),
                InheritedVisibility::default(),
                ViewVisibility::default(),
            ))
            .id(),
    )
}
```

### Configuration

Trail effect duration multiplier is available in [`src/resources/render_configuration.rs`](../src/resources/render_configuration.rs):
```rust
pub struct RenderConfiguration {
    pub passthrough_terrain_textures: bool,
    pub trail_effect_duration_multiplier: f32,
}
```

### Dependencies

- **Bevy Systems**: `Time`, `GlobalTransform`, `Commands`
- **Assets**: `Handle<Image>` for trail texture
- **Rendering**: Custom shader/render pipeline (not implemented)

### Bevy 0.16 Compatibility Issues

1. **No Render Pipeline**: The plugin builds but doesn't register any render systems
2. **Material System**: Needs custom material implementing `Material` trait
3. **Shader Assets**: No WGSL shader file for trail rendering
4. **Buffer Management**: Position history needs GPU buffer management for rendering

### Re-implementation Approach

1. Create a `TrailMaterial` implementing `Material` trait
2. Write WGSL shader for trail geometry (line strip with UV scrolling)
3. Implement position history update system
4. Generate mesh dynamically from position history
5. Register material plugin in [`RoseRenderPlugin`](../src/render/mod.rs:91)

---

## 2. Gem Effects

### Current State

**File**: [`src/model_loader.rs:581-637`](../src/model_loader.rs:581)

The gem effect spawning function is **commented out** with:
```rust
// Gem effects temporarily disabled (use custom materials)
```

### Disabled Code Block

```rust
// src/model_loader.rs:581-637
/*
fn spawn_character_gem_effect(
    &self,
    commands: &mut Commands,
    asset_server: &AssetServer,
    particle_materials: &mut Assets<ParticleMaterial>,
    effect_mesh_materials: &mut Assets<EffectMeshMaterial>,  // NOTE: This type doesn't exist!
    model_list: &ZscFile,
    model_parts: &[Entity],
    item_model_id: usize,
    gem_item_number: usize,
    gem_position: usize,
) -> Option<Entity> {
    let gem_item = self.item_database.get_gem_item(gem_item_number)?;
    let gem_effect_id = gem_item.gem_effect_id?;
    let gem_effect = self.effect_database.get_effect(gem_effect_id)?;
    let effect_file_id = gem_effect.point_effects.get(0)?;
    let effect_file = self.effect_database.get_effect_file(*effect_file_id)?;

    let zsc_object = model_list.objects.get(item_model_id)?;
    let gem_effect_point = zsc_object.effects.get(gem_position)?;
    let parent_part_entity = model_parts.get(gem_effect_point.parent.unwrap_or(0) as usize)?;

    let effect_entity = spawn_effect(
        &self.vfs,
        commands,
        asset_server,
        particle_materials,
        effect_mesh_materials,
        effect_file.into(),
        false,
        None,
    )?;

    commands
        .entity(*parent_part_entity)
        .add_child(effect_entity);

    commands.entity(effect_entity).insert(
        Transform::from_translation(
            Vec3::new(
                gem_effect_point.position.x,
                gem_effect_point.position.z,
                -gem_effect_point.position.y,
            ) / 100.0,
        )
        .with_rotation(Quat::from_xyzw(
            gem_effect_point.rotation.x,
            gem_effect_point.rotation.z,
            -gem_effect_point.rotation.y,
            gem_effect_point.rotation.w,
        )),
    );

    Some(effect_entity)
}
*/
```

### Usage Sites (Also Disabled)

**File**: [`src/model_loader.rs:834-884`](../src/model_loader.rs:834)

```rust
// src/model_loader.rs:834-884
// Gem effects temporarily disabled (use custom materials)
/*
if matches!(model_part, CharacterModelPart::Weapon) {
    if let Some(item) = equipment.get_equipment_item(EquipmentIndex::Weapon) {
        if item.has_socket && item.gem > 300 {
            if let Some(item_data) =
                self.item_database.get_weapon_item(item.item.item_number)
            {
                if let Some(gem_effect_entity) = self.spawn_character_gem_effect(
                    commands,
                    asset_server,
                    particle_materials,
                    effect_mesh_materials,
                    model_list,
                    &model_parts,
                    model_id,
                    item.gem as usize,
                    item_data.gem_position as usize,
                ) {
                    model_parts.push(gem_effect_entity);
                }
            }
        }
    }
}
// Similar block for SubWeapon...
*/
```

### Data Flow

```
GemItem.gem_effect_id
    ↓
EffectDatabase.get_effect(gem_effect_id)
    ↓
Effect.point_effects[0]
    ↓
EffectDatabase.get_effect_file(effect_file_id)
    ↓
spawn_effect(effect_file)
    ↓
Attach to weapon bone with ZSC effect position/rotation
```

### Dependencies

- **`ItemDatabase`**: `get_gem_item(gem_item_number)`
- **`EffectDatabase`**: `get_effect(gem_effect_id)`, `get_effect_file(effect_file_id)`
- **`spawn_effect()`**: From [`src/effect_loader.rs`](../src/effect_loader.rs:30)
- **ZSC Object**: Contains effect position/rotation data

### Related Components

- [`CharacterModelPartIndex`](../src/components/character_model.rs:25) - Contains `gem: usize` field
- Equipment items have `has_socket` and `gem` fields

### Bevy 0.16 Compatibility Issues

1. **Type Mismatch**: `EffectMeshMaterial` type doesn't exist - should be `ExtendedMaterial<StandardMaterial, RoseEffectExtension>`
2. **Material System**: The comment "use custom materials" suggests the old material approach was incompatible
3. **Effect Loading**: `spawn_effect()` uses storage buffers which had prepass pipeline issues

### Re-implementation Approach

1. **Fix Type**: Change `EffectMeshMaterial` to `ExtendedMaterial<StandardMaterial, RoseEffectExtension>`
2. **Add Parameters**: The function needs `storage_buffers` and `meshes` parameters for `spawn_effect()`
3. **Update Call Sites**: Uncomment and fix the weapon/subweapon gem effect spawning blocks
4. **Test**: Verify gem effects appear on socketed weapons

---

## 3. Zone Object Effect Spawning

### Current State

**File**: [`src/zone_loader.rs:3045-3076`](../src/zone_loader.rs:3045)

Effect spawning on zone objects is **commented out** with:
```rust
// Effect spawning temporarily disabled (use custom materials)
```

### Disabled Code Block

```rust
// src/zone_loader.rs:3045-3076
// Effect spawning temporarily disabled (use custom materials)
/*
if let Some(effect_path) = zsc.effects.get(object_effect.effect_id as usize) {
    if let Some(effect_entity) = spawn_effect(
        &vfs_resource.vfs,
        commands,
        asset_server,
        particle_materials,
        effect_mesh_materials,
        effect_path.into(),
        false,
        None,
    ) {
        if let Some(parent_part_entity) = object_effect
            .parent
            .and_then(|parent_part_index| part_entities.get(parent_part_index as usize))
        {
            commands
                .entity(*parent_part_entity)
                .add_child(effect_entity);
        } else {
            commands.entity(object_entity).add_child(effect_entity);
        }

        commands.entity(effect_entity).insert(effect_transform);

        if matches!(object_effect.effect_type, ZscEffectType::DayNight) {
            commands.entity(effect_entity).insert(NightTimeEffect);
        }
    }
}
*/
```

### Context: `spawn_object()` Function

The disabled code is inside the [`spawn_object()`](../src/zone_loader.rs:2672) function which handles:
- Deco objects (decorations)
- Cnst objects (constructions/buildings)
- Event objects
- Warp objects

### Effect Object Data

Effects on zone objects are defined in ZSC files:
```rust
// From rose_file_readers
pub struct ZscEffect {
    pub effect_id: u32,
    pub effect_type: ZscEffectType,
    pub parent: Option<u32>,
    pub position: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}
```

### Special Effect Types

- **`ZscEffectType::DayNight`**: Adds [`NightTimeEffect`](../src/components/night_time_effect.rs:113) component - effect only visible at night

### Effect Objects vs Object Effects

Note the distinction:
1. **Effect Objects** (`ifo.effect_objects`) - Standalone effect entities, **ARE spawned** via [`spawn_effect_object()`](../src/zone_loader.rs:3209)
2. **Object Effects** (`object.effects`) - Effects attached to object parts, **DISABLED** in `spawn_object()`

### Dependencies

- **`spawn_effect()`**: From [`src/effect_loader.rs`](../src/effect_loader.rs:30)
- **`vfs_resource.vfs`**: Virtual filesystem for loading .eft files
- **`particle_materials`**: `Assets<ParticleMaterial>`
- **`effect_mesh_materials`**: `Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>`

### Bevy 0.16 Compatibility Issues

1. **Material System**: Same "use custom materials" comment as gem effects
2. **Storage Buffers**: Particle system uses storage buffers with prepass issues
3. **Parent-Child Linking**: Effect entities need proper hierarchy with object parts

### Re-implementation Approach

1. Uncomment the effect spawning block
2. Add missing parameters: `storage_buffers`, `meshes`
3. Verify effect transforms are applied correctly
4. Test day/night cycle effects

---

## Effect System Architecture

### Core Components

| Component | File | Purpose |
|-----------|------|---------|
| [`Effect`](../src/components/effect.rs:4) | `effect.rs` | Marker with `manual_despawn` flag |
| [`EffectMesh`](../src/components/effect.rs:14) | `effect.rs` | Marker for effect mesh entities |
| [`EffectParticle`](../src/components/effect.rs:17) | `effect.rs` | Marker for particle entities |
| [`ParticleSequence`](../src/components/particle_sequence.rs:77) | `particle_sequence.rs` | Full particle system configuration |
| [`ActiveParticle`](../src/components/particle_sequence.rs:11) | `particle_sequence.rs` | Runtime particle state |

### Effect Loading

**File**: [`src/effect_loader.rs`](../src/effect_loader.rs)

```rust
pub fn spawn_effect(
    vfs: &VirtualFilesystem,
    commands: &mut Commands,
    asset_server: &AssetServer,
    particle_materials: &mut Assets<ParticleMaterial>,
    effect_mesh_materials: &mut Assets<ExtendedMaterial<StandardMaterial, RoseEffectExtension>>,
    storage_buffers: &mut Assets<ShaderStorageBuffer>,
    meshes: &mut Assets<bevy::prelude::Mesh>,
    effect_path: VfsPath,
    manual_despawn: bool,
    effect_entity: Option<Entity>,
) -> Option<Entity>
```

The function:
1. Loads `.eft` file from VFS
2. Spawns particle entities via `spawn_particle()`
3. Spawns mesh entities via `spawn_mesh()`
4. Creates parent `Effect` entity with children

### Materials

| Material | File | Purpose |
|----------|------|---------|
| [`ParticleMaterial`](../src/render/particle_material.rs:19) | `particle_material.rs` | GPU particle rendering with storage buffers |
| `RoseEffectExtension` | `effect_mesh_extension.rs` | Extended material for animated effect meshes |

### File Formats

- **`.eft`** - Effect file containing particles and meshes
- **`.ptl`** - Particle template file with sequences and keyframes
- **`.zmo`** - Motion file for mesh/transform animation

---

## Re-implementation Priority

### High Priority
1. **Gem Effects** - Players expect visual feedback from socketed gems
2. **Zone Object Effects** - Important for atmosphere (torches, lights, etc.)

### Medium Priority
3. **Trail Effects** - Nice visual feedback for weapon swings, but not critical

---

## Technical Recommendations

### For Trail Effects

1. Use `Material<Mesh>` trait implementation similar to [`ParticleMaterial`](../src/render/particle_material.rs)
2. Store trail points in a storage buffer
3. Generate geometry in vertex shader using `vertex_index`
4. Support both additive and alpha blending modes

### For Gem/Zone Effects

1. The `spawn_effect()` function is **already working** - used by:
   - [`spawn_effect_object()`](../src/zone_loader.rs:3259) for standalone effects
   - [`spawn_effect_system`](../src/systems/spawn_effect_system.rs:35) for runtime effects
   - Vehicle model effects in [`spawn_vehicle_model()`](../src/model_loader.rs:1051)

2. The fix is primarily about:
   - Adding missing parameters to the disabled functions
   - Fixing the `EffectMeshMaterial` type mismatch
   - Uncommenting the code blocks

### Testing Approach

1. Create test zone with known effect objects
2. Create test character with socketed gem weapon
3. Verify effects spawn at correct positions
4. Check day/night cycle for `NightTimeEffect` entities

---

## Related Files

### Source Files
- [`src/render/trail_effect.rs`](../src/render/trail_effect.rs) - Trail effect components and disabled plugin
- [`src/model_loader.rs`](../src/model_loader.rs) - Gem effect spawning (disabled)
- [`src/zone_loader.rs`](../src/zone_loader.rs) - Zone object effect spawning (disabled)
- [`src/effect_loader.rs`](../src/effect_loader.rs) - Effect loading and spawning
- [`src/components/effect.rs`](../src/components/effect.rs) - Effect components
- [`src/components/particle_sequence.rs`](../src/components/particle_sequence.rs) - Particle system
- [`src/render/particle_material.rs`](../src/render/particle_material.rs) - Particle material

### Data Files
- `3DDATA/EFFECT/*.EFT` - Effect files
- `3DDATA/PARTICLE/*.PTL` - Particle templates
- Zone ZSC files - Object definitions with effects

### Documentation
- [`plans/particle-quality-fix-plan.md`](./particle-quality-fix-plan.md) - Related particle system analysis
