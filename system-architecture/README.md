# System Architecture Documentation

## Project Overview
`rose-offline-client` is a high-performance offline client built using the **Bevvy 0.18.1** game engine. This documentation provides a deep dive into the architectural patterns, module responsibilities, and technical implementations within the project.

## Documentation Index
Explore the specific subsystems of the client using the links below:

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

## Architecture Overview
The system follows a highly modular, data-driven approach powered by Bevy's ECS. The primary data flow generally follows this pattern:

**Assets** $\rightarrow$ **ECS (Entities/Components)** $\rightarrow$ **Transform/Physics** $\rightarrow$ **Render/Camera** $\rightarrow$ **Window**

1. **Assets & Resources**: Data is loaded into the engine and managed as centralized resources.
2. **ECS Core**: Systems operate on components to drive game logic, physics, and state transitions.
3. **Visual Pipeline**: Transforms and lighting data are processed to feed the rendering pipeline.
4. **Output**: The final frame is presented through the windowing system.

## Key Technical Details
* **Bevvy Version**: `0.18.1`
* **Bevvy Source Location**: `C:\Users\vicha\RustroverProjects\bevvy-collection\bevvy-0.18.1\crates\`

## Quick Reference Guide
* **State Management**: Uses Bevy's built-in `State<T>` for managing high-level application flows (e.g., Menu, Playing, Paused).
* **Event-Driven Architecture**: Systems communicate via `EventReader` and `EventWriter` to decouple logic.
* **Component-Based Design**: Logic is encapsulated in Systems that query specific Component sets.

## How to Use
This documentation is intended for developers to:
* **Troubleshoot**: When a specific subsystem fails, refer to its corresponding documentation to understand the expected behavior.
* **Implement Features**: Use the architectural patterns outlined here to ensure new code remains consistent with the existing codebase.
* **Understand Dependencies**: Use the Index to trace how different modules interact within the engine.