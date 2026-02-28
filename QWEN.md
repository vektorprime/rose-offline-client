# ROSE Offline Client - Project Context

## Project Overview

**rose-offline-client** is an open-source game client for ROSE Online, built with the Bevy game engine (v0.16). It's a fork of exjam's original rose-offline project, modified and extended with AI assistance. The client is compatible with the official 129_129en irose server or the rose-offline server.

### Key Technologies
- **Language:** Rust 2021 edition
- **Game Engine:** Bevy 0.16 (with custom features)
- **Physics:** bevy_rapier3d 0.31
- **UI:** bevy_egui 0.34, bevy-inspector-egui 0.33
- **Audio:** cpal 0.15, oddio 0.6, lewton 0.10
- **Dependencies:** rose-offline ecosystem (rose-data, rose-network, etc.)

### Architecture
The project follows Bevy's ECS (Entity-Component-System) architecture with modular plugin-based organization:

```
src/
├── main.rs              # Entry point, CLI argument parsing
├── lib.rs               # Core app setup, configuration, plugin registration
├── animation/           # Animation systems
├── audio/               # Audio handling with oddio
├── bundles/             # Entity component bundles
├── components/          # Component definitions
├── debug/               # Debug tools and inspectors
├── diagnostics/         # Performance diagnostics
├── events/              # Event definitions
├── map_editor/          # Map editor functionality
├── protocol/            # Network protocol handling
├── render/              # Custom materials, shaders, rendering plugins
├── resources/           # App state, game data, configuration resources
├── scripting/           # Lua scripting support
├── systems/             # Game systems (player, NPCs, physics, etc.)
├── ui/                  # UI systems using egui
├── types.rs             # Type definitions
└── Various loaders:     # Asset loading (VFS, models, zones, DDS, ZMS)
```

## Building and Running

### Prerequisites
- Rust toolchain (stable)
- rose-offline server components (rose-data, rose-network, etc.) in the expected paths
- Game data files (data.idx or extracted data directory)

### Build Commands
```bash
# Debug build
cargo build

# Release build
cargo build --release
```

### Run Commands

**Standard Game Mode:**
```bash
# Run from directory with data.idx
cargo run

# With custom data path
cargo run -- --data-idx=path/to/data.idx

# With server connection
cargo run -- --ip=127.0.0.1 --port=29000
```

**Special Modes:**
```bash
# Map Editor (with optional zone)
cargo run -- --map-editor --zone=1

# Model Viewer
cargo run -- --model-viewer

# Zone Viewer
cargo run -- --zone=1
```

**Auto-Login:**
```bash
cargo run -- --auto-login --username=user --password=pass --server-id=0 --channel-id=0
```

**All CLI Arguments:**
- `--config=<path>` - Path to config.toml
- `--data-idx=<path>` - Path to irose 129en data.idx
- `--data-aruavfs-idx=<path>` - Path to aruarose data.idx
- `--data-titanvfs-idx=<path>` - Path to titanrose data.idx
- `--data-path=<path>` - Override files from directory
- `--ip=<ip>` - Server IP (default: 127.0.0.1)
- `--port=<port>` - Server port (default: 29000)
- `--model-viewer` - Model viewer mode
- `--zone=<N>` - Zone viewer mode
- `--map-editor` - Map editor mode
- `--disable-vsync` - Disable v-sync for accurate frame times
- `--passthrough-terrain-textures` - GPU texture passthrough
- `--disable-sound` - Disable audio
- `--auto-login` - Enable auto-login
- `--username`, `--password` - Credentials
- `--server-id`, `--channel-id` - Server/channel selection
- `--character-name` - Auto-select character

### Configuration
Configuration can be provided via CLI arguments or a `config.toml` file with sections for:
- `account` - Username/password
- `auto_login` - Auto-login settings
- `filesystem` - VFS devices (vfs, directory, aruavfs, titanvfs, iroseph)
- `game` - Data/network/UI version selection
- `graphics` - Window mode, V-sync, terrain texture settings
- `server` - IP and port
- `sound` - Volume levels per category

## Features

### AI-Added Features
- **Bevy 0.16.1** upgrade with enhanced rendering
- **Map Editor** - Edit zone objects, terrain, and entity properties
- **Flying** - `/fly` command with space-bar activation
- **In-game Settings Menu** - Modify graphics and systems at runtime
- **Enhanced Graphics:**
  - SSAO (Screen Space Ambient Occlusion)
  - Volumetric fog (disabled by default)
  - Depth of field (disabled by default)
  - Better shadows with proper shadow filtering
  - Water quality settings
- **Environmental Effects:**
  - Leaves and grass swaying in wind
  - Animated fish in water
  - Animated birds in sky
  - Seasons/weather (fall leaves, snow, rain, thunder/lightning)
  - Starry sky with night/day cycle
  - Character dash effects (dirt/dust when running)

## Development Conventions

### Code Style
- Follows Rust 2021 edition conventions
- Uses `#![allow(warnings)]` and various `clippy` allow attributes for legacy code
- Extensive use of Bevy's ECS patterns (Components, Resources, Systems, Events)
- Modular plugin architecture for systems

### Testing
No automated test suite is currently present. Testing is done through manual gameplay and debug tools.

### Debug Tools
The project includes extensive debug UI accessible through the in-game debug menu:
- Entity inspector
- Diagnostics (render, physics)
- Zone/time debugging
- Client entity list
- Effect/skill/item lists
- Command viewer

### Key Patterns

**Custom Materials:**
- Implements `AsBindGroup` with `CreateBindGroupDirectly` error handling (Bevy 0.16+)
- Uses bind group index 2 for material bindings
- Shaders use `get_world_from_local(instance_index)` instead of deprecated `mesh.model`

**State Management:**
- Uses Bevy's `AppExtStates` for app state transitions
- States include: Login, Game, CharacterSelect, ModelViewer, ZoneViewer, MapEditor

**Asset Loading:**
- Virtual filesystem (VFS) abstraction for multiple data formats
- Custom loaders for game-specific formats (ZMS, DDS, EXE resources)
- Zone loading with heightmap-based terrain generation

### Known Pitfalls (See pitfalls.md)
- Depth of field requires HDR + Tonemapping + Bloom
- AmbientLight uses photometric units (cd/m²) in Bevy 0.15+ (use ~500.0, not 0.3)
- SSAO/TAA require `Msaa::Off`
- ExtendedMaterial shaders cannot access bind groups beyond 0-2
- Zone assets must be added to Assets collection before triggering load events
- Bevy Bundle trait limits tuples to ~15 components - split large spawns

### Documentation
- **README.md** - Project overview and screenshots
- **pitfalls.md** - Development issues and solutions
- **SUN_DOCUMENTATION.md** - Sun/lighting system documentation
- **bevy-*-migration-guide.md** - Bevy version migration notes

## External Dependencies

The project depends on rose-offline ecosystem crates (path dependencies):
- `rose-data` - Game data structures
- `rose-network` - Network protocol
- `rose-file-readers` - VFS and file format readers
- `rose-game-common` - Shared game logic

These are expected at: `C:/Users/vicha/RustroverProjects/rose-offline/*`