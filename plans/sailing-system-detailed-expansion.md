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

## How To Read This Document Now

This document began as a plan for unimplemented work. The client has since implemented the local prototype and several visual/UI sections. The sections below are still useful as design specs, but their status labels should be interpreted through this updated matrix:

| Section | Current status | Next engineering goal |
|---------|----------------|-----------------------|
| A. Sail Mesh Deformation | Implemented locally | Keep as-is unless tuning quality/perf; make remote boats use the same path after networking. |
| B. Boat Wake & Spray | Implemented locally | Reduce material churn, improve pooling/culling, support remote boats. |
| C. Sailing HUD | Implemented locally | Polish layout, condition prompts, and align with production UI style. |
| D. Ocean Zone Map | Client scaffold only | Author real map data and server registration. |
| E. Server Authority & Networking | Not implemented | Highest-risk remaining code work. Move wind/movement/validation authority server-side. |
| F. Audio System | Not implemented | Add looped and one-shot sounds using existing audio patterns. |
| G. Enhanced Sail Camera | Implemented locally | Tune comfort and add optional FOV/smoothing refinements. |
| H. Multiplayer Boat Rendering | Not implemented | Requires E first, then remote interpolation and visual spawning. |
| I. Disembark Mechanics | Partially implemented | Current E-key shore placement exists, but production boarding validation must be restored server-side. |

## Current Architecture Snapshot

### Entity Model

Current sailing is implemented as a mode on the local player entity:

```text
PlayerCharacter entity
  - Position
  - Transform
  - FacingDirection
  - BoatState { active, heading, speed, sail_trim, model_root_entity, ... }
  - WakeEmitter, only while active
  - child: boat model root
      - hull/deck/cabin/mast/sails/rudder child meshes
```

This is intentionally simpler than a fully replicated boat. It lets the current client reuse existing player position, camera target, collision, and zone systems. For multiplayer/server work, pick one of these two designs before writing packets:

| Design | Description | Pros | Cons |
|--------|-------------|------|------|
| Player movement mode | The player remains the authoritative network entity and receives `MoveMode::Sail` plus boat fields. | Smaller packet changes; easier migration from current code. | Harder to support abandoned boats, passengers, boat combat, or remote boat identity. |
| Standalone boat entity | Server creates a boat entity and attaches/rides the player. | Cleaner for multiplayer visuals, passengers, combat, persistence. | More packet/entity lifecycle work. |

Recommendation: use the player movement mode for the first server-authoritative MVP, but keep the component names and packet shape compatible with standalone boat entities later.

### Schedule Order

The client app currently schedules sailing behavior as ordinary Bevy systems gated by `AppState::Game`. Preserve this ordering unless a specific bug requires changing it:

1. `wind_update_system` updates `WindState`.
2. `ensure_boat_state_system` adds `BoatState` to the local player when missing.
3. `boat_toggle_system` handles board/disembark messages and visual spawn/despawn.
4. `ensure_boat_wake_emitter_system` adds/removes `WakeEmitter`.
5. `sailing_movement_system` reads input, wind, water, and updates `Position`.
6. `sail_animation_system`, `boat_buoyancy_system`, `sail_camera_system`, and wake spawn/update react to the moved boat.
7. `ui_sailing_hud_system` draws the current state.

Reasoning: movement must run after board/disembark so newly boarded players move in the same frame. Sail animation and wake should run after movement so visuals reflect the latest speed/heading. The HUD can run late because it only reads the final frame state.

### Coordinate And Unit Rules

The client uses two coordinate spaces:

| Data | Unit | Axes |
|------|------|------|
| `Position` | centimeters | X right, Y forward, Z up |
| `Transform` | meters | X right, Y up, Z back |

When implementing or reviewing sailing code, do not mix these directly. The common conversion is:

```rust
transform.translation.x = position.x / 100.0;
transform.translation.y = position.z / 100.0;
transform.translation.z = -position.y / 100.0;
```

The sailing movement system should continue to move `Position` in centimeters. Rendering/camera code can read `Transform` after the normal sync/collision path has applied the conversion.

