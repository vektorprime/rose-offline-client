# Sailing System — Detailed Expansion

## Status of Existing Implementation

The following components are **already implemented and compiling** (see [`sailing-system-implementation-tracking.md`](sailing-system-implementation-tracking.md)):

| Module | File | Status | Notes |
|--------|------|--------|-------|
| BoatState component | [`src/components/boat.rs`](../src/components/boat.rs) | ✅ Done | heading, speed, max_speed, sail_trim, rudder, hull_health, water_height_cm, wave_roll/pitch |
| BoatModel component | [`src/components/boat.rs`](../src/components/boat.rs) | ✅ Done | root/hull/mast/sail/rudder/rider_seat entity refs |
| SailMesh marker | [`src/components/boat.rs`](../src/components/boat.rs) | ✅ Done | billow + SailSide enum |
| WindState resource | [`src/resources/wind_state.rs`](../src/resources/wind_state.rs) | ✅ Done | direction, speed, angle, gust_factor |
| WindSettings resource | [`src/resources/wind_state.rs`](../src/resources/wind_state.rs) | ✅ Done | base_speed, drift, gust params |
| BoardBoatEvent / DisembarkBoatEvent | [`src/events/boat_event.rs`](../src/events/boat_event.rs) | ✅ Done | Message derive |
| Wind update system | [`src/systems/wind_system.rs`](../src/systems/wind_system.rs) | ✅ Done | Layered sine drift + gust |
| Vegetation wind sync | [`src/systems/wind_system.rs`](../src/systems/wind_system.rs) | ✅ Done | Syncs WindSwaySettings.global_intensity |
| Boat spawn/toggle | [`src/systems/boat_spawn_system.rs`](../src/systems/boat_spawn_system.rs) | ✅ Done | Procedural sailboat mesh (hull+bow+deck+cabin+mast+sails+rudder) |
| Sailing movement | [`src/systems/sailing_movement_system.rs`](../src/systems/sailing_movement_system.rs) | ✅ Done | Wind-relative physics with polar speed curve, trim efficiency, dynamic water height sampling |
| Buoyancy | [`src/systems/boat_buoyancy_system.rs`](../src/systems/boat_buoyancy_system.rs) | ✅ Done | Wave roll/pitch/heave |
| Sail camera | [`src/systems/sail_camera_system.rs`](../src/systems/sail_camera_system.rs) | ✅ Done | Clamps orbit camera distance |
| `/boat` chat command | Modified [`src/ui/ui_chatbox_system.rs`](../src/ui/ui_chatbox_system.rs) | ✅ Done | Sends BoardBoatEvent |
| Collision gating | Modified [`src/systems/collision_system.rs`](../src/systems/collision_system.rs) | ✅ Done | Wall blocking + land blocking |
| Input gating | Modified keyboard/mouse input systems | ✅ Done | Prevents normal walk when sailing |
| SailingGraphicsSettings | Modified [`src/graphics/graphics_settings.rs`](../src/graphics/graphics_settings.rs) | ✅ Done | wake_particles_enabled, sail quality, etc. |

---

## What Remains — Detailed Specifications

The following sections describe everything that still needs to be built. Each section is self-contained enough to be a separate work item.

---

## A. Sail Mesh Deformation System

**Status**: Not implemented. The `SailMesh` component exists with `billow` and `side` fields, but nothing reads or writes them, and the sail mesh geometry is never updated at runtime.

### A.1 Goal

Deform the sail mesh vertices each frame so the sail:
- Billows outward when filled with wind (beam reach / broad reach)
- Goes limp and flaps when in the no-go zone (luffing)
- Curves toward port or starboard depending on which side the wind hits

### A.2 Detailed Design

**New file: `src/systems/sail_animation_system.rs`**

```rust
/// System that deforms the sail mesh based on wind conditions.
///
/// This system runs every frame when any BoatState is active.
/// It reads the global WindState and each boat's BoatState to calculate:
/// 1. Which side the wind hits the sail (port vs starboard)
/// 2. How much the sail should billow (fill factor 0.0–1.0)
/// 3. Whether the sail is luffing (flapping in the no-go zone)
///
/// The sail mesh is a subdivided plane (NxN grid). Each vertex is offset
/// along the sail's normal based on a parabolic billow curve + optional
/// sine-wave luffing oscillation.
pub fn sail_animation_system(
    time: Res<Time>,
    wind: Res<WindState>,
    boat_query: Query<&BoatState>,
    mut sail_query: Query<(&mut SailMesh, &ChildOf, &Mesh3d)>,
    boat_model_query: Query<&BoatModel>,
    mut meshes: ResMut<Assets<Mesh>>,
) { /* ... */ }
```

### A.3 Mesh Subdivision Approach

The current sail is a `Plane3d` — a flat quad with only 4 vertices. This cannot deform smoothly. Replace it with a subdivided grid:

