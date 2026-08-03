# Monster Collision System Plan

## Overview

This plan outlines the implementation of collision detection and response for **hostile monsters** to prevent them from overlapping when fighting multiple monsters. The system uses a **simple soft collision** approach implemented as a custom ECS system running alongside the existing bevy_rapier3d infrastructure.

### Design Decisions (User Confirmed)
- **Equal separation** for all monsters regardless of size
- **Hostile monsters only** - friendly NPCs will not have this behavior
- **Simple overlap avoidance** - no tactical combat positioning

## Current State Analysis

### Existing Collision Infrastructure

The project already has a robust collision system using `bevy_rapier3d` v0.33:

1. **Collision Groups** (defined in [`src/components/collision.rs`](src/components/collision.rs:57-72)):
 - `COLLISION_GROUP_NPC` (bit 11) - Used for both NPCs and monsters
 - `COLLISION_GROUP_PLAYER` (bit 9)
 - `COLLISION_GROUP_CHARACTER` (bit 10)
 - `COLLISION_FILTER_MOVEABLE` (bit 19) - For entities that can move
 - `COLLISION_FILTER_COLLIDABLE` (bit 17) - For solid objects

2. **Monster Collider Creation** (in [`src/systems/npc_model_add_collider_system.rs`](src/systems/npc_model_add_collider_system.rs:92-106)):
 - Creates cuboid colliders based on AABB bounds
 - Uses `COLLISION_GROUP_NPC` for membership
 - Filter mask: `COLLISION_FILTER_INSPECTABLE | COLLISION_FILTER_CLICKABLE | COLLISION_GROUP_PHYSICS_TOY`
 - **Problem**: Does NOT include `COLLISION_GROUP_NPC` in filter mask, so monsters don't collide with each other

3. **Monster Movement** (in [`src/systems/update_position_system.rs`](src/systems/update_position_system.rs)):
 - Direct position updates based on `CommandMove`
 - No collision avoidance or separation behavior

### Why Monsters Currently Overlap

Monsters don't collide with each other because:
1. Their collision filter (`CollisionGroups`) doesn't include `COLLISION_GROUP_NPC` in the filter mask
2. Movement system doesn't check for collisions with other monsters
3. No separation/avoidance behavior exists

## Proposed Solution

### Approach: Soft Collision with Separation Force

Rather than using hard physics collisions (which would make monsters feel like solid objects), we'll implement a **soft collision** system that applies separation forces to push overlapping monsters apart. This approach:

- Keeps monsters from stacking/overlapping
- Allows monsters to still get close during combat
- Feels more natural for game gameplay
- Doesn't interfere with server-authoritative movement

### Architecture Diagram

```mermaid
flowchart TB
 subgraph Detection[Collision Detection]
  direction TB
  A[Monster Query] --> B[Spatial Lookup]
  B --> C[Overlap Detection]
 end
 subgraph Resolution[Collision Resolution]
  direction TB
  D[Calculate Separation Vector] --> E[Apply to Position]
  E --> F[Update Transform]
 end
 subgraph Existing[Existing Systems]
  direction TB
  G[Command System] --> H[Update Position System]
  H --> I[Collision Height System]
 end
 C --> D
 F --> I
```

## Implementation Steps

### Step 1: Identify Monsters with a Marker Component

No new Rapier collision group was added (there is no `COLLISION_GROUP_MONSTER` in `src/components/collision.rs`). Monsters are instead identified by the `MonsterSeparation` marker component combined with the `ClientEntityType::Monster` entity type, which the separation system filters on. Existing collision groups for click/inspect remain unchanged.

### Step 2: Create Monster Separation Component

Add a component to track separation state for hostile monsters only:

```rust
// In src/components/monster_separation.rs
use bevy::prelude::{Component, Reflect};

/// Component for hostile monster collision separation.
/// This is only added to monsters (ClientEntityType::Monster), not NPCs.
#[derive(Component, Reflect)]
pub struct MonsterSeparation {
    /// Radius for separation detection in meters
    pub separation_radius: f32,
    /// Force strength when overlapping
    pub separation_force: f32,
    /// Maximum separation per frame in meters
    pub max_separation: f32,
}

impl Default for MonsterSeparation {
    fn default() -> Self {
        Self {
            separation_radius: 1.0,  // 1 meter
            separation_force: 5.0,
            max_separation: 2.0,     // 2 meters per second max
        }
    }
}
```

