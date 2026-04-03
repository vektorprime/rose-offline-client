# Physics System Documentation

This document describes the Rapier physics integration in rose-offline-client, including configuration, collision detection patterns, and system integration.

## Overview

The project uses **bevy_rapier3d** (v0.31.0) for collision detection and terrain following. Unlike typical physics simulations, this implementation uses Rapier primarily for **scene queries** (raycasting, shape casting, intersection testing) rather than full rigid body dynamics.

### Key Design Decisions

- **NoUserData mode**: Physics runs without user data callbacks for performance
- **Static collision world**: Most colliders are kinematic/fixed, not dynamic
- **Manual entity movement**: Entities move via Position component, not physics forces
- **Raycast-based terrain following**: Ground detection uses downward raycasts

## Plugin Configuration

### RapierPhysicsPlugin Setup

Located in `src/lib.rs:847`:

```rust
app.add_plugins(bevy_rapier3d::prelude::RapierPhysicsPlugin::<bevy_rapier3d::prelude::NoUserData>::default());
```

#### NoUserData

The `NoUserData` type parameter (`pub type NoUserData = ();`) disables physics hooks:
- No contact pair filtering callbacks
- No collision modification callbacks
- Reduced overhead for query-only usage

#### Default Configuration

The plugin creates a default `RapierContext` with:
- **Length unit**: 1.0 (1 Bevy unit = 1 physics meter)
- **Gravity**: (0, -9.81, 0) m/s²
- **Timestep mode**: Variable with max_dt = 1/60 second

### PhysicsSet System Sets

From `bevy_rapier3d::plugin::PhysicsSet`:

| Set | Purpose | Runs |
|-----|---------|------|
| `SyncBackend` | Sync Bevy components → Rapier backend | PostUpdate |
| `StepSimulation` | Advance physics simulation | PostUpdate |
| `Writeback` | Write Rapier state → Bevy components | PostUpdate |

#### System Ordering in Project

From `src/lib.rs:1651-1660`:

```rust
app.configure_sets(
    PostUpdate,
    (GameStages::AfterUpdate,).before(PhysicsSet::SyncBackend),
);

app.configure_sets(
    PostUpdate,
    (GameStages::ZoneChange, GameStages::ZoneChangeFlush, GameStages::AfterUpdate)
        .before(PhysicsSet::SyncBackend),
);
```

This ensures:
1. Game systems (zone loading, entity spawning) complete first
2. Physics sync happens after all entity changes
3. Scene queries in collision systems see up-to-date colliders

## Collision Groups

### Group Definitions

From `src/components/collision.rs:57-71`:

```rust
// Zone objects (environment)
pub const COLLISION_GROUP_ZONE_OBJECT: Group = Group::from_bits_truncate(1 << 0);
pub const COLLISION_GROUP_ZONE_TERRAIN: Group = Group::from_bits_truncate(1 << 1);
pub const COLLISION_GROUP_ZONE_WATER: Group = Group::from_bits_truncate(1 << 2);
pub const COLLISION_GROUP_ZONE_EVENT_OBJECT: Group = Group::from_bits_truncate(1 << 3);
pub const COLLISION_GROUP_ZONE_WARP_OBJECT: Group = Group::from_bits_truncate(1 << 4);
pub const COLLISION_GROUP_PHYSICS_TOY: Group = Group::from_bits_truncate(1 << 5);

// Characters and entities
pub const COLLISION_GROUP_PLAYER: Group = Group::from_bits_truncate(1 << 9);
pub const COLLISION_GROUP_CHARACTER: Group = Group::from_bits_truncate(1 << 10);
pub const COLLISION_GROUP_NPC: Group = Group::from_bits_truncate(1 << 11);
pub const COLLISION_GROUP_ITEM_DROP: Group = Group::from_bits_truncate(1 << 12);

// Filter masks
pub const COLLISION_FILTER_INSPECTABLE: Group = Group::from_bits_truncate(1 << 16);
pub const COLLISION_FILTER_COLLIDABLE: Group = Group::from_bits_truncate(1 << 17);
pub const COLLISION_FILTER_CLICKABLE: Group = Group::from_bits_truncate(1 << 18);
pub const COLLISION_FILTER_MOVEABLE: Group = Group::from_bits_truncate(1 << 19);
```