```rust
/// Creates a subdivided sail mesh for runtime deformation.
/// 
/// Parameters:
/// - `width`: sail width in metres (e.g. 2.25 for mainsail)
/// - `height`: sail height in metres (e.g. 2.9 for mainsail)
/// - `subdivisions`: number of subdivisions (8 = 9x9 = 81 vertices)
///
/// Returns a Mesh with POSITION, NORMAL, UV_0 attributes.
/// The mesh lives in the XY plane (X = horizontal, Y = vertical),
/// with the bottom edge at Y=0 (attached to boom) and top at Y=height.
///
/// Vertex layout (subdivisions=4 example):
/// ```
///  20--21--22--23--24   ← top (attached to mast top)
///  |   |   |   |   |
///  15--16--17--18--19
///  |   |   |   |   |
///  10--11--12--13--14   ← middle
///  |   |   |   |   |
///   5---6---7---8---9
///  |   |   |   |   |
///   0---1---2---3---4   ← bottom (attached to boom)
/// ```
fn create_subdivided_sail_mesh(width: f32, height: f32, subdivisions: u32) -> Mesh {
    let cols = subdivisions + 1;
    let rows = subdivisions + 1;
    let mut positions = Vec::with_capacity((cols * rows) as usize);
    let mut normals = Vec::with_capacity((cols * rows) as usize);
    let mut uvs = Vec::with_capacity((cols * rows) as usize);
    let mut indices = Vec::new();

    for row in 0..rows {
        for col in 0..cols {
            let u = col as f32 / subdivisions as f32;
            let v = row as f32 / subdivisions as f32;
            positions.push([
                (u - 0.5) * width,  // X: centered
                v * height,          // Y: bottom to top
                0.0,                 // Z: flat initially
            ]);
            normals.push([0.0, 0.0, 1.0]);
            uvs.push([u, 1.0 - v]);
        }
    }

    for row in 0..(rows - 1) {
        for col in 0..(cols - 1) {
            let tl = row * cols + col;
            let tr = tl + 1;
            let bl = tl + cols;
            let br = bl + 1;
            indices.extend_from_slice(&[tl as u32, bl as u32, tr as u32]);
            indices.extend_from_slice(&[tr as u32, bl as u32, br as u32]);
        }
    }

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    mesh.insert_indices(Indices::U32(indices));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh
}
```

### A.4 Deformation Algorithm

Each frame, for each sail vertex at grid position `(u, v)`:

```
billow_amount = billow * parabola(u) * triangle(v)
luff_amount   = luff_factor * sin(time * 8.0 + v * 3.0 + u * 5.0) * (1.0 - v)

where:
  parabola(u) = 4.0 * u * (1.0 - u)          // max at center, zero at edges
  triangle(v) = 1.0 - abs(2.0 * v - 1.0)      // max at middle height, zero at top/bottom
  luff_factor = 1.0 - sail_speed_factor(angle_to_wind)  // 1.0 in no-go, 0.0 at beam reach

new_z = billow_amount * billow_depth + luff_amount * luff_depth
where:
  billow_depth = 0.6 metres (max outward curve)
  luff_depth   = 0.3 metres (max flutter amplitude)
```

The sign of `new_z` determines port/starboard billow direction (negative = port, positive = starboard), based on which side the wind is hitting.

### A.5 Quality Levels

From [`SailingGraphicsSettings`](../src/graphics/graphics_settings.rs):

| Quality | Subdivisions | Vertices | Update Rate |
|---------|-------------|----------|-------------|
| Low | 0 (static plane) | 4 | Never |
| Medium | 4 | 25 | Every frame |
| High | 8 | 81 | Every frame |

### A.6 Deliverables

- [ ] `create_subdivided_sail_mesh()` function in [`boat_spawn_system.rs`](../src/systems/boat_spawn_system.rs)
- [ ] Replace `Plane3d` sail mesh with subdivided mesh at spawn time
- [ ] `sail_animation_system` in new file `src/systems/sail_animation_system.rs`
- [ ] Register system in [`lib.rs`](../src/lib.rs) — run in `Update`, after `sailing_movement_system`
- [ ] Store base vertex positions in `SailMesh` component for reference
- [ ] Respect `SailingGraphicsSettings.sail_deformation_quality`

---

## B. Boat Wake & Spray Effects

**Status**: Not implemented. The graphics settings field exists but no particle system exists.

### B.1 Goal

Create two visual effects:
1. **V-shaped wake** trailing behind the boat when moving
2. **Bow spray** particles when speed exceeds 50% max

### B.2 Wake Effect Design

**New file: `src/systems/boat_wake_system.rs`**

Following the pattern of [`DirtDashEffect`](../src/components/dirt_dash_effect.rs) and [`dirt_dash_system.rs`](../src/systems/dirt_dash_system.rs):

```rust
/// Component for a single wake particle.
#[derive(Component, Reflect)]
pub struct WakeParticle {
    pub velocity: Vec3,
    pub lifetime: Timer,
    pub initial_alpha: f32,
    pub initial_scale: f32,
}

/// Emitter component attached to the boat entity.
#[derive(Component, Reflect)]
pub struct WakeEmitter {
    pub spawn_timer: Timer,
    /// Maximum particles alive at once for this boat.
    pub max_particles: usize,
}

