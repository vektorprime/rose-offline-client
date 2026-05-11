# ROSE Offline Client — Project Plan

> **Repository:** https://github.com/vektorprime/rose-offline-client
> **Engine:** Bevy 0.18.1 (Rust)
> **Platform:** Windows (compiles on Windows, validated via `cargo check` on Linux)
> **Last Updated:** 2026-05-11

---

## Goal

Create a fully functional ocean sailing zone (Zone 200) with water, islands, docks, and sailing boats — integrated into the existing ROSE Online offline client.

---

## Current State Summary

### What Exists (Working)

The sailing system is ~80% complete client-side after 10 implementation attempts:

- **Boat mechanics:** Full sailing physics with wind-relative polar speed curve, tacking, luffing, sail trim
- **Boat visuals:** Procedural sailboat mesh (hull, mast, rig, 2 sails, rudder), sail deformation (billow/luffing)
- **VFX:** Wake particles (V-shaped trail), bow spray at high speed
- **Camera:** Behind-boat orbit camera with free-look override
- **HUD:** Wind compass, speed gauge, sail trim indicator, disembark prompt
- **Controls:** WASD steering/trim, E to disembark, /boat chat command to spawn
- **Environment:** Wind system (directional drift + gusts), buoyancy (wave roll/pitch/heave)
- **Safety:** Collision gating (walls/land blocking), boarding validation (zone check, water proximity, combat state)
- **Disembark:** Shore placement algorithm (8-direction scan, terrain height check)
- **Zone 200 client behavior:** Ocean-specific water tuning (wave amplitude 1.5, frequency 0.8, foam 1.2)

### What's Missing

1. **Ocean zone map data** — No ZON/HIM/TIL/IFO files exist for Zone 200
2. **Server registration** — Zone 200 not registered in rose-data ZoneList
3. **Audio** — No wave/sail/boat ambient sounds
4. **Server authority** — No server-side sailing validation
5. **Multiplayer** — No remote boat rendering

---

## Implementation Plan

### Phase 1: Code Documentation ✅ (In Progress)

Document every module, system, component, and resource.

**Completed:**
- Full codebase structure mapped
- Zone loading pipeline traced end-to-end
- Sailing documents analyzed (3 plan files)
- Architecture docs reviewed (23 files)
- Pitfalls reviewed (15 files)
- Changelog created

### Phase 2: Ocean Zone Data Creation

**Goal:** Create Zone 200 data files so the zone loads with water, islands, and docks.

**Zone Loading Requirements (from zone_loader.rs):**

The zone loader expects:
1. **ZoneList registration** (rose-data) — ZON path, ZSC_CNST path, ZSC_DECO path
2. **ZON file** — Zone metadata: tile textures, grid_size, grid_per_patch, tile definitions
3. **Block files** (64×64 grid, each block = 160×160 world meters):
   - `{x}_{y}.HIM` — Heightmap (required)
   - `{x}_{y}.TIL` — Tile map (optional, defaults to 0)
   - `{x}_{y}.IFO` — Object placement (optional): water planes, NPCs, warps, deco, cnst, event objects
   - `{x}_{y}.LIT` — Lighting data (optional)

**Ocean Zone Design:**

| Feature | Specification |
|---------|--------------|
| Zone ID | 200 |
| Total area | ~40×40 blocks (6400×6400m ≈ 6.4 km²) |
| Islands | 6-8 islands (2×2 to 5×5 blocks each) |
| Water | Flat heightmap at -500cm with water planes at 0cm |
| Islands | Elevated terrain with grass/sand textures |
| Docks | CNST objects at island shores |
| NPCs | Dock vendor, quest NPC, warp gate keeper |
| Warp gate | Bi-directional transition to mainland |

**Steps:**

1. Register Zone 200 in server ZoneList (rose-data)
2. Create ZON file with ocean tile textures
3. Generate HIM files:
   - Ocean blocks: flat -500cm heightmap
   - Island blocks: elevated terrain with beach slopes
4. Generate TIL files:
   - Ocean: ocean floor texture
   - Islands: grass, sand, rock textures
5. Generate IFO files:
   - Water planes covering all ocean blocks
   - Island deco (palm trees, rocks)
   - Dock CNST objects
   - NPC spawns
   - Warp gate
6. Test zone loading in client (zone viewer mode)

### Phase 3: Sailing System Polish

**Audio (Detailed Expansion Section F):**
- Wave ambient sound (zone-level)
- Sail creaking (speed-dependent)
- Water splashing (rudder turns)
- Wind whooshing (gust-dependent)

**Server Authority (Detailed Expansion Section E):**
- Position validation packets
- Zone transition handling
- Entity sync for multiplayer boats

**Multiplayer (Detailed Expansion Section H):**
- Remote boat rendering
- Position interpolation
- Sail state sync

---

## Zone Loading Deep Dive

### Pipeline