### CollisionGroups Structure

```rust
pub struct CollisionGroups {
    pub memberships: Group,  // Which groups this collider belongs to
    pub filters: Group,      // Which groups this collider can interact with
}
```

**Interaction rule**: Two colliders interact if:
```
(self.memberships & other.filters) != 0 && (other.memberships & self.filters) != 0
```

### Usage Examples

**Terrain collider** (`src/zone_loader.rs:2670-2672`):
```rust
Collider::trimesh(collider_verts, collider_indices),
CollisionGroups::new(
    COLLISION_GROUP_ZONE_TERRAIN,  // memberships
    COLLISION_FILTER_MOVEABLE,     // filters (interacts with moveable entities)
)
```

**Character collider** (`src/systems/character_model_add_collider_system.rs:108-111`):
```rust
Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
ColliderParent::new(entity),
CollisionGroups::new(
    COLLISION_GROUP_CHARACTER,
    COLLISION_FILTER_COLLIDABLE,
)
```

## QueryFilter for Scene Queries

### QueryFilter Structure

From `bevy_rapier3d::pipeline::QueryFilter`:

```rust
pub struct QueryFilter<'a> {
    pub flags: QueryFilterFlags,           // Exclude fixed/kinematic/dynamic/sensors
    pub groups: Option<CollisionGroups>,   // Group filtering
    pub exclude_collider: Option<Entity>,  // Exclude specific collider
    pub exclude_rigid_body: Option<Entity>,// Exclude rigid body + colliders
    pub predicate: Option<&'a dyn Fn(Entity) -> bool>,
}
```

### Common Patterns

**Exclude physics toys from movement queries**:
```rust
QueryFilter::new().groups(CollisionGroups::new(
    COLLISION_FILTER_MOVEABLE,
    !COLLISION_GROUP_PHYSICS_TOY,  // Exclude physics toys
))
```

**Query only event/warp objects**:
```rust
QueryFilter::new().groups(CollisionGroups::new(
    Group::all(),
    COLLISION_GROUP_ZONE_EVENT_OBJECT | COLLISION_GROUP_ZONE_WARP_OBJECT,
))
```

## Collision Components

### CollisionPlayer

From `src/components/collision.rs:53`:

```rust
#[derive(Component)]
pub struct CollisionPlayer;
```

Marks the player entity for:
- Wall collision detection (shape casting)
- Ground detection (raycasting)
- Event/warp object proximity checks

### CollisionHeightOnly

```rust
#[derive(Component)]
pub struct CollisionHeightOnly;
```

For NPCs and non-player entities that only need:
- Ground height detection
- Falling/gravity simulation
- No wall collision

### ColliderEntity

```rust
#[derive(Component, Reflect)]
pub struct ColliderEntity {
    pub entity: Entity,  // Entity of the Rapier collider
}
```

Links a game entity to its Rapier collider entity for cleanup.

### ColliderParent

```rust
#[derive(Component, Reflect)]
pub struct ColliderParent {
    pub entity: Entity,  // Parent game entity
}
```

Attached to collider entities to trace back to the owning entity.

## Collision Detection Patterns

### 1. Ray Casting for Ground Detection

From `src/systems/collision_system.rs:248-266`:

```rust
let ray_origin = Vec3::new(
    position.x / 100.0,
    transform.translation.y + 1.35,  // Start above entity
    -position.y / 100.0,
);
let ray_direction = Vec3::new(0.0, -1.0, 0.0);  // Downward
let max_fall_distance = 10000.0;

let collision_height: Option<f32> = if let Some((_hit_entity, distance)) = 
    rapier_context.cast_ray(
        ray_origin,
        ray_direction,
        max_fall_distance,
        false,  // solid only
        QueryFilter::new().groups(CollisionGroups::new(
            COLLISION_FILTER_MOVEABLE,
            !COLLISION_GROUP_PHYSICS_TOY,
        )),
    ) {
    let hit_y = (ray_origin + ray_direction * distance).y;
    Some(hit_y)
} else {
    None
};
```