impl Default for WakeEmitter {
    fn default() -> Self {
        Self {
            spawn_timer: Timer::from_seconds(0.05, TimerMode::Repeating),
            max_particles: 100,
        }
    }
}
```

### B.3 Wake Particle Spawn Logic

Each spawn tick (20 Hz), emit 2 particles — one to port-rear, one to starboard-rear:

```
For each side in [Port, Starboard]:
    offset_angle = boat.heading + (side == Port ? -2.8 : 2.8)  // ~160° behind, angled out
    spawn_pos = boat_position + Vec3(offset_angle.sin(), offset_angle.cos(), 0) * 1.5
    velocity  = Vec3(offset_angle.sin(), offset_angle.cos(), 0) * boat.speed * 0.3
    lifetime  = 2.0 seconds
    scale     = 0.3 + boat.speed / boat.max_speed * 0.4
```

### B.4 Wake Particle Mesh

Use a flat billboard quad (0.5m × 0.5m) with a white semi-transparent material:

```rust
let wake_material = materials.add(StandardMaterial {
    base_color: Color::srgba(0.9, 0.95, 1.0, 0.4),
    alpha_mode: AlphaMode::Blend,
    unlit: true,  // Wake doesn't receive lighting
    ..default()
});
```

### B.5 Wake Particle Update

Each frame:
- Advance lifetime timer
- Fade alpha: `current_alpha = initial_alpha * (1.0 - lifetime.fraction())`
- Scale up slightly: `current_scale = initial_scale * (1.0 + lifetime.fraction() * 0.5)`
- Move by velocity (velocity decays: `velocity *= 0.97`)
- Despawn when lifetime expires

### B.6 Bow Spray Effect

When `boat.speed > 0.5 * boat.max_speed`:
- Spawn 3–5 small white particles per tick at the bow position
- Velocity: upward (Y +2..+4 m/s in world space) + backward along boat heading
- Short lifetime: 0.3–0.6 seconds
- Smaller scale: 0.1–0.2m

### B.7 Performance Budget

- Max 100 wake particles per boat
- Max 30 spray particles per boat
- Only spawn for boats within 50m of camera
- Respect `SailingGraphicsSettings.wake_particles_enabled`
- Respect `SailingGraphicsSettings.bow_spray_enabled`

### B.8 Deliverables

- [ ] `WakeParticle` and `WakeEmitter` components in `src/components/boat_wake.rs`
- [ ] `boat_wake_spawn_system` — spawns wake + spray particles
- [ ] `boat_wake_update_system` — updates position/alpha/scale, despawns expired
- [ ] Register systems in [`lib.rs`](../src/lib.rs)
- [ ] White billboard quad mesh + unlit material shared across all wake particles

---

## C. Sailing HUD (Wind Compass + Speed Gauge)

**Status**: A `ui_sailing_hud_system.rs` file exists but may be minimal. Needs full egui implementation.

### C.1 Goal

When the player is sailing, display:
1. **Wind Compass** — top-right corner, 120×120px circle showing wind direction relative to boat heading
2. **Speed Gauge** — bottom-center, horizontal bar showing current speed as fraction of max
3. **Sail Trim Indicator** — bottom-right, arc showing current vs optimal trim
4. **Contextual Prompt** — "Press E to disembark" when near dock/island

### C.2 Wind Compass Specification

```
┌──────────────────┐
│        N         │
│     ╱     ╲      │   120 × 120 pixels
│   W    ●    E    │   ● = center
│     ╲     ╱      │
│        S         │
│                  │
│   ← Wind arrow   │   Arrow points FROM the direction wind is coming
│   ▲ Boat heading │   Small triangle on compass ring shows boat direction
└──────────────────┘
```

Implementation using `egui::Painter`:

```rust
fn draw_wind_compass(ui: &mut egui::Ui, wind_angle: f32, boat_heading: f32) {
    let (response, painter) = ui.allocate_painter(egui::vec2(120.0, 120.0), egui::Sense::hover());
    let center = response.rect.center();
    let radius = 50.0;
    
    // Background circle
    painter.circle_filled(center, radius, egui::Color32::from_rgba_premultiplied(0, 0, 0, 140));
    painter.circle_stroke(center, radius, egui::Stroke::new(2.0, egui::Color32::WHITE));
    
    // Cardinal directions (rotated by boat heading so boat always points up)
    let dirs = [("N", 0.0), ("E", PI/2.0), ("S", PI), ("W", 3.0*PI/2.0)];
    for (label, angle) in dirs {
        let rotated = angle - boat_heading;
        let pos = center + egui::vec2(rotated.sin(), -rotated.cos()) * (radius + 12.0);
        painter.text(pos, egui::Align2::CENTER_CENTER, label,
            egui::FontId::proportional(10.0), egui::Color32::LIGHT_GRAY);
    }
    
    // Wind arrow (red, points from wind source direction)
    let wind_relative = wind_angle - boat_heading;
    let arrow_dir = egui::vec2(wind_relative.sin(), -wind_relative.cos());
    let arrow_start = center + arrow_dir * 10.0;
    let arrow_end = center + arrow_dir * (radius - 5.0);
    painter.arrow(arrow_start, arrow_end - arrow_start,
        egui::Stroke::new(3.0, egui::Color32::from_rgb(220, 60, 60)));
    
    // Boat heading indicator (green triangle at top of ring)
    let boat_tip = center + egui::vec2(0.0, -radius);
    painter.add(egui::Shape::convex_polygon(
        vec![boat_tip, boat_tip + egui::vec2(-5.0, 10.0), boat_tip + egui::vec2(5.0, 10.0)],
        egui::Color32::from_rgb(60, 220, 60),
        egui::Stroke::NONE,
    ));
}
```

### C.3 Speed Gauge Specification

```
┌──────────────────────────────────────────────────┐
│  ░░░░░░░░░░░░░░░░▓▓▓▓▓▓▓▓▓▓░░░░░░░░░░░░░░░░░░  │  250 × 20 pixels
│  0              4.5 m/s              9.0 m/s      │  Bottom-center of screen
│                  ↑ current speed                   │
└──────────────────────────────────────────────────┘
```

Color coding:
- **Red** (0–15%): In the no-go zone, almost no forward force
- **Yellow** (15–40%): Close-hauled, suboptimal angle
- **Green** (40–80%): Good sailing angle
- **Cyan** (80–100%): Beam reach, optimal speed

Text overlay: `"{speed:.1} m/s"` centered on bar + `"{speed_knots:.0} kt"` in smaller text.

### C.4 Sail Trim Indicator Specification

```
        Optimal ↓
    ────────┬────────
    ▏       │    ▲   ▏     60 × 60 pixels
    ▏       │    │   ▏     Arc from 0 to PI
    ▏       │    ●   ▏     ● = current trim position
    ▏       │        ▏     Optimal shown as ghost marker
    ────────┴────────
         Trim Angle
