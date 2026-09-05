# System Architecture Documentation

## Project Overview
`rose-offline-client` is a high-performance offline client built using the **Bevy 0.18.1** game engine. This documentation provides a deep dive into the architectural patterns, module responsibilities, and technical implementations within the project.

## Documentation Index
Explore the specific subsystems of the client using the links below.

### Engine foundations (Bevy 0.18.1 patterns used by this client)

* [Animation.md](Animation.md) - Skeletal and procedural animation systems.
* [Assets.md](Assets.md) - Asset loading, management, and lifecycle.
* [Audio.md](Audio.md) - Sound effects and music playback systems.
* [Camera.md](Camera.md) - Camera controls, projections, and view management.
* [ECS.md](ECS.md) - Entity Component System patterns and data structures.
* [Input.md](Input.md) - Input handling for keyboard, mouse, and controllers.
* [Lighting.md](Lighting.md) - Lighting models, shadow mapping, and environment lighting.
* [Physics.md](Physics.md) - Collision detection and physics simulation.
* [Render.md](Render.md) - Rendering pipeline, shaders, and GPU interaction.
* [Transform.md](Transform.md) - Spatial hierarchies and coordinate transformations.
* [UI.md](UI.md) - User interface components and layout management.
* [Window.md](Window.md) - Window management and OS integration.

### Game-specific systems (ROSE gameplay, not generic Bevy)

* [zone-pipeline.md](zone-pipeline.md) - Zone loading, spawning, and `GameStages::ZoneChange` ordering.
* [networking.md](networking.md) - Login/world/game connections, network thread, client messages.
* [combat-effects.md](combat-effects.md) - Hit, pending damage/skill, projectiles, damage digits, effects.
* [graphics-settings.md](graphics-settings.md) - `GraphicsSettings` + apply systems (shadow/SMAA/MSAA/tonemap/Bloom/SSAO/DoF/motion-blur).
* [sailing-system.md](sailing-system.md) - Boats, buoyancy, wakes, sailing movement/camera/HUD.
* [scripting-quests.md](scripting-quests.md) - Lua scripting, quest triggers/rewards, quest UI.
* [model-spawning.md](model-spawning.md) - Character/NPC/item-drop/personal-store models and colliders.
* [asset-loaders.md](asset-loaders.md) - VFS, ZMS/ZMO, DDS, EXE resources, dialogs, effect files.
* [blood-effect-system.md](blood-effect-system.md) - Terrain blood decals and UV-space wound overlays.
* [chat-bubble-and-name-tag-architecture.md](chat-bubble-and-name-tag-architecture.md) - World-space UI rendering and occlusion.
* [flying-system-architecture.md](flying-system-architecture.md) - `/fly` flight state, movement, pose, wind effects.
* [map-editor-architecture.md](map-editor-architecture.md) - Map editor modes, picking, gizmos, IFO save.
* [monster-collision-system.md](monster-collision-system.md) - Hostile-monster soft separation.
* [planar-water-reflection.md](planar-water-reflection.md) - Mirrored-camera planar water reflections.
* [weather-season-system.md](weather-season-system.md) - Seasons and weather particles.
* [zone_lighting.md](zone_lighting.md) - ZoneLighting resource, sun sync, terrain lighting. See also [Lighting.md](Lighting.md) for Bevy light types, [SUN_DOCUMENTATION.md](SUN_DOCUMENTATION.md) for sun movement detail, [sky_stars_architecture.md](sky_stars_architecture.md) for night sky.
* [admin-menu-skill-learn-feature.md](admin-menu-skill-learn-feature.md) - Admin menu skill-learn popup.

## Architecture Overview
The system follows a highly modular, data-driven approach powered by Bevy's ECS. The primary data flow generally follows this pattern:

**Network + Assets** $\rightarrow$ **Zone Loading / Spawning** $\rightarrow$ **ECS Gameplay (Command, Combat, Quests, Scripting, Sailing)** $\rightarrow$ **Transform/Physics** $\rightarrow$ **Render/Camera/Lighting** $\rightarrow$ **Window**

1. **Network & Assets**: Server messages (`src/protocol/`, `*_connection_system.rs`) and VFS game data (`data.idx`, ZMS/ZMO/DDS/EXE/Dialog loaders) feed entities and resources.
2. **Zone Loading & Spawning**: `LoadZoneEvent` drives async zone load and object/terrain/water spawning, gated by `GameStages::ZoneChange`. See [zone-pipeline.md](zone-pipeline.md).
3. **ECS Gameplay Core**: Command/movement, combat/effects, quests/scripts, sailing, stores operate on components and communicate via `MessageReader`/`MessageWriter`. See [ECS.md](ECS.md), [combat-effects.md](combat-effects.md), [scripting-quests.md](scripting-quests.md), [sailing-system.md](sailing-system.md).
4. **Visual Pipeline**: Transforms, Rapier scene queries, `ZoneLighting`, and graphics-settings apply systems feed the deferred PBR pipeline. See [Render.md](Render.md), [Lighting.md](Lighting.md), [graphics-settings.md](graphics-settings.md).
5. **Output**: The final frame (game view + egui + world-space UI) is presented through the windowing system.

## Key Technical Details
* **Bevy Version**: `0.18.1`
* **Bevy Source Location**: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\`

## Quick Reference Guide
* **State Management**: Uses Bevy's built-in `State<T>` for managing high-level application flows (e.g., Menu, Playing, Paused).
* **Event-Driven Architecture**: Systems communicate via `MessageReader` and `MessageWriter` to decouple logic.
* **Component-Based Design**: Logic is encapsulated in Systems that query specific Component sets.

## How to Use
This documentation is intended for developers to:
* **Troubleshoot**: When a specific subsystem fails, refer to its corresponding documentation to understand the expected behavior.
* **Implement Features**: Use the architectural patterns outlined here to ensure new code remains consistent with the existing codebase.
* **Understand Dependencies**: Use the Index to trace how different modules interact within the engine.