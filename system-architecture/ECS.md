# Bevy ECS Documentation for rose-offline-client

This document provides comprehensive documentation of Bevy ECS features used in the rose-offline-client project, with examples from the codebase and references to Bevy 0.18.1 source files.

**Bevy Version**: 0.18.1  
**Bevy Source**: `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\`

---

## Overview

This project uses Bevy ECS (Entity Component System) as its core architecture. The ECS pattern separates data (Components) from logic (Systems), with Entities serving as handles that group components together.

### Key Concepts

- **Entities**: Lightweight handles that group components
- **Components**: Plain data types attached to entities
- **Resources**: Singleton data stored in the World
- **Systems**: Functions that operate on entities and resources
- **Events**: Messages that trigger system responses
- **State**: Application-wide finite state machines

---

## Table of Contents

1. [Bevy API References](#bevy-api-references)
2. [Components](#components)
3. [Resources](#resources)
4. [Systems](#systems)
5. [SystemSets](#systemsets)
6. [Events](#events)
7. [State](#state)
8. [Queries](#queries)
9. [Commands](#commands)
10. [Hierarchy](#hierarchy)
11. [Custom Extensions](#custom-extensions)
12. [Code Examples](#code-examples)
13. [Configuration Options](#configuration-options)
14. [Common Patterns](#common-patterns)
15. [Troubleshooting](#troubleshooting)
16. [Source File References](#source-file-references)

---

## Bevy API References

### Core ECS Files

| Feature | Source File |
|---------|-------------|
| Components | `bevy_ecs/src/component/mod.rs` |
| Resources | `bevy_ecs/src/resource.rs` |
| Systems | `bevy_ecs/src/system/mod.rs` |
| Function Systems | `bevy_ecs/src/system/function_system.rs` |
| Queries | `bevy_ecs/src/system/query.rs` |
| Query System | `bevy_ecs/src/query/mod.rs` |
| Commands | `bevy_ecs/src/system/commands/mod.rs` |
| Events | `bevy_ecs/src/event/mod.rs` |
| Hierarchy | `bevy_ecs/src/hierarchy.rs` |
| Schedule | `bevy_ecs/src/schedule/mod.rs` |
| SystemSet | `bevy_ecs/src/schedule/set.rs` |
| State | `bevy_state/src/lib.rs` |
| States Derive | `bevy_state/src/state/mod.rs` |

### Additional References

| Feature | Source File |
|---------|-------------|
| Archetypes | `bevy_ecs/src/archetype.rs` |
| Entity | `bevy_ecs/src/entity/mod.rs` |
| World | `bevy_ecs/src/world/mod.rs` |
| Spawn | `bevy_ecs/src/spawn.rs` |
| Relationship | `bevy_ecs/src/relationship/mod.rs` |
| Observer | `bevy_ecs/src/observer/mod.rs` |
| Message | `bevy_ecs/src/message/mod.rs` |

---

## Components

Components are data types that store information about entities. They are derived using `#[derive(Component)]` and must satisfy `Send + Sync + 'static` trait bounds.

**Bevy Source**: `bevy_ecs/src/component/mod.rs`

### Basic Component Definition

```rust
// src/components/client_entity.rs
#[derive(Copy, Clone, Component, Reflect)]
pub struct ClientEntity {
    pub id: ClientEntityId,
    pub entity_type: ClientEntityType,
}
```

### Enum Components

```rust
// src/components/command.rs
#[derive(Component, Clone, Debug, PartialEq, Reflect)]
pub enum Command {
    Stop,
    Move(CommandMove),
    Attack(CommandAttack),
    Die,
    PersonalStore,
    PickupItem(Entity),
    Emote(CommandEmote),
    Sit(CommandSit),
    CastSkill(CommandCastSkill),
}
```

### Marker Components

```rust
// src/components/collision.rs
#[derive(Component)]
pub struct CollisionPlayer;

#[derive(Component)]
pub struct CollisionHeightOnly;
```

### Components with Helper Methods

```rust
// src/components/collision.rs
#[derive(Component, Reflect)]
pub struct ColliderEntity {
    pub entity: Entity,
}

impl ColliderEntity {
    pub fn new(entity: Entity) -> Self {
        Self { entity }
    }
}
```

### Component Patterns in This Project

1. **Data Components**: Store entity-specific data (e.g., `ClientEntity`, `Position`, `Command`)
2. **Marker Components**: Used for query filtering (e.g., `CollisionPlayer`, `Dead`, `PlayerCharacter`)
3. **Entity Reference Components**: Store references to other entities (e.g., `ColliderEntity`, `ColliderParent`)
4. **Enum Components**: Represent state machines or variant data (e.g., `Command`)

---

## Resources

Resources are unique, singleton-like data types stored in the World. Only one resource of each type can exist at a time. Access via `Res` (immutable) or `ResMut` (mutable).

**Bevy Source**: `bevy_ecs/src/resource.rs`

### Basic Resource Definition

```rust
// src/resources/app_state.rs
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    GameLogin,
    GameCharacterSelect,
    Game,
    ModelViewer,
    ZoneViewer,
    MapEditor,
}
```