**Key points**:
- Position is in centimeters, converted to meters for physics
- Ray starts 1.35m above entity (eye level)
- Returns `Option<f32>` - None if no collision (falling off world)

### 2. Shape Casting for Wall Collision

From `src/systems/collision_system.rs:196-238`:

```rust
let new_translation = Vec3::new(
    position.x / 100.0,
    transform.translation.y,
    -position.y / 100.0,
);
let collider_radius = 0.4;
let translation_delta = new_translation - transform.translation;

if translation_delta.length() > 0.00001 {
    let cast_origin = transform.translation + Vec3::new(0.0, 1.2, 0.0);
    let cast_direction = translation_delta.normalize();
    let ball_collider = Collider::ball(collider_radius);
    
    if let Some((_, distance)) = rapier_context.cast_shape(
        cast_origin + cast_direction * collider_radius,
        Quat::default(),
        cast_direction,
        <&dyn Shape>::from(&ball_collider),
        ShapeCastOptions {
            max_time_of_impact: translation_delta.length(),
            target_distance: 0.0,
            compute_impact_geometry_on_penetration: false,
            stop_at_penetration: false,
        },
        QueryFilter::new().groups(CollisionGroups::new(
            COLLISION_FILTER_COLLIDABLE,
            !COLLISION_GROUP_ZONE_TERRAIN & !COLLISION_GROUP_PHYSICS_TOY,
        )),
    ) {
        // Collision detected - clamp position
        let collision_translation =
            cast_origin + translation_delta * (distance.time_of_impact - 0.1).max(0.0);
        position.x = collision_translation.x * 100.0;
        position.y = -(collision_translation.z * 100.0);
        position.z = collision_translation.y * 100.0;
        
        commands.entity(entity).insert(NextCommand::with_stop());
    }
}
```

**Key points**:
- Ball shape with 0.4m radius for player collision
- Casts from current position toward intended position
- Excludes terrain (handled separately) and physics toys
- On collision: clamps position and sends stop command to server

### 3. Shape Intersection for Proximity Detection

From `src/systems/collision_system.rs:291-336`:

```rust
let ball_collider = Collider::ball(1.0);
rapier_context.intersect_shape(
    Vec3::new(
        position.x / 100.0,
        position.z / 100.0 + 1.0,
        -position.y / 100.0,
    ),
    Quat::default(),
    <&dyn Shape>::from(&ball_collider),
    QueryFilter::new().groups(CollisionGroups::new(
        Group::all(),
        COLLISION_GROUP_ZONE_EVENT_OBJECT | COLLISION_GROUP_ZONE_WARP_OBJECT,
    )),
    |hit_entity| {
        // Traverse to parent entity
        let hit_entity = query_collider_parent
            .get(hit_entity)
            .map_or(hit_entity, |collider_parent| collider_parent.entity);
        
        // Handle event objects
        if let Ok(mut hit_event_object) = query_event_object.get_mut(hit_entity) {
            if time.elapsed().as_secs_f64() - hit_event_object.last_collision > 5.0 {
                if !hit_event_object.quest_trigger_name.is_empty() {
                    quest_trigger_events.write(QuestTriggerEvent::DoTrigger(
                        hit_event_object.quest_trigger_name.as_str().into(),
                    ));
                }
                hit_event_object.last_collision = time.elapsed().as_secs_f64();
            }
        }
        // Handle warp objects
        else if let Ok(mut hit_warp_object) = query_warp_object.get_mut(hit_entity) {
            if time.elapsed().as_secs_f64() - hit_warp_object.last_collision > 5.0 {
                game_connection.client_message_tx
                    .send(ClientMessage::WarpGateRequest {
                        warp_gate_id: hit_warp_object.warp_id,
                    })
                    .ok();
                hit_warp_object.last_collision = time.elapsed().as_secs_f64();
            }
        }
        true  // Continue iterating
    },
);
```