### Step 3: Add MonsterSeparation at Spawn

`npc_model_add_collider_system.rs` was not modified. Instead, `MonsterSeparation::default()` is inserted directly at spawn time:
1. Network-spawned monsters: `SpawnEntityMonster` handler in [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs:717)
2. Sea monsters: [`src/zone_content/monsters.rs`](src/zone_content/monsters.rs:296) (e.g. `separation_radius: 2.0` for sharks)

Existing collision groups for click/inspect functionality are unchanged. The marker component resolves the `ClientEntityType` lookup problem: the separation system queries entities that have `MonsterSeparation` and re-checks `ClientEntityType::Monster` at runtime.

### Step 4: Create Monster Separation System

Create a new system that runs after movement and applies only to hostile monsters:

```rust
// In src/systems/monster_separation_system.rs
use crate::components::{ClientEntity, ClientEntityType, MonsterSeparation, Position};
use bevy::prelude::*;

/// System that pushes overlapping hostile monsters apart.
/// Only applies to entities with ClientEntityType::Monster.
/// Note: Position is in centimeters, so we need to scale our separation values accordingly.
pub fn monster_separation_system(
    mut query: Query<(Entity, &mut Position, &MonsterSeparation, &ClientEntity)>,
    time: Res<Time>,
) {
    // Convert separation radius from meters to centimeters for comparison with Position
    // Collect all monster positions for overlap checking
    let monster_positions: Vec<(Entity, Vec3, f32)> = query
        .iter()
        .filter(|(_, _, _, client_entity)| client_entity.entity_type == ClientEntityType::Monster)
        .map(|(e, pos, sep, _)| {
            // Convert separation_radius from meters to centimeters
            (e, pos.position, sep.separation_radius * 100.0)
        })
        .collect();

    for (entity, mut position, separation, client_entity) in query.iter_mut() {
        // Only apply to hostile monsters
        if client_entity.entity_type != ClientEntityType::Monster {
            continue;
        }

        let mut total_separation = Vec3::ZERO;
        let mut overlap_count = 0;

        let my_radius_cm = separation.separation_radius * 100.0; // Convert to centimeters

        for (other_entity, other_pos, other_radius_cm) in &monster_positions {
            if *other_entity == entity {
                continue;
            }

            let distance = (position.position - *other_pos).length();
            let min_distance = my_radius_cm + other_radius_cm;

            if distance < min_distance && distance > 0.001 {
                // Calculate overlap and push direction
                let overlap = min_distance - distance;
                let direction = (position.position - *other_pos).normalize();

                // Add separation force proportional to overlap
                // overlap is in centimeters, force is a multiplier
                total_separation += direction * overlap * separation.separation_force;
                overlap_count += 1;
            }
        }

        if overlap_count > 0 {
            // Apply averaged separation, clamped to max (converted to centimeters)
            // max_separation is in meters per second, convert to cm/s
            let max_sep_cm_per_sec = separation.max_separation * 100.0;
            let separation_vector = (total_separation / overlap_count as f32)
                .clamp_length_max(max_sep_cm_per_sec * time.delta_secs());
            position.position += separation_vector;
        }
    }
}
```

### Step 5: Integration with Movement Pipeline

The separation system should run in this order:
1. `update_position_system` - Apply movement commands
2. `monster_separation_system` - Push overlapping monsters apart
3. `collision_height_only_system` - Snap to terrain height

## Detailed Design

### Separation Algorithm