### Complex Resources

```rust
// src/resources/game_data.rs
#[derive(Resource)]
pub struct GameData {
    pub ability_value_calculator: Box<dyn AbilityValueCalculator + Send + Sync>,
    pub animation_event_flags: Vec<AnimationEventFlags>,
    pub character_motion_database: Arc<CharacterMotionDatabase>,
    pub client_strings: Arc<ClientStrings>,
    pub data_decoder: Box<dyn DataDecoder + Send + Sync>,
    pub effect_database: Arc<EffectDatabase>,
    pub items: Arc<ItemDatabase>,
    pub job_class: Arc<JobClassDatabase>,
    pub npcs: Arc<NpcDatabase>,
    pub quests: Arc<QuestDatabase>,
    pub skills: Arc<SkillDatabase>,
    pub skybox: Arc<SkyboxDatabase>,
    pub sounds: Arc<SoundDatabase>,
    pub status_effects: Arc<StatusEffectDatabase>,
    pub string_database: Arc<StringDatabase>,
    pub zone_list: Arc<ZoneList>,
    pub character_select_positions: Vec<Transform>,
}
```

### Resource Initialization

```rust
// src/lib.rs
app.insert_resource(RenderConfiguration {
    passthrough_terrain_textures: config.graphics.passthrough_terrain_textures,
    trail_effect_duration_multiplier: config.graphics.trail_effect_duration_multiplier,
    use_new_terrain: config.graphics.use_new_terrain,
})
.init_resource::<ClientEntityList>()
.init_resource::<WorldTime>()
.init_resource::<ZoneTime>();
```

### Resource Patterns in This Project

1. **Configuration Resources**: Store settings (e.g., `RenderConfiguration`, `SoundSettings`)
2. **Game State Resources**: Track runtime state (e.g., `CurrentZone`, `SelectedTarget`)
3. **Data Resources**: Hold loaded game data (e.g., `GameData`, `SoundCache`)
4. **Connection Resources**: Manage network state (e.g., `GameConnection`, `LoginConnection`)

---

## Systems

Systems are functions that operate on the ECS world. They receive system parameters like `Query`, `Res`, `ResMut`, `Commands`, etc.

**Bevy Source**: `bevy_ecs/src/system/mod.rs`, `bevy_ecs/src/system/function_system.rs`

### System Parameters

| Parameter | Description | Mutability |
|-----------|-------------|------------|
| `Query<T>` | Access components on entities | Varies by T |
| `Res<T>` | Immutable resource access | Immutable |
| `ResMut<T>` | Mutable resource access | Mutable |
| `Commands` | Deferred world modifications | N/A |
| `Local<T>` | System-local storage | Mutable |
| `MessageWriter<T>` | Write messages | N/A |
| `Option<Res<T>>` | Optional resource | Immutable |

### Basic System Example

```rust
// src/systems/update_position_system.rs
pub fn update_position_system(
    mut query: Query<(&Command, &MoveSpeed, &mut FacingDirection, &mut Position)>,
    time: Res<Time>,
) {
    for (command, move_speed, mut facing_direction, mut position) in query.iter_mut() {
        let Command::Move(CommandMove { destination, .. }) = *command else {
            continue;
        };

        let direction = destination.xy() - position.xy();
        let distance_squared = direction.length_squared();

        if distance_squared == 0.0 {
            position.position = destination;
        } else {
            // Update rotation
            facing_direction.set_desired_vector(destination - position.position);

            // Move to position
            let move_vector = direction.normalize() * move_speed.speed * time.delta_secs();
            if move_vector.length_squared() >= distance_squared {
                position.position = destination;
            } else {
                position.x += move_vector.x;
                position.y += move_vector.y;
            }
        }
    }
}
```

### System with Multiple Parameters

```rust
// src/systems/collision_system.rs
pub fn collision_height_only_system(
    mut query_collision_entity: Query<
        (Entity, &mut Position, &mut Transform),
        With<CollisionHeightOnly>,
    >,
    rapier_context: ReadRapierContext,
    current_zone: Option<Res<CurrentZone>>,
    zone_loader_assets: Res<Assets<ZoneLoaderAsset>>,
    time: Res<Time>,
) {
    // System implementation
}
```

### System with Commands

```rust
// src/systems/command_system.rs
pub fn command_system(
    mut commands: Commands,
    mut query: Query<
        (
            Entity,
            Option<&PlayerCharacter>,
            &AbilityValues,
            Option<&CharacterModel>,
            Option<&NpcModel>,
            Option<&Equipment>,
            &Position,
            &MoveMode,
            &MoveSpeed,
            Option<&Vehicle>,
            &mut Command,
            &mut NextCommand,
            &mut FacingDirection,
            Option<&Dead>,
        ),
        Or<(With<CharacterModel>, With<NpcModel>)>,
    >,
    // ... additional queries and resources
    mut client_entity_events: MessageWriter<ClientEntityEvent>,
    game_connection: Option<Res<GameConnection>>,
) {
    for (entity, player_character, ability_values, character_model, npc_model, equipment, position, move_mode, move_speed, vehicle, command, mut next_command, mut facing_direction, dead) in query.iter_mut() {
        match command {
            Command::Die => {
                commands.entity(entity).insert(Dead);
            }
            Command::Attack(cmd_attack) => {
                // Handle attack command
            }
            _ => {}
        }
    }
}
```