**Key points**:
- 1.0m radius ball for generous trigger range
- 5-second cooldown between triggers
- Callback pattern for handling multiple intersections
- Uses `ColliderParent` to find owning entity

## Terrain Following System

### Height-Only Collision (NPCs)

From `src/systems/collision_system.rs:27-110`:

```rust
pub fn collision_height_only_system(
    mut query_collision_entity: Query<(Entity, &mut Position, &mut Transform), With<CollisionHeightOnly>>,
    rapier_context: ReadRapierContext,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    time: Res<Time>,
) {
    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        // Get terrain height from heightmap
        let terrain_height: f32 = current_zone_data.get_terrain_height(position.x, position.y) / 100.0;
        
        // Cast ray downward to detect collision objects (bridges, platforms)
        let collision_height: Option<f32> = if let Some((_hit_entity, distance)) = 
            rapier_context.cast_ray(/* ... */) {
            Some(hit_y)
        } else {
            None
        };

        // Target height is max of terrain and collision height
        let target_y = if let Some(collision_height) = collision_height {
            collision_height.max(terrain_height)
        } else {
            terrain_height
        };

        // Apply gravity-based falling
        let fall_distance = time.delta().as_secs_f32() * 9.81;
        let old_y = transform.translation.y;
        
        if old_y - target_y > fall_distance {
            // Falling
            transform.translation.y = old_y - fall_distance;
        } else {
            // On ground
            transform.translation.y = target_y;
        }
        
        position.z = transform.translation.y * 100.0;
    }
}
```

### Player Collision System

From `src/systems/collision_system.rs:137-338`:

Key differences from height-only:
1. **Wall collision**: Shape casting before movement
2. **Flight mode**: Skips ground collision when flying
3. **Event/warp triggers**: Intersection queries
4. **Server sync**: Sends `MoveCollision` message on wall hit

#### Flight Mode Handling

```rust
let is_flying = flight_state.map_or(false, |fs| fs.is_flying);

if is_flying {
    // Direct position sync, no ground collision
    transform.translation.x = position.x / 100.0;
    transform.translation.y = position.z / 100.0;  // Use position.z for height
    transform.translation.z = -position.y / 100.0;
    continue;
}
```

## Collider Creation

### Zone Objects (Async)

From `src/zone_loader.rs:3036-3040`:

```rust
ColliderParent::new(object_entity),
AsyncCollider(ComputedColliderShape::TriMesh(
    bevy_rapier3d::prelude::TriMeshFlags::FIX_INTERNAL_EDGES,
)),
CollisionGroups::new(collision_group, collision_filter),
```

**TriMeshFlags**:
- `FIX_INTERNAL_EDGES`: Fixes non-manifold edges for better collision
- `MERGE_DUPLICATE_VERTICES`: Combines duplicate vertices
- `empty()`: No processing (faster, used for some objects)

### Character/Entity Colliders (Cuboid)

From `src/systems/character_model_add_collider_system.rs:107-132`:

```rust
let half_extents = compute_collider_half_extents(/* from mesh bounds */);

let collider_entity = commands.spawn((
    Name::new("Collider"),
    Collider::cuboid(half_extents.x, half_extents.y, half_extents.z),
    ColliderParent::new(entity),
    CollisionGroups::new(
        COLLISION_GROUP_CHARACTER,
        COLLISION_FILTER_COLLIDABLE,
    ),
    Transform::from_translation(Vec3::Y * 0.5 * half_extents.y),  // Center at feet
)).id();

commands.entity(entity).insert(ColliderEntity::new(collider_entity));
```

## Coordinate Systems

### Game World vs Physics