```

When current trim matches optimal within ±0.2 rad, the indicator glows green. Otherwise yellow/red.

### C.5 Contextual Prompts

- "Press E to disembark" — shown when `BoatState.active && near_land(position)`
- "Luffing! Turn away from wind" — shown when angle_to_wind < 0.78 rad
- "Wind: {speed:.0} m/s from {compass_dir}" — small text below compass

### C.6 Deliverables

- [ ] Full egui implementation of wind compass in [`ui_sailing_hud_system.rs`](../src/ui/ui_sailing_hud_system.rs)
- [ ] Speed gauge bar with color coding
- [ ] Sail trim arc indicator
- [ ] Contextual prompt text
- [ ] Only render when `BoatState.active == true`
- [ ] Hide when egui debug windows are focused

---

## D. Ocean Zone Map Creation

**Status**: Not started. No zone data files exist.

### D.1 Goal

Create a dedicated ocean zone (e.g. `ZoneId(200)`) with ~6.4 km² of ocean and 6–8 islands.

### D.2 Detailed Block Layout

The zone is 64×64 blocks. Only blocks 12–52 in each dimension are used (40×40 = 1600 active blocks). Blocks outside this range have no HIM files (skipped by the loader).

```
Block coordinates (x, y) where active:

Water blocks: ~1550 blocks
  - Flat heightmap at -500 cm (5m below water surface)
  - Single water plane per block at height 0 cm covering full block
  - No TIL/tile textures needed (underwater, never visible)

Island A (blocks 15-18, 14-17): 4×4 = 16 blocks
  - Dock NPC position: block (16, 15) at local (80, 80, 200) cm
  - Boat vendor NPC: block (16, 15) at local (60, 85, 200) cm
  - Heightmap: rises from -500 at edges to +300 at center (beach + low hill)
  - Deco: palm trees, dock wooden planks, small hut

Island B (blocks 30-33, 18-20): 4×3 = 12 blocks
  - Trading post NPC
  - Heightmap: rocky island, max height +500 cm
  - Deco: rocks, trading post building, flag

Island C (blocks 18-20, 32-34): 3×3 = 9 blocks
  - Quest NPC
  - Small flat island with a single large tree
  - Heightmap: max +200 cm

Island D (blocks 38-39, 36-37): 2×2 = 4 blocks
  - Treasure chest interaction object
  - Tiny rocky outcrop
  - Heightmap: max +150 cm

Island E (blocks 25-30, 42-47): 6×6 = 36 blocks
  - Main town island, largest island
  - Warp gate to mainland (connects to existing zone, e.g. ZoneId(1))
  - Multiple NPCs: shopkeeper, quest giver, sailor trainer
  - Heightmap: varied terrain, max +800 cm, with harbor bay
  - Deco: buildings, market stalls, lighthouse

Island F (blocks 42-44, 26-28): 3×3 = 9 blocks
  - Fishing spot island
  - Small dock with fishing NPC
  - Heightmap: max +200 cm
  - Deco: pier, fishing nets, crates