### System Ordering

```rust
// src/lib.rs
app.add_systems(
    Update,
    (
        zone_loader_system,
        zone_loaded_from_vfs_system.after(zone_loader_system),
    )
);

app.add_systems(
    Update,
    game_zone_change_system
        .after(zone_loader_system)
        .after(zone_loaded_from_vfs_system),
);
```

### System Patterns in This Project

1. **Update Systems**: Run every frame to update entity state
2. **Event Handler Systems**: Process events and trigger actions
3. **Input Systems**: Handle player input
4. **Cleanup Systems**: Remove obsolete entities/components
5. **Sync Systems**: Synchronize client with server state

---

## SystemSets

SystemSets allow grouping systems and defining execution order between groups.

**Bevy Source**: `bevy_ecs/src/schedule/mod.rs`

### Custom SystemSet Definition

```rust
// src/lib.rs
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum GameSystemSets {
    UpdateCamera,
    Ui,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum ModelSystemSets {
    CharacterModelUpdate,
    CharacterModelAddCollider,
    PersonalStoreModel,
    PersonalStoreModelAddCollider,
    NpcModelUpdate,
    NpcModelAddCollider,
    ItemDropModel,
    ItemDropModelAddCollider,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum EffectSystemSets {
    AnimationEffect,
    Projectile,
    SpawnProjectile,
    PendingDamage,
    PendingSkillEffect,
    HitEvent,
    SpawnEffect,
}
```

### Configuring SystemSet Ordering

```rust
// src/lib.rs
// Configure system ordering for model systems
app.configure_sets(
    Update,
    (
        ModelSystemSets::CharacterModelUpdate,
        ModelSystemSets::CharacterModelAddCollider.after(ModelSystemSets::CharacterModelUpdate),
        ModelSystemSets::PersonalStoreModel.after(ModelSystemSets::CharacterModelAddCollider),
        ModelSystemSets::PersonalStoreModelAddCollider.after(ModelSystemSets::PersonalStoreModel),
        ModelSystemSets::NpcModelUpdate.after(ModelSystemSets::PersonalStoreModelAddCollider),
        ModelSystemSets::NpcModelAddCollider.after(ModelSystemSets::NpcModelUpdate),
        ModelSystemSets::ItemDropModel.after(ModelSystemSets::NpcModelAddCollider),
        ModelSystemSets::ItemDropModelAddCollider.after(ModelSystemSets::ItemDropModel),
    ),
);

// Configure system ordering for effect systems
app.configure_sets(
    Update,
    (
        EffectSystemSets::AnimationEffect,
        EffectSystemSets::Projectile.after(EffectSystemSets::AnimationEffect),
        EffectSystemSets::SpawnProjectile.after(EffectSystemSets::AnimationEffect),
        EffectSystemSets::PendingDamage
            .after(EffectSystemSets::AnimationEffect)
            .after(EffectSystemSets::Projectile),
        EffectSystemSets::PendingSkillEffect
            .after(EffectSystemSets::AnimationEffect)
            .after(EffectSystemSets::Projectile),
        EffectSystemSets::HitEvent
            .after(EffectSystemSets::AnimationEffect)
            .after(EffectSystemSets::PendingSkillEffect)
            .after(EffectSystemSets::Projectile),
        EffectSystemSets::SpawnEffect
            .after(EffectSystemSets::AnimationEffect)
            .after(EffectSystemSets::HitEvent),
    ),
);
```

### Adding Systems to Sets

```rust
// src/lib.rs
app.add_systems(
    Update,
    (free_camera_system, orbit_camera_system)
        .in_set(GameSystemSets::UpdateCamera)
        .after(bevy_egui::EguiPreUpdateSet::InitContexts),
);
```

### SystemSet Patterns in This Project

1. **GameStages**: High-level execution stages (ZoneChange, AfterUpdate, DebugRender)
2. **ModelSystemSets**: Order model update and collider addition systems
3. **EffectSystemSets**: Order combat effect systems
4. **UiSystemSets**: Order UI rendering systems
5. **State-gated groups**: Systems are grouped per state with `run_if(in_state(AppState::...))` (no `GameStateSystemSets` enum exists)

---

## Events

Events are messages that "happen" at a given moment. In Bevy 0.18, `Event`s are handled via `Observer`s, while the new `Message` API provides pull-based message passing. This project uses Bevy's built-in `Message` trait (derived with `#[derive(Message)]`) for its event-like patterns.

**Bevy Source**: `bevy_ecs/src/message/mod.rs` (Message), `bevy_ecs/src/event/mod.rs` (Event)

### Event Definition (Message Pattern)