| System | Units | Y Direction | Z Direction |
|--------|-------|-------------|-------------|
| Game Position | Centimeters | Forward | Up |
| Physics/Transform | Meters | Up | Back (negative = forward) |

### Conversion

```rust
// Position (cm) → Transform (m)
transform.translation.x = position.x / 100.0;      // Right
transform.translation.y = position.z / 100.0;      // Up
transform.translation.z = -position.y / 100.0;     // Back (negative forward)

// Transform (m) → Position (cm)
position.x = transform.translation.x * 100.0;
position.y = -(transform.translation.z * 100.0);
position.z = transform.translation.y * 100.0;
```

## ReadRapierContext

### Usage

```rust
use bevy_rapier3d::plugin::context::systemparams::ReadRapierContext;

pub fn my_system(
    rapier_context: ReadRapierContext,
) {
    let Ok(rapier_context) = rapier_context.single() else {
        return;  // No physics context available
    };
    
    // Use rapier_context for scene queries
    rapier_context.cast_ray(/* ... */);
    rapier_context.cast_shape(/* ... */);
    rapier_context.intersect_shape(/* ... */);
}
```

### Available Methods

- `cast_ray(origin, direction, max_distance, solid, filter)` → `Option<(Entity, f32)>`
- `cast_shape(origin, rotation, direction, shape, options, filter)` → `Option<(Entity, ShapeCastHit)>`
- `intersect_shape(origin, rotation, shape, filter, callback)` → iterates all intersections

## File References

| File | Purpose |
|------|---------|
| `src/components/collision.rs` | Collision groups, marker components |
| `src/systems/collision_system.rs` | Player/NPC collision detection |
| `src/zone_loader.rs` | Terrain/water/zone object colliders |
| `src/systems/character_model_add_collider_system.rs` | Character colliders |
| `src/systems/npc_model_add_collider_system.rs` | NPC colliders |
| `src/systems/item_drop_model_system.rs` | Item drop colliders |
| `src/lib.rs:847` | RapierPhysicsPlugin configuration |
| `src/lib.rs:1651-1688` | Physics system ordering |

## Bevy Rapier Source