```

### D.3 Zone File Specifications

**ZON file (`3DDATA/MAPS/OCEAN/OCEAN.ZON`)**:
- `grid_size`: 2.5 (standard)
- `grid_per_patch`: 4 (standard)
- Tile textures: sand, ocean floor, grass, rock, beach (5–8 textures)
- Tile definitions: mappings from tile indices to texture layers

**HIM files**: For each active block, a 65×65 heightmap grid. Ocean blocks: all values = -500. Island blocks: shaped terrain.

**TIL files**: For each active block, a 16×16 tile grid. Ocean blocks: all sand. Island blocks: grass/rock/beach.

**IFO files**: For each active block:
- `water_planes`: one water plane at z=0 covering the block
- `deco_objects`: trees, rocks, buildings (island blocks only)
- `cnst_objects`: dock structures, buildings (island blocks only)
- `warps`: warp gate on Island E block (25, 42)
- `npcs`: NPC spawn positions (island blocks only)
- `monster_spawns`: sea creature spawns (future; some ocean blocks)
- `sound_objects`: ocean ambient sound emitters

### D.4 ZSC Object Definitions

Need new ZSC entries for:
- Palm tree variations (3 models)
- Dock wooden planks (walkway segments)
- Small wooden hut
- Trading post building
- Lighthouse
- Market stall
- Fishing pier
- Crate/barrel props
- Rock formations (3 variations)

These can reuse existing game art or be created as simple procedural geometry.

### D.5 Server-Side Zone Registration

In `rose-offline`:

1. Add to zone list STB data:
   - Zone ID: 200
   - Zone name: "Open Sea"
   - Zone path: `3DDATA/MAPS/OCEAN/`
   - ZSC cnst/deco paths

2. Add NPC definitions:
   - Boat Vendor (new NPC type, sells boats)
   - Trading Post Merchant
   - Quest NPCs
   - Warp gate keeper

3. Add warp gate:
   - From Zone 200 (Island E) → Zone 1 (mainland dock area)
   - From Zone 1 (dock area) → Zone 200 (Island E harbor)

### D.6 Map Editor Workflow

Use the existing [`map editor`](../src/map_editor/mod.rs) to create zone data:
1. Load empty 64×64 zone template
2. Paint heightmaps for island blocks using HIM editor
3. Place water planes in IFO editor
4. Place deco/cnst objects for islands
5. Place NPC spawn points
6. Place warp gates
7. Export all files using [`save_system.rs`](../src/map_editor/save/save_system.rs)

### D.7 Ocean-Specific Rendering Settings

When the ocean zone is loaded, apply special settings:

```rust
// In zone_loader or a zone-enter system:
if zone_id == ZoneId(200) {
    // Increase water wave amplitude for open ocean
    water_settings.wave_amplitude = 1.5;  // Default is 0.5
    water_settings.wave_frequency = 0.8;  // Longer, slower ocean swells
    water_settings.foam_intensity = 1.2;
    
    // Set atmospheric fog to ocean blue
    // Bevy FogSettings will be set by the zone's fog data
    
    // Enable horizon blending in water shader
    water_settings.horizon_blend_enabled = true;
}
```

### D.8 Deliverables

- [ ] Zone data files (HIM, TIL, IFO) for all active blocks
- [ ] ZON metadata file
- [ ] ZSC object definitions for island props
- [ ] Server zone registration (STB data)
- [ ] NPC definitions (boat vendor, traders, quest givers)
- [ ] Warp gate connections (ocean ↔ mainland)
- [ ] Zone-specific water/fog settings
- [ ] Test: zone loads without errors in the client

---

## E. Server Authority & Networking

**Status**: Not started. No server-side changes exist.

### E.1 MoveMode Extension

**File: `rose-game-common/src/components/move_mode.rs`**

Add `Sail` variant:

```rust
#[derive(Component, Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Reflect)]
pub enum MoveMode {
    Walk,
    Run,
    Drive,
    Sail,  // NEW
}
```

**File: `rose-network-irose/src/common_packets.rs`**

Update packet encoding:
```rust
// PacketReadMoveMode
3 => Ok(MoveMode::Sail),