```
User triggers zone load (server packet / zone viewer / map editor)
  ↓
LoadZoneEvent { id: ZoneId(200), despawn_other_zones: true }
  ↓
zone_loader_system reads event
  ↓
AsyncComputeTaskPool spawns load_zone_direct()
  ↓
load_zone_direct reads:
  - ZON file (zone metadata)
  - ZSC_CNST file (construction object definitions)
  - ZSC_DECO file (decoration object definitions)
  - For each of 4096 blocks (64×64):
    - HIM file (heightmap) — REQUIRED
    - TIL file (tile map) — optional
    - IFO file (objects) — optional
    - LIT files (lighting) — optional
  ↓
Returns ZoneLoaderAsset via mpsc channel
  ↓
zone_loader_system receives, adds to Assets<ZoneLoaderAsset>
  ↓
Sends ZoneLoadedFromVfsEvent { zone_id, zone_handle }
  ↓
zone_loaded_from_vfs_system calls spawn_zone()
  ↓
spawn_zone:
  - Spawns zone entity at Transform(5200, 0, -5200)
  - Loads tile textures from ZON
  - Creates procedural water material
  - For each active block:
    - Spawns terrain mesh (heightmap → vertices)
    - Spawns water planes from IFO
    - Spawns NPCs, warps, deco, cnst, event objects
    - Sends WaterSpawnedEvent (fish spawning)
  ↓
Sends ZoneEvent::Loaded(ZoneId(200))
  ↓
game_zone_change_system:
  - Sends JoinZoneRequest to server
  - Applies ocean water settings (if Zone 200)
```

### Critical Gotchas (from pitfalls)

1. **All assets loaded via asset_server.load() must be tracked** in zone_loading_assets, or zone marks as "loaded" before assets are ready → race conditions
2. **Entities spawned within zones MUST be parented** to the zone entity → they inherit the Transform(5200, 0, -5200) offset
3. **Real filesystem takes priority over VFS** — files at base_path override VFS (for map editor modifications)
4. **HIM files are required** for each block — blocks without HIM are skipped entirely
5. **ZoneList must have the zone registered** — otherwise ZoneLoadError::InvalidZoneId

### Coordinate System

| Space | X | Y | Z |
|-------|---|---|---|
| Position (game, cm) | right | forward | up |
| Transform (world, m) | right | up | back |

Zone entities spawn at `Transform(5200, 0, -5200)` to center the 64×64 block grid in the world.

---

## File Inventory

### Sailing-Related Files

| File | Purpose |
|------|---------|
| `src/components/boat.rs` | BoatState, BoatModel, SailMesh, SailSide |
| `src/components/boat_wake.rs` | WakeEmitter, WakeParticle, BowSprayParticle |
| `src/resources/wind_state.rs` | WindState, WindSettings |
| `src/events/boat_event.rs` | BoardBoatEvent, DisembarkBoatEvent |
| `src/systems/wind_system.rs` | Wind update + vegetation sync |
| `src/systems/boat_spawn_system.rs` | Boat spawn/despawn, boarding validation, disembark |
| `src/systems/sailing_movement_system.rs` | Wind-relative sailing physics |
| `src/systems/boat_buoyancy_system.rs` | Wave roll/pitch/heave |
| `src/systems/sail_animation_system.rs` | Sail billow/luffing deformation |
| `src/systems/sail_camera_system.rs` | Behind-boat camera |
| `src/systems/boat_wake_system.rs` | Wake + spray particle system |
| `src/ui/ui_sailing_hud_system.rs` | Compass, speed gauge, trim indicator |
| `src/systems/game_system.rs` | Ocean zone water tuning (OCEAN_ZONE_ID = 200) |
| `3DDATA/MAPS/OCEAN/` | Ocean zone data scaffold |

### Zone Loading Files

| File | Purpose |
|------|---------|
| `src/zone_loader.rs` | Zone loading pipeline (3628 lines) |
| `src/resources/current_zone.rs` | CurrentZone resource |
| `src/events/zone_event.rs` | ZoneEvent (Loaded, Unloaded) |
| `src/events/load_zone_event.rs` | LoadZoneEvent |
| `src/components/event_object.rs` | Zone, ZoneObject, WarpObject, EventObject |
| `src/render/water_material.rs` | WaterMaterial (custom shader) |
| `src/render/terrain_material.rs` | TerrainMaterial (custom shader) |
| `src/render/underwater_effect.rs` | UnderwaterVolumes |

---

## Validation Strategy

Since the project compiles on Windows only:

1. **Syntax validation:** `cargo check` on Linux (catches type errors, missing imports, trait bounds)
2. **Full compilation:** `cargo build` on Windows (required by AGENTS.md rules)
3. **Runtime testing:** Windows only (zone viewer, map editor, game mode)

---

## Risk Register

| Risk | Impact | Mitigation |
|------|--------|------------|
| Zone 200 not in ZoneList | Zone fails to load | Register in server rose-data first |
| HIM files missing | Blocks skipped, empty zones | Generate HIM for all active blocks |
| Water planes not in IFO | No water rendering | Add water planes to every ocean block IFO |
| Entity not parented to zone | Wrong world position | Always use `commands.entity(zone_entity).add_child()` |
| Asset tracking incomplete | Race conditions on load | Track all asset_server.load() handles |