```rust
// src/events/hit_event.rs
#[derive(Message)]
pub struct HitEvent {
    pub attacker: Entity,
    pub defender: Entity,
    pub effect_id: Option<EffectId>,
    pub skill_id: Option<SkillId>,
    pub blood_profile: BloodImpactProfile,
    pub apply_damage: bool,
    pub ignore_miss: bool,
}

impl HitEvent {
    pub fn with_weapon(attacker: Entity, defender: Entity, effect_id: Option<EffectId>) -> Self {
        Self {
            attacker,
            defender,
            effect_id,
            skill_id: None,
            blood_profile: BloodImpactProfile::Slash,
            apply_damage: true,
            ignore_miss: false,
        }
    }

    pub fn with_skill_damage(attacker: Entity, defender: Entity, skill_id: SkillId) -> Self {
        Self {
            attacker,
            defender,
            effect_id: None,
            skill_id: Some(skill_id),
            blood_profile: BloodImpactProfile::SkillMagic,
            apply_damage: true,
            ignore_miss: false,
        }
    }
}
```

### Event Registration

```rust
// src/lib.rs
app.add_message::<BankEvent>()
    .add_message::<ChatBubbleEvent>()
    .add_message::<HitEvent>()
    .add_message::<LoadZoneEvent>()
    .add_message::<PlayerCommandEvent>()
    .add_message::<ZoneEvent>();
```

### Sending Events

```rust
// src/systems/command_system.rs
pub fn command_system(
    // ...
    mut client_entity_events: MessageWriter<ClientEntityEvent>,
    // ...
) {
    // Send event (MessageWriter::write, not EventWriter::send)
    client_entity_events.write(ClientEntityEvent::Die(entity));
}
```

### Event Types in This Project

| Event | Purpose |
|-------|---------|
| `HitEvent` | Combat hit between entities |
| `ClientEntityEvent` | Entity state updates from server |
| `PlayerCommandEvent` | Player input commands |
| `ZoneEvent` | Zone loading/unloading |
| `SpawnEffectEvent` | Visual effect spawning |
| `ChatBubbleEvent` | Chat message display |
| `QuestTriggerEvent` | Quest progression |

---

## State

State manages application-wide finite state machines. Systems can run conditionally based on state or during state transitions.

**Bevy Source**: `bevy_state/src/lib.rs`, `bevy_state/src/state/mod.rs`

### State Definition

```rust
// src/resources/app_state.rs
#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash, States)]
pub enum AppState {
    #[default]
    GameLogin,
    GameCharacterSelect,
    Game,
    ModelViewer,
    ZoneViewer,
    MapEditor,
}
```

### State Initialization

```rust
// src/lib.rs
app.insert_state(app_state);
```

### State Transition Systems (OnEnter/OnExit)

```rust
// src/lib.rs
// Game Login
app.add_systems(OnEnter(AppState::GameLogin), login_state_enter_system)
    .add_systems(OnExit(AppState::GameLogin), login_state_exit_system);

// Game Character Select
app.add_systems(
    OnEnter(AppState::GameCharacterSelect),
    character_select_enter_system,
)
.add_systems(
    OnExit(AppState::GameCharacterSelect),
    character_select_exit_system,
);

// Game
app.add_systems(OnEnter(AppState::Game), game_state_enter_system);

// Zone Viewer
app.add_systems(OnEnter(AppState::ZoneViewer), zone_viewer_enter_system);

// Map Editor
app.add_systems(OnEnter(AppState::MapEditor), map_editor::map_editor_enter_system);
app.add_systems(OnExit(AppState::MapEditor), map_editor::map_editor_exit_system);
```

### Conditional System Execution (in_state)

```rust
// src/lib.rs
// Only run when in Game state
app.add_systems(
    Update,
    (
        move_speed_set_system,
        // ... other game systems
    )
        .run_if(in_state(AppState::Game)),
);
app.add_systems(
    Update,
    player_command_system.run_if(in_state(AppState::Game)),
);
app.add_systems(
    Update,
    ensure_boat_state_system.run_if(in_state(AppState::Game)),
);

// Model viewer system only runs in ModelViewer state
app.add_systems(
    PostUpdate,
    model_viewer_system.run_if(in_state(AppState::ModelViewer)),
);
```

### State Patterns in This Project

1. **AppState**: Main application state machine (Login → CharacterSelect → Game)
2. **State-scoped systems**: Systems that only run in specific states
3. **Enter/Exit systems**: Cleanup and initialization on state transitions
4. **Resource existence conditions**: `run_if(resource_exists::<CurrentZone>)`

---

## Queries

Queries provide selective access to component data on entities. They support filtering, ordering, and parallel iteration.

**Bevy Source**: `bevy_ecs/src/system/query.rs`, `bevy_ecs/src/query/mod.rs`

### Basic Query

```rust
// src/systems/update_position_system.rs
pub fn update_position_system(
    mut query: Query<(&Command, &MoveSpeed, &mut FacingDirection, &mut Position)>,
    time: Res<Time>,
) {
    for (command, move_speed, mut facing_direction, mut position) in query.iter_mut() {
        // Process entities with all four components
    }
}
```