// PacketWriteMoveMode
MoveMode::Sail => 3,
```

### E.2 New Server Packets

**File: `rose-network-irose/src/game_server_packets.rs`**

Add these packet types to the `ServerPackets` enum:

```rust
// New opcodes (pick unused values)
SpawnBoatEntity = 0x850,
UpdateBoatState = 0x851,
WindStateUpdate = 0x852,
```

**PacketServerSpawnBoatEntity**:
```rust
pub struct PacketServerSpawnBoatEntity {
    pub entity_id: ClientEntityId,
    pub position: Vec3,
    pub heading: f32,
    pub speed: f32,
    pub sail_trim: f32,
    pub rider_entity_id: Option<ClientEntityId>,
}
```

**PacketServerUpdateBoatState** (sent at 10 Hz to nearby clients):
```rust
pub struct PacketServerUpdateBoatState {
    pub entity_id: ClientEntityId,
    pub position_x: u16,  // compressed position
    pub position_y: u16,
    pub heading: u16,      // heading * 10000 compressed
    pub speed: u16,        // speed * 100 compressed
    pub sail_trim: u8,     // sail_trim / PI * 255
    pub wind_angle: u16,   // current wind angle * 10000
    pub wind_speed: u8,    // wind speed * 10
}
```

**PacketServerWindStateUpdate** (broadcast at 1 Hz to all players in ocean zone):
```rust
pub struct PacketServerWindStateUpdate {
    pub wind_angle: u16,   // angle * 10000
    pub wind_speed: u8,    // speed * 10
    pub gust_factor: u8,   // gust * 255
}
```

**Client → Server packets**:

```rust
// New client opcodes
pub const CLIENT_BOARD_BOAT: u16 = 0x850;
pub const CLIENT_DISEMBARK_BOAT: u16 = 0x851;
pub const CLIENT_SAIL_INPUT: u16 = 0x852;
```

**PacketClientSailInput** (sent at 10 Hz from the sailing client):
```rust
pub struct PacketClientSailInput {
    pub rudder: i8,       // -127..127 mapped to -1..1
    pub sail_trim: u8,    // 0..255 mapped to 0..PI
}
```

### E.3 Server-Side Sailing Physics

**New file: `rose-offline-server/src/game/systems/sailing_system.rs`**

The server runs simplified sailing physics for validation:

```rust
pub fn server_sailing_system(
    time: Res<Time>,
    wind_state: Res<ServerWindState>,
    mut query: Query<(&mut ServerBoatState, &mut Position, &MoveMode), With<GameClient>>,
) {
    for (mut boat, mut position, move_mode) in query.iter_mut() {
        if *move_mode != MoveMode::Sail { continue; }
        
        // Apply client-sent rudder/trim inputs
        // Run same sail_speed_factor calculation
        // Validate position delta against maximum possible speed
        // If client position diverges > threshold, correct it
    }
}
```

### E.4 Server-Side Wind State

**New resource: `ServerWindState`**

The server maintains the authoritative wind state and broadcasts it:

```rust
#[derive(Resource)]
pub struct ServerWindState {
    pub angle: f32,
    pub speed: f32,
    pub gust_factor: f32,
    pub time_accumulator: f32,
}

pub fn server_wind_update_system(time: Res<Time>, mut wind: ResMut<ServerWindState>) {
    // Same algorithm as client wind_update_system
    // This ensures server and client wind match
}

