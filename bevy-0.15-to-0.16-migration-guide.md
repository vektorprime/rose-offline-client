# 0.15 to 0.16
Accessibility [#](#accessibility)
---------------------------------

### Replace `bevy::a11y::Focus` with `bevy::input_focus::InputFocus` [#](#replace-bevy-a11y-focus-with-bevy-input-focus-inputfocus)

Bevy now has first-class input handling, available in the `bevy::input_focus` module. As such, `bevy::a11y::Focus` has been replaced with `bevy::input_focus::InputFocus`. Please replace all references and imports.

Animation [#](#animation)
-------------------------

### Configure `EasingFunction::Steps` jumping with `JumpAt` [#](#configure-easingfunction-steps-jumping-with-jumpat)

`EaseFunction::Steps` now has a second parameter, `JumpAt`, which can customize jumping behavior. `JumpAt`'s default is `JumpAt::End`, which indicates that the last steps happens when the animation ends.

```
// 0.15
let ease_function = EaseFunction::Steps(10);

// 0.16
let ease_function = EaseFunction::Steps(10, JumpAt::default());

```


* * *

### Fix `EaseFunction::Exponential*` to be continuous [#](#fix-easefunction-exponential-to-be-continuous)

`EaseFunction::ExponentialIn`, `EaseFunction::ExponentialOut`, and `EaseFunction::ExponentialInOut` has slight discontinuities in 0.15, leading to [jumping behavior at the start and end of the function](https://github.com/bevyengine/bevy/issues/16676). In 0.16, these functions have been slightly adjusted so that they are continuous.

The new functions differ from the old by less than 0.001, so in most cases this change is not breaking. If, however, you depend on these easing functions for determinism, you will need to define custom curves using the previous functions.

Assets [#](#assets)
-------------------

### Remove `meta` field from `LoadedAsset` and `ErasedLoadedAsset` [#](#remove-meta-field-from-loadedasset-and-erasedloadedasset)

`LoadedAsset` used to have a `meta` field for storing metadata. This field was unused and inaccessible, however, so in 0.16 it has been removed. Due to this change, several method signatures have also changed:

*   `ErasedAssetLoader::load()` now takes `meta: &(dyn AssetMetaDyn + 'static)` instead of a `Box<dyn AssetMetaDyn>`.
*   `LoadedAsset::new_with_dependencies()` no longer requires a `meta` argument.
*   `LoadContext::finish()` no longer requires a `meta` argument.

* * *

### Deprecate `Handle::weak_from_u128()` [#](#deprecate-handle-weak-from-u128)

`Handle::weak_from_u128()` has been deprecated in favor of the new `weak_handle!` macro, which takes a UUID as a string instead of a `u128`. `weak_handle!` is preferred because it both makes the string form of the UUID visible and it verifies that the UUID is compliant with UUIDv4.

```
// 0.15
const SHADER: Handle<Shader> = Handle::weak_from_u128(314685653797097581405914117016993910609);

// 0.16
const SHADER: Handle<Shader> = weak_handle!("1347c9b7-c46a-48e7-b7b8-023a354b7cac");

```


* * *

### Add `AssetChanged` query filter [#](#add-assetchanged-query-filter)

The `Assets::asset_events()` system is no longer public. If you wish to order your systems relative to asset events, use the new `AssetEvents` system set instead.

Audio [#](#audio)
-----------------

### Add ability to mute audio sinks [#](#add-ability-to-mute-audio-sinks)

It is now possible to mute audio sinks. Several breaking changes have been introduced to implement this feature.

First, `AudioSinkPlayback::set_volume()` now takes a mutable `&mut AudioSinkPlayback` argument instead of an immutable one. This may require you to update your system parameters:

```
// 0.15
fn increase_volume(sink: Single<&AudioSink, With<Music>>) {
    sink.set_volume(sink.volume() + 0.1);
}

// 0.16
fn increase_volume(mut sink: Single<&mut AudioSink, With<Music>>) {
    let current_volume = sink.volume();
    sink.set_volume(current_volume + 0.1);
}

```


Secondly, `PlaybackSettings` has a new `muted` field to specify whether an entity should start muted. You may need to set this field when creating `PlaybackSettings` if you do not use function update syntax (`..default()`).

Finally, if you manually implemented audio muting using an audio sink's volume, you can switch over to using the new `AudioSinkPlayback` methods: `is_muted()`, `mute()`, `unmute()` and `toggle_mute()`.

* * *

### Rename `AudioSinkPlayback::toggle()` to `toggle_playback()` [#](#rename-audiosinkplayback-toggle-to-toggle-playback)

`AudioSinkPlayback::toggle()` has been renamed to `toggle_playback()`. This was done to create consistency with the `toggle_mute()` method added in [#16813](https://github.com/bevyengine/bevy/pull/16813). Please update all references to use the new name.

```
// 0.15
fn pause(keyboard_input: Res<ButtonInput<KeyCode>>, sink: Single<&AudioSink>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        sink.toggle();
    }
}

// 0.16
fn pause(keyboard_input: Res<ButtonInput<KeyCode>>, sink: Single<&AudioSink>) {
    if keyboard_input.just_pressed(KeyCode::Space) {
        sink.toggle_playback();
    }
}

```


* * *

### Support decibels in `Volume` [#](#support-decibels-in-volume)

Audio volume can now be configured using decibel values, as well as using linear scale values. To enable this, some types and functions in `bevy::audio` have changed. First, `Volume` is now an enum with `Linear` and `Decibels` variants:

```
// 0.15
let v = Volume(1.0);

// 0.16
let volume = Volume::Linear(1.0);

// Alternatively, you can use decibels instead.
let volume = Volume::Decibels(0.0);

```


`Volume::Linear` is equivalent to the old `f32` volume.

With this change, `AudioSinkPlayback`'s volume-related methods (`volume()` and `set_volume()`) and `GlobalVolume` now deal in `Volume`s rather than `f32`s.

Finally, `Volume::ZERO` has been renamed to the more semantically correct `Volume::SILENT`. This is because 0 decibels is equivalent to "normal volume", which could lead to confusion with the old naming.

Cross-Cutting [#](#cross-cutting)
---------------------------------

### Add `no_std` support to `bevy` [#](#add-no-std-support-to-bevy)

The main `bevy` crate now officially supports `no_std`. As part of this change, some functionality that used to always be included in `bevy` is now behind feature flags. The features of note are:

*   `default_no_std`
*   `bevy_log`
*   `bevy_input_focus`
*   `async_executor`
*   `std`
*   `critical-section`
*   `libm`

Additionally, if you depend on `bevy_reflect` directly, its `bevy` feature flag has been split into two separate flags: `smallvec` and `smol_str` for their corresponding types.

If your application has default features enabled, congratulations! You don't need to do anything extra! If your application has `default-features = false`, however, you may need to enabled the `std` and `async_executor` features:

```
# 0.15
[dependencies]
bevy = { version = "0.15", default-features = false }

# 0.16
[dependencies]
bevy = { version = "0.16", default-features = false, features = ["std", "async_executor"] }

```


#### For library authors [#](#for-library-authors)

It is recommended for libraries to depend on Bevy with `default-features = false` to give developers more control over what features are enabled. Here are some recommended features that a library crate may want to expose:

```
[features]
# Most users will be on a platform which has `std` and can use the more-powerful `async_executor`.
default = ["std", "async_executor"]

# Features for typical platforms.
std = ["bevy/std"]
async_executor = ["bevy/async_executor"]

# Features for `no_std` platforms.
libm = ["bevy/libm"]
critical-section = ["bevy/critical-section"]

[dependencies]
# We disable default features to ensure we don't accidentally enable `std` on `no_std` targets, for
# example. 
bevy = { version = "0.16", default-features = false }

```


* * *

### Support for non-browser WASM [#](#support-for-non-browser-wasm)

Bevy now has support for the [`wasm32v1-none` target](https://doc.rust-lang.org/rustc/platform-support/wasm32v1-none.html), which is a barebones `no_std` version of `wasm32-unknown-unknown` that disables all features past the original [W3C WebAssembly Core 1.0 spec](https://www.w3.org/TR/wasm-core-1/). As part of this change, Bevy's browser-specific WASM features have been put behind of the `web` feature flag, which is enabled by default. If you have `default-features = false` and wish to build Bevy to run on a browser, you will need to re-enable this flag:

```
# 0.15
[dependencies]
bevy = { version = "0.15", default-features = false }

# 0.16
[dependencies]
bevy = { version = "0.16", default-features = false, features = ["web"] }

```


* * *

### Upgrade to Rust Edition 2024 [#](#upgrade-to-rust-edition-2024)

As part of Bevy's migration to [Rust 2024](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0.html#rust-2024), the lifetimes of several functions that use return-position impl-trait (RPIT) syntax may have been changed to be slightly more conservative. If you run into lifetime issues with functions that return `impl Trait`, please [create a new issue](https://github.com/bevyengine/bevy/issues).

### Add `strict` field to BRP `bevy/query` [#](#add-strict-field-to-brp-bevy-query)

Bevy Remote Protocol's `bevy/query` request now skips missing or invalid components by default instead of returning an error. This can be configured with `BrpQueryParams`'s new `strict` boolean field.

If you wish `bevy/query` to return to its previous behavior of erroring on missing / invalid components, set `"strict": true`:

```
{
    "method": "bevy/query",
    "id": 0,
    "params": {
        "data": {
            "components": ["foo::bar::MyComponent"]
        },
        // Error if `foo::bar::MyComponent` doesn't exist.
        "strict": true
    }
}

```


* * *

### Rename `track_change_detection` flag to `track_location` [#](#rename-track-change-detection-flag-to-track-location)

The `track_change_detection` feature flag no longer just tracks the source code location for change detection, but also where entities are spawned and despawned. As such, the feature flag has been renamed to `track_location` to better reflect its extended capabilities.

* * *

### Draw the UI debug overlay using the UI renderer [#](#draw-the-ui-debug-overlay-using-the-ui-renderer)

The `bevy_dev_tools::ui_debug_overlay` module has been replaced with a new debug overlay implemented using `bevy_ui`'s renderer. The new debug UI overlay still requires the `bevy_ui_debug` feature flag, but this flag is now available through `bevy` and `bevy_ui` instead of `bevy_dev_tools`. `UiDebugOptions` has been moved to `bevy_ui` as well, and now has several new options.

```
// 0.15
App::new()
    .add_plugins((DefaultPlugins, DebugUiPlugin))
    .insert_resource(UiDebugOptions {
        enabled: true,
    })
    .run();

// 0.16
App::new()
    // You no longer need `DebugUiPlugin`; enabling the `bevy_ui_debug` feature handles this for
    // you.
    .add_plugins(DefaultPlugins)
    .insert_resource(UiDebugOptions {
        enabled: true,
        // `UiDebugOptions` has a few new options, but for now we'll leave the defaults.
        ..default()
    })
    .run();

```


Diagnostics [#](#diagnostics)
-----------------------------

### Allow users to customize history length in `FrameTimeDiagnosticsPlugin` [#](#allow-users-to-customize-history-length-in-frametimediagnosticsplugin)

`FrameTimeDiagnosticsPlugin` now contains two fields: `max_history_length` and `smoothing_factor`. If you manually construct this plugin and wish to retain 0.15 behavior, simply call `FrameTimeDiagnosticsPlugin::default()`. If you wish to configure the maximum history length, you may use `FrameTimeDiagnosticsPlugin::new()` instead.

ECS [#](#ecs)
-------------

### Improve required component syntax [#](#improve-required-component-syntax)

Required component syntax has been reworked to be more intuitive with Rust's syntax. Custom-constructor requires should use the new expression-style syntax:

```
// 0.15
#[derive(Component)]
#[require(A(returns_a))]
struct Foo;

// 0.16
#[derive(Component)]
#[require(A = returns_a())]
struct Foo;

```


Inline-closure-constructor requires should use the inline value syntax where possible:

```
// 0.15
#[derive(Component)]
#[require(A(|| A(10)))]
struct Foo;

// 0.16
#[derive(Component)]
#[require(A(10))]
struct Foo;

```


In cases where that is not possible, use the expression-style syntax:

```
// 0.15
#[derive(Component)]
#[require(A(|| A(10)))]
struct Foo;

// 0.16
#[derive(Component)]
#[require(A = A(10))]
struct Foo;

```


* * *

### Get names of queued components [#](#get-names-of-queued-components)

Bevy now supports queueing components to be registered with read only world access, as opposed to registering them directly with mutable access. For now, that's an implementation detail, but it opens up some exciting possibilities for the future. Today, however, it causes a breaking change.

In order to support getting the names of components that are queued but not registered (important for debugging), `Components::get_name` now returns `Option<Cow<'_, str>` instead of `Option<&str>`. If that behavior is not desired, or you know the component is not queued, you can use `components.get_info().map(ComponentInfo::name)` instead. Similarly, `ScheduleGraph::conflicts_to_string` now returns `impl Iterator<Item = (String, String, Vec<Cow<str>>)>` instead of `impl Iterator<Item = (String, String, Vec<&str>)>`.

Because `Cow<str>` derefs to `&str`, most use cases can remain unchanged. If you're curious about queued registration, check out the original pr [here](https://github.com/bevyengine/bevy/pull/18173).

* * *

### Return `EntityDoesNotExistError` on error for several methods [#](#return-entitydoesnotexisterror-on-error-for-several-methods)

The return types of several `World` and `UnsafeWorldCell` methods have been modified to return a `Result<T, EntityDoesNotExist>`.

*   `World::inspect_entity()` now returns `Result<impl Iterator<Item = &ComponentInfo>, EntityDoesNotExistError>` instead of `impl Iterator<Item = &ComponentInfo>`. As such, this method no longer panics if the entity does not exist.
*   `World::get_entity()` now returns `EntityDoesNotExistError` as an error instead of `Entity`. You can still access the entity's ID through `EntityDoesNotExistErrorentity::entity`, however.
*   `UnsafeWorldCell::get_entity()` now returns `Result<UnsafeEntityCell, EntityDoesNotExistError>` instead of `Option<UnsafeEntityCell>`, giving you access to the entity's ID and other details on the error.

* * *

### Cache systems by `S` instead of `S::System` [#](#cache-systems-by-s-instead-of-s-system)

As part of a bug fix for system caching, the `CachedSystemId` resource has been changed to store an `Entity` instead of a `SystemId`. `CachedSystemId` construction has also been changed to use the `new()` method.

```
// 0.15
let cached_id = CachedSystemId::<S::System>::(id);
assert!(id == cached_id.0);

// 0.16
let cached_id = CachedSystemId::<S>::new(id);
// You can convert a valid `Entity` into a `Systemid` with `SystemId::from_entity()`.
assert!(id == SystemId::from_entity(cached_id.entity));

```


* * *

### Change `World::try_despawn()` and `World::try_insert_batch()` to return `Result` [#](#change-world-try-despawn-and-world-try-insert-batch-to-return-result)

`World::try_despawn()` now returns a `Result` rather than a `bool`. Additionally, `World::try_insert_batch()` and `World::try_insert_batch_if_new()` now return a `Result` instead of silently failing.

* * *

### Remove `IntoSystemConfigs` implementation for `BoxedSystem<(), ()>` [#](#remove-intosystemconfigs-implementation-for-boxedsystem)

`bevy::ecs::IntoSystemConfigs`, now known as `IntoScheduleConfigs`, is no longer implemented for `BoxedSystem<(), ()>`. This can lead to convoluted trait errors when you try to add a `BoxedSystem<(), ()>` to a schedule or app:

```
error[E0277]: `std::boxed::Box<dyn bevy::prelude::System<In = (), Out = ()>>` does not describe a valid system configuration

```


In order to avoid this error, either wrap your system in an `InfallibleSystemWrapper` before boxing it or make the system return a `Result<(), BevyError>`.

```
// 0.15
fn my_system() {
    println!("Hello, world!");
}

// Convert the function into a boxed system, which is a `Box<dyn System<In = (), Out = ()>>`.
let system = Box::new(IntoSystem::into_system(my_system)) as BoxedSystem;

App::new()
    .add_systems(Startup, system)
    .run();

// 0.16 (Using `InfallibleSystemWrapper`)
fn my_system() {
    println!("Hello, world!");
}

// Use `InfallibleSystemWrapper::new()` to make a system unconditionally return `Result::Ok`. The
// boxed system is now a `Box<dyn System<In = (), Out = Result<(), BevyError>>>`.
let system = Box::new(InfallibleSystemWrapper::new(IntoSystem::into_system(my_system))) as BoxedSystem<_, _>;

App::new()
    .add_systems(Startup, system)
    .run();

// 0.16 (Returning `Result<(), BevyError>`)
fn my_system() -> Result {
    println!("Hello, world!");
    Ok(())
}

// The boxed system is now a `Box<dyn System<In = (), Out = Result<(), BevyError>>>`.
let system = Box::new(IntoSystem::into_system(my_system)) as BoxedSystem<_, _>;

App::new()
    // Add the boxed system to the app.
    .add_systems(Startup, system)
    .run();

```


Note that in several cases you do not need to box your systems before adding them, such as with `App::add_systems()`, which lets you avoid this issue.

* * *

### Improve ergonomics of `NonSendMarker` [#](#improve-ergonomics-of-nonsendmarker)

`NonSendMarker`, a type used to force systems to run on the main thread, is now a system parameter. This means that it no longer needs to be wrapped in `Option<NonSend<_>>`. Furthermore, `NonSendMarker` has been moved from `bevy::core` to `bevy::ecs::system`, so please update your imports accordingly.

```
// 0.15
use bevy::core::NonSendMarker;

fn my_system(_: Option<NonSend<NonSendMarker>>) {
    // ...
}

// 0.16
use bevy::ecs::system::NonSendMarker;

fn my_system(_: NonSendMarker) {
    // ...
}

```


* * *

### Define `SystemParam` validation on a per-system parameter basis [#](#define-systemparam-validation-on-a-per-system-parameter-basis)

Various system and system parameter validation methods (`SystemParam::validate_param`, `System::validate_param` and `System::validate_param_unsafe`) now return and accept a `ValidationOutcome` enum, rather than a `bool`. The previous `true` values map to `ValidationOutcome::Valid`, while `false` maps to `ValidationOutcome::Invalid`.

However, if you wrote a custom schedule executor, you should now respect the new `ValidationOutcome::Skipped` parameter, skipping any systems whose validation was skipped. By contrast, `ValidationOutcome::Invalid` systems should also be skipped, but you should call the `default_error_handler` on them first, which by default will result in a panic.

If you are implementing a custom `SystemParam`, you should consider whether failing system param validation is an error or an expected state, and choose between `Invalid` and `Skipped` accordingly. In Bevy itself, `Single` and `Populated` now once again skip the system when their conditions are not met. This is the 0.15.0 behavior, but stands in contrast to the 0.15.1 behavior, where they would panic.

* * *

### Deprecate `insert_or_spawn()` function family [#](#deprecate-insert-or-spawn-function-family)

The following functions have been deprecated:

*   `Commands::insert_or_spawn_batch()`
*   `World::insert_or_spawn_batch()`
*   `World::insert_or_spawn_batch_with_caller()`
*   `Entities::alloc_at()`

These methods, when used incorrectly, can cause major performance problems and are generally viewed as anti-patterns and foot guns. These are planned to be removed altogether in 0.17.

Instead of the above functions, consider doing one of the following:

1.  Use the new `Disabled` component. Instead of despawning entities, simply disable them until you need them again. You can even use `Commands::try_insert_batch()` and `EntityCommands::remove()` to adjust what components an entity has.
2.  Instead of despawning and respawning entities with the same `Entity` ID, simply use `spawn_batch()` and update the IDs to the new values.

* * *

### Deprecate `Query::many()` and `Query::many_mut()` [#](#deprecate-query-many-and-query-many-mut)

Due to improvements in **Bevy 0.16**'s error handling capabilities, `Query::many()` and `Query::many_mut()` have been deprecated in favor of their non-panicking variants: `Query::get_many()` and `Query::get_many_mut()`.

```
#[derive(Resource)]
struct Player1(Entity);

#[derive(Resource)]
struct Player2(Entity);

// 0.15
fn my_system(player1: Res<Player1>, player2: Res<Player2>, query: Query<&Transform>) {
    let [transform1, transform2] = query.many([player1.0, player2.0]);

    // ...
}

// 0.16
// Make the system return a `Result`, which is automatically imported in Bevy's prelude.
fn my_system(player1: Res<Player1>, player2: Res<Player2>, query: Query<&Transform>) -> Result {
    // Use `get_many()` and the `?` operator to return early on an error.
    let [transform1, transform2] = query.get_many([player1.0, player2.0])?;

    // ...

    Ok(())
}

```


Please note that `Query::get_many()` is very similar to `Query::get()`. To increase the consistency between the two methods, the name `get_many()` was kept over plain `many()`. Although in 0.15 `Query::many()` seemed similar to `Query::single()` due to their naming, they are quite distinct. This change is meant to reinforce this distinction.

* * *

### Encapsulate location tracking data into `MaybeLocation` [#](#encapsulate-location-tracking-data-into-maybelocation)

Methods like `Ref::changed_by()` that used to return a `&'static Location<'static>` will now be available even when the `track_location` feature is disabled, but they will now return the new `MaybeLocation` type. `MaybeLocation` wraps a `&'static Location<'static>` when the feature is enabled, and is a ZST when the feature is disabled.

Existing code that needs a `&Location` can call `MaybeLocation::into_option()` to recover it. Many trait impls are forwarded, so if you only need `Display` then no changes will be necessary.

If that code was conditionally compiled, you may instead want to use the methods on `MaybeLocation` to remove the need for conditional compilation.

Code that constructs a `Ref`, `Mut`, `Res`, or `ResMut` will now need to provide location information unconditionally. If you are creating them from existing Bevy types, you can obtain a `MaybeLocation` from methods like `Table::get_changed_by_slice_for()` or `ComponentSparseSet::get_with_ticks`. Otherwise, you will need to store a `MaybeLocation` next to your data and use methods like `as_ref()` or `as_mut()` to obtain wrapped references.

* * *

### Support fallible systems [#](#support-fallible-systems)

If you've written a custom executor, there are a few changes you will need to make in order to support fallible systems.

1.  Many uses of `BoxedSystem<(), ()>` have been replaced with `ScheduleSystem`, which is a type alias to `BoxedSystem<(), Result>`.
2.  Executors should obey the `SystemParamValidationError` returned by `SystemParam::validate_param()` in order to determine whether to raise an error or skip the system.
3.  When an executor encounters an error, it should pass that error to `default_error_handler()`, whose behavior can be configured with the `GLOBAL_ERROR_HANDLER` static.

For more information on fallible systems, please read the module docs for `bevy::ecs::error`.

* * *

### Fix unsoundness in `QueryIter::sort_by()` [#](#fix-unsoundness-in-queryiter-sort-by)

The `sort()` family of methods on `QueryIter` unsoundly gave access `L::Item<'w>` with the full world `'w` lifetime, meaning it was possible to smuggle items out of the compare closure. This has been fixed by shortening the lifetime so that items cannot escape the closure on the following methods on `QueryIter` and `QueryManyIter`:

*   `sort()`
*   `sort_unstable()`
*   `sort_by()`
*   `sort_unstable_by()`
*   `sort_by_key()`
*   `sort_unstable_by_key()`
*   `sort_by_cached_key()`

This fix may cause your code to get lifetimes errors, such as:

```
error: implementation of `FnMut` is not general enough

```


To fix this, you will need to make the comparer generic over the new lifetime. Often this can be done by replacing named `'w` with `'_`, or by replacing the use of a function item with a closure:

```
// 0.15
query.iter().sort_by::<&C>(Ord::cmp);

// 0.16
query.iter().sort_by::<&C>(|l, r| Ord::cmp(l, r));

```


```
// 0.15
fn comparer(left: &&'w C, right: &&'w C) -> Ordering {
    // ...
}

query.iter().sort_by::<&C>(comparer);

// 0.16
fn comparer(left: &&C, right: &&C) -> Ordering {
    // ...
}

query.iter().sort_by::<&C>(comparer);

```


* * *

### Flush commands after every mutation in `WorldEntityMut` [#](#flush-commands-after-every-mutation-in-worldentitymut)

Previously, `EntityWorldMut` triggered command queue flushes in unpredictable places, which could interfere with hooks and observers. Now the command queue is always flushed immediately after `EntityWorldMut` spawns or despawns an entity, or adds, removes, or replaces a component.

As a side effect of this change, there is a new possibility that a hook or observer may despawn an entity that is being referred to by `EntityWorldMut`. If any of `EntityWorldMut`'s methods detect that the entity is despawned, they will panic. If you know this is a possibility and wish to avoid panicking, you may check that the entity is despawned with `EntityWorldMut::is_despawned()`.

* * *

### Make system configuration generic [#](#make-system-configuration-generic)

In order to reduce internal duplication between scheduling systems and system sets, the new generic `ScheduleConfigs<T>` type and `IntoScheduleConfigs<T>` trait have been added. These take a generic parameter, `T`, that may be `ScheduleSystem` (for systems) or `InternedSystemSet` (for system sets).


|0.15 Item           |0.16 Item                                |
|--------------------|-----------------------------------------|
|SystemConfigs       |ScheduleConfigs<ScheduleSystem>          |
|SystemSetConfigs    |ScheduleConfigs<InternedSystemSet>       |
|IntoSystemConfigs   |IntoScheduleConfigs<ScheduleSystem, M>   |
|IntoSystemSetConfigs|IntoScheduleConfigs<InternedSystemSet, M>|


* * *

### Introduce `EntityWorldMut` and move `EntityCommand::with_entity()` [#](#introduce-entityworldmut-and-move-entitycommand-with-entity)

The `EntityCommands::apply()` method now takes a `EntityWorldMut`, which is an optimized version of the previous `Entity` and `&mut World` pair. `EntityWorldMut` has several existing methods for working with entities, although you may use `EntityWorldMut::id()` to access the `Entity` and `EntityWorldMut::world_scope()` to access the `&mut World`.

```
struct MyCommand;

fn print_entity(In(entity): In<Entity>) {
    info!("Entity: {entity}");
}

// 0.15
impl EntityCommand for MyCommand {
    fn apply(self, entity: Entity, world: &mut World) {
        world
            .run_system_cached_with(print_entity, entity)
            .unwrap();
    }
}

// 0.16
impl EntityCommand for MyCommand {
    fn apply(self, entity_world: EntityWorldMut) {
        let entity = entity_world.id();

        entity_world.world_scope(move |world: &mut World| {
            world.run_system_cached_with(print_entity, entity).unwrap();
        });
    }
}

```


Additionally, the method `EntityCommand::with_entity()` has been moved to a separate trait, `CommandWithEntity`, so that it can be generic over commands that return `Result`s.

* * *

### Add bundle effects [#](#add-bundle-effects)

As part of improvements to the bundle spawning API, the `DynamicBundle` trait now has a new `Effect` associated type. If you manually implemented `DynamicBundle`, you likely want to set `Effect = ()`, which retains the same behavior as 0.15 bundles:

```
// 0.15
impl DynamicBundle for MyBundle {
    // ...
}

// 0.16
impl DynamicBundle for MyBundle {
    type Effect = ();

    // ...
}

```


* * *

### Rename `Query::to_readonly()` to `Query::as_readonly()` [#](#rename-query-to-readonly-to-query-as-readonly)

`Query::to_readonly()` has been renamed to `Query::as_readonly()` to reflect that it is cheap to call.

* * *

### Isolate component registration from `Storages` [#](#isolate-component-registration-from-storages)

In order to decouple `Storages` from `Components`, the following methods no longer take a `&mut Storages` argument:

*   `Components::register_component()`
*   `Components::register_component_with_descriptor()`
*   `Bundle::register_required_components()`
*   `Component::register_required_components()`

With this change, note that `SparseSets` will no longer be created when components are registered. Instead, they will only be constructed when those components are spawned.

* * *

### Unified error handling [#](#unified-error-handling)

`Query::single()`, `Query::single_mut()` and their `QueryState` equivalents now return a `Result`. Generally, you'll want to:

*   Use **Bevy 0.16**'s system error handling to return a `Result` using the `?` operator.
*   Use a `let Ok(data) = result else {}` block to early return if there's an expected failure.
*   Use `unwrap()` or `Ok` destructuring inside of tests.

The old `Query::get_single()` and related methods have been deprecated.

If you are using `anyhow`, you will experience namespace clashes between Bevy's catch-all `Result` and `anyhow::Result`. Within Bevy-specific projects, you should migrate to use the new `bevy::ecs::error::Result` due to its improved backtraces. (If you discover missing functionality, please feel free to open a pull request adding it!) For projects that support both Bevy and non-Bevy users, you should define a feature-gated type alias and avoid glob-importing `bevy::prelude`:

```
#[cfg(feature = "bevy")]
type Result = bevy::ecs::error::Result;

#[cfg(not(feature = "bevy"))]
type Result = anyhow::Result;

```


* * *

### Make system parameter validation use the `GLOBAL_ERROR_HANDLER` [#](#make-system-parameter-validation-use-the-global-error-handler)

`ParamWarnPolicy` and the `WithParamWarnPolicy` have been removed completely. Failures during system param validation are now handled via the `GLOBAL_ERROR_HANDLER`. Please see the `bevy_ecs::error` module docs for more information.

* * *

### Move `Item` and `fetch()` from `WorldQuery` to `QueryData` [#](#move-item-and-fetch-from-worldquery-to-querydata)

The `WorldQuery::Item` associated type and `WorldQuery::fetch()` method have been moved to `QueryData`, as they were not useful for `QueryFilter`\-based types.

* * *

### Move `Resource` trait to its own module [#](#move-resource-trait-to-its-own-module)

The `Resource` trait has been moved from `bevy::ecs::system::Resource` to `bevy::ecs::resource::Resource`. Please update your imports accordingly.

* * *

### Rename `EntityFetchError` and introduce `EntityDoesNotExist` [#](#rename-entityfetcherror-and-introduce-entitydoesnotexist)

`EntityFetchError` enum has been renamed to `EntityMutableFetchError`, and its `NoSuchEntity` variant has been renamed to `EntityDoesNotExist`. Furthermore, the `EntityDoesNotExist` variant now contains an `EntityDoesNotExistError` type, which provides further details on the entity that does not exist.

* * *

### Rename `Parent` to `ChildOf` and change how the parent is accessed [#](#rename-parent-to-childof-and-change-how-the-parent-is-accessed)

The `Parent` component has been renamed to `ChildOf` to make it more clear that entities with a `ChildOf` component are children, not parents.

Furthermore, it is now only possible to access the parent `Entity` from `ChildOf::parent()`. The `Deref` implementation has been removed and the `get()` method deprecated.

```
// 0.15
let parent = *child_of
// 0.16
let parent = child_of.parent()

// 0.15
let parent = child_of.get()
// 0.16
let parent = child_of.parent()

```


* * *

### Queued component registration [#](#queued-component-registration)

Component registration can now be queued with shared access to `World`, instead of requiring mutable access (`&mut World`). To facilitate this, a few APIs have been moved around.

The following functions have moved from `Components` to `ComponentsRegistrator`:

*   `register_component()`
*   `register_component_with_descriptor()`
*   `register_resource_with_descriptor()`
*   `register_non_send()`
*   `register_resource()`
*   `register_required_components_manual()`

Accordingly, functions in `Bundle` and `Component` now take `ComponentsRegistrator` instead of `Components`. You can obtain `ComponentsRegistrator` from the new `World::components_registrator()` method. You can obtain `ComponentsQueuedRegistrator` from the new `World::components_queue()`, and use it to stage component registration if desired.

* * *

### Remove various structs that implemented `Command` [#](#remove-various-structs-that-implemented-command)

Several commands have been refactored to internally use closures instead of individual structs, and their structs have been removed.

If you were queuing the structs of hierarchy-related commands directly, you will need to change them to methods implemented on `EntityCommands`:



* Struct: commands.queue(AddChild { child, parent })
  * Method: commands.entity(parent).add_child(child) OR commands.entity(child).insert(ChildOf(parent))
* Struct: commands.queue(AddChildren { children, parent })
  * Method: commands.entity(parent).add_children(children)
* Struct: commands.queue(InsertChildren { children, parent, index })
  * Method: commands.entity(parent).insert_children(index, children)
* Struct: commands.queue(RemoveChildren { children, parent })
  * Method: commands.entity(parent).remove_children(children)
* Struct: commands.queue(ReplaceChildren { children, parent })
  * Method: commands.entity(parent).replace_children(children)
* Struct: commands.queue(ClearChildren { parent })
  * Method: commands.entity(parent).remove::<Children>()
* Struct: commands.queue(RemoveParent { child })
  * Method: commands.entity(child).remove::<ChildOf>()
* Struct: commands.queue(DespawnRecursive { entity, warn: true })
  * Method: commands.entity(entity).despawn()
* Struct: commands.queue(DespawnRecursive { entity, warn: false })
  * Method: commands.entity(entity).try_despawn()
* Struct: commands.queue(DespawnChildrenRecursive { entity, warn })
  * Method: commands.entity(entity).despawn_related::<Children>()


If you were queuing the structs of event-related commands directly, you will need to change them to methods implemented on `Commands`:


|Struct                                         |Method                                  |
|-----------------------------------------------|----------------------------------------|
|commands.queue(SendEvent { event })            |commands.send_event(event)              |
|commands.queue(TriggerEvent { event, targets })|commands.trigger_targets(event, targets)|


* * *

### Refactor `ComponentHook` parameters into `HookContext` [#](#refactor-componenthook-parameters-into-hookcontext)

The function signature of component hooks (`ComponentHook`) has been simplified so that all arguments beyond the `DeferredWorld` is passed in a `HookContext`. Note that because `HookContext` is plain data with all public fields, you can use de-structuring to simplify migration.

```
// 0.15
fn my_hook(
    mut world: DeferredWorld,
    entity: Entity,
    component_id: ComponentId,
) {
    // ...
}

// 0.16
fn my_hook(
    mut world: DeferredWorld,
    HookContext { entity, component_id, caller }: HookContext,
) {
    // ...
}

```


Likewise, if you were discarding certain parameters, you can use `..` in the de-structuring:

```
// 0.15
fn my_hook(
    mut world: DeferredWorld,
    entity: Entity,
    _: ComponentId,
) {
    // ...
}

// 0.16
fn my_hook(
    mut world: DeferredWorld,
    HookContext { entity, .. }: HookContext,
) {
    // ...
}

```


* * *

### Use new relationship system for parent-child hierarchies [#](#use-new-relationship-system-for-parent-child-hierarchies)

Entity relationships are now built-in to the ECS, providing significant performance and user-experience improvements. There are several changes you may need in order to update your existing code.

First, when adding children to an entity with `EntityCommands::with_children()`, the method now passes a `ChildSpawnerCommands` type to the closure instead of a `ChildBuilder`. `ChildSpawnerCommands` is slightly different from `ChildBuilder`, but is still able to accomplish the same things as before.

```
// 0.15
commands.spawn_empty().with_children(|builder: &mut ChildBuilder<'_>| {
    // Spawn a child of the parent entity;
    builder.spawn(MyComponent(255));

    // Get the `Entity` ID of the parent.
    let parent = builder.parent_entity();

    // Queue a new `Command` to be executed.
    builder.enqueue_command(MyCommand::new(parent));
});

// 0.16
commands.spawn_empty().with_children(|spawner: &mut ChildSpawnerCommands<'_>| {
    spawner.spawn(MyComponent(255));

    // `parent_entity()` is now `target_entity()`.
    let parent = spawner.target_entity();

    // You can now access the `Commands` struct directly, which you can then use to queue commands.
    spawner.commands().queue(my_command(parent));
});

```


Furthermore, the new relationship system encourages working with the relationship components (`ChildOf`, `Children`) directly. For example, setting the parent of an entity is as simple as inserting a `ChildOf` component:

```
// 0.15
commands.spawn_empty().set_parent(parent);

// 0.16
commands.spawn_empty().insert(ChildOf(parent));

```


Replacing the children of a parent now requires removing the `Children` component and re-adding children individually:

```
// 0.15
commands.entity(parent).replace_children(&[child1, child2]);

// 0.16
commands.entity(parent)
    .remove::<Children>()
    .add_children(&[child1, child2]);

```


Despawning has also been changed to remove the complexities of `despawn_recursive()` and `despawn_descendants()` from `EntityCommands`:


|Action                     |0.15                 |0.16                                |
|---------------------------|---------------------|------------------------------------|
|Despawn parent and children|despawn_recursive()  |despawn()                           |
|Despawn children           |despawn_descendants()|despawn_related::<Children>()       |
|Despawn parent             |despawn()            |remove::<Children>(), then despawn()|


```
// 0.15
commands.entity(parent).despawn_recursive();
commands.entity(parent).despawn_descendants();
commands.entity(parent).despawn();

// 0.16
commands.entity(parent).despawn();
commands.entity(parent).despawn_related::<Children>();
commands.entity(parent).remove::<Children>().despawn();

```


Because relationships are now part of `bevy_ecs` itself, all methods from the previous `HierarchyQueryExt` extension trait are now inherent methods on `Query`. While these have mostly been migrated unchanged, `parent` is now `related` and `children` now `relationship_sources`, as these methods work for any relationship, not just parent-child ones.

* * *

### Remove `Event: Component` trait bound [#](#remove-event-component-trait-bound)

In 0.15 the `Event` trait required the `Component` trait. This bound has been removed, as it was deemed confusing for users (events aren't typically attached to entities or queried in systems).

If you require an event to implement `Component` (which usually isn't the case), you may manually derive it and update your trait bounds.

```
// 0.15
#[derive(Event)]
struct MyEvent;

fn handle_event_component<T: Event>(event_component: T) {
    // Access some `Component`-specific property of the event.
    let storage_type = T::STORAGE_TYPE;
}

// 0.16
#[derive(Event, Component)]
struct MyEvent;

fn handle_event_component<T: Event + Component>(event_component: T) {
    // Access some `Component`-specific property of the event.
    let storage_type = T::STORAGE_TYPE;
}

```


* * *

### Remove `petgraph` from `bevy::ecs` [#](#remove-petgraph-from-bevy-ecs)

Bevy's ECS no longer depends on the `petgraph` crate. As such, usage of `petgraph::graph::DiGraph` has been replaced with `bevy::ecs::schedule::graph::DiGraph`. This mainly affects code that uses the `Dag::graph()` method.

If you require the `petgraph` version of `DiGraph`, you can manually construct it by iterating over all edges and nodes in Bevy's `DiGraph`.

* * *

### Remove deprecated ECS items [#](#remove-deprecated-ecs-items)

The following `bevy::ecs` items that were deprecated in 0.15 have been removed:

*   `Events::get_reader()`
*   `Events::get_reader_current()`
*   `ManualEventReader`
*   `Condition::and_then()`
*   `Condition::or_else()`
*   `World::many_entities()`
*   `World::many_entities_mut()`
*   `World::get_many_entities()`
*   `World::get_many_entities_dynamic()`
*   `World::get_many_entities_mut()`
*   `World::get_many_entities_dynamic_mut()`
*   `World::get_many_entities_from_set_mut()`

* * *

### Remove deprecated `component_reads_and_writes()` and family [#](#remove-deprecated-component-reads-and-writes-and-family)

The following methods are now replaced by `Access::try_iter_component_access()`:

*   `Access::component_reads_and_writes()`
*   `Access::component_reads()`
*   `Access::component_writes()`

As `try_iter_component_access()` returns a `Result`, you’ll now need to handle the failing case (e.g. return early from a system). There is currently a single failure mode, `UnboundedAccess`, which occurs when the `Access` is for all `Components` _except_ certain exclusions. Since this list is infinite, there is no meaningful way for `Access` to provide an iterator. Instead, get a list of components (e.g. from the `Components` structure) and iterate over that instead, filtering using `Access::has_component_read()`, `Access::has_component_write()`, etc.

Additionally, you’ll need to `filter_map()` the accesses based on which method you’re attempting to replace:


|0.15                                |0.16                    |
|------------------------------------|------------------------|
|Access::component_reads_and_writes()|Exclusive(_) | Shared(_)|
|Access::component_reads()           |Shared(_)               |
|Access::component_writes()          |Exclusive(_)            |


To ease migration, please consider the below extension trait which you can include in your project:

```
pub trait AccessCompatibilityExt {
    /// Returns the indices of the components this has access to.
    fn component_reads_and_writes(&self) -> impl Iterator<Item = T> + '_;

    /// Returns the indices of the components this has non-exclusive access to.
    fn component_reads(&self) -> impl Iterator<Item = T> + '_;

    /// Returns the indices of the components this has exclusive access to.
    fn component_writes(&self) -> impl Iterator<Item = T> + '_;
}

impl<T: SparseSetIndex> AccessCompatibilityExt for Access<T> {
    fn component_reads_and_writes(&self) -> impl Iterator<Item = T> + '_ {
        self
            .try_iter_component_access()
            .expect("Access is unbounded. Please refactor the usage of this method to directly use try_iter_component_access")
            .filter_map(|component_access| {
                let index = component_access.index().sparse_set_index();

                match component_access {
                    ComponentAccessKind::Archetypal(_) => None,
                    ComponentAccessKind::Shared(_) => Some(index),
                    ComponentAccessKind::Exclusive(_) => Some(index),
                }
            })
    }

    fn component_reads(&self) -> impl Iterator<Item = T> + '_ {
        self
            .try_iter_component_access()
            .expect("Access is unbounded. Please refactor the usage of this method to directly use try_iter_component_access")
            .filter_map(|component_access| {
                let index = component_access.index().sparse_set_index();

                match component_access {
                    ComponentAccessKind::Archetypal(_) => None,
                    ComponentAccessKind::Shared(_) => Some(index),
                    ComponentAccessKind::Exclusive(_) => None,
                }
            })
    }

    fn component_writes(&self) -> impl Iterator<Item = T> + '_ {
        self
            .try_iter_component_access()
            .expect("Access is unbounded. Please refactor the usage of this method to directly use try_iter_component_access")
            .filter_map(|component_access| {
                let index = component_access.index().sparse_set_index();

                match component_access {
                    ComponentAccessKind::Archetypal(_) => None,
                    ComponentAccessKind::Shared(_) => None,
                    ComponentAccessKind::Exclusive(_) => Some(index),
                }
            })
    }
}

```


Please take note of the use of `expect()` in this code. You should consider using this as a starting point for a more appropriate migration based on your specific needs.

* * *

### Remove `flush_and_reserve_invalid_assuming_no_entities()` [#](#remove-flush-and-reserve-invalid-assuming-no-entities)

`Entities::flush_and_reserve_invalid_assuming_no_entities()` was a specialized method primarily used in 0.14 and earlier's rendering world design. With 0.15, `flush_and_reserve_invalid_assuming_no_entities()` went unused, so it now has been removed. If you previously required this method, you should switch to calling `Entities::reserve_entities()` and `Entities::flush_as_invalid()`.

* * *

### Remove lifetime from `QueryEntityError` [#](#remove-lifetime-from-queryentityerror)

`QueryEntityError::QueryDoesNotMatch` now stores an `ArchetypeId` instead of an `UnsafeWorldCell`. As such, `QueryEntityError`'s lifetime parameter has been removed.

* * *

### Remove unsound `Clone` impl for `EntityMutExcept` [#](#remove-unsound-clone-impl-for-entitymutexcept)

`EntityMutExcept` can no-longer be cloned, as doing so violates Rust's memory safety rules.

* * *

### Remove unused generic in `DeferredWorld::trigger()` [#](#remove-unused-generic-in-deferredworld-trigger)

In 0.15 `DeferredWorld::trigger()` had an unused generic parameter that did not affect the type of the trigger event, so it has been removed.

* * *

### Rename `EventWriter::send()` methods to `write()` [#](#rename-eventwriter-send-methods-to-write)

`EventWriter::send()` and its family of methods have been renamed to `EventWriter::write()` in order to reduce confusion and increase consistency. The old methods have been deprecated.


|0.15                       |0.16                        |
|---------------------------|----------------------------|
|EventWriter::send()        |EventWriter::write()        |
|EventWriter::send_batch()  |EventWriter::write_batch()  |
|EventWriter::send_default()|EventWriter::write_default()|


* * *

### Replace `VisitEntities` with `MapEntities` [#](#replace-visitentities-with-mapentities)

`VisitEntities` and `VisitEntitiesMut` have been removed in favor of `MapEntities`, as the prior is less generally applicable (doesn't work on collections like `HashSet`s). If you previously derived `VisitEntities` and family, you can now derive `MapEntities` and use the `#[entities]` attribute to annotate the list of `Entity`s.

```
// 0.15
#[derive(VisitEntities, VisitEntitiesMut)]
struct Inventory {
    items: Vec<Entity>,
    // Opt-out of mapping this field, as its a string.
    #[visit_entities(ignore)]
    label: String,
}

// 0.16
#[derive(MapEntities)]
struct Inventory {
    // Opt-in to mapping this field.
    #[entities]
    items: Vec<Entity>,
    label: String,
}

```


Note `Component::visit_entities()` and `Component::visit_entities_mut()` have also been removed in favor of the new `Component::map_entities()` method. When deriving `Component`, you may also use `#[entities]` to specify which `Entity`s may be mapped.

Finally, entity mapping is no longer implemented for all types that implement `IntoIterator<Item = &Entity>`. If you previously depended on a custom data type to support the `#[entities]` attribute, please manually derive / implement `MapEntities` for it.

* * *

### Run observers before hooks for `on_replace()` and `on_remove()` [#](#run-observers-before-hooks-for-on-replace-and-on-remove)

The order of hooks and observers for `on_replace()` and `on_remove()` has been swapped, so now observers are run before hooks. As hooks are more primitive, they are designated as the first and last thing run when a component is added and removed. The total order for component removal can now be seen in the following table:


|0.15                 |0.16                 |
|---------------------|---------------------|
|on_replace() hook    |on_replace() observer|
|on_replace() observer|on_replace() hook    |
|on_remove() hook     |on_remove() observer |
|on_remove() observer |on_remove() hook     |


* * *

### Shorten the lifetime returned from `QueryLens::query()` [#](#shorten-the-lifetime-returned-from-querylens-query)

There was a lifetime issue found with `QueryLens::query()` where calling `get_inner()` on the returned value would allow for multiple mutable references to the same entity. This has been fixed by shrinking the lifetime of `QueryLens::query()`'s result, however it may break existing code.

If you run into lifetime issues while calling `get_inner()` or `iter_inner()` on `QueryLens::query()`'s result, you may need to switch to the new `QueryLens::query_inner()` method that only works on immutable queries.

* * *

### Split `Component::register_component_hooks()` into individual methods [#](#split-component-register-component-hooks-into-individual-methods)

Component hook registration is now split out into individual methods of `Component`. The original `Component::register_component_hooks()` has been deprecated, so please switch to the new `Component::on_add()`, `Component::on_remove()`, and related methods.

```
// 0.15
impl Component for Foo {
    fn register_component_hooks(hooks: &mut ComponentHooks) {
        hooks.on_add(foo_on_add);
    }

    // ...
}

// 0.16
impl Component for Foo {
    fn on_add() -> Option<ComponentHook> {
        Some(foo_on_add)
    }

    // ...
}

```


* * *

### Turn `apply_deferred()` into a ZST system [#](#turn-apply-deferred-into-a-zst-system)

The special `apply_deferred()` system has been converted into the zero-sized `ApplyDeferred` type that implements `System` for performance reasons. If you manually schedule `apply_deferred()`, which usually isn't the case, you will need to use the new `ApplyDeferred` type instead. If you manually called `apply_deferred()` without your code, you may delete it, as that function did nothing.

* * *

### Replace `register_dynamic()` with `register_dynamic_with()` [#](#replace-register-dynamic-with-register-dynamic-with)

`RequiredComponents::register_dynamic()` has been replaced by `RequiredComponents::register_dynamic_with()`, which avoids unnecessary cloning.

```
// 0.15
required_components.register_dynamic(
    component_id,
    component_constructor.clone(),
    requirement_inheritance_depth,
);

// 0.16
required_components.register_dynamic_with(
    component_id,
    requirement_inheritance_depth,
    || component_constructor.clone(),
);

```


* * *

### add Entity default to the entity set wrappers [#](#add-entity-default-to-the-entity-set-wrappers)

Switch type parameter order for the relevant wrapper types/aliases.

* * *

### impl EntityBorrow for more types [#](#impl-entityborrow-for-more-types)

`NormalizedWindowRef::entity` has been replaced with an `EntityBorrow::entity` impl.

* * *

### implement EntitySet and iter\_many\_unique methods [#](#implement-entityset-and-iter-many-unique-methods)

Any custom type used as a `Borrow<Entity>` entity list item for an `iter_many` method now has to implement `EntityBorrow` instead. Any type that implements `Borrow<Entity>` can trivially implement `EntityBorrow`.

* * *

### make EntityHashMap and EntityHashSet proper types [#](#make-entityhashmap-and-entityhashset-proper-types)

Users of `with_hasher` and `with_capacity_and_hasher` on `EntityHashMap`/`Set` must now use `new` and `with_capacity` respectively. If the non-newtyped versions are required, they can be obtained via `Deref`, `DerefMut` or `into_inner` calls.

* * *

### one shot system cleanup [#](#one-shot-system-cleanup)

*   Change all occurrences of `World::run_system_with_input` to `World::run_system_with`.
*   swap the order of input parameters for `World::run_system_once_with` such that the system comes before the input.

* * *

### Make `ComponentTicks` field public [#](#make-componentticks-field-public)

*   Instead of using `ComponentTicks::last_changed_tick` and `ComponentTicks::added_tick` methods, access fields directly.

* * *

### Add Immutable `Component` Support [#](#add-immutable-component-support)

*   When implementing `Component` manually, you must now provide a type for `Mutability`. The type `Mutable` provides equivalent behaviour to earlier versions of `Component`:

```
impl Component for Foo {
    type Mutability = Mutable;
    // ...
}

```


*   When working with generic components, you may need to specify that your generic parameter implements `Component<Mutability = Mutable>` rather than `Component` if you require mutable access to said component.
*   The entity entry API has had to have some changes made to minimise friction when working with immutable components. Methods which previously returned a `Mut<T>` will now typically return an `OccupiedEntry<T>` instead, requiring you to add an `into_mut()` to get the `Mut<T>` item again.

* * *

### Faster entity cloning [#](#faster-entity-cloning)

*   `&EntityCloner` in component clone handlers is changed to `&mut ComponentCloneCtx` to better separate data.
*   Changed `EntityCloneHandler` from enum to struct and added convenience functions to add default clone and reflect handler more easily.

* * *

### FilteredResource returns a Result instead of a simple Option [#](#filteredresource-returns-a-result-instead-of-a-simple-option)

Users will need to handle the different return type on FilteredResource::get, FilteredResource::get\_id, FilteredResource::get\_mut as it is now a Result not an Option.

* * *

### `ReflectBundle::remove` improvement [#](#reflectbundle-remove-improvement)

If you don’t need the returned value from `remove`, discard it.

* * *

### Improved `#[derive(Event)]` [#](#improved-derive-event)

In **Bevy 0.16** you can now use `#[derive(Event)]` for more specialized implementations.

```
// 0.15
struct MyEvent;

impl Event for MyEvent {
    const AUTO_PROPAGATE: bool = true;
    type Traversal = &'static ChildOf
}

// 0.16
#[derive(Event)]
#[event(traversal = &'static ChildOf, auto_propagate)]
struct MyEvent;

```


Input [#](#input)
-----------------

### Gamepad improvements [#](#gamepad-improvements)

*   `Gamepad` fields are now public.
*   Instead of using `Gamepad` delegates like `Gamepad::just_pressed`, call these methods directly on the fields.

* * *

### Scale input to account for deadzones [#](#scale-input-to-account-for-deadzones)

`GamepadButtonChangedEvent.value` is now linearly rescaled to be from `0.0..=1.0` (instead of `low..=high`) and `GamepadAxisChangedEvent.value` is now linearly rescaled to be from `-1.0..=0.0`/`0.0..=1.0` (accounting for the deadzone).

* * *

### Use `Name` component for gamepad [#](#use-name-component-for-gamepad)

*   `GamepadInfo` no longer exists:
    *   Name now accessible via `Name` component.
    *   Other information available on `Gamepad` component directly.
    *   `GamepadConnection::Connected` now stores all info fields directly.

* * *

### Expose `text` field from winit in `KeyboardInput` [#](#expose-text-field-from-winit-in-keyboardinput)

The `KeyboardInput` event now has a new `text` field.

Math [#](#math)
---------------

### Fix atan2 docs [#](#fix-atan2-docs)

I’m not sure if this counts as a breaking change, since the implementation clearly meant to use `f32::atan2` directly, so it was really just the parameter names that were wrong.

* * *

### Fix rounding in steps easing function [#](#fix-rounding-in-steps-easing-function)

`EaseFunction::Steps` now behaves like css’s default, “jump-end.” If you were relying on the old behavior, we plan on providing it. See [https://github.com/bevyengine/bevy/issues/17744](https://github.com/bevyengine/bevy/issues/17744).

* * *

### Improve cubic segment bezier functionality [#](#improve-cubic-segment-bezier-functionality)

Replace `CubicCurve::new_bezier` with `CubicCurve::new_bezier_easing`.

* * *

### Refactor non-core Curve methods into extension traits [#](#refactor-non-core-curve-methods-into-extension-traits)

`Curve` has been refactored so that much of its functionality is now in extension traits. Adaptors such as `map`, `reparametrize`, `reverse`, and so on now require importing `CurveExt`, while the resampling methods `resample_*` require importing `CurveResampleExt`. Both of these new traits are exported through `bevy::math::curve` and through `bevy::math::prelude`.

* * *

### Rename `Rot2::angle_between` to `Rot2::angle_to` [#](#rename-rot2-angle-between-to-rot2-angle-to)

`Rot2::angle_between` has been deprecated, use `Rot2::angle_to` instead, the semantics of `Rot2::angle_between` will change in the future.

* * *

### Reworked Segment types into their cartesian forms [#](#reworked-segment-types-into-their-cartesian-forms)

The segment type constructors changed so if someone previously created a Segment2d with a direction and length they would now need to use the `from_direction` constructor

* * *

### Use `IntoIterator` instead of `Into<Vec<..>>` in cubic splines interfaces [#](#use-intoiterator-instead-of-into-vec-in-cubic-splines-interfaces)

The cubic splines API now uses `IntoIterator` in places where it used `Into<Vec<..>>`. For most users, this will have little to no effect (it is largely more permissive). However, in case you were using some unusual input type that implements `Into<Vec<..>>` without implementing `IntoIterator`, you can migrate by converting the input to a `Vec<..>` before passing it into the interface.

* * *

### \[math\] Add `SmoothStep` and `SmootherStep` easing functions [#](#math-add-smoothstep-and-smootherstep-easing-functions)

This version of bevy marks `EaseFunction` as `#[non_exhaustive]` to that future changes to add more easing functions will be non-breaking. If you were exhaustively matching that enum – which you probably weren’t – you’ll need to add a catch-all (`_ =>`) arm to cover unknown easing functions.

* * *

### Make `bevy_reflect` feature of `bevy_math` non-default [#](#make-bevy-reflect-feature-of-bevy-math-non-default)

`bevy_reflect` has been made a non-default feature of `bevy_math`. (It is still enabled when `bevy_math` is used through `bevy`.) You may need to enable this feature if you are using `bevy_math` on its own and desire for the types it exports to implement `Reflect` and other reflection traits.

Picking [#](#picking)
---------------------

### Add flags to `SpritePlugin` and `UiPlugin` to allow disabling their picking backend (without needing to disable features). [#](#add-flags-to-spriteplugin-and-uiplugin-to-allow-disabling-their-picking-backend-without-needing-to-disable-features)

*   `UiPlugin` now contains an extra `add_picking` field if `bevy_ui_picking_backend` is enabled.
*   `SpritePlugin` is no longer a unit struct, and has one field if `bevy_sprite_picking_backend` is enabled (otherwise no fields).

* * *

### Add optional transparency passthrough for sprite backend with bevy\_picking [#](#add-optional-transparency-passthrough-for-sprite-backend-with-bevy-picking)

Sprite picking now ignores transparent regions (with an alpha value less than or equal to 0.1). To configure this, modify the `SpriteBackendSettings` resource.

* * *

### Allow users to easily use `bevy_sprite` and `bevy_ui` without picking [#](#allow-users-to-easily-use-bevy-sprite-and-bevy-ui-without-picking)

`bevy_sprite_picking_backend` is no longer included by default when using the `bevy_sprite` feature. If you are using Bevy without default features and relied on sprite picking, add this feature to your `Cargo.toml`.

`bevy_ui_picking_backend` is no longer included by default when using the `bevy_ui` feature. If you are using Bevy without default features and relied on sprite picking, add this feature to your `Cargo.toml`.

* * *

### Fix `bevy_picking` plugin suffixes [#](#fix-bevy-picking-plugin-suffixes)

*   `MeshPickingBackend` is now named `MeshPickingPlugin`.
*   `MeshPickingBackendSettings` is now named `MeshPickingSettings`.
*   `SpritePickingBackend` is now named `SpritePickingPlugin`.
*   `UiPickingBackendPlugin` is now named `UiPickingPlugin`.
*   `DefaultPickingPlugins` is now a a `PluginGroup` instead of a `Plugin`.

* * *

### Flattened `PointerAction::Pressed` into `Press` and `Release`. [#](#flattened-pointeraction-pressed-into-press-and-release)

*   `PointerAction::Pressed` has been separated into two variants, `PointerAction::Press` and `PointerAction::Release`.
*   `PointerAction::Moved` has been renamed to `PointerAction::Move`.
*   `PointerAction::Canceled` has been renamed to `PointerAction::Cancel`.

* * *

### If there is no movement, DragStart is not triggered. [#](#if-there-is-no-movement-dragstart-is-not-triggered)

Fix the missing part of Drag [https://github.com/bevyengine/bevy/pull/16950](https://github.com/bevyengine/bevy/pull/16950)

* * *

### Make RayMap map public [#](#make-raymap-map-public)

The `bevy_picking::backend::ray::RayMap::map` method is removed as redundant, In systems using `Res<RayMap>` replace `ray_map.map()` with `&ray_map.map`

* * *

### Make sprite picking opt-in [#](#make-sprite-picking-opt-in)

The sprite picking backend is now strictly opt-in using the `SpritePickingCamera` and `Pickable` components. You should add the `Pickable` component any entities that you want sprite picking to be enabled for, and mark their respective cameras with `SpritePickingCamera`.

* * *

### Make sprite picking opt-in [#](#make-sprite-picking-opt-in-1)

*   Sprite picking are now opt-in, make sure you insert `Pickable` component when using sprite picking.

```
-commands.spawn(Sprite { .. } );
+commands.spawn((Sprite { .. }, Pickable::default());

```


* * *

### Rename "focus" in `bevy_picking` to "hover" [#](#rename-focus-in-bevy-picking-to-hover)

Various terms related to “focus” in `bevy_picking` have been renamed to refer to “hover” to avoid confusion with `bevy_input_focus`. In particular:

*   The `update_focus` system has been renamed to `generate_hovermap`
*   `PickSet::Focus` and `PostFocus` have been renamed to `Hover` and `PostHover`
*   The `bevy_picking::focus` module has been renamed to `bevy_picking::hover`
*   The `is_focus_enabled` field on `PickingPlugin` has been renamed to `is_hover_enabled`
*   The `focus_should_run` run condition has been renamed to `hover_should_run`

* * *

### Rename `Pointer<Down/Up>` to `Pointer<Pressed/Released>` in `bevy_picking` [#](#rename-pointer-down-up-to-pointer-pressed-released-in-bevy-picking)

#### `bevy_picking/src/pointer.rs` [#](#bevy-picking-src-pointer-rs)

**`enum PressDirection`:**

*   `PressDirection::Down` changes to `PressDirection::Pressed`.
*   `PressDirection::Up` changes to `PressDirection::Released`.

These changes are also relevant when working with `enum PointerAction`

#### `bevy_picking/src/events.rs` [#](#bevy-picking-src-events-rs)

Clicking and pressing Events in events.rs categories change from \[Down\], \[Up\], \[Click\] to \[Pressed\], \[Released\], \[Click\].

*   `struct Down` changes to `struct Pressed` - fires when a pointer button is pressed over the ‘target’ entity.
*   `struct Up` changes to `struct Released` - fires when a pointer button is released over the ‘target’ entity.
*   `struct Click` now fires when a pointer sends a Pressed event followed by a Released event on the same ‘target’.
*   `struct DragStart` now fires when the ‘target’ entity receives a pointer Pressed event followed by a pointer Move event.
*   `struct DragEnd` now fires when the ‘target’ entity is being dragged and receives a pointer Released event.
*   `PickingEventWriters<'w>::down_events: EventWriter<'w, Pointer<Down>>` changes to `PickingEventWriters<'w>::pressed_events: EventWriter<'w, Pointer<Pressed>>`.
*   `PickingEventWriters<'w>::up_events changes to PickingEventWriters<'w>::released_events`.

* * *

### Rename `PickingBehavior` to `Pickable` [#](#rename-pickingbehavior-to-pickable)

Change all instances of `PickingBehavior` to `Pickable`.

* * *

### Rename `RayCastSettings` to `MeshRayCastSettings` [#](#rename-raycastsettings-to-meshraycastsettings)

`RayCastSettings` has been renamed to `MeshRayCastSettings` to avoid naming conflicts with other ray casting backends and types.

* * *

### Unify picking backends [#](#unify-picking-backends)

`UiPickingPlugin` and `SpritePickingPlugin` are no longer included in `DefaultPlugins`. They must be explicitly added.

`RayCastPickable` has been replaced in favor of the `MeshPickingCamera` and `Pickable` components. You should add them to cameras and entities, respectively, if you have `MeshPickingSettings::require_markers` set to `true`.

Reflection [#](#reflection)
---------------------------

### Include ReflectFromReflect in all dynamic data types. [#](#include-reflectfromreflect-in-all-dynamic-data-types)

The hasher in reflected `HashMap`s and `HashSet`s now have to implement `Default`. This is the case for the ones provided by Bevy already, and is generally a sensible thing to do.

* * *

### Make `bevy_remote` feature enable `serialize` feature [#](#make-bevy-remote-feature-enable-serialize-feature)

The `bevy_remote` feature of `bevy` now enables the `serialize` feature automatically. If you wish to use `bevy_remote` without enabling the `serialize` feature for Bevy subcrates, you must import `bevy_remote` on its own.

* * *

### Rename `ArgList::push` methods to `with` and add new `push` methods which take `&mut self` [#](#rename-arglist-push-methods-to-with-and-add-new-push-methods-which-take-mut-self)

Uses of the `ArgList::push` methods should be replaced with the `with` counterpart.


|old       |new       |
|----------|----------|
|push_arg  |with_arg  |
|push_ref  |with_ref  |
|push_mut  |with_mut  |
|push_owned|with_owned|
|push_boxed|with_boxed|


* * *

### bevy\_reflect: Deprecate `PartialReflect::clone_value` [#](#bevy-reflect-deprecate-partialreflect-clone-value)

`PartialReflect::clone_value` is being deprecated. Instead, use `PartialReflect::to_dynamic` if wanting to create a new dynamic instance of the reflected value. Alternatively, use `PartialReflect::reflect_clone` to attempt to create a true clone of the underlying value.

Similarly, the following methods have been deprecated and should be replaced with these alternatives:

*   `Array::clone_dynamic` → `Array::to_dynamic_array`
*   `Enum::clone_dynamic` → `Enum::to_dynamic_enum`
*   `List::clone_dynamic` → `List::to_dynamic_list`
*   `Map::clone_dynamic` → `Map::to_dynamic_map`
*   `Set::clone_dynamic` → `Set::to_dynamic_set`
*   `Struct::clone_dynamic` → `Struct::to_dynamic_struct`
*   `Tuple::clone_dynamic` → `Tuple::to_dynamic_tuple`
*   `TupleStruct::clone_dynamic` → `TupleStruct::to_dynamic_tuple_struct`

* * *

### bevy\_reflect: Remove `PartialReflect::serializable` [#](#bevy-reflect-remove-partialreflect-serializable)

`PartialReflect::serializable` has been removed. If you were using this to pass on serialization information, use `ReflectSerialize` instead or create custom type data to generate the `Serializable`.

* * *

### Remove unnecessary `PartialReflect` bound on `DeserializeWithRegistry` [#](#remove-unnecessary-partialreflect-bound-on-deserializewithregistry)

`DeserializeWithRegistry` types are no longer guaranteed to be `PartialReflect` as well. If you were relying on this type bound, you should add it to your own bounds manually.

```
- impl<T: DeserializeWithRegistry> Foo for T { .. }
+ impl<T: DeserializeWithRegistry + PartialReflect> Foo for T { .. }

```


Rendering [#](#rendering)
-------------------------

### Add `uv_transform` to `ColorMaterial` [#](#add-uv-transform-to-colormaterial)

Add `uv_transform` field to constructors of `ColorMaterial`

* * *

### Add a bindless mode to `AsBindGroup`. [#](#add-a-bindless-mode-to-asbindgroup)

*   `RenderAssets::prepare_asset` now takes an `AssetId` parameter.
*   Bin keys now have Bevy-specific material bind group indices instead of `wgpu` material bind group IDs, as part of the bindless change. Use the new `MaterialBindGroupAllocator` to map from bind group index to bind group ID.

* * *

### Add bevy\_anti\_aliasing [#](#add-bevy-anti-aliasing)

When using anti aliasing features, you now need to import them from `bevy::anti_aliasing` instead of `bevy::core_pipeline`

* * *

### Adding alpha\_threshold to OrderIndependentTransparencySettings for user-level optimization [#](#adding-alpha-threshold-to-orderindependenttransparencysettings-for-user-level-optimization)

If you previously explicitly initialized OrderIndependentTransparencySettings with your own `layer_count`, you will now have to add either a `..default()` statement or an explicit `alpha_threshold` value:

```
fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        OrderIndependentTransparencySettings {
            layer_count: 16,
            ..default()
        },
    ));
}

```


* * *

### Allowed creating uninitialized images (for use as storage textures) [#](#allowed-creating-uninitialized-images-for-use-as-storage-textures)

Code that directly access `Image` data will now need to use unwrap or handle the case where no data is provided. Behaviour of new\_fill slightly changed, but not in a way that is likely to affect anything. It no longer panics and will fill the whole texture instead of leaving black pixels if the data provided is not a nice factor of the size of the image.

* * *

### Bind only the written parts of storage buffers. [#](#bind-only-the-written-parts-of-storage-buffers)

*   Fixed a bug with StorageBuffer and DynamicStorageBuffer binding data from the previous frame(s) due to caching GPU buffers between frames.

* * *

### Change `GpuImage::size` from `UVec2` to `Extent3d` [#](#change-gpuimage-size-from-uvec2-to-extent3d)

*   `GpuImage::size` is now an `Extent3d`. To easily get 2D size, use `size_2d()`.

* * *

### Cold Specialization [#](#cold-specialization)

TODO

*   `AssetEvents` has been moved into the `PostUpdate` schedule.

* * *

### Expose Pipeline Compilation Zero Initialize Workgroup Memory Option [#](#expose-pipeline-compilation-zero-initialize-workgroup-memory-option)

*   add `zero_initialize_workgroup_memory: false,` to `ComputePipelineDescriptor` or `RenderPipelineDescriptor` structs to preserve 0.14 functionality, add `zero_initialize_workgroup_memory: true,` to restore bevy 0.13 functionality.

* * *

### Fix sprite performance regression since retained render world [#](#fix-sprite-performance-regression-since-retained-render-world)

*   `ExtractedSprites` is now using `MainEntityHashMap` for storage, which is keyed on `MainEntity`.
*   The render world entity corresponding to an `ExtractedSprite` is now stored in the `render_entity` member of it.

* * *

### Fix the `texture_binding_array`, `specialized_mesh_pipeline`, and `custom_shader_instancing` examples after the bindless change. [#](#fix-the-texture-binding-array-specialized-mesh-pipeline-and-custom-shader-instancing-examples-after-the-bindless-change)

*   Bevy will now unconditionally call `AsBindGroup::unprepared_bind_group` for your materials, so you must no longer panic in that function. Instead, return the new `AsBindGroupError::CreateBindGroupDirectly` error, and Bevy will fall back to calling `AsBindGroup::as_bind_group` as before.

* * *

### Implement bindless lightmaps. [#](#implement-bindless-lightmaps)

*   The `Opaque3dBinKey::lightmap_image` field is now `Opaque3dBinKey::lightmap_slab`, which is a lightweight identifier for an entire binding array of lightmaps.

* * *

### Implement experimental GPU two-phase occlusion culling for the standard 3D mesh pipeline. [#](#implement-experimental-gpu-two-phase-occlusion-culling-for-the-standard-3d-mesh-pipeline)

*   When enqueuing a custom mesh pipeline, work item buffers are now created with `bevy::render::batching::gpu_preprocessing::get_or_create_work_item_buffer`, not `PreprocessWorkItemBuffers::new`. See the `specialized_mesh_pipeline` example.

* * *

### Introduce support for mixed lighting by allowing lights to opt out of contributing diffuse light to lightmapped objects. [#](#introduce-support-for-mixed-lighting-by-allowing-lights-to-opt-out-of-contributing-diffuse-light-to-lightmapped-objects)

*   The `AmbientLight` resource, the `IrradianceVolume` component, and the `EnvironmentMapLight` component now have `affects_lightmapped_meshes` fields. If you don’t need to use that field (for example, if you aren’t using lightmaps), you can safely set the field to true.
*   `DirectionalLight`, `PointLight`, and `SpotLight` now have `affects_lightmapped_mesh_diffuse` fields. If you don’t need to use that field (for example, if you aren’t using lightmaps), you can safely set the field to true.

* * *

### Introduce two-level bins for multidrawable meshes. [#](#introduce-two-level-bins-for-multidrawable-meshes)

*   The _batch set key_ is now separate from the _bin key_ in `BinnedPhaseItem`. The batch set key is used to collect multidrawable meshes together. If you aren’t using the multidraw feature, you can safely set the batch set key to `()`.

* * *

### Key render phases off the main world view entity, not the render world view entity. [#](#key-render-phases-off-the-main-world-view-entity-not-the-render-world-view-entity)

* * *

### Make indirect drawing opt-out instead of opt-in, enabling multidraw by default. [#](#make-indirect-drawing-opt-out-instead-of-opt-in-enabling-multidraw-by-default)

*   Indirect drawing (GPU culling) is now enabled by default, so the `GpuCulling` component is no longer available. To disable indirect mode, which may be useful with custom render nodes, add the new `NoIndirectDrawing` component to your camera.

* * *

### Make the `get` function on `InstanceInputUniformBuffer` less error prone [#](#make-the-get-function-on-instanceinputuniformbuffer-less-error-prone)

`InstanceInputUniformBuffer::get` now returns `Option<BDI>` instead of `BDI` to reduce panics. If you require the old functionality of `InstanceInputUniformBuffer::get` consider using `InstanceInputUniformBuffer::get_unchecked`.

* * *

### Make the default directional light shadow cascade settings similar to those of other engines. [#](#make-the-default-directional-light-shadow-cascade-settings-similar-to-those-of-other-engines)

*   The default shadow cascade far distance has been changed from 1000 to 150, and the default first cascade far bound has been changed from 5 to 10, in order to be similar to the defaults of other engines.

* * *

### Mesh::merge to return a Result [#](#mesh-merge-to-return-a-result)

*   `Mesh::merge` now returns a `Result<(), MeshMergeError>`.

* * *

### Move `TextureAtlas` and friends into `bevy_image` [#](#move-textureatlas-and-friends-into-bevy-image)

The following types have been moved from `bevy_sprite` to `bevy_image`: `TextureAtlas`, `TextureAtlasBuilder`, `TextureAtlasSources`, `TextureAtlasLayout` and `DynamicTextureAtlasBuilder`.

If you are using the `bevy` crate, and were importing these types directly (e.g. before `use bevy::sprite::TextureAtlas`), be sure to update your import paths (e.g. after `use bevy::image::TextureAtlas`)

If you are using the `bevy` prelude to import these types (e.g. `use bevy::prelude::*`), you don’t need to change anything.

If you are using the `bevy_sprite` subcrate, be sure to add `bevy_image` as a dependency if you do not already have it, and be sure to update your import paths.

* * *

### Move non-generic parts of the PrepassPipeline to internal field [#](#move-non-generic-parts-of-the-prepasspipeline-to-internal-field)

If you were using a field of the `PrepassPipeline`, most of them have now been move to `PrepassPipeline::internal`.

* * *

### Native unclipped depth on supported platforms [#](#native-unclipped-depth-on-supported-platforms)

*   `MeshPipelineKey::DEPTH_CLAMP_ORTHO` is now `MeshPipelineKey::UNCLIPPED_DEPTH_ORTHO`
*   The `DEPTH_CLAMP_ORTHO` shaderdef has been renamed to `UNCLIPPED_DEPTH_ORTHO_EMULATION`
*   `clip_position_unclamped: vec4<f32>` is now `unclipped_depth: f32`

* * *

### Newtype `Anchor` [#](#newtype-anchor)

The anchor component has been changed from an enum to a struct newtyping a `Vec2`. The `Custom` variant has been removed, instead to construct a custom `Anchor` use its tuple constructor:

```
Sprite {
     anchor: Anchor(Vec2::new(0.25, 0.4)),
     ..default()
}

```


The other enum variants have been replaced with corresponding constants:

*   `Anchor::BottomLeft` to `Anchor::BOTTOM_LEFT`
*   `Anchor::Center` to `Anchor::CENTER`
*   `Anchor::TopRight` to `Anchor::TOP_RIGHT`
*   .. and so on for the remaining variants

* * *

### Only use the AABB center for mesh visibility range testing if specified. [#](#only-use-the-aabb-center-for-mesh-visibility-range-testing-if-specified)

*   The `VisibilityRange` component now has an extra field, `use_aabb`. Generally, you can safely set it to false.

* * *

### Reduce the clusterable object UBO size below 16384 for WebGL 2. [#](#reduce-the-clusterable-object-ubo-size-below-16384-for-webgl-2)

`MAX_UNIFORM_BUFFER_CLUSTERABLE_OBJECTS` has been reduced from `256` to `204`.

* * *

### Refactor and simplify custom projections [#](#refactor-and-simplify-custom-projections)

*   `PerspectiveProjection` and `OrthographicProjection` are no longer components. Use `Projection` instead.
*   Custom projections should no longer be inserted as a component. Instead, simply set the custom projection as a value of `Projection` with `Projection::custom()`.

* * *

### Remove the type parameter from `check_visibility` and `VisibleEntities` [#](#remove-the-type-parameter-from-check-visibility-and-visibleentities)

`check_visibility` no longer takes a `QueryFilter`, and there’s no need to add it manually to your app schedule anymore for custom rendering items. Instead, entities with custom renderable components should add the appropriate type IDs to `VisibilityClass`. See `custom_phase_item` for an example.

Similarly, all methods on `VisibleEntities` (such as `get` and `iter`) no longer take a generic parameter, and instead must be passed a `TypeId` corresponding to the component used in the `VisibilityClass` of the entity.

Before:

```
visible_entities.get_mut<With<Mesh3d>>();

```


After:

```
visible_entities.get_mut(TypeId::of::<Mesh3d>());

```


* * *

### Support scale factor for image render targets [#](#support-scale-factor-for-image-render-targets)

`RenderTarget::Image` now takes an `ImageRenderTarget` instead of a `Handle<Image>`. You can call `handle.into()` to construct an `ImageRenderTarget` using the same settings as before.

* * *

### Upgrade to wgpu v24 [#](#upgrade-to-wgpu-v24)

*   Bevy has upgraded to [wgpu v24](https://github.com/gfx-rs/wgpu/blob/trunk/CHANGELOG.md#v2400-2025-01-15).
*   When using the DirectX 12 rendering backend, the new priority system for choosing a shader compiler is as follows:
    *   If the `WGPU_DX12_COMPILER` environment variable is set at runtime, it is used
    *   Else if the new `statically-linked-dxc` feature is enabled, a custom version of DXC will be statically linked into your app at compile time.
    *   Else Bevy will look in the app’s working directory for `dxcompiler.dll` and `dxil.dll` at runtime.
    *   Else if they are missing, Bevy will fall back to FXC (not recommended)

* * *

### Use `multi_draw_indirect_count` where available, in preparation for two-phase occlusion culling. [#](#use-multi-draw-indirect-count-where-available-in-preparation-for-two-phase-occlusion-culling)

*   Systems that add custom phase items now need to populate the indirect drawing-related buffers. See the `specialized_mesh_pipeline` example for an example of how this is done.

* * *

### Use unchecked shaders for better performance [#](#use-unchecked-shaders-for-better-performance)

*   Bevy no longer turns on wgpu’s runtime safety checks https://docs.rs/wgpu/latest/wgpu/struct.ShaderRuntimeChecks.html. If you were using Bevy with untrusted shaders, please file an issue.

* * *

### cleanup bevy\_render/lib.rs [#](#cleanup-bevy-render-lib-rs)

`RenderCreation::Manual` variant fields are now wrapped in a struct called `RenderResources`

* * *

### `ExtractedSprites` slice buffer [#](#extractedsprites-slice-buffer)

*   `ExtractedSprite` has a new `kind: ExtractedSpriteKind` field with variants `Single` and `Slices`.
    
    *   `Single` represents a single sprite. `ExtractedSprite`’s `anchor`, `rect`, `scaling_mode` and `custom_size` fields have been moved into `Single`.
    *   `Slices` contains a range that indexes into a new resource `ExtractedSlices`. Slices are used to draw elements composed from multiple sprites such as text or nine-patched borders.
*   `ComputedTextureSlices::extract_sprites` has been renamed to `extract_slices`. Its `transform` and `original_entity` parameters have been removed.
    

* * *

### Improved `UiImage` and `Sprite` scaling and slicing APIs [#](#improved-uiimage-and-sprite-scaling-and-slicing-apis)

The `ImageScaleMode` component has been removed. Instead, `SpriteImageMode` and `NodeImageMode` have been created for a new field `image_mode` on both `Sprite` and `UiImage`

In most cases, this means code that spawns an entity with

```
(
    UiImage::new(image.clone()),
    ImageScaleMode::Sliced(slicer.clone()),
)

```


should be converted to:

```
(
    UiImage::new(image.clone())
        .with_mode(NodeImageMode::Sliced(slicer.clone())),
)

```


* * *

### Rename `DefaultCameraView` [#](#rename-defaultcameraview)

`DefaultCameraView` has been renamed to `UiCameraView`

* * *

### Rename `TargetCamera` to `UiTargetCamera` [#](#rename-targetcamera-to-uitargetcamera)

`TargetCamera` has been renamed to `UiTargetCamera`.

* * *

### `BorderRect` maintenance [#](#borderrect-maintenance)

The `square` and `rectangle` functions belonging to `BorderRect` have been renamed to `all` and `axes`.

Scenes [#](#scenes)
-------------------

### Only despawn scene entities still in the hierarchy [#](#only-despawn-scene-entities-still-in-the-hierarchy)

If you previously relied on scene entities no longer in the hierarchy being despawned when the scene root is despawned , use `SceneSpawner::despawn_instance()` instead.

Tasks [#](#tasks)
-----------------

### Support `on_thread_spawn` and `on_thread_destroy` for `TaskPoolPlugin` [#](#support-on-thread-spawn-and-on-thread-destroy-for-taskpoolplugin)

*   `TaskPooolThreadAssignmentPolicy` now has two additional fields: `on_thread_spawn` and `on_thread_destroy`. Please consider defaulting them to `None`.

Text [#](#text)
---------------

### Add byte information to `PositionedGlyph` [#](#add-byte-information-to-positionedglyph)

`PositionedGlyph::new()` has been removed as there is no longer an unused field. Create new `PositionedGlyph`s directly.

* * *

### Remove the `atlas_scaling` field from `ExtractedUiItem::Glyphs`. [#](#remove-the-atlas-scaling-field-from-extracteduiitem-glyphs)

The `atlas_scaling` field from `ExtractedUiItem::Glyphs` has been removed. This shouldn’t affect any existing code as it wasn’t used for anything.

* * *

### add line height to `TextFont` [#](#add-line-height-to-textfont)

`TextFont` now has a `line_height` field. Any instantiation of `TextFont` that doesn’t have `..default()` will need to add this field.

UI [#](#ui)
-----------

### Fixing `ValArithmeticError` typo and unused variant [#](#fixing-valarithmeticerror-typo-and-unused-variant)

*   `ValArithmeticError::NonEvaluateable` has been renamed to `NonEvaluateable::NonEvaluable`
*   `ValArithmeticError::NonIdenticalVariants` has been removed

* * *

### Move TextureAtlas into UiImage and remove impl Component for TextureAtlas [#](#move-textureatlas-into-uiimage-and-remove-impl-component-for-textureatlas)

Before:

```
commands.spawn((
  UiImage::new(image),
  TextureAtlas { index, layout },
));

```


After:

```
commands.spawn(UiImage::from_atlas_image(image, TextureAtlas { index, layout }));

```


Before:

```
commands.spawn(UiImage {
    texture: some_image,
    ..default()
})

```


After:

```
commands.spawn(UiImage {
    image: some_image,
    ..default()
})

```


* * *

### Multiple box shadow support [#](#multiple-box-shadow-support)

Bevy UI now supports multiple shadows per node. A new struct `ShadowStyle` is used to set the style for each shadow. And the `BoxShadow` component is changed to a tuple struct wrapping a vector containing a list of `ShadowStyle`s. To spawn a node with a single shadow you can use the `new` constructor function:

```
commands.spawn((
    Node::default(),
    BoxShadow::new(
        Color::BLACK.with_alpha(0.8),
        Val::Percent(offset.x),
        Val::Percent(offset.y),
        Val::Percent(spread),
        Val::Px(blur),
    )
));

```


* * *

### Only use physical coords internally in `bevy_ui` [#](#only-use-physical-coords-internally-in-bevy-ui)

`ComputedNode`’s fields and methods now use physical coordinates. `ComputedNode` has a new field `inverse_scale_factor`. Multiplying the physical coordinates by the `inverse_scale_factor` will give the logical values.

* * *

### Remove custom rounding [#](#remove-custom-rounding)

`UiSurface::get_layout` now also returns the final sizes before rounding. Call `.0` on the `Ok` result to get the previously returned `taffy::Layout` value.

* * *

### Remove the `min` and `max` fields from `LayoutContext`. [#](#remove-the-min-and-max-fields-from-layoutcontext)

The `min` and `max` fields have been removed from `LayoutContext`. To retrieve these values call `min_element` and `max_element` on `LayoutContent::physical_size` instead.

* * *

### Rename `UiBoxShadowSamples` to `BoxShadowSamples`. [#](#rename-uiboxshadowsamples-to-boxshadowsamples)

`UiBoxShadowSamples` has been renamed to `BoxShadowSamples`

* * *

### UiImage -> ImageNode, UiImageSize -> ImageNodeSize [#](#uiimage-imagenode-uiimagesize-imagenodesize)

Before:

```
commands.spawn(UiImage::new(image));

```


After:

```
commands.spawn(ImageNode::new(image));

```


Windowing [#](#windowing)
-------------------------

### Make CustomCursor variants CustomCursorImage/CustomCursorUrl structs [#](#make-customcursor-variants-customcursorimage-customcursorurl-structs)

The `CustomCursor` enum’s variants now hold instances of `CustomCursorImage` or `CustomCursorUrl`. Update your uses of `CustomCursor` accordingly.

* * *

### Make `RawHandleWrapper` fields private to save users from themselves [#](#make-rawhandlewrapper-fields-private-to-save-users-from-themselves)

The `window_handle` and `display_handle` fields on `RawHandleWrapper` are no longer public. Use the newly added getters and setters to manipulate them instead.

* * *

### Rework WindowMode::Fullscreen API [#](#rework-windowmode-fullscreen-api)

`WindowMode::SizedFullscreen(MonitorSelection)` and `WindowMode::Fullscreen(MonitorSelection)` has become `WindowMode::Fullscreen(MonitorSelection, VideoModeSelection)`. Previously, the VideoMode was selected based on the closest resolution to the current window size for SizedFullscreen and the largest resolution for Fullscreen. It is possible to replicate that behaviour by searching `Monitor::video_modes` and selecting it with `VideoModeSelection::Specific(VideoMode)` but it is recommended to use `VideoModeSelection::Current` as the default video mode when entering fullscreen.

* * *

### Support texture atlases in CustomCursor::Image [#](#support-texture-atlases-in-customcursor-image)

The `CustomCursor::Image` enum variant has some new fields. Update your code to set them.

Before:

```
CustomCursor::Image {
    handle: asset_server.load("branding/icon.png"),
    hotspot: (128, 128),
}

```


After:

```
CustomCursor::Image {
    handle: asset_server.load("branding/icon.png"),
    texture_atlas: None,
    flip_x: false,
    flip_y: false,
    rect: None,
    hotspot: (128, 128),
}

```


Utils [#](#utils)
-----------------

### `bevy_utils` Refactor [#](#bevy-utils-refactor)

In 0.16 `bevy_utils` (and by extension `bevy::utils`) was significantly reduced with many of its items either being removed, spun-out into their own crates, or just moved into more appropriate existing crates. Below is a series of tables for all items that were in `bevy_utils` 0.15 that have since been moved or removed in 0.16.

Note that certain items have been completely removed, see below for further details.

**Re-Exports**


|Item     |0.15 Path |0.16 Path|
|---------|----------|---------|
|hashbrown|bevy_utils|Removed  |
|tracing  |bevy_utils|bevy_log |


**Structs**


|Item                 |0.15 Path |0.16 Path          |
|---------------------|----------|-------------------|
|AHasher              |bevy_utils|ahash              |
|Duration             |bevy_utils|core::time         |
|FixedState           |bevy_utils|bevy_platform::hash|
|Hashed               |bevy_utils|bevy_platform::hash|
|Instant              |bevy_utils|bevy_platform::time|
|NoOpHash             |bevy_utils|bevy_platform::time|
|PassHash             |bevy_utils|bevy_platform::time|
|PassHasher           |bevy_utils|bevy_platform::time|
|RandomState          |bevy_utils|bevy_platform::time|
|SystemTime           |bevy_utils|std::time          |
|SystemTimeError      |bevy_utils|std::time          |
|TryFromFloatSecsError|bevy_utils|core::time         |


**Traits**


|Item                 |0.15 Path |0.16 Path |
|---------------------|----------|----------|
|ConditionalSend      |bevy_utils|bevy_tasks|
|ConditionalSendFuture|bevy_utils|bevy_tasks|


**Macros**


|Item                |0.15 Path |0.16 Path       |
|--------------------|----------|----------------|
|assert_object_safe  |bevy_utils|Removed         |
|debug               |bevy_utils|bevy_log        |
|error               |bevy_utils|bevy_log        |
|info                |bevy_utils|bevy_log        |
|warn                |bevy_utils|bevy_log        |
|all_tuples          |bevy_utils|variadics_please|
|all_tuples_with_size|bevy_utils|variadics_please|
|debug_once          |bevy_utils|bevy_log        |
|detailed_trace      |bevy_utils|Removed         |
|error_once          |bevy_utils|bevy_log        |
|info_once           |bevy_utils|bevy_log        |
|trace_once          |bevy_utils|bevy_log        |
|warn_once           |bevy_utils|bevy_log        |


Note that if you were previously relying on `bevy_utils` to get access to the re-exported `tracing` macros like `info!`, `warn!` or `debug!`, you should now rely on `bevy_log` instead (or `tracing` itself, being sure to keep the versions aligned).

**Functions**


|Item        |0.15 Path          |0.16 Path          |
|------------|-------------------|-------------------|
|check_ready |bevy_utils::futures|bevy_tasks::futures|
|now_or_never|bevy_utils::futures|bevy_tasks::futures|


**Type Aliases**


|Item         |0.15 Path |0.16 Path                           |
|-------------|----------|------------------------------------|
|BoxedFuture  |bevy_utils|bevy_tasks                          |
|Entry        |bevy_utils|bevy_platform::collections::hash_map|
|HashMap      |bevy_utils|bevy_platform::collections          |
|HashSet      |bevy_utils|bevy_platform::collections          |
|StableHashMap|bevy_utils|Removed                             |
|StableHashSet|bevy_utils|Removed                             |


**Removed Items**

*   `assert_object_safe` was removed in part because the term is now outdated (replaced with _dyn compatibility_) and otherwise because it is trivial to inline.
    
    ```
// Before
const _: () = assert_object_safe::<dyn MyTrait>();

// After
const _: Option<Box<dyn MyTrait>> = None;

```

    
*   `hashbrown` was removed from `bevy_utils` as a re-export due to its significant API change from `hashbrown` 0.14 to 0.15. Instead of exposing a large public API out of our direct control, we've taken a more explicit subset and moved it into `bevy_platform::collections`, mimicking the layout of the standard library. If you need access to `hashbrown`, take a direct dependency instead.
    
*   `detailed_trace` was removed due to its minimal use within the engine. If you still wish to use it, make sure you have taken a direct dependency on `tracing` and have a feature name `detailed_trace` defined in your `Cargo.toml`. You can use the below as a replacement:
    
    ```
macro_rules! detailed_trace {
    ($($tts:tt)*) => {
        if cfg!(feature = "detailed_trace") {
            ::tracing::trace!($($tts)*);
        }
    }
}

```

    
*   `dbg`, `info`, `warn`, and `error` were all removed due to minimal use within the engine. If you still wish to use them, make sure you have taken a direct dependency on `tracing`. You can use the below as a replacement:
    
    ```
/// Calls the [`tracing::info!`] macro on a value.
pub fn info<T: core::fmt::Debug>(data: T) {
    ::tracing::info!("{:?}", data);
}

/// Calls the [`tracing::debug!`] macro on a value.
pub fn dbg<T: core::fmt::Debug>(data: T) {
    ::tracing::debug!("{:?}", data);
}

/// Processes a [`Result`] by calling the [`tracing::warn!`] macro in case of an [`Err`] value.
pub fn warn<E: core::fmt::Debug>(result: Result<(), E>) {
    if let Err(warn) = result {
        ::tracing::warn!("{:?}", warn);
    }
}

/// Processes a [`Result`] by calling the [`tracing::error!`] macro in case of an [`Err`] value.
pub fn error<E: core::fmt::Debug>(result: Result<(), E>) {
    if let Err(error) = result {
        ::tracing::error!("{:?}", error);
    }
}

```

    
*   `StableHashMap` and `StableHashSet` were removed due to minimal use within the engine. You can use the below as a replacement:
    
    ```
/// A stable hash-map.
pub type StableHashMap<K, V> = bevy::platform_support::collections::HashMap<K, V, bevy::platform_support::hash::FixedState>;

/// A stable hash-set.
pub type StableHashSet<K> = bevy::platform_support::collections::HashSet<K, bevy::platform_support::hash::FixedState>;

```

    

Without area [#](#without-area)
-------------------------------

### Link iOS example with `rustc`, and avoid C trampoline [#](#link-ios-example-with-rustc-and-avoid-c-trampoline)

**If you have been building your application for iOS:**

Previously, the `#[bevy_main]` attribute created a `main_rs` entry point that most Xcode templates were using to run your Rust code from C. This was found to be unnecessary, as you can simply let Rust build your application as a binary, and run that directly.

You have two options for dealing with this.

#### New, suggested approach [#](#new-suggested-approach)

Preferred option is to remove your “compile” and “link” build phases, and instead replace it with a “run script” phase that invokes `cargo build --bin ...`, and moves the built binary to the Xcode path `$TARGET_BUILD_DIR/$EXECUTABLE_PATH`. An example of how to do this can be viewed in [mobile example](https://github.com/bevyengine/bevy/tree/main/examples/mobile).

If you are not sure how to do this, consider one of two ways:

*   replace local mobile `game` crate with the one in repo and reapply your changes.
*   replicate the changes from [pull request](https://github.com/bevyengine/bevy/pull/14780) in your `mobile` crate.

To make the debugging experience in Xcode nicer after this, you might also want to consider either enabling `panic = "abort"` or to set a breakpoint on the `rust_panic` symbol.

#### Restoring old behaviour [#](#restoring-old-behaviour)

If you’re using additional ObjC code, Swift packages, Xcode customizations, or if it otherwise it makes sense for your use-case to continue link with Xcode, you can revert to the old behavior by adding code below to your `main.rs` file:

```
#[cfg(target_os = "ios")]
#[unsafe(no_mangle)]
extern "C" fn main_rs() {
    main()
}

```


Note that the old approach of linking a static library prevents the Rust standard library from doing runtime initialization, so certain functionality provided by `std` might be unavailable (stack overflow handlers, stdout/stderr flushing and other such functionality provided by the initialization routines).

* * *

### Remove Image::from\_buffer `name` argument (only present in debug "dds" builds) [#](#remove-image-from-buffer-name-argument-only-present-in-debug-dds-builds)

*   `Image::from_buffer()` no longer has a `name` argument that’s only present in debug builds when the `"dds"` feature is enabled. If you happen to pass a name, remove it.

* * *

### Remove `bevy_core` [#](#remove-bevy-core)

`bevy_core` has been removed and its items moved into more appropriate locations. Below are some tables showing where items have been moved to

#### Structs [#](#structs)


|Item                          |0.15 Path|0.16 Path       |
|------------------------------|---------|----------------|
|FrameCount                    |bevy_core|bevy_diagnostic |
|FrameCountPlugin              |bevy_core|bevy_diagnostic |
|Name                          |bevy_core|bevy_ecs::name  |
|NameOrEntity                  |bevy_core|bevy_ecs::name  |
|NameOrEntityItem              |bevy_core|bevy_ecs::name  |
|NonSendMarker                 |bevy_core|bevy_ecs::system|
|TaskPoolOptions               |bevy_core|bevy_app        |
|TaskPoolPlugin                |bevy_core|bevy_app        |
|TaskPoolThreadAssignmentPolicy|bevy_core|bevy_app        |
|TypeRegistrationPlugin        |bevy_core|Removed         |


#### Functions [#](#functions)


|Item              |0.15 Path|0.16 Path      |
|------------------|---------|---------------|
|update_frame_count|bevy_core|bevy_diagnostic|


#### Removed [#](#removed)

`TypeRegistrationPlugin` no longer exists. If you can’t use a default `App` but still need `Name` registered, do so manually.

```
// Before
app.add_plugins(TypeRegistrationPlugin);

// After
app.register_type::<Name>();

```


* * *

### Rename `trigger.entity()` to `trigger.target()` [#](#rename-trigger-entity-to-trigger-target)

*   Rename `Trigger::entity()` to `Trigger::target()`.
*   Rename `ObserverTrigger::entity` to `ObserverTrigger::target`

* * *

### Use `target_abi = "sim"` instead of `ios_simulator` feature [#](#use-target-abi-sim-instead-of-ios-simulator-feature)

If you're using a project that builds upon the mobile example, remove the `ios_simulator` feature from your `Cargo.toml` (Bevy now handles this internally).