### Query with Entity Access

```rust
// src/systems/collision_system.rs
pub fn collision_height_only_system(
    mut query_collision_entity: Query<
        (Entity, &mut Position, &mut Transform),
        With<CollisionHeightOnly>,
    >,
    // ...
) {
    for (entity, mut position, mut transform) in query_collision_entity.iter_mut() {
        // Access entity ID and modify components
    }
}
```

### Query with Filters

```rust
// With filter - entity must have this component
Query<&mut Position, With<CollisionPlayer>>

// Without filter - entity must NOT have this component
Query<&mut Position, Without<Dead>>

// Added filter - component was just added this frame
Query<&Position, Added<Position>>

// Removed filter - component was just removed this frame
Query<Entity, Removed<Position>>
```

### Complex Query with Multiple Filters

```rust
// src/systems/command_system.rs
pub fn command_system(
    mut query: Query<
        (
            Entity,
            Option<&PlayerCharacter>,
            &AbilityValues,
            Option<&CharacterModel>,
            Option<&NpcModel>,
            Option<&Equipment>,
            &Position,
            &MoveMode,
            &MoveSpeed,
            Option<&Vehicle>,
            &mut Command,
            &mut NextCommand,
            &mut FacingDirection,
            Option<&Dead>,
        ),
        Or<(With<CharacterModel>, With<NpcModel>)>,
    >,
    // ...
) {
    // Only queries entities with a CharacterModel OR NpcModel component
    // CharacterModel, NpcModel, Equipment, and Vehicle are optional
}
```

### Query with Or Filter

```rust
// Query entities that have EITHER CharacterModel OR NpcModel
// (Or is a query filter, not a query data item)
Query<&mut Transform, Or<(With<CharacterModel>, With<NpcModel>)>>
```

### Query Patterns in This Project

1. **Single Component Query**: `Query<&Component>`
2. **Multi-Component Query**: `Query<(&A, &mut B, &C)>`
3. **Filtered Query**: `Query<&Component, With<Filter>>`
4. **Optional Component Query**: `Query<(&Component, Option<&OptionalComponent>)>`
5. **Entity Access Query**: `Query<(Entity, &Component)>`

---

## Commands

Commands provide deferred, thread-safe world modifications. They are queued and applied at the end of the schedule.

**Bevy Source**: `bevy_ecs/src/system/commands/mod.rs`

### Basic Commands Usage

```rust
// src/systems/spawn_effect_system.rs
pub fn spawn_effect_system(
    mut commands: Commands,
    mut events: MessageReader<SpawnEffectEvent>,
    // ... other params
) {
    for event in events.read() {
        match event {
            SpawnEffectEvent::AtEntity(entity, data) => {
                spawn_effect(
                    &vfs_resource.vfs,
                    &mut commands,
                    &asset_server,
                    // ...
                    Some(*entity),
                    Some(&effect_cache),
                );
            }
            // ...
        }
    }
}
```

### Entity Commands

```rust
// src/systems/command_system.rs
pub fn command_system(
    mut commands: Commands,
    query: Query<(Entity, &Command)>,
) {
    for (entity, command) in query.iter() {
        if matches!(command, Command::Die) {
            commands.entity(entity).insert(Dead);
        }
    }
}
```

### Deferred Commands with Closure

```rust
// src/components/collision.rs
impl<'a> RemoveColliderCommand for EntityCommands<'a> {
    fn remove_and_despawn_collider(&mut self) -> &mut Self {
        let entity = self.id();
        
        self.commands().queue(move |world: &mut World| {
            let mut world_entity = world.entity_mut(entity);
            if let Some(collider_entity) = world_entity.get::<ColliderEntity>() {
                let collider_entity = collider_entity.entity;
                world_entity.remove::<ColliderEntity>();
                world.despawn(collider_entity);
            }
        });
        
        self
    }
}
```

### Command Patterns in This Project

1. **Spawn Commands**: `commands.spawn((ComponentA, ComponentB))`
2. **Entity Commands**: `commands.entity(entity).insert(Component)`
3. **Resource Commands**: `commands.insert_resource(Resource)`
4. **Despawn Commands**: `commands.entity(entity).despawn()`
5. **Queue Commands**: `commands.queue(|world| { /* world modification */ })`

---

## Hierarchy

Bevy provides parent-child relationships through the `ChildOf` relationship component and `Children` relationship target.

**Bevy Source**: `bevy_ecs/src/hierarchy.rs`

### Parent-Child Relationship

```rust
// Using Bevy's built-in hierarchy
commands.spawn((
    Name::new("Parent"),
    Transform::default(),
))
.add_child(
    commands.spawn((
        Name::new("Child"),
        Transform::from_translation(Vec3::X),
    ))
    .id(),
);
```

### Custom Entity References

This project uses custom components for entity relationships:

```rust
// src/components/collision.rs
#[derive(Component, Reflect)]
pub struct ColliderEntity {
    pub entity: Entity,
}

#[derive(Component, Reflect)]
pub struct ColliderParent {
    pub entity: Entity,
}
```