### Bevy 0.18.1 Rules Confirmed From Source

These are source-validated Bevy details that matter for implementation:

| Topic | Confirmed behavior | Sailing implication |
|-------|--------------------|---------------------|
| Messages | Use `#[derive(Message)]`, `App::add_message::<T>()`, `MessageWriter::write`, and `MessageReader::read`. | `BoardBoatEvent` and `DisembarkBoatEvent` should stay as Bevy messages for local UI/input orchestration. |
| Hierarchy | `ChildOf` is the authoritative parent relation. `Children` is maintained through relationship hooks. Despawning a parent recursively despawns descendants. | The boat model root can be parented to the player with `add_child`; root `despawn()` removes hull/sail/rudder children in Bevy 0.18. |
| Mesh mutation | Runtime mesh data must remain available in the main world. Meshes created only for render extraction cannot be mutated later. | Subdivided sail meshes must use `RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD` before `sail_animation_system` edits vertices. |
| Input | `ButtonInput<KeyCode>::pressed` is held-state input; `just_pressed` is one-frame input. | Rudder/trim use `pressed`; board/disembark command triggers should use message or just-pressed style inputs. |
| Time | `Time::delta_secs()` and `elapsed_secs()` are available. `Timer::tick`, `just_finished`, and `fraction` drive repeating effects. | Wake emitters and bow spray should continue to use Bevy timers. |
| System registration | `add_systems(Update, ...)`, `PostStartup`, and `in_state(AppState::Game)` are the correct Bevy 0.18 APIs. | Keep setup-only mesh/material resources in `PostStartup`; keep gameplay systems in `Update` with state gating. |
| Query disjointness | Bevy detects conflicting mutable query access at runtime (`B0001`). | Particle update queries that both mutate `Transform` must use disjoint filters such as `Without<BowSprayParticle>` / `Without<WakeParticle>`. |
| Egui | `EguiContexts::ctx_mut()` returns `Result`; input focus helpers are available through egui context/input resources. | HUD systems should return cleanly when no context exists and avoid stealing input from chat/debug windows. |

---

## Remaining And Production-Hardening Specifications

The following sections describe the remaining work and hardening tasks. Each section is self-contained enough to be assigned as a separate work item. When a section is already implemented locally, use its deliverables as a checklist for review/tuning rather than as a request to rewrite working code.

---

## A. Sail Mesh Deformation System

**Status**: Implemented locally. `SailMesh` now stores base vertex positions, dimensions, and subdivision level; `sail_animation_system` updates mesh vertices at runtime and honors `SailingGraphicsSettings.sailing.sail_deformation_quality`.

**Remaining work**: tune deformation constants, verify remote boats reuse the same animation path, and add regression tests around quality-level mesh generation if this becomes fragile.

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

The original prototype plan assumed a flat `Plane3d`. The current implementation already replaces that with a subdivided grid via `create_subdivided_sail_mesh()`. Keep that approach for all future sail visuals, including remote boats:

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

    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
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

- [x] `create_subdivided_sail_mesh()` function in [`boat_spawn_system.rs`](../src/systems/boat_spawn_system.rs)
- [x] Replace static plane sail mesh with subdivided mesh at spawn time
- [x] `sail_animation_system` in `src/systems/sail_animation_system.rs`
- [x] Register system in [`lib.rs`](../src/lib.rs), running in `Update` after `sailing_movement_system`
- [x] Store base vertex positions in `SailMesh` component for reference
- [x] Respect `SailingGraphicsSettings.sailing.sail_deformation_quality`
- [x] Add remote-boat coverage after multiplayer boat rendering exists
- [x] Add focused tests for sailing polar curve and angle edge cases
- [ ] Add focused tests for mesh subdivision if future changes destabilize tuning

---

## B. Boat Wake & Spray Effects

**Status**: Implemented locally. `WakeEmitter`, `WakeParticle`, `BowSprayParticle`, and `WakeSource` exist in `src/components/boat_wake.rs`; `boat_wake_system.rs` creates shared particle mesh/material resources, spawns wake/spray, and updates/despawns particles.

