# Sailing System — Comprehensive Implementation Plan

## Executive Summary

This plan describes how to introduce a complete sailing system into ROSE Online, built on top of the existing Bevy 0.18.1 game client (`rose-offline-client`) and the `rose-offline` server. The feature spans:

1. **A new ocean zone** — a large, mostly-water map with islands
2. **Boat entities** with procedural sail meshes and animations
3. **Wind simulation** that drives sailing physics
4. **Sailing movement mechanics** — wind-relative steering, tacking, luffing
5. **Client UI** — compass / wind indicator, speed gauge, sail trim controls
6. **Server authority** — position validation, zone transitions, entity sync
7. **Audio & VFX** — wake particles, wave splashes, creaking sounds, sail flapping

Each section below is self-contained enough to be assigned as a separate work item.

---

## Table of Contents

1. [Architecture Overview](#1-architecture-overview)
2. [Phase 1 — Ocean Zone Map](#2-phase-1--ocean-zone-map)
3. [Phase 2 — Boat Entity & Model](#3-phase-2--boat-entity--model)
4. [Phase 3 — Wind System](#4-phase-3--wind-system)
5. [Phase 4 — Sailing Movement](#5-phase-4--sailing-movement)
6. [Phase 5 — Server Authority & Networking](#6-phase-5--server-authority--networking)
7. [Phase 6 — Camera & Controls](#7-phase-6--camera--controls)
8. [Phase 7 — Sail Trim UI](#8-phase-7--sail-trim-ui)
9. [Phase 8 — Visual Effects](#9-phase-8--visual-effects)
10. [Phase 9 — Audio](#10-phase-9--audio)
11. [Phase 10 — Island Content & NPCs](#11-phase-10--island-content--npcs)
12. [Phase 11 — Polish & Optimisation](#12-phase-11--polish--optimisation)
13. [File Inventory](#13-file-inventory)
14. [Risk Register](#14-risk-register)
15. [Glossary](#15-glossary)

---

## 1. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                            Sailing System                                    │
├───────────────┬──────────────┬────────────────┬──────────────────────────────┤
│  Ocean Zone   │  Boat Entity │  Wind System   │  Sailing Movement            │
│  (map data)   │  & Model     │  (resource)    │  (physics system)            │
├───────────────┼──────────────┼────────────────┼──────────────────────────────┤
│  Zone files   │  Components  │  WindState     │  sailing_movement_system     │
│  HIM/TIL/IFO  │  BoatState   │  WindSettings  │  boat_buoyancy_system        │
│  Water planes │  SailModel   │  wind_system   │  sail_animation_system       │
│  Island deco  │  BoatModel   │                │  wake_effect_system          │
└───────────────┴──────────────┴────────────────┴──────────────────────────────┘
                                     │
                              ┌──────┴──────┐
                              │  Server     │
                              │  Authority  │
                              │  (rose-     │
                              │   offline)  │
                              └─────────────┘
```

### Coordinate System Reference (from [`flight_movement_system.rs`](src/systems/flight_movement_system.rs:13))

| Space | X | Y | Z |
|-------|---|---|---|
| **Position** (game, cm) | right | forward | up |
| **Transform** (world, m) | right | up | back |

Zone entities spawn at `Transform(5200, 0, -5200)` as seen in [`zone_loader.rs:2080`](src/zone_loader.rs:2080).

---

## 2. Phase 1 — Ocean Zone Map

### Goal
Create a new zone ID (e.g. `ZoneId(200)`) that is predominantly water, with scattered islands and a few dock NPCs.

### 2.1 Zone Data Files

The zone system uses a 64×64 block grid. Each block is 160×160 world-metres. The existing loader ([`zone_loader.rs`](src/zone_loader.rs)) reads:

| File | Purpose |
|------|---------|
| `*.ZON` | Zone metadata: tile textures, grid size, tile definitions |
| `{x}_{y}.HIM` | Heightmap per block (height in cm) |
| `{x}_{y}.TIL` | Tile map per block (texture indices) |
| `{x}_{y}.IFO` | Object placement (water planes, NPCs, warps, objects) |

For the ocean zone:

- **Heightmap**: Blocks that are purely ocean should have flat low heightmaps (e.g. height = -500 cm everywhere). Island blocks have elevated terrain.
- **Water planes**: Every ocean block's IFO should include a water plane covering the full block, at height = 0 cm.
- **Island blocks** (approximately 6–10 islands): Normal terrain heightmaps rising above water level, with deco objects for palm trees, docks, buildings.
- **Tile textures**: Ocean floor sand, island grass, rock, beach sand.

### 2.2 Zone Registration

**Server** (`rose-offline`):

- Add zone entry to [`ZoneList`](C:/Users/vicha/RustroverProjects/rose-offline/rose-data/src/zone_database.rs) — typically loaded from STB files.
- Create zone-specific data: skybox settings, ambient lighting, fog (set ocean fog to blue-green).
- Create NPC spawn data for dock NPCs and island inhabitants.

**Client** (`rose-offline-client`):

- The existing [`zone_loader_system`](src/zone_loader.rs:1287) and [`spawn_zone`](src/zone_loader.rs:2000) handle loading automatically.
- No client code changes needed for basic zone loading.

### 2.3 Map Design Specifications

```
 ┌────────────────────────────────────────────────────┐
 │                    OCEAN ZONE                       │
 │                                                      │
 │     🏝 Island A                                     │
 │       (Dock NPC, Boat vendor)                       │
 │                                                      │
 │                        🏝 Island B                  │
 │                          (Trading post)              │
 │                                                      │
 │   🏝 Island C                                       │
 │     (Quest NPC)        🏝 Island D                  │
 │                          (Small, treasure)           │
 │                                                      │
 │                                                      │
 │              🏝 Island E                            │
 │                (Town, warp gate)                     │
 │                                                      │
 │                              🏝 Island F            │
 │                                (Fishing spot)        │
 │                                                      │
 └────────────────────────────────────────────────────┘
```

- Total playable area: ~40×40 blocks = 6400×6400 metres (~6.4 km²)
- 6–8 islands ranging from 2×2 to 5×5 blocks
- Warp gate on Island E connecting back to the mainland

### 2.4 Deliverables

| Item | Owner | Notes |
|------|-------|-------|
| Zone HIM/TIL/IFO files | Level Designer | Use map editor ([`src/map_editor/`](src/map_editor/mod.rs)) |
| Zone STB entry | Server Dev | New ZoneId registration |
| Warp gate to/from mainland | Server Dev | Bi-directional zone transition |
| Island deco objects (ZSC) | Art / Level Designer | Palm trees, docks, rocks |
| Ocean fog settings | Client Dev | Blue-green distance fog |

---

## 3. Phase 2 — Boat Entity & Model

### Goal
Create a boat that a player can board, see visually, and ride on the water surface.

### 3.1 New Components

**File: `src/components/boat.rs`**

```rust
/// Represents a boat entity that can carry a player
#[derive(Component, Reflect)]
pub struct BoatState {
    /// Entity of the player character riding this boat
    pub rider_entity: Option<Entity>,
    /// Current heading in radians (0 = North, clockwise)
    pub heading: f32,
    /// Current speed in game units/sec
    pub speed: f32,
    /// Maximum speed achievable with perfect wind
    pub max_speed: f32,
    /// Current sail trim angle relative to boat heading (0..PI)
    pub sail_trim: f32,
    /// Rudder angle (-1.0 to 1.0, left to right)
    pub rudder: f32,
    /// Hull health (for future combat)
    pub hull_health: f32,
    pub hull_max_health: f32,
}

/// Visual model parts of the boat
#[derive(Component, Reflect)]
pub struct BoatModel {
    pub hull_entity: Entity,
    pub mast_entity: Entity,
    pub sail_entity: Entity,
    pub rudder_entity: Entity,
    pub rider_seat_entity: Entity,
}

/// Marker component for the sail mesh (animated by wind)
#[derive(Component, Reflect)]
pub struct SailMesh {
    /// Billow factor 0.0 (limp) to 1.0 (fully filled)
    pub billow: f32,
    /// Which side the sail is on relative to wind
    pub side: SailSide,
}

#[derive(Reflect, Clone, Copy, PartialEq)]
pub enum SailSide { Port, Starboard, Center }
```

### 3.2 Boat Model Approach

Two approaches for the boat mesh:

| Approach | Pros | Cons |
|----------|------|------|
| **A: Procedural mesh** | No art dependency, easy iteration | Less detail |
| **B: ZMS model files** | High quality, consistent with game | Requires art pipeline |

**Recommended**: Start with **procedural mesh** (like the wing system in [`flight_movement_system.rs`](src/systems/flight_movement_system.rs)) for rapid prototyping, then swap to ZMS models later.

### 3.3 Procedural Boat Mesh

```rust
fn create_boat_hull_mesh() -> Mesh {
    // Simple hull: elongated oval / pointed bow
    // ~30 vertices, ~50 triangles
    // Dimensions: 4m long, 1.5m wide, 0.5m draft, 0.3m freeboard
}

fn create_sail_mesh(billow: f32) -> Mesh {
    // Rectangular sail, 3m tall, 2m wide
    // Subdivided into 8x8 grid for deformation
    // billow parameter curves the sail outward (Bezier-like)
}

fn create_mast_mesh() -> Mesh {
    // Simple cylinder: 0.05m radius, 4m tall
}
```

### 3.4 Boat Spawning

Following the pattern of the existing vehicle system ([`Vehicle`](src/components/vehicle.rs), [`VehicleModel`](src/components/vehicle_model.rs)):

```rust
fn spawn_boat(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    position: Vec3,
    rider_entity: Entity,
) -> Entity {
    let boat_entity = commands.spawn((
        BoatState { rider_entity: Some(rider_entity), heading: 0.0, speed: 0.0, .. },
        Position::new(position),
        FacingDirection::default(),
        Transform::default(),
        GlobalTransform::default(),
        Visibility::Visible,
        // ... other required components
    )).id();

    // Spawn hull, mast, sail as children
    let hull = commands.spawn((Mesh3d(..), MeshMaterial3d(..), ..)).id();
    let mast = commands.spawn((Mesh3d(..), MeshMaterial3d(..), ..)).id();
    let sail = commands.spawn((SailMesh { billow: 0.0, side: SailSide::Center }, Mesh3d(..), ..)).id();
    
    commands.entity(boat_entity).add_child(hull);
    commands.entity(boat_entity).add_child(mast);
    commands.entity(boat_entity).add_child(sail);
    
    boat_entity
}
```

### 3.5 Player ↔ Boat Interaction

| Action | Trigger | Effect |
|--------|---------|--------|
| Board boat | Click boat / interact with NPC | Player entity becomes child of boat, hidden, BoatState.rider_entity set |
| Disembark | Press `E` near dock / island | Player unparented, teleported to shore, BoatState.rider_entity cleared |
| Acquire boat | Buy from NPC / chat command `/boat` | Boat spawned at nearest dock |

Boarding follows the existing vehicle pattern: the existing [`MoveMode::Drive`](C:/Users/vicha/RustroverProjects/rose-offline/rose-game-common/src/components/move_mode.rs:5) maps to vehicle mode. We introduce a new `MoveMode::Sail` or repurpose `Drive` with a boat-specific flag.

### 3.6 Deliverables

| Item | Owner | Notes |
|------|-------|-------|
| `src/components/boat.rs` | Client Dev | BoatState, BoatModel, SailMesh |
| `src/systems/boat_spawn_system.rs` | Client Dev | Spawn/despawn boats |
| Procedural hull/sail meshes | Client Dev | Placeholder geometry |
| Board/disembark interaction | Client Dev + Server Dev | Network sync |

---

## 4. Phase 3 — Wind System

### Goal
A global wind resource that changes over time, driving sail physics and visual effects.

### 4.1 Wind Resource

**File: `src/resources/wind_state.rs`**

```rust
/// Global wind state — updated each frame
#[derive(Resource, Reflect, Clone)]
pub struct WindState {
    /// Wind direction as a 2D vector (in position-space XY plane)
    /// Magnitude = wind speed in m/s
    pub direction: Vec2,
    /// Wind speed (convenience, = direction.length())
    pub speed: f32,
    /// Wind angle in radians (0 = blowing from North)
    pub angle: f32,
    /// Gusting factor (0.0 = calm, 1.0 = strong gusts)
    pub gust_factor: f32,
    /// Time accumulator for Perlin-noise wind changes
    pub time_accumulator: f32,
}

/// Settings for wind behaviour
#[derive(Resource, Reflect, Clone)]
pub struct WindSettings {
    /// Base wind speed (m/s) — the "average" wind
    pub base_speed: f32,
    /// How fast the wind direction drifts (radians/sec)
    pub direction_drift_speed: f32,
    /// Gust frequency (cycles per second)
    pub gust_frequency: f32,
    /// Maximum gust strength multiplier (e.g. 1.5 = 50% above base)
    pub gust_max_multiplier: f32,
    /// How quickly wind direction can shift during a gust
    pub gust_direction_variance: f32,
}

impl Default for WindSettings {
    fn default() -> Self {
        Self {
            base_speed: 5.0,           // 5 m/s ≈ 10 knots, gentle breeze
            direction_drift_speed: 0.05, // Very slow direction change
            gust_frequency: 0.1,        // One gust every 10 seconds
            gust_max_multiplier: 1.5,
            gust_direction_variance: 0.3, // ~17° shift during gusts
        }
    }
}
```

### 4.2 Wind Update System

**File: `src/systems/wind_system.rs`**

```rust
pub fn wind_update_system(
    time: Res<Time>,
    settings: Res<WindSettings>,
    mut wind: ResMut<WindState>,
) {
    wind.time_accumulator += time.delta_secs();
    let t = wind.time_accumulator;

    // Slowly drift base direction (Perlin-like via layered sine waves)
    let base_angle = (t * settings.direction_drift_speed).sin() * 0.5
        + (t * settings.direction_drift_speed * 0.37).sin() * 0.3
        + (t * settings.direction_drift_speed * 0.13).sin() * 0.2;
    
    // Gusting: periodic speed bumps
    let gust = ((t * settings.gust_frequency * std::f32::consts::TAU).sin() * 0.5 + 0.5)
        .powf(3.0); // Sharp peaks
    let gust_angle_offset = gust * settings.gust_direction_variance
        * (t * 2.3).sin(); // Slight direction shift during gust
    
    wind.angle = base_angle + gust_angle_offset;
    wind.speed = settings.base_speed * (1.0 + gust * (settings.gust_max_multiplier - 1.0));
    wind.gust_factor = gust;
    
    wind.direction = Vec2::new(wind.angle.sin(), wind.angle.cos()) * wind.speed;
}
```

### 4.3 Integration with Existing Wind Sway

The existing [`WindSwaySettings`](src/components/wind_effect.rs:6) and [`wind_sway_system`](src/components/wind_effect.rs:127) affect vegetation. We should integrate the global `WindState` so trees/grass sway in the same direction as the sailing wind:

```rust
pub fn sync_vegetation_wind_system(
    wind: Res<WindState>,
    mut sway_settings: ResMut<WindSwaySettings>,
) {
    // Scale vegetation sway intensity by actual wind speed
    sway_settings.global_intensity = (wind.speed / 10.0).clamp(0.05, 0.3);
}
```

### 4.4 Deliverables

| Item | Owner | Notes |
|------|-------|-------|
| `src/resources/wind_state.rs` | Client Dev | WindState, WindSettings |
| `src/systems/wind_system.rs` | Client Dev | wind_update_system |
| Vegetation sync system | Client Dev | Optional but nice |
| Wind visual indicator (UI) | UI Dev | See Phase 7 |

---

## 5. Phase 4 — Sailing Movement

### Goal
Physics-based sailing: the boat accelerates based on the angle between wind and sail, steered by rudder input.

### 5.1 Sailing Physics Model

```
                   Wind Direction
                       ↓
              ╲        │        ╱
               ╲       │       ╱
                ╲      │      ╱    ← No-go zone (~45° either side)
                 ╲     │     ╱        No forward force; boat luffs
                  ╲    │    ╱
     Close Hauled  ╲   │   ╱  Close Hauled
         (~45°)     ╲  │  ╱     (~45°)
                     ╲ │ ╱
                      ╲│╱
                       │
         Beam Reach ───┼─── Beam Reach  (fastest, ~90° to wind)
                       │
                      ╱│╲
                     ╱ │ ╲
         Broad Reach╱  │  ╲Broad Reach
                   ╱   │   ╲
                  ╱    │    ╲
         Running ╱     │     ╲ Running (slower than beam reach)
                ╱      │      ╲
```

**Polar speed diagram** simplified to a curve:

```rust
/// Calculate boat speed factor based on angle between boat heading and wind direction.
/// Returns 0.0 (no motion) to 1.0 (maximum speed).
/// angle_to_wind: absolute angle in radians between boat heading and wind direction (0..PI)
fn sail_speed_factor(angle_to_wind: f32) -> f32 {
    let angle = angle_to_wind.abs();
    if angle < 0.78 {
        // No-go zone: < ~45°, can't sail into wind
        // Gradual falloff from 45° to 0°
        (angle / 0.78).powf(2.0) * 0.3
    } else if angle < 1.57 {
        // Close-hauled to beam reach (45° to 90°): speed increases
        let t = (angle - 0.78) / (1.57 - 0.78);
        0.3 + t * 0.7 // 0.3 at 45° → 1.0 at 90°
    } else if angle < 2.36 {
        // Beam reach to broad reach (90° to 135°): still fast
        let t = (angle - 1.57) / (2.36 - 1.57);
        1.0 - t * 0.2 // 1.0 at 90° → 0.8 at 135°
    } else {
        // Running (135° to 180°): slower
        let t = (angle - 2.36) / (std::f32::consts::PI - 2.36);
        0.8 - t * 0.3 // 0.8 at 135° → 0.5 at 180°
    }
}
```

### 5.2 Sailing Movement System

**File: `src/systems/sailing_movement_system.rs`**

Following the pattern of [`flight_movement_system.rs`](src/systems/flight_movement_system.rs):

```rust
pub fn sailing_movement_system(
    time: Res<Time>,
    wind: Res<WindState>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut boat_query: Query<(
        &mut BoatState,
        &mut Position,
        &mut FacingDirection,
    ), With<PlayerCharacter>>,
) {
    for (mut boat, mut position, mut facing) in boat_query.iter_mut() {
        let dt = time.delta_secs();
        
        // --- Steering Input ---
        // A/D or Left/Right arrows rotate heading
        let steer_input = if keyboard.pressed(KeyCode::KeyA) || keyboard.pressed(KeyCode::ArrowLeft) {
            -1.0
        } else if keyboard.pressed(KeyCode::KeyD) || keyboard.pressed(KeyCode::ArrowRight) {
            1.0
        } else {
            0.0
        };
        boat.rudder = steer_input;
        
        // Turn rate depends on speed (can't turn well when stopped)
        let turn_rate = 1.0 * (boat.speed / boat.max_speed).clamp(0.1, 1.0);
        boat.heading += steer_input * turn_rate * dt;
        boat.heading = boat.heading.rem_euclid(std::f32::consts::TAU);
        
        // --- Sail Trim Input ---
        // W/S adjust sail trim
        if keyboard.pressed(KeyCode::KeyW) {
            boat.sail_trim = (boat.sail_trim - 0.5 * dt).max(0.0);
        }
        if keyboard.pressed(KeyCode::KeyS) {
            boat.sail_trim = (boat.sail_trim + 0.5 * dt).min(std::f32::consts::PI);
        }
        
        // --- Wind Force Calculation ---
        let wind_angle = wind.angle;
        let angle_to_wind = (boat.heading - wind_angle).rem_euclid(std::f32::consts::TAU);
        let angle_to_wind_abs = if angle_to_wind > std::f32::consts::PI {
            std::f32::consts::TAU - angle_to_wind
        } else {
            angle_to_wind
        };
        
        let speed_factor = sail_speed_factor(angle_to_wind_abs);
        let target_speed = boat.max_speed * speed_factor * (wind.speed / 5.0).clamp(0.0, 2.0);
        
        // Sail trim efficiency: optimal trim depends on angle to wind
        let optimal_trim = angle_to_wind_abs * 0.5; // Simplified
        let trim_efficiency = 1.0 - ((boat.sail_trim - optimal_trim).abs() / std::f32::consts::PI);
        let target_speed = target_speed * trim_efficiency.clamp(0.1, 1.0);
        
        // Acceleration/deceleration
        let accel = if target_speed > boat.speed { 2.0 } else { 1.5 };
        boat.speed += (target_speed - boat.speed) * accel * dt;
        boat.speed = boat.speed.clamp(0.0, boat.max_speed);
        
        // --- Apply Movement ---
        let forward = Vec3::new(boat.heading.sin(), boat.heading.cos(), 0.0);
        let movement = forward * boat.speed * dt * 100.0; // Convert to cm
        position.position.x += movement.x;
        position.position.y += movement.y;
        // Z stays at water surface height
        
        // Update facing direction
        facing.desired = boat.heading;
    }
}
```

### 5.3 Buoyancy System

**File: `src/systems/boat_buoyancy_system.rs`**

```rust
/// Makes boats bob on the water surface with wave-synchronized motion.
pub fn boat_buoyancy_system(
    time: Res<Time>,
    mut query: Query<(&BoatState, &mut Transform, &Position)>,
) {
    let t = time.elapsed_secs();
    for (boat, mut transform, position) in query.iter_mut() {
        // Rock side-to-side based on wave phase at position
        let wave_phase = position.x * 0.01 + t * 1.5;
        let roll = wave_phase.sin() * 0.05; // ±3° roll
        let pitch = (wave_phase * 0.7 + 1.3).sin() * 0.03; // ±2° pitch
        let heave = (wave_phase * 1.2).sin() * 0.1; // ±0.1m vertical bob
        
        // Apply rotation on top of heading
        let heading_rot = Quat::from_rotation_y(-boat.heading);
        let wave_rot = Quat::from_euler(EulerRot::XZY, pitch, 0.0, roll);
        transform.rotation = heading_rot * wave_rot;
        
        // Adjust Y (world up) for bobbing
        // Water surface Y is determined by the zone's water plane height
        transform.translation.y += heave;
    }
}
```

### 5.4 Deliverables

| Item | Owner | Notes |
|------|-------|-------|
| `src/systems/sailing_movement_system.rs` | Client Dev | Core sailing physics |
| `src/systems/boat_buoyancy_system.rs` | Client Dev | Wave bob/roll |
| Sail speed polar curve | Game Designer | Tune `sail_speed_factor` |
| Movement integration tests | Client Dev | Unit tests for physics |

---

## 6. Phase 5 — Server Authority & Networking

### Goal
The server must validate boat movement, sync boat entities to all clients, and handle zone transitions.

### 6.1 New MoveMode

**Server: `rose-game-common/src/components/move_mode.rs`**

```rust
#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Reflect)]
pub enum MoveMode {
    Walk,
    Run,
    Drive,
    Sail,  // NEW
}
```

**Network packet encoding** ([`common_packets.rs`](C:/Users/vicha/RustroverProjects/rose-offline/rose-network-irose/src/common_packets.rs:636)):

```rust
fn read_move_mode_u8(&mut self) -> Result<MoveMode, PacketError> {
    match self.read_u8()? {
        0 => Ok(MoveMode::Walk),
        1 => Ok(MoveMode::Run),
        2 => Ok(MoveMode::Drive),
        3 => Ok(MoveMode::Sail),   // NEW
        _ => Err(PacketError::InvalidPacket),
    }
}
```

### 6.2 New Server Components

```rust
/// Server-side boat state
#[derive(Component)]
pub struct ServerBoatState {
    pub hull_health: i32,
    pub sail_trim: f32,
    pub heading: f32,
    pub speed: f32,
    pub wind_zone_id: Option<ZoneId>, // Which ocean zone
}
```

### 6.3 New Network Packets

| Packet | Direction | Data |
|--------|-----------|------|
| `BoardBoat` | Client → Server | target_boat_entity_id |
| `DisembarkBoat` | Client → Server | (none) |
| `SailInput` | Client → Server | rudder: f32, sail_trim: f32 |
| `SpawnBoatEntity` | Server → Client | entity_id, position, heading, sail_state |
| `UpdateBoatState` | Server → Client | entity_id, position, heading, speed, sail_trim, wind_angle |
| `WindUpdate` | Server → Client | wind_angle, wind_speed (periodic broadcast) |

### 6.4 Server Movement Validation

The server should:

1. Accept `SailInput` packets from the riding client
2. Run simplified sailing physics server-side (same `sail_speed_factor` function)
3. Broadcast `UpdateBoatState` to nearby clients (10 Hz)
4. Validate boat doesn't move through islands (server-side collision with terrain)
5. Handle zone edge: if boat reaches zone boundary, trigger zone transition

### 6.5 Deliverables

| Item | Owner | Notes |
|------|-------|-------|
| MoveMode::Sail in rose-game-common | Server Dev | Shared enum |
| Network packet definitions | Server Dev + Client Dev | Both sides |
| Server boat physics loop | Server Dev | Simplified physics |
| Client packet handlers | Client Dev | Handle spawn/update |
| Zone transition for boats | Server Dev | Cross-zone sailing |

---

## 7. Phase 6 — Camera & Controls

### Goal
A specialized camera mode for sailing that shows the boat from above/behind, with smooth following.

### 7.1 Sail Camera Mode

Extends the existing [`OrbitCamera`](src/systems/game_mouse_input_system.rs) with a sail-specific mode:

```rust
pub fn sail_camera_system(
    boat_query: Query<(&BoatState, &Transform), With<PlayerCharacter>>,
    mut camera_query: Query<&mut Transform, (With<OrbitCamera>, Without<PlayerCharacter>)>,
    time: Res<Time>,
) {
    if let Ok((boat, boat_transform)) = boat_query.single() {
        if let Ok(mut camera_transform) = camera_query.single_mut() {
            // Position camera behind and above boat
            let behind_offset = 12.0; // meters behind
            let above_offset = 8.0;   // meters above
            
            let heading_dir = Vec3::new(boat.heading.sin(), 0.0, -boat.heading.cos());
            let target_pos = boat_transform.translation 
                - heading_dir * behind_offset
                + Vec3::Y * above_offset;
            
            // Smooth follow
            let lerp_speed = 3.0 * time.delta_secs();
            camera_transform.translation = camera_transform.translation.lerp(target_pos, lerp_speed);
            
            // Look at boat
            let look_target = boat_transform.translation + Vec3::Y * 1.0;
            *camera_transform = camera_transform.looking_at(look_target, Vec3::Y);
        }
    }
}
```

### 7.2 Input Mapping

| Key | Action |
|-----|--------|
| **A / Left Arrow** | Steer left (rudder) |
| **D / Right Arrow** | Steer right (rudder) |
| **W** | Tighten sail (pull in) |
| **S** | Loosen sail (let out) |
| **E** | Disembark (when near dock/island) |
| **Mouse** | Free-look camera (hold right click) |
| **Scroll** | Zoom camera in/out |

---

## 8. Phase 7 — Sail Trim UI

### Goal
HUD elements showing wind direction, boat heading, speed, and sail trim.

### 8.1 Wind Compass

An egui overlay showing:

```
      N
   NW   NE
  W   +   E     ← Compass rose
   SW   SE      ← Wind arrow overlaid (shows where wind blows FROM)
      S         ← Boat heading indicator (triangle)
```

**File: `src/ui/ui_sailing_hud_system.rs`**

```rust
pub fn ui_sailing_hud_system(
    mut egui_ctx: Query<&mut bevy_egui::EguiContext>,
    wind: Res<WindState>,
    boat_query: Query<&BoatState, With<PlayerCharacter>>,
) {
    // Draw wind compass (top-right corner)
    // Draw speed gauge (bottom-center)
    // Draw sail trim indicator (bottom-right)
    // Draw "Press E to disembark" when near dock
}
```

### 8.2 Speed Gauge

A horizontal bar showing:
- Current speed as fraction of max
- Color coding: red (luffing), yellow (suboptimal), green (good), blue (perfect beam reach)

### 8.3 Sail Trim Indicator

Arc showing:
- Current sail angle
- Optimal sail angle (ghost indicator)
- Wind direction relative to boat

---

## 9. Phase 8 — Visual Effects

### Goal
Water wake, spray, sail animation, and ambient ocean effects.

### 9.1 Wake Effect

Following the pattern of [`DirtDashEffect`](src/components/dirt_dash_effect.rs):

```rust
/// Component for boat wake particles
#[derive(Component, Reflect)]
pub struct BoatWakeEmitter {
    pub spawn_timer: Timer,
    pub wake_width: f32,
}
```

Two V-shaped wake lines trailing behind the boat, using billboarded quad particles.

### 9.2 Bow Spray

When speed > 50% of max, spray particles at the bow:
- White billboard particles
- Short lifetime (0.5s)
- Upward + backward velocity
- Scale with speed

### 9.3 Sail Animation

The sail mesh deforms based on:

1. **Billow** — how filled the sail is (from wind)
2. **Flap** — oscillation when in the no-go zone (luffing)
3. **Side** — which side the sail curves toward

```rust
pub fn sail_animation_system(
    time: Res<Time>,
    wind: Res<WindState>,
    boat_query: Query<&BoatState>,
    mut sail_query: Query<(&mut SailMesh, &mut Mesh3d)>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Calculate billow from wind angle and trim
    // Update sail mesh vertices to curve outward
    // Add flapping oscillation when luffing
}
```

### 9.4 Ocean Ambience

Enhance the existing water shader ([`water_material.wgsl`](src/render/shaders/water_material.wgsl)) for the ocean zone:
- Larger wave amplitude
- Longer wave period
- Increased foam on wave crests
- Horizon fog blending

### 9.5 Deliverables

| Item | Owner | Notes |
|------|-------|-------|
| `src/systems/boat_wake_system.rs` | Client Dev | Wake particles |
| `src/systems/sail_animation_system.rs` | Client Dev | Sail mesh deformation |
| Enhanced ocean water settings | Client Dev | WaterSettings for ocean zone |
| Bow spray effect | Client Dev | Speed-based particles |

---

## 10. Phase 9 — Audio

### Goal
Immersive ocean soundscape while sailing.

### 10.1 Sound Design

| Sound | Trigger | Type |
|-------|---------|------|
| Ocean waves (ambient) | Always in ocean zone | Looping, global |
| Wind rushing | Speed > 0 | Looping, pitch = f(speed) |
| Sail flapping | Luffing (in no-go zone) | Looping, volume = f(luff_amount) |
| Hull creaking | Always on boat | Looping, subtle |
| Splash | Bow hits wave crest | One-shot, periodic |
| Rope tightening | Sail trim change | One-shot |
| Board/disembark | Interaction | One-shot |

### 10.2 Implementation

Following the spatial sound pattern from [`SpatialSound`](src/audio/spatial_sound.rs):

```rust
#[derive(Component)]
pub struct BoatSound {
    pub wind_entity: Entity,
    pub creak_entity: Entity,
    pub splash_timer: Timer,
    pub state: BoatSoundState,
}
```

---

## 11. Phase 10 — Island Content & NPCs

### Goal
Populate islands with interactive content.

### 11.1 Dock NPC — Boat Vendor

- Sells boats of different tiers (small, medium, large)
- Repairs damaged hulls (for future combat)
- Located at Island A and Island E

### 11.2 Trading Post NPC

- Island B: buy/sell goods that can only be obtained by sailing
- "Trade route" quest: buy goods at Island B, deliver to Island E

### 11.3 Quest NPCs

- **Sailing tutorial quest**: NPC teaches basic sailing mechanics
- **Treasure hunt**: Sail to coordinates, use item to find treasure
- **Fishing expansion**: Fish from boat for ocean-only fish types

### 11.4 Warp Gate

Island E has a warp gate connecting to the mainland. This reuses the existing [`WarpObject`](src/components/warp_object.rs) system — no new code needed.

---

## 12. Phase 11 — Polish & Optimisation

### 12.1 Performance Considerations

| Concern | Mitigation |
|---------|------------|
| Large ocean zone with many water planes | LOD for distant water tiles; merge adjacent water planes |
| Wake particles per boat | Limit to 100 particles per boat; cull off-screen |
| Sail mesh updates | Only update when `BoatState` changes; skip if off-screen |
| Network bandwidth | Boat updates at 10 Hz, wind updates at 1 Hz |
| Memory | Ocean zone has fewer deco objects than land zones |

### 12.2 Quality Settings

Add to existing [`GraphicsSettings`](src/graphics/graphics_settings.rs):

```rust
pub struct SailingGraphicsSettings {
    pub wake_particles_enabled: bool,
    pub sail_deformation_quality: SailQuality, // Low (static), Medium (4x4), High (8x8)
    pub ocean_wave_quality: WaveQuality,       // Low (simple), High (multi-octave)
    pub bow_spray_enabled: bool,
}
```

### 12.3 Testing Checklist

- [ ] Boat spawns at dock when acquired
- [ ] Player boards/disembarks correctly
- [ ] Boat moves with wind; stops in no-go zone
- [ ] Sail animates based on wind
- [ ] Steering feels responsive
- [ ] Camera follows smoothly
- [ ] UI compass shows correct wind direction
- [ ] Speed gauge works
- [ ] Wake/spray effects render
- [ ] Sounds play appropriately
- [ ] Server validates position
- [ ] Other players see the boat
- [ ] Zone transitions work (warp gate + zone edge)
- [ ] Performance acceptable at 60 FPS with 10 boats visible
- [ ] Works with existing underwater effect when camera dips
- [ ] Fish system works in ocean zone water planes

---

## 13. File Inventory

### New Client Files

| File | Phase | Description |
|------|-------|-------------|
| `src/components/boat.rs` | 2 | BoatState, BoatModel, SailMesh components |
| `src/resources/wind_state.rs` | 3 | WindState, WindSettings resources |
| `src/systems/wind_system.rs` | 3 | Wind update system |
| `src/systems/sailing_movement_system.rs` | 4 | Core sailing physics |
| `src/systems/boat_buoyancy_system.rs` | 4 | Wave bob/roll |
| `src/systems/boat_spawn_system.rs` | 2 | Spawn/despawn boat entities |
| `src/systems/sail_animation_system.rs` | 8 | Sail mesh deformation |
| `src/systems/boat_wake_system.rs` | 8 | Wake particle effects |
| `src/systems/sail_camera_system.rs` | 6 | Camera follow for sailing |
| `src/ui/ui_sailing_hud_system.rs` | 7 | Wind compass, speed gauge UI |
| `src/events/boat_event.rs` | 2 | BoardBoatEvent, DisembarkEvent |
| `src/audio/boat_sound.rs` | 9 | Boat sound management |

### Modified Client Files

| File | Phase | Changes |
|------|-------|---------|
| [`src/components/mod.rs`](src/components/mod.rs) | 2 | Add boat module |
| [`src/resources/mod.rs`](src/resources/) | 3 | Add wind_state module |
| [`src/systems/mod.rs`](src/systems/) | 3–4 | Add new system modules |
| [`src/events/mod.rs`](src/events/mod.rs) | 2 | Add boat events |
| [`src/lib.rs`](src/lib.rs) | All | Register plugin, systems, resources |
| [`src/components/wind_effect.rs`](src/components/wind_effect.rs) | 3 | Integrate with WindState |
| [`src/graphics/graphics_settings.rs`](src/graphics/graphics_settings.rs) | 11 | Add SailingGraphicsSettings |

### New Server Files

| File | Phase | Description |
|------|-------|-------------|
| `game/components/boat.rs` | 5 | ServerBoatState |
| `game/systems/sailing_system.rs` | 5 | Server-side sailing physics |
| `game/systems/wind_broadcast_system.rs` | 5 | Periodic wind state broadcast |

### Modified Server Files

| File | Phase | Changes |
|------|-------|---------|
| `rose-game-common/src/components/move_mode.rs` | 5 | Add MoveMode::Sail |
| `rose-network-irose/src/common_packets.rs` | 5 | Encode/decode MoveMode::Sail |
| `rose-network-irose/src/game_server_packets.rs` | 5 | New packet types |

### Zone Data Files (New)

| File | Phase | Description |
|------|-------|-------------|
| `3DDATA/MAPS/OCEAN/*.ZON` | 1 | Zone definition |
| `3DDATA/MAPS/OCEAN/{x}_{y}.HIM` | 1 | Heightmaps (many blocks) |
| `3DDATA/MAPS/OCEAN/{x}_{y}.TIL` | 1 | Tile maps |
| `3DDATA/MAPS/OCEAN/{x}_{y}.IFO` | 1 | Object/water placement |

---

## 14. Risk Register

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Large ocean zone causes memory issues | High | Medium | See [`pitfalls/performance-memory.md`](pitfalls/performance-memory.md); reuse zone loader memory tracking |
| Water rendering breaks with new zone | Medium | Low | Existing [`water_material.rs`](src/render/water_material.rs) is battle-tested; see [`pitfalls/water-system.md`](pitfalls/water-system.md) |
| Sailing physics feels bad | High | Medium | Extensive playtesting; expose all constants as settings |
| Server validation too strict | Medium | Medium | Start permissive, tighten over time |
| Sail mesh deformation is expensive | Low | Low | LOD system; skip when off-screen |
| Network bandwidth for boat sync | Medium | Low | 10 Hz update rate; delta compression |
| Zone transitions while on boat | High | Medium | Special handling: despawn boat, respawn at new dock |

---

## 15. Glossary

| Term | Definition |
|------|------------|
| **Beam reach** | Sailing perpendicular (90°) to the wind; fastest point of sail |
| **Close-hauled** | Sailing as close to the wind as possible (~45°) |
| **Running** | Sailing directly downwind (180°) |
| **Luffing** | Sail flapping because the boat is pointed too close to the wind |
| **Tacking** | Zigzag maneuver to make progress upwind |
| **No-go zone** | Angle range (~0–45° from wind) where the boat cannot make forward progress |
| **Sail trim** | Adjusting the angle of the sail relative to the boat |
| **Rudder** | Steering mechanism; turns the boat left or right |
| **Billow** | How much the sail curves outward when filled with wind |
| **Wake** | Trail of disturbed water behind a moving boat |
| **Draft** | How deep the boat hull sits below the water surface |
| **Freeboard** | Height of the hull above the water surface |

---

## Implementation Priority Order

```
Phase 1: Ocean Zone Map ──────────────── [Week 1–2]  Level Designer + Server Dev
Phase 2: Boat Entity & Model ─────────── [Week 1–2]  Client Dev
Phase 3: Wind System ─────────────────── [Week 2]    Client Dev
Phase 4: Sailing Movement ────────────── [Week 2–3]  Client Dev
Phase 5: Server Authority & Networking ── [Week 3–4]  Server Dev + Client Dev
Phase 6: Camera & Controls ───────────── [Week 3]    Client Dev
Phase 7: Sail Trim UI ────────────────── [Week 4]    UI Dev
Phase 8: Visual Effects ──────────────── [Week 4–5]  Client Dev
Phase 9: Audio ────────────────────────── [Week 5]    Audio Dev
Phase 10: Island Content & NPCs ──────── [Week 5–6]  Level Designer + Server Dev
Phase 11: Polish & Optimisation ──────── [Week 6+]   All
```

Phases 1–2 and Phase 3 can run in parallel. Phase 4 depends on 2+3. Phase 5 depends on 4. Phases 6–9 can mostly run in parallel after Phase 4.