For reference, bevy_rapier3d v0.31.0 source is available at:
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy_rapier3d-0.31.0\src\`

Key files:
- `plugin/plugin.rs:1-200`: PhysicsSet, RapierPhysicsPlugin
- `plugin/configuration.rs:1-150`: RapierConfiguration, TimestepMode
- `geometry/collider.rs:1-400`: Collider, CollisionGroups, Group
- `pipeline/query_filter.rs:1-100`: QueryFilter
- `plugin/context/mod.rs:1-300`: RapierContext, scene query methods

## Troubleshooting

### Bevy 0.18 Migration Issues

#### Issue 1: ReadRapierContext Requires `.single()` Call

**Problem**: After migrating to Bevy 0.18, `ReadRapierContext` no longer auto-dereferences.

**Before (Bevy 0.14)**:
```rust
pub fn my_system(rapier_context: ReadRapierContext) {
    rapier_context.cast_ray(/* ... */);  // Direct access
}
```

**After (Bevy 0.18)**:
```rust
pub fn my_system(rapier_context: ReadRapierContext) {
    let Ok(rapier_context) = rapier_context.single() else {
        return;  // No physics context available
    };
    rapier_context.cast_ray(/* ... */);  // Now works
}
```

**Solution**: Always call `.single()` on `ReadRapierContext` and handle the `Result`:
- `src/systems/collision_system.rs:37`: Height-only collision system
- `src/systems/collision_system.rs:154`: Player collision system

**Source**: Bevy 0.18 changed system parameter semantics - resources wrapped in `NonSend` now require explicit extraction.

---

#### Issue 2: PhysicsSet Location Changed

**Problem**: `PhysicsSet` moved from `bevy_rapier3d::prelude` to `bevy_rapier3d::plugin`.

**Before**:
```rust
use bevy_rapier3d::prelude::PhysicsSet;
```

**After**:
```rust
use bevy_rapier3d::plugin::PhysicsSet;
```

**Solution**: Updated import in `src/lib.rs:49`.

**Source**: bevy_rapier3d v0.31 reorganized module structure.

---

#### Issue 3: AsyncCollider ComputedColliderShape API Change

**Problem**: `AsyncCollider` constructor signature changed in Rapier 0.24.

**Before**:
```rust
AsyncCollider::new(ComputedColliderShape::TriMesh {
    collision_shape: (&shape).into(),
    flags: TriMeshFlags::FIX_INTERNAL_EDGES,
})
```

**After**:
```rust
AsyncCollider(ComputedColliderShape::TriMesh(
    TriMeshFlags::FIX_INTERNAL_EDGES,
))
```

**Solution**: Updated all async collider spawns:
- `src/zone_loader.rs:3037`: Zone object colliders
- `src/zone_loader.rs:3297`: Water colliders

**Source**: `bevy_rapier3d::geometry::async_collider::AsyncCollider` now uses tuple struct syntax.

---

#### Issue 4: ShapeCastOptions Field Name Changes

**Problem**: Some `ShapeCastOptions` fields were renamed or reorganized.

**Verified fields in current code** (`src/systems/collision_system.rs:210-216`):
```rust
ShapeCastOptions {
    max_time_of_impact: translation_delta.length(),
    target_distance: 0.0,
    compute_impact_geometry_on_penetration: false,
    stop_at_penetration: false,
}
```

**Solution**: These fields are correct for Rapier 0.24. If migration issues occur, check:
- `multibody`: Removed in favor of separate multibody query methods
- `solid`: Moved to query filter flags

---

#### Issue 5: QueryFilter Groups Negation Syntax

**Problem**: Negating collision groups uses `!` prefix, not subtraction.

**Incorrect**:
```rust
COLLISION_FILTER_COLLIDABLE - COLLISION_GROUP_PHYSICS_TOY  // Doesn't work
```

**Correct**:
```rust
COLLISION_FILTER_COLLIDABLE & !COLLISION_GROUP_PHYSICS_TOY  // Bitwise AND with NOT
```

**Examples in code**:
- `src/systems/collision_system.rs:219`: Wall collision excludes terrain + physics toys
- `src/systems/collision_system.rs:65`: Ground ray excludes physics toys

**Source**: `bevy_rapier3d::prelude::Group` implements bitwise operations via `bitflags`.

---

#### Issue 6: Collider Parent Entity Cleanup

**Problem**: When entities are despawned, their child colliders may orphan if not properly tracked.

**Solution**: Use `ColliderEntity` component to track collider entity:
```rust
// When spawning collider (src/systems/character_model_add_collider_system.rs:132)
commands.entity(entity).insert(ColliderEntity::new(collider_entity));