### Querying Children

```rust
// Using Bevy's Children component
// Children derefs to [Entity]; query each child individually (get_many
// in Bevy 0.18 takes a fixed-size array, e.g. `get_many([entity])`)
fn process_children(
    query: Query<(&Name, &Children)>,
    child_query: Query<&Name, With<ChildOf>>,
) {
    for (parent_name, children) in query.iter() {
        for child_entity in children.iter() {
            if let Ok(child_name) = child_query.get(*child_entity) {
                // Process child entity
            }
        }
    }
}
```

---

## Schedule Structure

The project uses Bevy's built-in schedules with custom system sets. The full ordering lives in `src/lib.rs` — the snippet below is abbreviated; see `GameStages`, `GameSystemSets`, `UiSystemSets`, `ModelSystemSets`, `EffectSystemSets`, `UiSystemOrdering` (`src/lib.rs:655-709`).

```rust
// src/lib.rs
// Main schedules: PreUpdate, Update, PostUpdate

// PreUpdate: Network systems (before game logic)
app.add_systems(
    PreUpdate,
    (
        login_connection_system,
        world_connection_system,
        game_connection_system.run_if(resource_exists::<CurrentZone>),
    ),
);

// Update: Main game logic (abbreviated — ~100 systems in src/systems/,
// ordered by ModelSystemSets / EffectSystemSets / GameSystemSets / UiSystemSets)
app.add_systems(Update, command_system.run_if(in_state(AppState::Game)));
app.add_systems(Update, collision_player_system.run_if(in_state(AppState::Game)));

// Zone loading chain (Update): zone_loader_system
//   -> zone_loaded_from_vfs_system -> game_zone_change_system
//   (see zone-pipeline.md; src/lib.rs:1227-1253)

// PostUpdate: deferred ops, animation-before-propagate, physics sync,
// cleanup and debug render
app.add_systems(PostUpdate, character_model_blink_system);
app.add_systems(PostUpdate, network_thread_system);
```

Related ordering constraints (all in `src/lib.rs`):
- Animation systems run in `PostUpdate` before `TransformSystems::Propagate`.
- `(GameStages::AfterUpdate,).before(PhysicsSet::SyncBackend)` (`:1734`) and `(ZoneChange, ZoneChangeFlush, AfterUpdate).before(PhysicsSet::SyncBackend)` (`:1740-1742`).
- `GameStages::AfterUpdate.before(TransformSystems::Propagate)` (`:1756`).
- Bevy 0.18 supports at most ~20 systems per tuple — `world_ui_occlusion_system` is registered in a separate `add_systems` call for this reason.

### ApplyDeferred

Commands are applied at specific points in the schedule:

```rust
// src/lib.rs
app.add_systems(PostUpdate, ApplyDeferred);
app.add_systems(
    PostUpdate,
    (ApplyDeferred,).in_set(GameStages::DebugRenderPreFlush),
);
```

---

## Common Patterns

### 1. Entity Spawn with Multiple Components

```rust
commands.spawn((
    Transform::from_translation(position),
    GlobalTransform::default(),
    ClientEntity::new(entity_id, entity_type),
    Position::new(Vec3::new(x, y, z)),
    // Additional components...
));
```

### 2. Query with Optional Components

```rust
Query<
    (
        &mut Transform,
        Option<&CharacterModel>,
        Option<&NpcModel>,
        Option<&VehicleModel>,
    ),
    With<ClientEntity>,
>
```

### 3. State-Gated System

```rust
app.add_systems(
    Update,
    my_system.run_if(in_state(AppState::Game)),
);
```

### 4. Resource-Conditional System

```rust
app.add_systems(
    Update,
    my_system.run_if(resource_exists::<CurrentZone>),
);
```

### 5. Ordered System Chain

```rust
app.add_systems(
    Update,
    (
        system_a,
        system_b.after(system_a),
        system_c.after(system_b),
    ),
);
```

---

## Custom Extensions

### Message System

`Message` is a **built-in Bevy 0.18 trait** (not defined by this project). It is a marker trait for pull-based message passing:

```rust
// bevy_ecs/src/message/mod.rs (Bevy 0.18.1, line 96)
pub trait Message: Send + Sync + 'static {}
```

The project derives it for its event-like types, e.g. `#[derive(Message)] pub struct HitEvent` in `src/events/hit_event.rs`.

**Source**: `bevy_ecs/src/message/mod.rs`

### MessageReader and MessageWriter

Built-in Bevy system parameters for reading and writing messages (replacing the old `EventReader`/`EventWriter` for pull-based handling):

```rust
// bevy_ecs/src/message/message_reader.rs
pub struct MessageReader<'w, 's, M: Message> { ... }

// bevy_ecs/src/message/message_writer.rs
pub struct MessageWriter<'w, M: Message> { ... }
```

Messages are written with `MessageWriter::write()` and read with `MessageReader::read()`, which returns an iterator (`for event in events.read()`).

**Source**: `bevy_ecs/src/message/message_reader.rs`, `bevy_ecs/src/message/message_writer.rs`