```
for each monster A:
    total_separation = Vec3::ZERO
    overlap_count = 0
    
    for each nearby monster B:
        distance = distance_between(A.position, B.position)
        min_distance = A.radius + B.radius
        
        if distance < min_distance:
            // Calculate overlap
            overlap = min_distance - distance
            direction = normalize(A.position - B.position)
            
            // Add separation force (stronger when more overlap)
            force = separation_force * (overlap / min_distance)
            total_separation += direction * force
            overlap_count += 1
    
    if overlap_count > 0:
        // Apply averaged separation, clamped to max
        separation = clamp(total_separation, max_separation)
        A.position += separation * time.delta_secs()
```

### Configuration Parameters

| Parameter | Default Value | Description |
|-----------|---------------|-------------|
| `separation_radius` | 1.0 meters | Distance at which separation begins |
| `separation_force` | 5.0 | Strength of separation push |
| `max_separation` | 2.0 m/s | Maximum separation velocity |

**Note**: All monsters use the same parameters (equal separation). No size-based variation.

## Alternative Approaches Considered

### 1. Hard Physics Collision (Rejected)
Using `RigidBody::Dynamic` with full physics simulation.
- **Pros**: Realistic collision response
- **Cons**: Would interfere with server-authoritative movement, feels too rigid

### 2. Kinematic Character Controller (Rejected)
Using Rapier's `KinematicCharacterController`.
- **Pros**: Built-in collision handling
- **Cons**: Major refactor required, designed for player-style movement

### 3. Sensor Colliders Only (Rejected)
Using sensor colliders with events.
- **Pros**: Event-driven, clean separation
- **Cons**: Requires additional event handling infrastructure

### 4. Chosen: Soft Separation Forces (Simple)
Custom system applying gentle separation forces with equal strength for all monsters.
- **Pros**: Simple, controllable, doesn't fight server movement, easy to tune
- **Cons**: Custom implementation required, O(n²) complexity without spatial partitioning

## Files to Modify

| File | Changes |
|------|---------|
| [`src/components/monster_separation.rs`](src/components/monster_separation.rs) | New component definition |
| [`src/components/mod.rs`](src/components/mod.rs) | Export `MonsterSeparation` component |
| [`src/systems/game_connection_system.rs`](src/systems/game_connection_system.rs) | Add `MonsterSeparation` component when spawning hostile monsters (line 717) |
| [`src/zone_content/monsters.rs`](src/zone_content/monsters.rs) | Add `MonsterSeparation` to sea monsters (line 296) |
| [`src/systems/monster_separation_system.rs`](src/systems/monster_separation_system.rs) | Separation logic system |
| [`src/systems/mod.rs`](src/systems/mod.rs) | Export new system |
| [`src/lib.rs`](src/lib.rs) | Register new system in schedule after `update_position_system` |

## System Ordering

The separation system must run in this specific order:

```
update_position_system      // Apply movement commands
    ↓
monster_separation_system   // Push overlapping monsters apart (NEW)
    ↓
collision_height_only_system // Snap to terrain height
```

## Testing Strategy

1. **Visual Debug**: Add optional gizmo drawing for separation radii (debug mode)
2. **In-Game Testing**:
 - Spawn multiple monsters (e.g., `/mon 1 5`)
 - Verify they spread out naturally
 - Verify combat still works correctly
 - Verify no performance degradation with 10+ monsters

## Performance Considerations

For the simple implementation:
- O(n²) overlap checks - acceptable for <20 monsters
- Only processes hostile monsters (not NPCs)
- Early exit if no overlaps detected

Future optimization if needed:
- Use spatial partitioning (grid or quadtree) for O(n) checks
- Only run separation for moving monsters
- Use Rapier's `intersections_with` for efficient overlap queries

## Summary

This plan implements a **simple soft collision system** for hostile monsters using separation forces. The approach:

- **Equal separation** for all monsters regardless of size
- **Hostile monsters only** - friendly NPCs are excluded
- **Simple overlap avoidance** - no tactical positioning
- Reuses existing bevy_rapier3d infrastructure where possible
- Adds minimal new code (~100 lines)
- Doesn't interfere with server-authoritative movement
- Is configurable through component parameters

### Implementation Complexity: Low
- 1 new component file
- 1 new system file
- ~5 lines of changes to existing files