**Remaining work**: reduce per-particle material churn, add support for remote boats, and tune budgets once multiple boats can be visible at once.

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

- [x] `WakeParticle`, `WakeEmitter`, `BowSprayParticle`, and `WakeSource` components in `src/components/boat_wake.rs`
- [x] `boat_wake_spawn_system` spawns wake and spray particles
- [x] `boat_wake_update_system` updates position/alpha/scale and despawns expired particles
- [x] Register setup/spawn/update systems in [`lib.rs`](../src/lib.rs)
- [x] Shared billboard quad mesh and base materials for wake/spray particles
- [x] Replace per-particle alpha material cloning with pooled alpha-bucket materials before supporting many remote boats
- [x] Confirm wake/spray can be driven by remote replicated boat speed and heading

---

## C. Sailing HUD (Wind Compass + Speed Gauge)

**Status**: Implemented locally. `ui_sailing_hud_system.rs` draws the wind compass, speed gauge, trim indicator, and prompt window while `BoatState.active` is true.

**Remaining work**: production UI styling, prompt conditions, localization-ready text, and layout checks against chat/debug windows.

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

- [x] Full egui implementation of wind compass in [`ui_sailing_hud_system.rs`](../src/ui/ui_sailing_hud_system.rs)
- [x] Speed gauge bar with color coding
- [x] Sail trim arc indicator
- [x] Contextual prompt text
- [x] Only render when `BoatState.active == true`
- [x] Convert always-visible disembark prompt into contextual shore-availability prompt
- [ ] Verify layout with chat, debug, and settings windows open

---

## D. Ocean Zone Map Creation

**Status**: Client scaffold started. Zone `200` applies ocean water tuning in the client, and `3DDATA/MAPS/OCEAN` exists as an authoring scaffold. Real exported zone data and server registration are not implemented.

### D.1 Goal

Create a dedicated ocean zone (e.g. `ZoneId(200)`) with ~6.4 km² of ocean and 6–8 islands.

### D.1.1 Minimum Viable Ocean Zone

The first playable zone does not need all island content. It does need enough data to exercise every existing sailing system:

| Required data | Why it matters |
|---------------|----------------|
| `OCEAN.ZON` loading successfully as zone `200` | Lets normal zone loading, fog, sky, and map metadata paths run. |
| At least one active water block with an IFO water plane | `UnderwaterVolumes` must exist so boat height sampling uses real map water. |
| One island or dock with walkable terrain above water | Required for `find_nearest_shore_position` and E-key disembark testing. |
| A clear launch point near water | Required for production boarding validation and QA. |
| Server zone registration | Required for normal in-game travel instead of only zone viewer/editor testing. |

Suggested MVP layout:

1. Use a small active block range first, such as `24..28` by `24..28`.
2. Put flat water at height `0 cm` across all active blocks.
3. Put one dock/island that rises above water by at least `50 cm` so shore detection succeeds.
4. Register a warp from an existing mainland dock to the ocean island.
5. Only after this loads reliably, expand to the full `40 x 40` active ocean area.

### D.1.2 Client Validation Checklist

After exporting the first map data, validate these client behaviors:

- Zone `200` loads and `game_zone_change_system` applies ocean water settings.
- The water volume height matches the visible water plane and the boat sits at that height.
- `/boat` can be used for dev testing near the water.
- `E` disembark finds the island/dock terrain and places the player above water.
- The boat cannot sail through island terrain or wall/collision objects.
- Returning to a non-ocean zone resets default `WaterSettings`.

Do not consider the zone ready for server work until the water plane creates runtime water volume data. The current movement system can fall back to global water height, but that fallback hides map authoring mistakes.

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

### E.0 Implementation Strategy From Current Client Code

The current client is the reference implementation for feel, but not for authority. The server work should preserve the same input vocabulary and physics tuning while moving trust out of the client.

Current client responsibilities that must move or be mirrored server-side:

| Current client behavior | Production server behavior |
|-------------------------|----------------------------|
| `/boat` writes `BoardBoatEvent` locally. | Client sends a board request; server validates zone/water/dock/combat/inventory and replies with success/failure state. |
| `sailing_movement_system` mutates `Position` directly from local input. | Client sends compressed rudder/trim input; server simulates or validates the boat position. |
| `WindState` is generated locally. | Server owns wind state per zone and broadcasts angle/speed/gust. Client can predict between broadcasts. |
| Collision branch blocks land/walls locally. | Server validates island/terrain/water collision and corrects illegal boat positions. |
| `BoatState` exists only on the local player. | Network state must represent either a `MoveMode::Sail` player or a standalone boat entity. |
| Wake/sail/HUD read local `BoatState`. | Remote visuals should read replicated boat state and never local keyboard input. |

Recommended first milestone:

1. Keep local `BoatState` and input code, but add a `SailingAuthorityMode` or equivalent resource flag: `LocalPrototype`, `ClientPredicted`, `RemoteReplicated`.
2. Add server wind packets and make the client accept authoritative wind while preserving local fallback for offline/dev use.
3. Add sail input packets at 10 Hz. Inputs should be small: rudder axis, trim axis/current trim, and a client tick/time if the protocol already has prediction support.
4. Server simulates movement using the same polar curve and sends authoritative boat/player updates at 10 Hz.
5. Client reconciles local `Position`, `BoatState.heading`, `BoatState.speed`, and `BoatState.sail_trim` from server updates.

Do not begin by deleting the current local sailing movement. Keep it as prediction/offline fallback until server updates are stable.

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

Protocol note: verify unused opcodes in `rose-network-irose` before selecting final values. If older clients or tools assume only `Walk/Run/Drive`, adding `MoveMode::Sail` changes the shared enum contract and must be handled in every packet read/write path that serializes movement mode.

### E.2.1 Client Packet Handler Touchpoints

Start in the existing network message handling path rather than adding a parallel networking loop. The client-side implementation should touch:

| Client area | Work |
|-------------|------|
| `game_connection_system.rs` or existing packet dispatch | Decode boat spawn/update/wind packets and write Bevy messages/resources. |
| `WindState` resource | Add an authoritative override path from server packets. Keep local generation as fallback when offline or when no authoritative wind has arrived. |
| `BoatState` on local player | Apply correction data: heading, speed, trim, active state, water height if sent. |
| Remote player/boat lookup | Resolve server entity ids to client entities before spawning/updating remote visuals. |
| Chat/system messages | Surface server rejection reasons for board/disembark requests. |

Implementation rule: packet handlers should not directly spawn complicated boat hierarchies inline. Convert packets into a small internal message or function call that reuses the existing visual spawn helper, so local and remote boat visuals stay visually identical.

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

Add these validation details to make the implementation unambiguous:

| Validation | Detail |
|------------|--------|
| Water availability | Reject or correct positions where no water plane/volume exists near the boat unless the zone is explicitly configured as open ocean fallback. |
| Land margin | Use the same conceptual rule as the client: terrain height must remain below water height for the hull footprint, with a small tolerance for beaches/docks. |
| Tick-rate independence | Server physics must use actual delta time, clamped to a maximum catch-up step, so lag spikes do not create oversized movement deltas. |
| Input clamping | Clamp rudder to `[-1, 1]`, trim to `[0, PI]`, and reject NaN/infinite values before simulation. |
| Correction threshold | Send a correction when the client diverges beyond a small distance threshold; avoid correcting every frame for tiny floating-point drift. |
| Boarding state | Reject sail inputs from clients that are not currently in server sailing mode. |

### E.6.1 Shared Physics Extraction

To prevent client/server drift, extract the pure sailing math into a small shared module if workspace boundaries allow it:

```rust
pub struct SailingInput {
    pub rudder: f32,
    pub sail_trim_delta: f32,
}

pub struct SailingStepInput {
    pub heading: f32,
    pub speed: f32,
    pub sail_trim: f32,
    pub wind_angle: f32,
    pub wind_speed: f32,
    pub dt: f32,
}

pub struct SailingStepOutput {
    pub heading: f32,
    pub speed: f32,
    pub sail_trim: f32,
    pub forward_cm: Vec2,
}
```