// Cleanup happens automatically via Bevy entity hierarchy
// or explicit despawn in entity cleanup systems
```

**Pattern**: Always pair `ColliderParent` (on collider) with `ColliderEntity` (on owner).

---

#### Issue 7: Terrain Height Missing After Zone Load

**Symptom**: NPCs fall through terrain or float above it immediately after zone transition.

**Cause**: `collision_height_only_system` and `collision_player_system` check for `CurrentZone` resource:
- `src/systems/collision_system.rs:43`: Warns if no CurrentZone
- `src/systems/collision_system.rs:51`: Warns if zone data not loaded

**Solution**: 
1. `collision_player_system_join_zone` runs first to position player from server data
2. Zone loader sets `CurrentZone` resource before entities spawn
3. Physics systems skip terrain following until zone data is available

**Diagnostic**: Check logs for `[NPC_TERRAIN_DIAG]` or `[TERRAIN_DIAG]` warnings.

---

#### Issue 8: Collision Groups Not Filtering Correctly

**Symptom**: Player collides with physics toys or can't interact with event objects.

**Debug steps**:
1. Verify `CollisionGroups` membership and filter are set correctly on spawn
2. Check `QueryFilter` uses correct groups for query type
3. Remember: interaction requires bidirectional filter match

**Formula** (`src/components/collision.rs:109`):
```
(self.memberships & other.filters) != 0 && (other.memberships & self.filters) != 0
```

**Common mistake**: Setting membership but forgetting filter, or vice versa.

---

#### Issue 9: Raycast Returns None for Terrain

**Symptom**: Entity falls off world even when terrain should be present.

**Check**:
1. Terrain trimesh collider spawned correctly (check zone loader)
2. Ray origin is above terrain (`transform.translation.y + 1.35`)
3. Ray direction is downward (`Vec3::new(0.0, -1.0, 0.0)`)
4. Max distance is sufficient (`10000.0` meters in current code)
5. QueryFilter includes `COLLISION_FILTER_MOVEABLE` and terrain group

**Source**: `src/systems/collision_system.rs:68-82`

---

#### Issue 10: Physics Toys Interfere with Movement

**Symptom**: Player gets stuck on decorative physics objects.

**Solution**: Physics toys are excluded from movement queries:
- Ground raycast: `!COLLISION_GROUP_PHYSICS_TOY` (line 65)
- Wall cast: `!COLLISION_GROUP_PHYSICS_TOY` (line 219)

**Trade-off**: Physics toys still collide with each other and can be inspected/picked up.

---

### Common Physics Debugging Commands

**Enable physics debug rendering** (if available):
```rust
// Add to app plugins
RapierDebugRenderPlugin::default()
```

**Log collision queries**:
```rust
log::info!("Raycast result: {:?}", rapier_context.cast_ray(/* ... */));
```

**Check collider count**:
```rust
let collider_count = query_colliders.iter().count();
log::info!("Active colliders: {}", collider_count);
```

---

### Performance Considerations

1. **Minimize shape casts**: Wall collision runs every frame for player - keep radius small (0.4m)
2. **Batch terrain queries**: Use heightmap for base terrain, raycast only for elevation changes
3. **Exclude unnecessary groups**: Physics toys excluded from movement queries saves CPU
4. **Async colliders**: Zone objects use async loading to avoid main thread blocking

---

### Known Limitations

1. **No continuous collision detection (CCD)**: Fast-moving objects may tunnel through colliders
2. **No physics-based movement**: All movement is manual with collision checks, not forces
3. **No joint constraints**: Rapier joints not used in current implementation
4. **Single physics world**: All entities share one collision world (no per-zone worlds)

## Source File References

### Bevy Rapier3d (v0.31.0)

| File | Path | Purpose |
|------|------|---------|
| Plugin | `bevy_rapier3d/plugin/plugin.rs` | PhysicsSet, RapierPhysicsPlugin |
| Configuration | `bevy_rapier3d/plugin/configuration.rs` | RapierConfiguration, TimestepMode |
| Collider | `bevy_rapier3d/geometry/collider.rs` | Collider, CollisionGroups, Group |
| QueryFilter | `bevy_rapier3d/pipeline/query_filter.rs` | QueryFilter, QueryFilterFlags |
| Context | `bevy_rapier3d/plugin/context/mod.rs` | RapierContext, scene queries |
| ShapeCast | `bevy_rapier3d/geometry/shape_cast.rs` | ShapeCastOptions, ShapeCastHit |

### Project Files

| File | Purpose |
|------|---------|
| `src/components/collision.rs` | Collision groups, marker components |
| `src/systems/collision_system.rs` | Player/NPC collision detection |
| `src/zone_loader.rs` | Terrain/water/zone object colliders |
| `src/systems/character_model_add_collider_system.rs` | Character colliders |
| `src/systems/npc_model_add_collider_system.rs` | NPC colliders |
| `src/systems/item_drop_model_system.rs` | Item drop colliders |
| `src/lib.rs:847` | RapierPhysicsPlugin configuration |
| `src/lib.rs:1651-1688` | Physics system ordering |