pub fn wind_broadcast_system(
    wind: Res<ServerWindState>,
    mut timer: Local<Timer>,
    time: Res<Time>,
    clients: Query<&GameClient>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() { return; }
    
    // Broadcast WindStateUpdate packet to all clients in ocean zone
}
```

### E.5 Client-Side Packet Handling

**File: `src/systems/game_connection_system.rs`**

Add handlers for:
- `ServerPackets::SpawnBoatEntity` — spawn a remote player's boat model
- `ServerPackets::UpdateBoatState` — update remote boat position/heading/sail
- `ServerPackets::WindStateUpdate` — override local wind state with server authoritative value

### E.6 Anti-Cheat Validation

Server validates:
1. **Speed**: Client cannot exceed `max_speed * 1.2` (20% tolerance for latency)
2. **Position delta**: Per-tick movement cannot exceed `max_speed * dt * 1.5`
3. **Terrain collision**: Boat cannot be at a position where terrain height > water surface
4. **Zone boundaries**: Boat cannot leave the active zone area

### E.7 Zone Transition While Sailing

When a sailing player hits a warp gate:
1. Server despawns the boat entity from the ocean zone
2. Player transitions to the new zone as `MoveMode::Run` (on foot)
3. Boat is stored as an inventory item / persistent state
4. When returning to the ocean zone, player can re-summon the boat at a dock

### E.8 Deliverables

- [ ] `MoveMode::Sail` variant in `rose-game-common`
- [ ] Packet encoding/decoding for new opcodes
- [ ] `ServerBoatState` component and `ServerWindState` resource
- [ ] `server_sailing_system` — validates client sailing
- [ ] `server_wind_update_system` — server-authoritative wind
- [ ] `wind_broadcast_system` — sends wind to clients at 1 Hz
- [ ] Client packet handlers in [`game_connection_system.rs`](../src/systems/game_connection_system.rs)
- [ ] Remote boat entity spawn/update on other clients
- [ ] Anti-cheat validation (speed, position, terrain)
- [ ] Zone transition handling for boats

---

## F. Audio System

**Status**: Not started.

### F.1 Sound Design Specification

| Sound ID | File | Trigger | Type | Volume | Pitch Variation |
|----------|------|---------|------|--------|-----------------|
| `ocean_ambient` | `SOUND/OCEAN_AMBIENT_01.WAV` | Always in ocean zone | Looping, global | 0.3 | None |
| `wind_rushing` | `SOUND/WIND_RUSH_01.WAV` | `boat.speed > 0` | Looping, spatial | 0.1 + speed/max * 0.4 | 0.8 + speed/max * 0.4 |
| `sail_flap` | `SOUND/SAIL_FLAP_01.WAV` | Luffing (angle < 45°) | Looping, spatial | luff_factor * 0.5 | 0.9 + random * 0.2 |
| `hull_creak` | `SOUND/WOOD_CREAK_01.WAV` | Always on boat | Looping, spatial | 0.15 | 0.9 + wave_amplitude * 0.2 |
| `splash_bow` | `SOUND/SPLASH_01.WAV` | `speed > 0.5 * max`, periodic (every 1–3s) | One-shot, spatial | 0.2 + speed/max * 0.3 | 0.85 + random * 0.3 |
| `rope_tighten` | `SOUND/ROPE_CREAK_01.WAV` | Sail trim changes > 0.1 rad | One-shot, spatial | 0.25 | 0.9 + random * 0.2 |
| `board_boat` | `SOUND/BOARD_BOAT_01.WAV` | Board event | One-shot | 0.4 | 1.0 |
| `disembark_boat` | `SOUND/DISEMBARK_01.WAV` | Disembark event | One-shot | 0.4 | 1.0 |

### F.2 Implementation

**New file: `src/audio/boat_sound.rs`**

```rust
#[derive(Component)]
pub struct BoatSoundState {
    pub wind_entity: Option<Entity>,
    pub creak_entity: Option<Entity>,
    pub flap_entity: Option<Entity>,
    pub splash_timer: Timer,
    pub last_sail_trim: f32,
}
```

**New system: `boat_sound_system`**

```rust
pub fn boat_sound_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    wind: Res<WindState>,
    mut query: Query<(&BoatState, &mut BoatSoundState, &Transform)>,
) {
    for (boat, mut sound_state, transform) in query.iter_mut() {
        if !boat.active {
            // Despawn all sound entities if boat is not active
            // ...
            continue;
        }
        
        // Update wind rushing volume/pitch based on speed
        // Update sail flapping based on luff factor
        // Update hull creaking
        // Trigger splash one-shots periodically
        // Trigger rope sounds on trim changes
    }
}
```

### F.3 Deliverables

- [ ] Sound asset files (WAV/OGG) — can use placeholder sounds initially
- [ ] `BoatSoundState` component
- [ ] `boat_sound_system` — manages looping + one-shot sounds
- [ ] `boat_sound_spawn_system` — creates sound entities when boat activates
- [ ] `boat_sound_cleanup_system` — removes sound entities on disembark
- [ ] Register in [`lib.rs`](../src/lib.rs)

---

## G. Enhanced Sail Camera

**Status**: Basic implementation exists (clamps orbit distance). Needs expansion.

### G.1 Improvements Needed

The current [`sail_camera_system.rs`](../src/systems/sail_camera_system.rs) only clamps orbit distance. It should also:

1. **Smooth orbit tracking**: Camera should slowly orbit behind the boat heading (not snap instantly)
2. **Pitch adjustment**: Lower the pitch slightly so more horizon is visible
3. **Speed-based FOV**: Slight FOV increase at high speed for a sense of velocity
4. **Wave-compensated position**: Camera should NOT bob with the boat's wave motion (smooth it out)
5. **Free-look override**: While right mouse button is held, allow free orbit; release to snap back behind boat

### G.2 Detailed Implementation

```rust
pub fn sail_camera_system(
    time: Res<Time>,
    boat_query: Query<(&BoatState, &Transform), With<PlayerCharacter>>,
    mut camera_query: Query<&mut OrbitCamera>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Ok((boat, boat_transform)) = boat_query.single() else { return; };
    if !boat.active { return; }

    for mut orbit in camera_query.iter_mut() {
        // Clamp zoom range for sailing
        orbit.follow_distance = orbit.follow_distance.clamp(10.0, 22.0);
        
        // Only auto-rotate when not free-looking
        if !mouse.pressed(MouseButton::Right) {
            // Target yaw = behind boat heading
            let target_yaw = boat.heading + std::f32::consts::PI;
            let current_yaw = orbit.yaw;
            
            // Smooth rotation toward target (slerp-like for angles)
            let diff = (target_yaw - current_yaw).rem_euclid(TAU);
            let shortest = if diff > PI { diff - TAU } else { diff };
            orbit.yaw += shortest * 2.0 * time.delta_secs();
            
            // Slightly lower pitch for more horizon visibility
            let target_pitch = -0.35; // ~20° above horizontal
            orbit.pitch += (target_pitch - orbit.pitch) * 1.5 * time.delta_secs();
        }
    }
}
```

### G.3 Deliverables

- [ ] Expand [`sail_camera_system.rs`](../src/systems/sail_camera_system.rs) with smooth tracking
- [ ] Free-look override with right mouse button
- [ ] Pitch adjustment for horizon visibility
- [ ] Wave-compensation (camera follows smoothed position, not raw buoyancy)

---

## H. Multiplayer Boat Rendering

**Status**: Not started. Only local player boat exists.

### H.1 Goal

When another player is sailing in the same zone, their boat should be visible to all nearby clients.

### H.2 Remote Boat Entity

When the client receives `SpawnBoatEntity` for a remote player:

```rust
fn handle_spawn_boat_entity(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
    packet: &PacketServerSpawnBoatEntity,
) {
    // Reuse the same spawn_boat_visual() function from boat_spawn_system.rs
    let boat_root = spawn_boat_visual(commands, meshes, materials, &Position::new(packet.position));
    
    // Attach to the remote player entity
    commands.entity(remote_player_entity).add_child(boat_root);
    
    // Add RemoteBoatState component for interpolation
    commands.entity(remote_player_entity).insert(RemoteBoatState {
        target_position: packet.position,
        target_heading: packet.heading,
        target_speed: packet.speed,
        interpolation_time: 0.0,
    });
}
```

### H.3 Remote Boat Interpolation

Remote boats receive position updates at 10 Hz. Between updates, interpolate:

```rust
#[derive(Component)]
pub struct RemoteBoatState {
    pub prev_position: Vec3,
    pub target_position: Vec3,
    pub prev_heading: f32,
    pub target_heading: f32,
    pub target_speed: f32,
    pub interpolation_time: f32,
    pub update_interval: f32,  // typically 0.1 seconds
}