Keep the shared function free of Bevy queries, assets, commands, and UI. The client system can call it after reading Bevy resources; the server system can call it after decoding network input. Unit-test this pure function with no Bevy app setup.

### E.7 Zone Transition While Sailing

When a sailing player hits a warp gate:
1. Server despawns the boat entity from the ocean zone
2. Player transitions to the new zone as `MoveMode::Run` (on foot)
3. Boat is stored as an inventory item / persistent state
4. When returning to the ocean zone, player can re-summon the boat at a dock

### E.8 Deliverables

- [x] `MoveMode::Sail` variant in `rose-game-common`
- [x] Packet encoding/decoding for `MoveMode::Sail` in the existing move-mode byte path
- [ ] Packet encoding/decoding for standalone boat opcodes if that design is selected
- [ ] `ServerBoatState` component and `ServerWindState` resource
- [ ] `server_sailing_system` — validates client sailing
- [ ] `server_wind_update_system` — server-authoritative wind
- [ ] `wind_broadcast_system` — sends wind to clients at 1 Hz
- [x] Client handling for remote player-mode sailing through existing move-mode messages
- [x] Remote boat entity spawn/update on other clients for player-mode sailing
- [ ] Anti-cheat validation (speed, position, terrain)
- [ ] Zone transition handling for boats

---

## F. Audio System

**Status**: Implemented locally. `src/audio/boat_sound.rs` now creates generated in-memory placeholder `AudioSource` handles for wind, creak, flap, splash, rope, board, and disembark sounds; loop entities are spawned when sailing starts, gains are updated while active, and cleanup/one-shots are handled through the existing Oddio `SpatialSound` path.

### F.0 Existing Audio Patterns To Reuse

The client already has a custom Oddio-backed audio layer. Sailing audio should reuse it rather than introducing a second audio system.

| Existing code | Reuse pattern |
|---------------|---------------|
| `src/audio/spatial_sound.rs` | Attach `SpatialSound::new_repeating(handle)` to child entities for looping boat-local sounds. Use `SoundGain` and `SoundRadius` components for volume/radius control. |
| `src/audio/global_sound.rs` | Use `GlobalSound::new_repeating(handle)` for zone-wide ocean ambience if it should not attenuate with distance. |
| `src/resources/sound_cache.rs` | Use `SoundCache::load` when sound ids are available through `GameData`; avoid repeated `asset_server.load` calls every frame. |
| `src/systems/vehicle_sound_system.rs` | Good reference for adding/removing looped spatial sounds as movement state changes. |

Implementation rule: do not spawn or load sounds every frame. Create loop entities when sailing starts, update gain/state while active, and remove or stop them when sailing ends.

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

### F.2.1 Component Shape

Prefer a component attached to the player or boat visual root:

```rust
#[derive(Component)]
pub struct BoatSoundState {
    pub wind_loop: Option<Entity>,
    pub creak_loop: Option<Entity>,
    pub flap_loop: Option<Entity>,
    pub splash_timer: Timer,
    pub last_sail_trim: f32,
    pub last_luffing: bool,
}
```

Child sound entities should contain:

```rust
(
    SpatialSound::new_repeating(handle),
    SoundGain::Ratio(initial_gain),
    SoundRadius(12.0),
    Transform::default(),
    GlobalTransform::default(),
)
```

Attach these sound entities to the boat model root or player entity so `SpatialSound` receives a valid `GlobalTransform`.

### F.2.2 System Split

Use three small systems instead of one large state machine:

| System | Responsibility |
|--------|----------------|
| `ensure_boat_sound_state_system` | Add `BoatSoundState` when a player has `BoatState`; create/cleanup loop entities when `active` changes. |
| `boat_loop_sound_update_system` | Adjust gain for wind, creak, and flap loops based on speed, luffing, and wave motion. |
| `boat_one_shot_sound_system` | Spawn splash, rope, board, and disembark one-shots from timers/messages. |

