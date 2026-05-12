# Sailing Zone Features - Bevy 0.18.1 Compile Issues

Scope: Bevy/ECS/API issues in the `sailing-zone-features` branch.

Validated against Bevy 0.18.1 source:
- `bevy_app/src/app.rs:84` defines `App`.
- `bevy_app/src/plugin.rs:57` defines `Plugin`.
- `bevy_app/src/main_schedule.rs:173` defines `Update`.
- `bevy_app/src/lib.rs:64-66` re-exports schedules including `Update`.
- `bevy_ecs/src/system/commands/mod.rs:104` defines `Commands<'w, 's>`.
- `bevy_ecs/src/system/commands/mod.rs:127` implements `SystemParam` for `Commands<'_, '_>`, not for `&mut Commands`.
- `bevy_ecs/src/world/mod.rs:1487-1507` and `world/entity_access/world_mut.rs:1755-1758` document that Bevy 0.18 `despawn()` recursively despawns children configured through hierarchy relationships.

## Current Errors

### 1. Bevy prelude is not imported in the plugin module

File: `src/sailing/mod.rs:28-36`

Problem: `Plugin`, `App`, and `Update` are used without importing Bevy prelude items.

Fix:

```rust
use bevy::prelude::*;
```

or import only `App`, `Plugin`, and `Update`.

### 2. Private module paths used for system ordering

Files:
- `src/sailing/mod.rs:40`
- `src/sailing/mod.rs:62`

Problem: `crate::systems::boat_spawn_system` and `crate::resources::wind_state` are private modules. Also, `wind_update_system` is a system re-exported from `src/systems/mod.rs`, not a function in `resources::wind_state`.

Fix: Import the public re-exports from `crate::systems`, for example:

```rust
use crate::systems::{
    ensure_boat_state_system,
    sailing_movement_system,
    wind_update_system,
};
```

Then use `.after(ensure_boat_state_system)`, `.after(sailing_movement_system)`, and `.after(wind_update_system)`.

### 3. Missing `Position` import in storm systems

File: `src/sailing/mod_storms.rs:162`

Problem: `Position` is used in a Bevy `Query`, but only `BoatState` is imported from `crate::components`.

Fix:

```rust
use crate::components::{BoatState, Position};
```

## Masked Bevy API Errors

### 4. Systems use `&mut Commands` instead of `Commands`

Files:
- `src/sailing/mod_pirates.rs:135`
- `src/sailing/mod_pirates.rs:318`
- `src/sailing/mod_sea_creatures.rs:157`
- `src/sailing/mod_sea_creatures.rs:369`
- `src/sailing/mod_cannons.rs:72`
- `src/sailing/mod_cannons.rs:153`
- `src/sailing/mod_treasure.rs:104`
- `src/sailing/mod_treasure.rs:136`
- `src/sailing/mod_storms.rs:142`

Problem: Bevy systems must take `Commands` by value as a system param. `&mut Commands` is valid for helper functions you call yourself, but not for functions passed to `add_systems`.

Fix:

```rust
pub fn some_system(mut commands: Commands, ...)
```

Keep `&mut Commands` only in non-system helper functions like visual spawn helpers.

### 5. Immutable `Res<StormState>` is mutated

File: `src/sailing/mod_storms.rs:141-151`

Problem: `storm_visual_system` takes `storm: Res<StormState>` but assigns `storm.lightning_active = false`.

Fix: Use `ResMut<StormState>` if the system owns that state mutation, or move visual-only mutation into a `StormVisual` component/resource.

### 6. `despawn_recursive()` is not the Bevy 0.18 API

Files:
- `src/sailing/mod_pirates.rs:328`
- `src/sailing/mod_sea_creatures.rs:375`

Problem: Bevy 0.18 uses `EntityCommands::despawn()`. Recursive child despawn behavior is now handled by hierarchy relationships configured to despawn descendants.

Fix:

```rust
commands.entity(entity).despawn();
```

### 7. Nested pirate plugin registers non-reflect types

Files:
- `src/sailing/pirates/plugin.rs:20-21`
- `src/sailing/pirates/components.rs:7`
- `src/sailing/pirates/components.rs:117`

Problem: `App::register_type<T>()` requires `T: GetTypeRegistration`, normally provided by deriving `Reflect`. The nested pirate `PirateShip` and `PirateCannonProjectile` derive `Component`, `Debug`, and `Clone`, but not `Reflect`. This is latent because the nested `pirates` module is not currently compiled.

Fix: Add `Reflect` and the component reflection attribute, or remove the `register_type` calls:

```rust
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
```

