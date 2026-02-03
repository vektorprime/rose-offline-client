# Rose Offline Client - High-Level Memory Bank

## Table of Contents
1. [Project Overview](#project-overview)
2. [Architecture Overview](#architecture-overview)
3. [Technology Stack](#technology-stack)
4. [Core Subsystems](#core-subsystems)
5. [Key Components](#key-components)
6. [Project Structure](#project-structure)
7. [Important File Locations](#important-file-locations)
8. [Design Patterns and Decisions](#design-patterns-and-decisions)
9. [Game Flow](#game-flow)
10. [Configuration](#configuration)

---

## Project Overview

**Project Name:** rose-offline-client  
**Version:** 0.1.0  
**Language:** Rust (2021 edition)  
**Primary Purpose:** An open-source client for ROSE Online, compatible with the official 129_129en iRose server or rose-offline server

### Project Description
The rose-offline-client is a modern reimplementation of the ROSE Online game client built with the Bevy game engine. It provides full offline capability and can connect to compatible servers. The client supports all major game features including character creation, combat, quests, party systems, and more.

### Key Features
- Full 3D rendering with custom materials
- Multi-server support (irose, aruarose, titanrose, iroseph)
- Network protocol implementation for iRose
- Complete UI system with drag-and-drop
- Animation system with skeletal mesh support
- Audio system with spatial sound
- Scripting system with Lua 4
- Physics integration with Rapier3D
- Zone loading and management
- Debug and diagnostic tools

---

## Architecture Overview

The project follows an Entity-Component-System (ECS) architecture using Bevy's ECS implementation. The codebase is organized into clear subsystems with well-defined responsibilities.

### Architectural Layers

```
┌─────────────────────────────────────────────────────────────┐
│                   Application Layer                        │
│  (main.rs - Entry point, CLI parsing, mode selection)    │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                    Game Engine Layer                      │
│           (Bevy ECS - Core engine systems)               │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                   Rose Systems Layer                      │
│  (Custom game systems, components, resources)             │
└─────────────────────────────────────────────────────────────┘
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                  Subsystem Layer                          │
│  (Assets, UI, Animation, Audio, Networking, etc.)       │
└─────────────────────────────────────────────────────────────┘
```

### Application Modes
The client supports three main modes:
1. **Game Mode** - Full game experience with login, character selection, and gameplay
2. **Model Viewer** - Standalone tool for viewing 3D models
3. **Zone Viewer** - Standalone tool for viewing game zones

---

## Technology Stack

### Core Dependencies

#### Game Engine
- **Bevy** (v0.14.2) - Primary game engine with ECS architecture
  - bevy_asset - Asset management
  - bevy_winit - Window management
  - bevy_core_pipeline - Rendering pipeline
  - bevy_pbr - Physically-based rendering
  - bevy_render - Rendering system
  - bevy_state - State management
  - Multi-threaded execution enabled

#### Third-Party Libraries
- **bevy_egui** (v0.28) - Immediate mode GUI for debug tools
- **bevy-inspector-egui** (v0.25) - Entity inspector for debugging
- **bevy_rapier3d** (v0.27) - Physics engine for collision detection
- **egui** (v0.28) - UI framework
- **tokio** (v1.17) - Async runtime for networking

#### Asset Loading
- **image** (v0.24) - Image processing (DDS, ICO, TGA, PNG, JPEG)
- **rose-file-readers** - Custom library for Rose Online file formats

#### Audio
- **oddio** (v0.6) - Audio playback
- **cpal** (v0.15) - Audio device abstraction
- **lewton** (v0.10) - OGG audio decoding
- **hound** (v3.4) - WAV audio decoding

#### Networking
- **tokio** - Async networking
- **rose-network-irose** - iRose protocol implementation

#### Scripting
- **lua4** - Custom Lua 4 interpreter integration

#### Utilities
- **anyhow** (v1.0) - Error handling
- **serde** (v1.0) - Serialization
- **toml** (v0.7) - Configuration parsing
- **regex** (v1) - Pattern matching
- **md5** (v0.7) - Hashing
- **uuid** (v1) - UUID generation
- **dashmap** (v5.5) - Concurrent hash map
- **lru** (v0.12) - LRU cache

#### Data Structures
- **glam** (v0.27) - Vector math
- **enum-map** (v2.0) - Enum-based maps
- **arrayvec** (v0.7) - Stack-allocated vectors
- **bitflags** (v2.3) - Bit flag enums

### Custom Dependencies (Local)
- **rose-data** - Game data structures
- **rose-data-irose** - iRose-specific data
- **rose-file-readers** - File format parsers
- **rose-game-common** - Common game logic
- **rose-game-irose** - iRose-specific game logic
- **rose-network-common** - Common networking code
- **rose-network-irose** - iRose protocol

---

## Core Subsystems

### 1. Asset Loading System

**Purpose:** Load and manage game assets from various file formats and storage devices

**Key Files:**
- `src/vfs_asset_io.rs` - Virtual Filesystem Asset I/O
- `src/dds_image_loader.rs` - DDS texture loader
- `src/zms_asset_loader.rs` - ZMS mesh loader
- `src/model_loader.rs` - 3D model loader
- `src/zone_loader.rs` - Zone data loader
- `src/effect_loader.rs` - Effect asset loader
- `src/exe_resource_loader.rs` - EXE resource loader

**Features:**
- Multi-device virtual filesystem (VFS, AruaVFS, TitanVFS, iRosePH, host directory)
- Custom asset loaders for Rose Online formats (.zms, .zmo, .dds, .zone_loader)
- Async zone loading with memory tracking
- Asset caching and management through Bevy's asset system

**File Formats Supported:**
- **ZMS** - 3D mesh format
- **ZMO** - Animation format
- **DDS** - DirectDraw Surface textures
- **TGA** - Targa images
- **PNG** - Portable Network Graphics
- **JPEG** - JPEG images
- **LTB** - Language table files
- **STB** - Structured table files
- **ZSC** - Zone configuration files

### 2. Rendering System

**Purpose:** Handle all 3D rendering, materials, and visual effects

**Key Files:**
- `src/render/mod.rs` - Main render plugin
- `src/render/damage_digit_material.rs` - Damage number rendering
- `src/render/effect_mesh_material.rs` - Effect mesh materials
- `src/render/particle_material.rs` - Particle system materials
- `src/render/object_material_simple.rs` - Simple object materials
- `src/render/sky_material.rs` - Skybox rendering
- `src/render/terrain_material.rs` - Terrain rendering
- `src/render/water_material.rs` - Water rendering
- `src/render/trail_effect.rs` - Trail effect rendering
- `src/render/zone_lighting.rs` - Zone lighting system
- `src/render/world_ui.rs` - World-space UI
- `src/render/post_processing.rs` - Post-processing effects
- `src/render/shaders/` - WGSL shader files

**Features:**
- Custom material plugins for specialized rendering
- WGSL shaders for GPU rendering
- Post-processing effects (bloom, etc.)
- Zone-based lighting system
- Particle system with custom materials
- Damage digit floating text
- Trail effects for movement
- Skybox rendering
- Water and terrain rendering

### 3. UI System

**Purpose:** Implement all user interface elements and interactions

**Key Files:**
- `src/ui/mod.rs` - Main UI module
- `src/ui/ui_login_system.rs` - Login screen
- `src/ui/ui_character_select_system.rs` - Character selection
- `src/ui/ui_inventory_system.rs` - Inventory management
- `src/ui/ui_hotbar_system.rs` - Hotbar UI
- `src/ui/ui_skill_list_system.rs` - Skill list
- `src/ui/ui_chatbox_system.rs` - Chat interface
- `src/ui/ui_party_system.rs` - Party UI
- `src/ui/ui_bank_system.rs` - Bank interface
- `src/ui/ui_npc_store_system.rs` - NPC store
- `src/ui/ui_personal_store_system.rs` - Personal store
- `src/ui/ui_drag_and_drop_system.rs` - Drag and drop
- `src/ui/widgets/` - UI widget implementations

**Features:**
- Complete game UI recreation
- Drag-and-drop inventory system
- Dialog system for NPCs
- Character creation UI
- Party and clan interfaces
- Quest list and tracking
- Settings menu
- Debug UI windows
- Tooltip system
- Message boxes and input dialogs

### 4. Animation System

**Purpose:** Handle character and object animations

**Key Files:**
- `src/animation/mod.rs` - Animation plugin
- `src/animation/animation_state.rs` - Animation state management
- `src/animation/skeletal_animation.rs` - Skeletal mesh animation
- `src/animation/mesh_animation.rs` - Mesh deformation animation
- `src/animation/transform_animation.rs` - Transform-based animation
- `src/animation/camera_animation.rs` - Camera animation
- `src/animation/zmo_asset_loader.rs` - ZMO animation loader
- `src/animation/zmo_asset_loader_fixed.rs` - Fixed ZMO loader

**Features:**
- Skeletal animation system
- Mesh animation support
- Transform-based animations
- Camera animations
- Animation state machine
- Animation blending and transitions
- Animation events for triggering effects/sounds

### 5. Audio System

**Purpose:** Manage all audio playback and sound effects

**Key Files:**
- `src/audio/mod.rs` - Audio plugin (OddioPlugin)
- `src/audio/audio_source.rs` - Audio source management
- `src/audio/global_sound.rs` - Global/ambient sounds
- `src/audio/spatial_sound.rs` - 3D positional audio
- `src/audio/streaming_sound.rs` - Streaming audio for music
- `src/audio/ogg.rs` - OGG format support
- `src/audio/wav.rs` - WAV format support

**Features:**
- Spatial audio with 3D positioning
- Background music streaming
- Sound effect categories (combat, footstep, UI, etc.)
- Volume control per category
- Sound caching system
- Support for OGG and WAV formats

### 6. Networking System

**Purpose:** Handle network communication with game servers

**Key Files:**
- `src/protocol/mod.rs` - Protocol module
- `src/protocol/irose/mod.rs` - iRose protocol
- `src/protocol/irose/login_client.rs` - Login server client
- `src/protocol/irose/world_client.rs` - World server client
- `src/protocol/irose/game_client.rs` - Game server client
- `src/resources/network_thread.rs` - Network thread management
- `src/resources/game_connection.rs` - Game connection state
- `src/resources/world_connection.rs` - World connection state
- `src/resources/login_connection.rs` - Login connection state

**Features:**
- Multi-server connection (login, world, game)
- Async network I/O with Tokio
- Protocol implementation for iRose
- Network thread for non-blocking I/O
- Connection state management
- Packet serialization/deserialization

### 7. Scripting System

**Purpose:** Execute game scripts and quest logic

**Key Files:**
- `src/scripting/mod.rs` - Scripting plugin
- `src/scripting/quest.rs` - Quest system
- `src/scripting/script_function_context.rs` - Script execution context
- `src/scripting/script_function_resources.rs` - Script resources
- `src/scripting/lua_game_constants.rs` - Lua game constants
- `src/scripting/lua_game_functions.rs` - Lua game functions
- `src/scripting/lua_quest_functions.rs` - Lua quest functions
- `src/scripting/quest_condition_functions.rs` - Quest conditions
- `src/scripting/quest_reward_functions.rs` - Quest rewards
- `src/scripting/quest_function_context.rs` - Quest context
- `src/scripting/lua4/` - Lua 4 interpreter integration

**Features:**
- Lua 4 scripting engine integration
- Quest system with conditions and rewards
- Game function bindings for scripts
- Quest trigger system
- Script execution context management

### 8. Physics System

**Purpose:** Handle collision detection and physics simulation

**Key Files:**
- `src/components/collision.rs` - Collision components
- `src/systems/collision_system.rs` - Collision detection systems
- `src/systems/collision_player_system.rs` - Player collision
- `src/systems/collision_height_only_system.rs` - Height-based collision

**Features:**
- Rapier3D physics engine integration
- Height-only collision for terrain
- Player collision detection
- NPC and object collision
- Query pipeline for spatial queries

---

## Key Components

### Entity Components

The project uses Bevy's ECS with many custom components:

**Character Components:**
- `CharacterModel` - Character 3D model
- `CharacterModelBlinkTimer` - Blink animation timer
- `PlayerCharacter` - Player-specific data
- `NpcModel` - NPC model data
- `VehicleModel` - Vehicle model data
- `Vehicle` - Vehicle state
- `VehicleSound` - Vehicle audio

**Gameplay Components:**
- `Position` - Entity position
- `FacingDirection` - Entity orientation
- `Command` - Movement/action commands
- `Collision` - Collision shape
- `Cooldowns` - Ability cooldowns
- `AbilityValues` - Character stats
- `DamageDigits` - Floating damage numbers
- `Dead` - Death state
- `Effect` - Visual effects
- `Projectile` - Projectiles

**UI Components:**
- `NameTagEntity` - Name tag display
- `ClientEntityName` - Entity name
- `PartyInfo` - Party information
- `ClanMembership` - Clan data
- `PersonalStore` - Store state
- `Bank` - Bank data

**System Components:**
- `Zone` - Zone data
- `ZoneObject` - Zone object
- `EventObject` - Event object
- `WarpObject` - Warp point
- `ItemDropModel` - Dropped item
- `VisibleStatusEffects` - Status effect display
- `NightTimeEffect` - Night-time effects

### Resources

Global resources managed by Bevy:

**Game Data:**
- `GameData` - All game databases (items, NPCs, skills, quests, etc.)
- `CurrentZone` - Currently loaded zone
- `ClientEntityList` - List of client entities
- `WorldTime` - Global game time
- `ZoneTime` - Zone-specific time

**Configuration:**
- `RenderConfiguration` - Rendering settings
- `SoundSettings` - Audio settings
- `ServerConfiguration` - Server connection settings
- `NameTagSettings` - Name tag display settings

**Network:**
- `NetworkThread` - Network thread handle
- `GameConnection` - Game server connection
- `WorldConnection` - World server connection
- `LoginConnection` - Login server connection

**Assets:**
- `ModelLoader` - Model loading system
- `SoundCache` - Sound effect cache
- `SpecularTexture` - Specular map texture
- `DamageDigitsSpawner` - Damage number spawner

**UI:**
- `UiResources` - UI asset resources
- `SelectedTarget` - Currently selected target
- `DebugRenderConfig` - Debug rendering settings
- `DebugInspector` - Entity inspector state

**Diagnostics:**
- `RenderExtractionDiagnostics` - Render world extraction tracking
- `ZoneDebugDiagnostics` - Zone loading diagnostics

### Events

Bevy events for game communication:

**Network Events:**
- `NetworkEvent` - Generic network event
- `GameConnectionEvent` - Game server events
- `WorldConnectionEvent` - World server events
- `LoginEvent` - Login server events

**Game Events:**
- `ClientEntityEvent` - Entity spawn/despawn
- `HitEvent` - Combat hits
- `SpawnEffectEvent` - Visual effects
- `SpawnProjectileEvent` - Projectile spawning
- `ZoneEvent` - Zone changes

**UI Events:**
- `MessageBoxEvent` - Message boxes
- `NumberInputDialogEvent` - Number input
- `ChatboxEvent` - Chat messages
- `PartyEvent` - Party events
- `ClanDialogEvent` - Clan dialogs
- `ConversationDialogEvent` - NPC conversations

**System Events:**
- `BankEvent` - Bank operations
- `NpcStoreEvent` - Store operations
- `PersonalStoreEvent` - Personal store
- `UseItemEvent` - Item usage
- `PlayerCommandEvent` - Player commands
- `QuestTriggerEvent` - Quest triggers
- `SystemFuncEvent` - System function calls

---

## Project Structure

### Directory Organization

```
rose-offline-client/
├── src/
│   ├── main.rs                    # Application entry point
│   ├── lib.rs                     # Library root, app initialization
│   │
│   ├── animation/                 # Animation system
│   │   ├── animation_state.rs
│   │   ├── skeletal_animation.rs
│   │   ├── mesh_animation.rs
│   │   ├── transform_animation.rs
│   │   ├── camera_animation.rs
│   │   ├── zmo_asset_loader.rs
│   │   └── zmo_asset_loader_fixed.rs
│   │
│   ├── audio/                     # Audio system
│   │   ├── audio_source.rs
│   │   ├── global_sound.rs
│   │   ├── spatial_sound.rs
│   │   ├── streaming_sound.rs
│   │   ├── ogg.rs
│   │   └── wav.rs
│   │
│   ├── bundles/                   # Entity bundles
│   │   ├── ability_values.rs
│   │   └── mod.rs
│   │
│   ├── components/                # ECS components
│   │   ├── character_model.rs
│   │   ├── player_character.rs
│   │   ├── npc_model.rs
│   │   ├── vehicle_model.rs
│   │   ├── position.rs
│   │   ├── collision.rs
│   │   ├── command.rs
│   │   ├── effect.rs
│   │   ├── projectile.rs
│   │   ├── zone.rs
│   │   └── ... (many more)
│   │
│   ├── debug/                    # Debug tools
│   │   ├── mod.rs
│   │   └── renderdoc.rs
│   │
│   ├── events/                   # Bevy events
│   │   ├── login_event.rs
│   │   ├── network_event.rs
│   │   ├── client_entity_event.rs
│   │   ├── hit_event.rs
│   │   ├── spawn_effect_event.rs
│   │   └── ... (many more)
│   │
│   ├── protocol/                  # Network protocols
│   │   ├── mod.rs
│   │   └── irose/
│   │       ├── mod.rs
│   │       ├── login_client.rs
│   │       ├── world_client.rs
│   │       └── game_client.rs
│   │
│   ├── render/                   # Rendering system
│   │   ├── mod.rs
│   │   ├── damage_digit_material.rs
│   │   ├── effect_mesh_material.rs
│   │   ├── particle_material.rs
│   │   ├── sky_material.rs
│   │   ├── terrain_material.rs
│   │   ├── water_material.rs
│   │   ├── trail_effect.rs
│   │   ├── zone_lighting.rs
│   │   ├── world_ui.rs
│   │   ├── post_processing.rs
│   │   └── shaders/              # WGSL shaders
│   │       ├── damage_digit.wgsl
│   │       ├── particle.wgsl
│   │       ├── sky_material.wgsl
│   │       ├── terrain_material.wgsl
│   │       ├── water_material.wgsl
│   │       └── ... (more shaders)
│   │
│   ├── resources/                 # Bevy resources
│   │   ├── game_data.rs
│   │   ├── current_zone.rs
│   │   ├── client_entity_list.rs
│   │   ├── network_thread.rs
│   │   ├── game_connection.rs
│   │   ├── world_connection.rs
│   │   ├── login_connection.rs
│   │   ├── model_loader.rs
│   │   ├── sound_cache.rs
│   │   ├── ui_resources.rs
│   │   ├── virtual_filesystem.rs
│   │   └── ... (many more)
│   │
│   ├── scripting/                 # Scripting system
│   │   ├── mod.rs
│   │   ├── quest.rs
│   │   ├── script_function_context.rs
│   │   ├── lua_game_constants.rs
│   │   ├── lua_game_functions.rs
│   │   ├── lua_quest_functions.rs
│   │   ├── quest_condition_functions.rs
│   │   ├── quest_reward_functions.rs
│   │   └── lua4/                 # Lua 4 integration
│   │
│   ├── systems/                   # Game systems
│   │   ├── login_system.rs
│   │   ├── character_select_system.rs
│   │   ├── game_system.rs
│   │   ├── model_viewer_system.rs
│   │   ├── zone_viewer_system.rs
│   │   ├── collision_system.rs
│   │   ├── command_system.rs
│   │   ├── effect_system.rs
│   │   ├── projectile_system.rs
│   │   ├── name_tag_system.rs
│   │   ├── ui_*.rs              # UI systems
│   │   └── ... (many more)
│   │
│   ├── ui/                       # UI implementation
│   │   ├── mod.rs
│   │   ├── ui_login_system.rs
│   │   ├── ui_character_select_system.rs
│   │   ├── ui_inventory_system.rs
│   │   ├── ui_hotbar_system.rs
│   │   ├── ui_skill_list_system.rs
│   │   ├── ui_chatbox_system.rs
│   │   ├── ui_party_system.rs
│   │   ├── ui_bank_system.rs
│   │   ├── ui_npc_store_system.rs
│   │   ├── ui_personal_store_system.rs
│   │   ├── ui_drag_and_drop_system.rs
│   │   ├── widgets/              # UI widgets
│   │   │   ├── button.rs
│   │   │   ├── listbox.rs
│   │   │   ├── radio_button.rs
│   │   │   └── draw.rs
│   │   └── ... (many more UI systems)
│   │
│   ├── dds_image_loader.rs        # DDS texture loader
│   ├── zms_asset_loader.rs        # ZMS mesh loader
│   ├── zone_loader.rs             # Zone loading system
│   ├── effect_loader.rs           # Effect asset loader
│   ├── exe_resource_loader.rs     # EXE resource loader
│   ├── vfs_asset_io.rs            # VFS asset I/O
│   ├── model_loader.rs            # 3D model loader
│   ├── loader.rs                 # General loader utilities
│   └── types.rs                 # Common type definitions
│
├── fonts/                       # Font files
│   └── Ubuntu-M.ttf
│
├── docs/                        # Documentation
│   ├── diagnostic-summary.md
│   └── renderdoc-guide.md
│
├── plans/                       # Project plans
│   ├── rendering-extraction-failure-resolution.md
│   └── rendering-extraction-failure-resolution-updated.md
│
├── Cargo.toml                    # Project dependencies
├── Cargo.lock                   # Dependency lock file
├── README.md                     # Project documentation
├── bevy-0.13-to-0.14-migration-guide.md  # Migration guide
├── LICENSE                       # License file
├── .gitignore                    # Git ignore rules
├── .kilocodeignore             # Kilo Code ignore rules
└── filter_logs.py               # Log filtering utility
```

---

## Important File Locations

### Entry Points
- **`src/main.rs`** - Application entry point, CLI argument parsing, mode selection
- **`src/lib.rs`** - Library initialization, Bevy app setup, plugin registration

### Core Systems
- **`src/lib.rs`** (lines 438-1468) - Main `run_client()` function with complete app setup
- **`src/lib.rs`** (lines 1470-1587) - Game data loading (`load_game_data_irose`)
- **`src/lib.rs`** (lines 1589-1686) - Common game data loading (`load_common_game_data`)

### Asset Loading
- **`src/vfs_asset_io.rs`** - Virtual filesystem asset I/O implementation
- **`src/zms_asset_loader.rs`** - ZMS 3D mesh format loader
- **`src/dds_image_loader.rs`** - DDS texture format loader
- **`src/model_loader.rs`** - High-level model loading system
- **`src/zone_loader.rs`** - Zone data loading and entity spawning

### Rendering
- **`src/render/mod.rs`** - Render plugin registration
- **`src/render/shaders/`** - All WGSL shader files
- **`src/lib.rs`** (lines 771-790) - Material plugin registration

### UI
- **`src/ui/mod.rs`** - UI module exports
- **`src/ui/ui_login_system.rs`** - Login screen implementation
- **`src/ui/ui_character_select_system.rs`** - Character selection
- **`src/ui/widgets/`** - Reusable UI widgets

### Networking
- **`src/protocol/irose/`** - iRose protocol implementation
- **`src/resources/network_thread.rs`** - Network thread management
- **`src/resources/game_connection.rs`** - Game connection state

### Scripting
- **`src/scripting/mod.rs`** - Scripting plugin
- **`src/scripting/quest.rs`** - Quest system
- **`src/scripting/lua4/`** - Lua 4 interpreter

### Configuration
- **`Cargo.toml`** - Project dependencies and features
- **`src/main.rs`** (lines 159-401) - Configuration structures

### Diagnostics
- **`src/lib.rs`** (lines 1080-1230) - Various diagnostic systems
- **`docs/diagnostic-summary.md`** - Diagnostic interpretation guide

---

## Design Patterns and Decisions

### 1. Entity-Component-System (ECS) Architecture
The entire game is built on Bevy's ECS architecture:
- **Entities** - Unique identifiers for game objects
- **Components** - Data attached to entities (position, model, etc.)
- **Systems** - Logic that operates on entities with specific components

### 2. Plugin-Based Architecture
Each major subsystem is implemented as a Bevy plugin:
- `RoseAnimationPlugin` - Animation system
- `RoseRenderPlugin` - Rendering system
- `RoseScriptingPlugin` - Scripting system
- `OddioPlugin` - Audio system
- `DebugInspectorPlugin` - Debug tools

### 3. State Machine Pattern
Bevy's state system manages application states:
- `AppState::GameLogin` - Login screen
- `AppState::GameCharacterSelect` - Character selection
- `AppState::Game` - Main gameplay
- `AppState::ModelViewer` - Model viewer mode
- `AppState::ZoneViewer` - Zone viewer mode

### 4. Virtual Filesystem (VFS) Abstraction
Multiple storage devices abstracted through VFS:
- Standard VFS (data.idx)
- AruaVFS format
- TitanVFS format
- iRosePH format
- Host directory (for extracted files)

### 5. Asset Loading Pipeline
Custom asset loaders integrate with Bevy's asset system:
- Asynchronous loading
- Asset caching
- Hot-reloading support (through Bevy)
- Custom format support

### 6. Event-Driven Communication
Systems communicate through Bevy events:
- Decoupled system interaction
- Event queuing and processing
- Type-safe event handling

### 7. Resource-Based Global State
Global data stored in Bevy resources:
- Game databases
- Configuration
- Network connections
- Asset loaders

### 8. System Ordering
Explicit system ordering for determinism:
- System sets define execution order
- `.before()` and `.after()` for dependencies
- Chained systems for sequential processing

### 9. Material Plugin Pattern
Custom rendering materials implemented as plugins:
- `DamageDigitMaterialPlugin`
- `EffectMeshMaterialPlugin`
- `ParticleMaterialPlugin`
- `ObjectMaterialPlugin`
- `SkyMaterialPlugin`
- `WaterMaterialPlugin`
- `TerrainMaterialPlugin`
- `TrailEffectRenderPlugin`
- `ZoneLightingPlugin`

### 10. Diagnostic-First Development
Extensive diagnostic systems for debugging:
- Render world extraction tracking
- Main world mesh validation
- GPU upload verification
- Visibility state diagnostics
- Memory profiling

---

## Game Flow

### Application Startup

```
1. main.rs
   ↓ Parse CLI arguments
   ↓ Load configuration (config.toml or defaults)
   ↓
2. run_client() (lib.rs)
   ↓ Initialize Virtual Filesystem
   ↓ Create Bevy App
   ↓ Register VFS Asset Reader
   ↓
3. Register Plugins
   ↓ DefaultPlugins (Bevy core)
   ↓ Third-party plugins (egui, rapier, etc.)
   ↓ Rose plugins (animation, render, scripting, etc.)
   ↓
4. Register Asset Loaders
   ↓ DdsImageLoader
   ↓ ZmsAssetLoader
   ↓ ExeResourceLoader
   ↓ DialogLoader
   ↓ ZoneLoader
   ↓
5. Register Resources
   ↓ GameData, ModelLoader, SoundCache
   ↓ ServerConfiguration, SoundSettings
   ↓ Network thread
   ↓
6. Register Events
   ↓ All game events (network, UI, gameplay)
   ↓
7. Register Systems
   ↓ Organized into system sets
   ↓ Configured with proper ordering
   ↓
8. Load Game Data
   ↓ load_game_data_irose() - Load databases
   ↓ load_common_game_data() - Load shared resources
   ↓
9. Run App
   ↓ Bevy ECS loop
   ↓
10. Cleanup
    ↓ Shutdown network thread
```

### Game State Flow

```
GameLogin State
    ↓
    ├─ Login System (handle login)
    ├─ Login Event System (process login events)
    └─ UI Login System (render login UI)
    ↓ (on login success)
GameCharacterSelect State
    ↓
    ├─ Character Select System (handle selection)
    ├─ Character Models System (display characters)
    └─ UI Character Select System (render UI)
    ↓ (on character selected)
Game State
    ↓
    ├─ Game Systems (gameplay logic)
    ├─ UI Systems (game UI)
    ├─ Network Systems (server communication)
    ├─ Physics Systems (collision)
    ├─ Animation Systems (animations)
    ├─ Audio Systems (sound)
    └─ Rendering Systems (visuals)
```

### Zone Loading Flow

```
LoadZoneEvent
    ↓
Zone Loader System
    ↓ Load zone data from VFS
    ↓ Spawn terrain entities
    ↓ Spawn object entities
    ↓ Spawn NPC entities
    ↓
ZoneLoadedFromVfsEvent
    ↓
Game Zone Change System
    ↓ Update CurrentZone resource
    ↓ Spawn player character
    ↓ Initialize zone systems
    ↓
Force Zone Visibility System
    ↓ Update entity visibility
```

---

## Configuration

### Configuration File (config.toml)

The client can be configured via a TOML file or CLI arguments:

```toml
[account]
username = "your_username"
password = "your_password"

[auto_login]
enabled = false
server_id = 0
channel_id = 0
character_name = null

[filesystem]
devices = [
    { type = "vfs", path = "data.idx" },
    { type = "directory", path = "./extracted" }
]

[game]
data_version = "irose"
network_version = "irose"
ui_version = "irose"

[graphics]
mode = { type = "window", width = 1920, height = 1080 }
passthrough_terrain_textures = false
trail_effect_duration_multiplier = 1.0
disable_vsync = false

[server]
ip = "127.0.0.1"
port = 29000

[sound]
enabled = true
volume.global = 0.6
volume.background_music = 0.15
volume.player_footstep = 0.9
volume.player_combat = 1.0
volume.other_footstep = 0.5
volume.other_combat = 0.5
volume.npc_sounds = 0.6
volume.ui_sounds = 0.5
```

### Command-Line Arguments

**Data Options:**
- `--data-idx=<path>` - Path to iRose data.idx
- `--data-aruavfs-idx=<path>` - Path to AruaVFS data.idx
- `--data-titanvfs-idx=<path>` - Path to TitanVFS data.idx
- `--data-iroseph-idx=<path>` - Path to iRosePH data.idx
- `--data-path=<path>` - Path to extracted data directory

**Server Options:**
- `--ip=<address>` - Server IP (default: 127.0.0.1)
- `--port=<port>` - Server port (default: 29000)

**Auto-Login:**
- `--auto-login` - Enable automatic login
- `--username=<username>` - Username for auto-login
- `--password=<password>` - Password for auto-login
- `--server-id=<id>` - Server ID for auto-login
- `--channel-id=<id>` - Channel ID for auto-login
- `--character-name=<name>` - Character name for auto-login

**Viewer Modes:**
- `--model-viewer` - Start in model viewer mode
- `--zone-viewer` - Start in zone viewer mode
- `--zone=<id>` - Load specific zone in zone viewer

**Graphics:**
- `--disable-vsync` - Disable vertical sync
- `--passthrough-terrain-textures` - Pass through terrain textures to GPU

**Audio:**
- `--disable-sound` - Disable all audio

**Version Selection:**
- `--data-version=<version>` - Data version (irose)
- `--network-version=<version>` - Network version (irose)
- `--ui-version=<version>` - UI version (irose)

**Other:**
- `--config=<path>` - Path to config.toml

---

## Migration Notes

### Bevy 0.13 to 0.14 Migration

The project is currently migrating from Bevy 0.13 to 0.14. Key changes documented in:
- **`bevy-0.13-to-0.14-migration-guide.md`** - Detailed migration guide

Major changes include:
- API updates for camera components
- Changes to material plugins
- Visibility system updates
- Transform system changes
- Asset loading modifications

---

## Debugging and Diagnostics

### Diagnostic Systems

The project includes extensive diagnostic systems for debugging:

**Render Diagnostics:**
- `render_diagnostics_system_lightweight` - Lightweight render checks
- `diagnose_render_world_extraction` - Track Main→Render world extraction
- `diagnose_render_phase` - Check render queue population
- `diagnose_camera_entity_distances` - Verify camera-entity distances
- `verify_material_plugins` - Check material plugin extraction

**Mesh Diagnostics:**
- `diagnose_main_world_meshes` - Verify Main World mesh visibility
- `diagnose_mesh_materials` - Check mesh material assignments
- `diagnose_gpu_mesh_upload` - Verify GPU buffer uploads
- `diagnose_asset_loading` - Check asset loading status

**Visibility Diagnostics:**
- `visibility_state_diagnostics` - Track visibility states
- `zone_entity_visibility_diagnostics` - Zone entity visibility
- `parent_child_visibility_diagnostics` - Hierarchy visibility
- `active_camera_diagnostics` - Camera status checks

**Transform Diagnostics:**
- `transform_propagation_diagnostics` - Verify transform propagation
- `transform_validation_diagnostics` - Validate transform data

**Other Diagnostics:**
- `frustum_culling_diagnostics` - Frustum culling checks
- `material_transparency_diagnostics` - Material transparency
- `aabb_validation_diagnostics` - Bounding box validation
- `render_pipeline_diagnostics` - Render pipeline checks
- `render_stage_diagnostics` - Render stage verification

### Debug UI Windows

Multiple debug UI windows available:
- Debug menu
- Entity inspector
- Client entity list
- Command viewer
- Effect list
- Item list
- NPC list
- Skill list
- Zone list
- Zone lighting
- Zone time
- Physics debug
- Render debug
- Camera info

---

## Notable Technical Details

### Memory Management
- Async zone loading to prevent frame drops
- Memory tracking for zone loading
- LRU cache for sound effects
- Asset caching through Bevy's asset system

### Performance Optimizations
- Multi-threaded execution enabled
- Physics pipeline disabled (query only)
- Spatial audio with distance attenuation
- Frustum culling for visibility
- Asset preloading strategies

### Network Architecture
- Separate network thread for non-blocking I/O
- Async Tokio runtime
- Connection pooling for multiple servers
- Packet serialization/deserialization

### Rendering Pipeline
- Custom WGSL shaders
- Post-processing effects
- Zone-based lighting
- Material plugins for specialized rendering
- GPU buffer management

### Error Handling
- `anyhow` for error propagation
- Comprehensive logging
- Graceful degradation on asset load failures
- Diagnostic systems for issue identification

---

## External Dependencies

### Local Dependencies
The project depends on several local crates in the rose-offline workspace:
- `rose-data` - Data structures and databases
- `rose-data-irose` - iRose-specific data
- `rose-file-readers` - File format parsers
- `rose-game-common` - Common game logic
- `rose-game-irose` - iRose game logic
- `rose-network-common` - Common networking code
- `rose-network-irose` - iRose protocol

These crates provide:
- Game data structures (items, NPCs, skills, quests)
- File format readers (ZMS, ZMO, STB, LTB, etc.)
- Network protocol implementations
- Game logic and calculations

---

## Future Considerations

### Known Areas for Enhancement
1. **Bevy 0.14 Migration Completion** - Finish migrating all systems
2. **Performance Optimization** - Further optimize rendering and asset loading
3. **Additional Protocol Support** - Support for other Rose Online versions
4. **Enhanced Diagnostics** - More comprehensive debugging tools
5. **UI Improvements** - Enhanced UI responsiveness and features

### Development Guidelines
- Follow Bevy ECS patterns
- Use system sets for ordering
- Implement proper error handling
- Add diagnostic logging for new features
- Document complex systems

---

## Summary

The rose-offline-client is a sophisticated game client built with Rust and Bevy, featuring:
- Modern ECS architecture
- Comprehensive subsystems for all game aspects
- Custom asset loading for Rose Online formats
- Extensive diagnostic and debugging tools
- Multi-server support
- Full UI implementation
- Animation, audio, physics, and networking systems

The project demonstrates best practices in game development with Rust, including proper separation of concerns, plugin-based architecture, event-driven communication, and comprehensive testing through diagnostics.

---

*Document Version: 1.0*  
*Last Updated: Based on project analysis*  
*Project Location: c:/Users/vicha/RustroverProjects/rose-offline-client*