pub fn remote_boat_interpolation_system(
    time: Res<Time>,
    mut query: Query<(&mut RemoteBoatState, &mut Position, &mut Transform)>,
) {
    for (mut remote, mut position, mut transform) in query.iter_mut() {
        remote.interpolation_time += time.delta_secs();
        let t = (remote.interpolation_time / remote.update_interval).clamp(0.0, 1.0);
        
        // Smooth position interpolation
        position.position = remote.prev_position.lerp(remote.target_position, t);
        
        // Smooth heading interpolation (shortest arc)
        let heading = lerp_angle(remote.prev_heading, remote.target_heading, t);
        transform.rotation = Quat::from_rotation_y(-heading);
    }
}
```

### H.4 Deliverables

- [ ] `RemoteBoatState` component
- [ ] Handle `SpawnBoatEntity` packet — spawn remote boat visual
- [ ] Handle `UpdateBoatState` packet — update interpolation targets
- [ ] `remote_boat_interpolation_system`
- [ ] Despawn remote boat when player leaves sailing mode
- [ ] Sail animation works for remote boats too

---

## I. Disembark Mechanics

**Status**: Partially implemented (event exists, toggle logic exists). Needs proper dock detection and shore placement.

### I.1 Current State

Currently `/boat` toggles sailing on/off at any location. This needs refinement:

### I.2 Boarding Rules

- Can only board a boat when:
  - Standing in the ocean zone
  - Near water (within 10m of a water plane)
  - Not in combat
  - Has a boat item in inventory (or free boats for MVP)

### I.3 Disembark Rules

- Can disembark when:
  - Near an island/dock (terrain height > water surface within 5m)
  - Press `E` key (not `/boat` for disembark — keep `/boat` for summoning)
  
- On disembark:
  - Player teleported to nearest walkable terrain point above water
  - Boat model remains at water position (or despawns)
  - `BoatState.active = false`
  - Character model becomes visible again

### I.4 Shore Detection Algorithm

```rust
fn find_nearest_shore_position(
    current_position: Vec3,
    zone_data: &ZoneLoaderAsset,
    water_height: f32,
) -> Option<Vec3> {
    // Cast 8 rays outward from current position in compass directions
    // For each ray, sample terrain height every 2m up to 20m
    // Return the first point where terrain_height > water_height + 50cm
    // Choose the closest such point
}
```

### I.5 Deliverables

- [ ] `E` key binding for disembark
- [ ] Shore detection algorithm
- [ ] Player teleport to nearest shore on disembark
- [ ] Boarding validation (near water, correct zone, not in combat)
- [ ] Character model visibility toggle on board/disembark

---

## Implementation Priority & Dependencies

```
                    ┌─────────────────────┐
                    │  D. Ocean Zone Map   │ ◄── Can start immediately
                    │  (Level Design)      │     No code dependency
                    └────────┬────────────┘
                             │
    ┌────────────────────────┼────────────────────────┐
    │                        │                         │
    ▼                        ▼                         ▼
┌───────────┐   ┌────────────────────┐   ┌──────────────────┐
│ A. Sail   │   │ E. Server Auth     │   │ F. Audio System  │
│ Animation │   │ & Networking       │   │                  │
└─────┬─────┘   └────────┬───────────┘   └────────┬─────────┘
      │                  │                          │
      ▼                  ▼                          │
┌───────────┐   ┌────────────────────┐              │
│ B. Wake & │   │ H. Multiplayer     │              │
│ Spray VFX │   │ Boat Rendering     │              │
└─────┬─────┘   └────────────────────┘              │
      │                                              │
      ▼                                              │
┌───────────┐                                        │
│ C. HUD    │ ◄─────────────────────────────────────┘
│ (compass, │
│  gauges)  │
└─────┬─────┘
      │
      ▼
┌───────────┐
│ G. Camera │
│ Enhance   │
└─────┬─────┘
      │
      ▼
┌───────────┐
│ I. Dis-   │
│ embark    │
└───────────┘
```

### Parallel Work Streams

| Stream | Items | Estimated Time |
|--------|-------|---------------|
| **Art/Level Design** | D (Ocean Zone) | 2–3 weeks |
| **Client VFX** | A → B → C | 2 weeks |
| **Server Networking** | E → H | 2–3 weeks |
| **Client Polish** | F, G, I | 1–2 weeks |

All streams can run in parallel. Total estimated time: **3–4 weeks** with 2–3 developers.