Schedule these after `boat_toggle_system` and after `sailing_movement_system`, so active state and speed are current.

### F.3 Deliverables

- [x] Placeholder sound assets generated in memory until final WAV/OGG files or catalogued sound IDs exist
- [x] `BoatSoundState` component
- [x] Loop sound update system for wind, hull creak, and sail flap gain
- [x] Spawn/cleanup system creates loop entities on board and removes them on disembark
- [x] One-shot system creates splash, rope, board, and disembark sounds
- [x] Register in [`lib.rs`](../src/lib.rs)
- [x] Avoid repeated per-frame `asset_server.load` calls
- [ ] Replace generated placeholders with `SoundCache`/`GameData` catalogued sounds when production sound IDs are added

---

## G. Enhanced Sail Camera

**Status**: Implemented locally. The current sail camera clamps sailing zoom, smoothly tracks behind the boat, supports right-mouse free-look, and adjusts pitch/follow distance for sailing.

**Remaining work**: tune comfort, decide whether to add speed-based FOV, and make sure camera smoothing follows the stable player position rather than visible boat heave when wave effects become larger.

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

- [x] Expand [`sail_camera_system.rs`](../src/systems/sail_camera_system.rs) with smooth tracking
- [x] Free-look override with right mouse button
- [x] Pitch adjustment for horizon visibility
- [ ] Optional speed-based FOV tuning
- [ ] Wave-compensation review if larger ocean swells make camera motion uncomfortable

---

## H. Multiplayer Boat Rendering

**Status**: Client render path implemented for server/player-mode sailing. The client now supports `MoveMode::Sail` decoding/encoding, `RemoteBoatState`, remote boat visual spawning/despawning, remote sail animation, and remote wake/spray through the same `BoatState` path used by local sailing. Custom standalone `SpawnBoatEntity` / `UpdateBoatState` packet opcodes are still server-protocol work.

### H.1 Goal

When another player is sailing in the same zone, their boat should be visible to all nearby clients.

### H.1.1 Integration Points In Current Client

Remote rendering should build on the existing entity lookup and visual systems:

| Existing piece | Use for multiplayer sailing |
|----------------|-----------------------------|
| `ClientEntityList` | Map server `ClientEntityId` values to Bevy `Entity` values before attaching boat visuals. |
| `ClientEntity` component | Identify remote players and despawn/update visuals when server entities leave scope. |
| `spawn_boat_visual` logic in `boat_spawn_system.rs` | Extract or expose a reusable helper so local and remote boats share the same model construction. |
| `SailMesh` and `sail_animation_system` | Remote boat sails should animate from replicated heading/speed/wind, not local keyboard state. |
| `WakeEmitter` and `boat_wake_system` | Remote boats can receive wake emitters once their replicated speed is available. |

Do not reuse the local `PlayerCharacter` filters for remote boats. Local systems that read keyboard, mouse, or camera input must only operate on the local player.

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

### H.3.1 Remote State Design

Use a remote component that is explicit about source-of-truth:

```rust
#[derive(Component)]
pub struct RemoteBoatState {
    pub server_entity_id: ClientEntityId,
    pub visual_root: Entity,
    pub prev_position: Vec3,
    pub target_position: Vec3,
    pub prev_heading: f32,
    pub target_heading: f32,
    pub target_speed: f32,
    pub sail_trim: f32,
    pub update_age: f32,
    pub update_interval: f32,
}
```

Attach `RemoteBoatState` to the remote player entity if using player movement mode. If using standalone boat entities, attach it to the boat entity and store the rider/player entity id separately.

### H.3.2 Spawn/Update/Despawn Flow

1. `SpawnBoatEntity` packet arrives.
2. Resolve `rider_entity_id` with `ClientEntityList`.
3. Spawn boat visual root using the shared visual helper.
4. Parent visual root to the remote player or standalone boat entity.
5. Insert `RemoteBoatState` and a non-local `BoatState`-like render state.
6. On `UpdateBoatState`, update interpolation targets and render state.
7. On player leaves zone, boat despawn packet, or mode changes away from sailing, despawn the visual root and remove remote boat components.