### RemoveColliderCommand

Project-defined extension trait for entity commands (`src/components/collision.rs`):

```rust
// src/components/collision.rs
pub trait RemoveColliderCommand {
    fn remove_and_despawn_collider(&mut self) -> &mut Self;
}

impl<'a> RemoveColliderCommand for EntityCommands<'a> {
    fn remove_and_despawn_collider(&mut self) -> &mut Self {
        // Implementation
    }
}
```

**Source**: `src/components/collision.rs:19-38`

---

## Code Examples

### Complete Entity Spawn

**File**: `src/systems/game_connection_system.rs` (JoinZone handler, around line 462)

```rust
let mut entity_commands = commands.entity(player_entity);
entity_commands.insert((
    ClientEntity::new(entity_id, ClientEntityType::Character),
    CollisionPlayer,
    Command::with_stop(),
    NextCommand::with_stop(),
    FacingDirection::default(),
    experience_points,
    team,
    health_points,
    mana_points,
));
```

### State Transition with Cleanup

**File**: `src/systems/game_system.rs:31-56`

```rust
pub fn game_state_enter_system(
    mut commands: Commands,
    query_cameras: Query<
        Entity,
        (With<Camera3d>, Without<crate::render::WaterReflectionCamera>),
    >,
    query_player: Query<Entity, With<PlayerCharacter>>,
) {
    // Reset camera to orbit the player character
    let player_entity = match query_player.single() {
        Ok(entity) => entity,
        Err(_) => return,
    };

    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<FreeCamera>()
            .remove::<CameraAnimation>()
            .insert(OrbitCamera::new(
                player_entity,
                Vec3::new(0.0, 1.7, 0.0),
                15.0,
            ));
    }
}
```

### Complex Query Pattern

**File**: `src/systems/collision_system.rs:266`

```rust
pub fn collision_player_system(
    mut commands: Commands,
    mut query_collision_entity: Query<
        (
            Entity,
            &mut Position,
            &mut Transform,
            Option<&FlightState>,
            Option<&mut BoatState>,
        ),
        With<CollisionPlayer>,
    >,
    // ... additional queries and resources
    current_zone: Option<Res<CurrentZone>>,
    rapier_context: ReadRapierContext,
    time: Res<Time>,
) {
    // Collision logic
}
```

---

## Configuration Options

### System Configuration

| Option | Description | Default |
|--------|-------------|---------|
| `passthrough_terrain_textures` | Render terrain textures without modification | `false` |
| `trail_effect_duration_multiplier` | Scale duration of trail effects | `1.0` |
| `use_new_terrain` | Enable new terrain rendering system | `false` |

**Source**: `src/resources/render_configuration.rs`

### State Configuration

| State | Entry System | Exit System |
|-------|--------------|-------------|
| `GameLogin` | `login_state_enter_system` | `login_state_exit_system` |
| `GameCharacterSelect` | `character_select_enter_system` | `character_select_exit_system` |
| `Game` | `game_state_enter_system` | - |
| `ZoneViewer` | `zone_viewer_enter_system` | - |
| `MapEditor` | `map_editor_enter_system` | `map_editor_exit_system` |

---

## Troubleshooting

### Bevy 0.18 Migration Issues

#### Issue 1: System Parameter Trait Bounds

**Problem**: Systems fail to compile because query data, filters, and system parameters do not satisfy the required trait bounds.

**Error**:
```
the trait bound `MyQueryData: QueryData` is not satisfied
the trait bound `MyFilter: QueryFilter` is not satisfied
```

**Solution**: Query data types must implement `QueryData` (`&T`, `&mut T`, tuples, etc.), filters must implement `QueryFilter` (`With<T>`, `Without<T>`, `Or<...>`), and system parameters must implement `SystemParam`. System functions support at most 16 parameters; group excess parameters into tuples or a custom `SystemParam` struct. Explicit `&'static` annotations are not required — lifetimes are inferred:

```rust
// Correct: inferred lifetimes
fn my_system(query: Query<(&mut A, &B), With<C>>) { ... }
```

**Source**: `bevy_ecs/src/system/system_param.rs`

#### Issue 2: MessageWriter vs EventWriter

**Problem**: Confusion between the old `EventWriter` API and Bevy 0.18's pull-based `MessageWriter`.

**Solution**: In Bevy 0.18, `Message`/`MessageReader`/`MessageWriter` are built-in types replacing the pull-based `Event`/`EventReader`/`EventWriter` pattern (this project does not define a custom `MessageWriter`). Register messages with `app.add_message::<T>()`, write with `MessageWriter::write()`:

```rust
fn my_system(
    mut events: MessageWriter<MyEvent>,
) {
    events.write(MyEvent {});
}
```

**Source**: `bevy_ecs/src/message/message_writer.rs`

#### Issue 3: State Transition Timing

**Problem**: Systems run in wrong order during state transitions.

**Error**: Resource not available when expected after state change.

**Solution**: Use `OnEnter` and `OnExit` schedules explicitly:

```rust
// Correct order
app.add_systems(
    OnEnter(AppState::Game),
    (
        initialize_resources,
        spawn_entities.after(initialize_resources),
    ),
);
```

**Source**: `bevy_state/src/state/mod.rs` (OnEnter/OnExit schedules)

#### Issue 4: Query Filter Conflicts

**Problem**: Multiple queries with `With`/`Without` filters cause borrow conflicts.

**Error**:
```
cannot borrow `world` as mutable more than once at a time
```

**Solution**: Combine filters in single query or use `Or`:

```rust
// Instead of two queries
Query<&mut A, With<B>>
Query<&mut A, Without<B>>

// Use one query with Or
Query<(&mut A, Option<&B>)>
```

**Source**: `bevy_ecs/src/query/filter.rs`

#### Issue 5: Commands Not Applied Immediately

**Problem**: Entity spawned with commands not accessible in same system.

**Solution**: Use `ApplyDeferred` or access via `Commands`:

```rust
app.add_systems(
    Update,
    (
        spawn_system,
        ApplyDeferred,
        use_spawned_entities.after(ApplyDeferred),
    ),
);
```

**Source**: `bevy_ecs/src/schedule/auto_insert_apply_deferred.rs`

#### Issue 6: SystemSet Ordering Not Respected

**Problem**: Systems in same SystemSet execute in unpredictable order.

**Solution**: Use `.after()` for explicit ordering within sets:

```rust
app.add_systems(
    Update,
    (
        system_a.in_set(MySet::Group1),
        system_b.in_set(MySet::Group1).after(system_a),
    ),
);
```

**Source**: `bevy_ecs/src/schedule/set.rs`

#### Issue 7: Resource Mutability Conflicts

**Problem**: Multiple systems trying to mutate same resource.

**Error**:
```
cannot borrow `*.ResMut<MyResource>` as mutable more than once
```

**Solution**: Use `Res` for read-only access or split systems:

```rust
// Read-only
fn system_a(resource: Res<MyResource>) { }

// Mutable (separate system)
fn system_b(mut resource: ResMut<MyResource>) { }
```

**Source**: `bevy_ecs/src/resource.rs`

#### Issue 8: MessageReader Consumes All Messages

**Problem**: A message written once is not seen by every system that needs it.

**Solution**: Each system gets its own `MessageReader`, and every reader sees all messages written since the last update (no cloning needed). Messages are cleared every frame by Bevy's built-in `message_update_system`:

```rust
// Both systems receive every message of type MyEvent
fn system_a(mut events: MessageReader<MyEvent>) { ... }
fn system_b(mut events: MessageReader<MyEvent>) { ... }
```

**Source**: `bevy_ecs/src/message/update.rs`

#### Issue 9: Component Required By Not Enforced

**Problem**: Entity missing required component at runtime.

**Solution**: Use `#[require(ComponentB)]` attribute:

```rust
#[derive(Component)]
#[require(ComponentB)]
pub struct ComponentA { }
```

**Source**: `bevy_ecs/src/component/required.rs`

#### Issue 10: Hierarchy Despawn Order

**Problem**: Children despawned before parents cause issues.

**Solution**: In Bevy 0.18, `EntityCommands::despawn()` automatically recursively despawns descendants through relationship targets (there is no `despawn_recursive()` method anymore — it was removed in the relationship refactor):

```rust
// Bevy hierarchy handles this automatically
commands.entity(parent).despawn();

// Manual: despawn children first
for child in children.iter() {
    commands.entity(*child).despawn();
}
commands.entity(parent).despawn();
```

**Source**: `bevy_ecs/src/system/commands/mod.rs` (`despawn`, around line 1860)

---

## Performance Considerations

1. **Query Optimization**: Use `With`/`Without` filters to reduce query iteration
2. **Component Design**: Keep components small and cache-friendly
3. **System Ordering**: Use SystemSets for clear execution order
4. **Commands**: Batch entity operations when possible
5. **State Management**: Use state conditions to skip unnecessary systems

---

## Source File References

### Bevy Source Files (v0.18.1)

| Component | Path |
|-----------|------|
| ECS Core | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\lib.rs` |
| Components | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\component\mod.rs` |
| Resources | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\resource.rs` |
| Systems | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\system\mod.rs` |
| Function System | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\system\function_system.rs` |
| Queries | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\system\query.rs` |
| Query Module | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\query\mod.rs` |
| Commands | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\system\commands\mod.rs` |
| Events | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\event\mod.rs` |
| Hierarchy | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\hierarchy.rs` |
| Schedule | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\schedule\mod.rs` |
| SystemSet | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_ecs\src\schedule\set.rs` |
| State | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_state\src\lib.rs` |
| States Derive | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_state\src\state\mod.rs` |

### Project Source Files

| Module | Path |
|--------|------|
| Components | `src/components/` |
| Resources | `src/resources/` |
| Systems | `src/systems/` |
| Events | `src/events/` |
| App State | `src/resources/app_state.rs` |
| Game Data | `src/resources/game_data.rs` |
| Main App | `src/lib.rs` |
