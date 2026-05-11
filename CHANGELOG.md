# Project Changelog — ROSE Offline Client (Bevy 0.18.1)

## Session Started: 2026-05-11

### Analysis Phase — Full Codebase Review

**Objective:** Document every section of code, explore sailing documents, analyze zone loading, and plan the ocean zone (Zone 200) implementation.

#### What Was Reviewed

**Sailing Plan Documents (plans/):**
- `sailing-system-plan.md` (1046 lines) — Master plan: 11 phases covering ocean zone, boat entity, wind, movement, server authority, camera, HUD, VFX, audio, island content, polish
- `sailing-system-detailed-expansion.md` (1062 lines) — Deep technical specs for remaining work: sail mesh deformation (A), wake/spray (B), HUD design (C), ocean zone block layout (D), server networking (E), audio (F), multiplayer boats (H), disembark (I), dependency graph
- `sailing-system-implementation-tracking.md` (241 lines) — 10 implementation attempts logged, all with successful `cargo build` validation

**System Architecture Docs (system-architecture/):**
- 23 architecture documents covering: ECS, Transform, Lighting, Camera, Input, Render, Physics, Audio, Animation, Assets, UI, Window, Flying System, Map Editor, Weather/Season, Blood Effects, Chat/Name Tags, Monster Collision, Zone Lighting, Sky/Stars

**Pitfalls (pitfalls/):**
- 15 documented pitfalls including: zone-loading (asset tracking, state init), water-system (shader migration, fish parenting), terrain-physics, materials-transparency, lighting, performance-memory, networking, rendering-camera, skill-bar-UI, blood-effects, model-viewer

**Source Code Modules (src/):**
- `main.rs` — Entry point, CLI args (config, zone, zone-viewer, map-editor, model-viewer, auto-login, etc.)
- `zone_loader.rs` (3628 lines) — Zone loading pipeline: ZoneLoaderAsset, ZoneLoaderBlock, async loading via channel, spawn_zone, terrain/water/NPC/object spawning
- `systems/` — 70+ systems: sailing_movement, boat_spawn, boat_buoyancy, boat_wake, sail_animation, sail_camera, wind, collision, flight, game_system (ocean zone water tuning), input, UI, animation, audio, particle, etc.
- `components/` — BoatState, BoatModel, SailMesh, boat_wake, player_character, position, facing_direction, collision, command, Zone, ZoneObject, WarpObject, EventObject, etc.
- `resources/` — WindState, WaterSettings, CurrentZone, GameData, AppState, SeasonSettings, etc.
- `events/` — BoardBoatEvent, DisembarkBoatEvent, LoadZoneEvent, ZoneEvent, ZoneLoadedFromVfsEvent, etc.
- `render/` — TerrainMaterial, WaterMaterial, ParticleMaterial, ZoneLighting, WorldUi, TrailEffect, WingMaterial
- `map_editor/` — Full map editor: selection, transform gizmo, model placement, undo/redo, save/export (IFO), UI panels
- `ui/` — 40+ UI systems: chat, inventory, skills, quest, sailing HUD, settings, minimap, etc.
- `protocol/` — iRose protocol clients (game, login, world)
- `scripting/` — Lua4 VM, quest system, game functions
- `audio/` — Custom audio: OGG/WAV loaders, spatial sound, streaming, monster sound cap
- `animation/` — Skeletal, mesh, camera, transform animations; ZMO asset loader
- `graphics/` — GraphicsSettings (VSync, MSAA, shadows, bloom, SSAO, DOF, sailing-specific toggles)
- `terrain/` — Procedural noise overlay for terrain enhancement
- `logging/` — JSON Lines structured logging with session tracking

#### Key Findings

**Sailing System Status: ~80% Complete (Client-Side)**