Failure handling: if a packet references a remote player that has not spawned yet, store a short-lived pending boat spawn keyed by `ClientEntityId` and retry for a few frames. Drop it if the player never appears.

### H.4 Deliverables

- [x] `RemoteBoatState` component
- [x] Handle server/player-mode sailing via `MoveMode::Sail` on remote client entities
- [x] Spawn remote boat visual with the shared `spawn_boat_visual()` helper
- [x] Update remote boat render state from replicated movement/facing data
- [x] Despawn remote boat when player leaves sailing mode
- [x] Sail animation works for remote boats too
- [x] Wake/spray works for remote boats, behind graphics settings and distance culling
- [x] Remote boat systems do not consume local keyboard/mouse/camera input
- [ ] Add custom standalone `SpawnBoatEntity` / `UpdateBoatState` packet handling if the server chooses standalone boat entities instead of player movement mode

---

## I. Disembark Mechanics

**Status**: Partially implemented. `E` key disembark, nearest-shore placement, boat visual cleanup, and character model visibility toggling exist. In the current source, several boarding validations are commented out for prototype testing, so production validation is still required.

### I.1 Current State

Currently `/boat` is a permissive dev/prototype entry point in the source. Earlier client validation work exists in this file's history, but several checks are currently commented out in `boat_spawn_system.rs` to keep sailing easy to test. Treat local checks as user feedback only; final rules belong on the server.

Implemented locally:

- `E` while sailing writes `DisembarkBoatEvent`.
- Shore search samples terrain in 8 directions and picks nearby terrain above water.
- Boat visual root is despawned on disembark.
- Character model parts are hidden while boarded and restored on disembark.

Still required:

- Server must reject invalid board/disembark attempts.
- Client should display the server rejection reason in chat/system UI.
- `/boat` should eventually become a debug-only command or call the same request path as the production interaction.

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

- [x] `E` key binding for disembark
- [x] Shore detection algorithm
- [x] Player teleport to nearest shore on disembark
- [x] Character model visibility toggle on board/disembark
- [ ] Restore client-side validation for user feedback after production rules are agreed
- [ ] Add server-authoritative boarding validation: correct zone, near water/dock, alive, not in combat, has/owns boat

---

## Implementation Priority & Dependencies

```
             ┌───────────────────────────┐
             │ D. Ocean Zone Data         │
             │ real ZON/HIM/TIL/IFO       │
             └────────────┬──────────────┘
                          │
                          ▼
             ┌───────────────────────────┐
             │ E. Server Authority        │
             │ wind, input, validation    │
             └────────────┬──────────────┘
                          │
                          ▼
             ┌───────────────────────────┐
             │ H. Multiplayer Rendering   │
             │ remote boats/interp/VFX    │
             └────────────┬──────────────┘
                          │
        ┌─────────────────┼─────────────────┐
        ▼                 ▼                 ▼
┌───────────────┐ ┌───────────────┐ ┌───────────────┐
│ I. Production │ │ F. Audio       │ │ A/B/C/G       │
│ boarding rules│ │ loops/one-shot │ │ polish/tuning │
└───────────────┘ └───────────────┘ └───────────────┘
```

### Parallel Work Streams

| Stream | Items | Estimated Time |
|--------|-------|---------------|
| **Art/Level Design** | D (Ocean Zone data and island content) | 2–3 weeks |
| **Server Networking** | E, then H packet support | 2–3 weeks |
| **Client Multiplayer** | H remote visuals/interpolation, remote VFX | 1–2 weeks after packet contract |
| **Client Polish** | F audio, I production interaction, A/B/C/G tuning | 1–2 weeks |

The current client prototype means A, B, C, and G are no longer blockers for server work. The critical path is now D -> E -> H. Audio and polish can run in parallel once boat active state and replicated speed/heading are stable.