| Component | Status | File(s) |
|-----------|--------|---------|
| BoatState/BoatModel/SailMesh components | ✅ Done | `src/components/boat.rs` |
| WindState/WindSettings resources | ✅ Done | `src/resources/wind_state.rs` |
| BoardBoatEvent/DisembarkBoatEvent | ✅ Done | `src/events/boat_event.rs` |
| Wind update system (drift + gust) | ✅ Done | `src/systems/wind_system.rs` |
| Boat spawn/procedural mesh | ✅ Done | `src/systems/boat_spawn_system.rs` |
| Sailing movement (polar speed curve) | ✅ Done | `src/systems/sailing_movement_system.rs` |
| Boat buoyancy (wave roll/pitch) | ✅ Done | `src/systems/boat_buoyancy_system.rs` |
| Sail camera (orbit + free-look) | ✅ Done | `src/systems/sail_camera_system.rs` |
| Sail animation (billow/luffing) | ✅ Done | `src/systems/sail_animation_system.rs` |
| Wake + bow spray particles | ✅ Done | `src/systems/boat_wake_system.rs` |
| Sailing HUD (compass/speed/trim) | ✅ Done | `src/ui/ui_sailing_hud_system.rs` |
| `/boat` chat command | ✅ Done | `src/ui/ui_chatbox_system.rs` |
| Disembark (E key, shore placement) | ✅ Done | `src/systems/boat_spawn_system.rs` |
| Collision gating (wall/land blocking) | ✅ Done | `src/systems/collision_system.rs` |
| Input gating (WASD blocked while sailing) | ✅ Done | `src/systems/game_keyboard_input_system.rs` |
| Ocean zone water tuning (Zone 200) | ✅ Done | `src/systems/game_system.rs` |
| **Ocean zone map data (ZON/HIM/TIL/IFO)** | ⬜ Not done | `3DDATA/MAPS/OCEAN/` (scaffold only) |
| Sail mesh deformation runtime | ✅ Done | `src/systems/sail_animation_system.rs` |
| Server authority & networking | ⬜ Not done | Server-side (rose-offline) |
| Audio system (creaking, waves) | ⬜ Not done | Planned in detailed expansion |
| Multiplayer boat rendering | ⬜ Not done | Planned in detailed expansion |

**Zone Loading Pipeline (Fully Understood):**

```
LoadZoneEvent → zone_loader_system
  → AsyncComputeTaskPool.spawn(load_zone_direct)
    → Reads ZON, ZSC_CNST, ZSC_DECO from VFS/filesystem
    → Iterates 64×64 blocks, loads HIM/TIL/IFO/LIT per block
    → Returns ZoneLoaderAsset via channel
  → zone_loader_system receives via ZoneLoadChannelReceiver
  → Adds to Assets<ZoneLoaderAsset>
  → Sends ZoneLoadedFromVfsEvent
  → zone_loaded_from_vfs_system calls spawn_zone()
    → Spawns zone entity at Transform(5200, 0, -5200)
    → Iterates blocks: spawns terrain, water, NPCs, objects
    → Sends ZoneEvent::Loaded
  → game_zone_change_system applies ocean water settings (if Zone 200)
```

**Critical Zone Data Requirements:**
- Zone must be registered in ZoneList (rose-data)
- ZON file: tile textures, grid config, tile definitions
- HIM files: heightmap per block (64×64 grid)
- TIL files: tile map per block (texture indices)
- IFO files: water planes, NPCs, warps, deco/cnst objects
- ZSC files: object definitions (CNST, DECO)
- Real filesystem (base_path) takes priority over VFS

**Ocean Zone (Zone 200) Current State:**
- Client code: water tuning applied on ZoneEvent::Loaded ✅
- Data scaffold: `3DDATA/MAPS/OCEAN/.gitkeep` + `OCEAN-zone-scaffold.md` ✅
- Actual zone data files: **NOT created yet** ⬜
- Server registration: **NOT done** ⬜

---

## Next Steps (Planned Work)

### Phase 1: Code Documentation (Current Session)
- [x] Analyze and document all sailing documents
- [x] Trace zone loading pipeline
- [x] Document codebase structure
- [ ] Create comprehensive code documentation per module

### Phase 2: Ocean Zone Data Creation
- [ ] Create Zone 200 ZON file
- [ ] Generate ocean HIM files (flat water = -500cm, islands elevated)
- [ ] Generate ocean TIL files (sand/grass/ocean floor textures)
- [ ] Generate ocean IFO files (water planes, island deco, docks, NPCs)
- [ ] Register zone in server ZoneList
- [ ] Validate zone loads in client

### Phase 3: Sailing System Polish
- [ ] Audio system (wave sounds, sail creaking)
- [ ] Server authority & networking
- [ ] Multiplayer boat rendering
- [ ] Island content (NPCs, quests, warp gates)

---

## Files Modified This Session

| File | Action | Description |
|------|--------|-------------|
| `CHANGELOG.md` | Created | Project changelog |
| `PROJECT_PLAN.md` | Created | Implementation plan |
