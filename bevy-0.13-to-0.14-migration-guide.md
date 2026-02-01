# 0.13 to 0.14
Animation [#](#animation)
-------------------------

### `AnimationClip` now uses UUIDs and `NoOpTypeIdHash` is now `NoOpHash` [#](#animationclip-now-uses-uuids-and-nooptypeidhash-is-now-noophash)

`AnimationClip` now uses UUIDs instead of hierarchical paths based on the `Name` component to refer to bones. This has several consequences:

*   A new component, `AnimationTarget`, should be placed on each bone that you wish to animate, in order to specify its UUID and the associated `AnimationPlayer`. The glTF loader automatically creates these components as necessary, so most uses of glTF rigs shouldn’t need to change.
*   Moving a bone around the tree, or renaming it, no longer prevents an `AnimationPlayer` from affecting it.
*   Dynamically changing the `AnimationPlayer` component will likely require manual updating of the `AnimationTarget` components.

Entities with `AnimationPlayer` components may now possess descendants that also have `AnimationPlayer` components. They may not, however, animate the same bones.

Furthermore, `NoOpTypeIdHash` and `NoOpTypeIdHasher` have been renamed to `NoOpHash` and `NoOpHasher`.

* * *

### Implement the `AnimationGraph` to blend animations together [#](#implement-the-animationgraph-to-blend-animations-together)

`AnimationPlayer`s can no longer play animations by themselves: they need to be paired with a `Handle<AnimationGraph>`. Code that used `AnimationPlayer` to play animations will need to create an `AnimationGraph` asset first, add a node for the clip (or clips) you want to play, and then supply the index of that node to the `AnimationPlayer`’s `play` method.

```
// 0.13
fn setup(mut commands: Commands, mut animations: ResMut<Assets<AnimationClip>>) {
    let mut animation = AnimationClip::default();

    // ...

    let mut player = AnimationPlayer::default();
    player.play(animations.add(animation));

    commands.spawn((
        player,
        // ...
    ));
}

// 0.14
fn setup(
    mut commands: Commands,
    mut animations: ResMut<Assets<AnimationClip>>,
    // You now need access to the `AnimationGraph` asset.
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let mut animation = AnimationClip::default();

    // ...

    // Create a new `AnimationGraph` and add the animation handle to it.
    let (graph, animation_index) = AnimationGraph::from_clip(animations.add(animation));

    let mut player = AnimationPlayer::default();
    // Play the animation index, not the handle.
    player.play(animation_index);

    commands.spawn((
        player,
        // Add the new `AnimationGraph` to the assets, and spawn the entity with its handle.
        graphs.add(graph),
        // ...
    ));
}

```


Furthermore, the `AnimationPlayer::play_with_transition()` method has been removed and replaced with the `AnimationTransitions` component. If you were previously using `AnimationPlayer::play_with_transition()`, add all animations that you were playing to the `AnimationGraph` and create an `AnimationTransitions` component to manage the blending between them.

For more information behind this change, you may be interested in [RFC 51](https://github.com/bevyengine/rfcs/blob/main/rfcs/51-animation-composition.md).

* * *

### Multiplying colors by `f32` no longer ignores alpha channel [#](#multiplying-colors-by-f32-no-longer-ignores-alpha-channel)

It was previously possible to multiply and divide a `Color` by an `f32`, which is now removed. You must now operate on a specific color space, such as `LinearRgba`. Furthermore, these operations used to skip the alpha channel, but that is no longer the case.

```
// 0.13
let color = Color::RgbaLinear {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
    alpha: 1.0,
} * 0.5;

// Alpha is preserved, ignoring the multiplier.
assert_eq!(color.a(), 1.0);

// 0.14
let color = LinearRgba {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
    alpha: 1.0,
} * 0.5;

// Alpha is included in multiplication.
assert_eq!(color.alpha, 0.5);

```


If you need the alpha channel to remain untouched, consider creating your own helper method:

```
fn legacy_div_f32(color: &mut LinearRgba, scale: f32) {
    color.red /= scale;
    color.green /= scale;
    color.blue /= scale;
}

let mut color = LinearRgba {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
    alpha: 1.0,
};

legacy_div_f32(&mut color, 2.0);

```


If you are fine with the alpha changing, but need it to remain within the range of 0.0 to 1.0, consider clamping it:

```
let mut color = LinearRgba {
    red: 1.0,
    green: 1.0,
    blue: 1.0,
    alpha: 1.0,
} * 10.0;

// Force alpha to be within [0.0, 1.0].
color.alpha = color.alpha.clamp(0.0, 1.0);

```


Note that in some cases, such as rendering sprites, the alpha is automatically clamped so you do not need to do it manually.

App [#](#app)
-------------

### `OnEnter` state schedules now run before Startup schedules [#](#onenter-state-schedules-now-run-before-startup-schedules)

In Bevy 0.13, the \[`OnEnter`\] schedules for states initialized via \[`app.init_state`\] would run after any systems in the `Startup` schedules. This is because \[`apply_state_transitions`\] was only run during the \[`StateTransition`\] schedule.

This was a subtle bug: it was possible for the game to be in a particular state without having first _entered_ it. Now, \[`OnEnter`\] state transition logic is handled immediately. See [bevy#13968](https://github.com/bevyengine/bevy/issues/13968) for more context on this decision.

To migrate, choose one of the following options:

1.  Moving your startup systems to a state, as a variant of the state you're waiting for (e.g. `AppState::Setup`), and then transition out of it once the setup is complete.
2.  Moving your startup systems to a state, and making the other state a [sub state](https://github.com/bevyengine/bevy/blob/v0.14.0-rc.4/examples/state/sub_states.rs) that depends on the startup state's completion (e.g. `SetupState::SetupComplete`).

```
// 0.13
#[derive(States, Default)]
enum AppState {
    #[default]
    InMenu,
    InGame,
}

app
   .init_state::<AppState>()
   .add_systems(Startup, initial_setup)
   .add_systems(OnEnter(AppState::InMenu), relies_on_initial_setup);

// 0.14 (Solution 1)
#[derive(States, Default)]
enum AppState {
    // Make this the default instead of `InMenu`.
    #[default]
    Setup
    InMenu,
    InGame,
}

fn transition_to_in_menu(mut app_state: ResMut<NextState<AppState>>) {
    app_state.set(AppState::InMenu);
}

app
    .init_state::<AppState>()
    .add_systems(OnEnter(AppState::Setup), initial_setup)
    .add_system(Update, transition_to_in_menu.run_if(in_state(AppState::Setup)))
    .add_systems(OnEnter(AppState::InMenu), relies_on_initial_setup);

// 0.14 (Solution 2)
#[derive(States, Default)]
enum SetupState {
    #[default]
    SettingUp,
    SetupComplete,
}

#[derive(SubStates, Default)]
#[source(SetupState = SetupState::SetupComplete)]
enum AppState {
    #[default]
    InMenu,
    InGame,
}

fn finish_setup(mut app_state: ResMut<NextState<SetupState>>) {
    app_state.set(SetupState::SetupComplete);
}

app
    .init_state::<SetupState>()
    // Note that we don't call `init_state()` for substates!
    .add_sub_state::<AppState>()
    .add_systems(OnEnter(AppState::InitialSetup), initial_setup)
    .add_system(Update, finish_setup.run_if(in_state(AppState::Setup)))
    .add_systems(OnEnter(AppState::InMenu), relies_on_initial_setup);

```


* * *

### Separate `SubApp` from `App` [#](#separate-subapp-from-app)

`SubApp` has been separated from `App`, so there are a few larger changes involved when interacting with these types.

#### Constructing a `SubApp` [#](#constructing-a-subapp)

`SubApp` no longer contains an `App`, so you no longer are able to convert an `App` into a `SubApp`. Furthermore, the extraction function must now be set outside of the constructor.

```
// 0.13
#[derive(AppLabel, Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct MySubApp;

let mut app = App::new();
let mut sub_app = App::empty();

sub_app.add_systems(Main, ...);
sub_app.insert_resource(...);

app.insert_sub_app(MySubApp, SubApp::new(sub_app, |main_world, sub_app| {
    // Extraction function.
}));

// 0.14
#[derive(AppLabel, Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct MySubApp;

let mut app = App::new();
// Use `SubApp::new()` instead of `App::new()`.
let mut sub_app = SubApp::new();

// Instead of setting the extraction function when you create the `SubApp`, you must set it
// afterwards. If you do not set an extraction function, it will do nothing.
sub_app.set_extract(|main_world, sub_world| {
    // Extraction function.
});

// You can still add systems and resources like normal.
sub_app.add_systems(Main, ...);
sub_app.insert_resource(...);

app.insert_sub_app(MySubApp, sub_app);

```


#### `App` changes [#](#app-changes)

`App` is not `Send` anymore, but `SubApp` still is.

Due to the separation of `App` and `SubApp`, a few other methods have been changed.

First, `App::world` as a property is no longer directly accessible. Instead use the getters `App::world` and `App::world_mut`.

```
#[derive(Component)]
struct MyComponent;

// 0.13
let mut app = App::new();
println!("{:?}", app.world.id());
app.world.spawn(MyComponent);

// 0.14
let mut app = App::new();
println!("{:?}", app.world().id()); // Notice the added paranthesese.
app.world_mut().spawn(MyComponent);

```


Secondly, all getters for the sub app now return a `SubApp` instead of an `App`. This includes `App::sub_app`, `App::sub_app_mut`, `App::get_sub_app`, and `App::get_sub_app_mut`.

```
#[derive(AppLabel, Clone, Copy, Hash, PartialEq, Eq, Debug)]
struct MySubApp;

let mut app = App::new();
app.insert_sub_app(MySubApp, SubApp::new());

assert_eq!(app.sub_app(MySubApp).type_id(), TypeId::of::<SubApp>());

```


Finally, `App::runner` and `App::main_schedule_label` are now private. It is no longer possible to get the runner closure, but you can get the main schedule label using `SubApp::update_schedule`.

```
let app = App::new();
let label = app.main().update_schedule;

```


#### 3rd-party traits on `App` [#](#3rd-party-traits-on-app)

If you implemented an extension trait on `App`, consider also implementing it on `SubApp`:

```
trait SpawnBundle {
    /// Spawns a new `Bundle` into the `World`.
    fn spawn_bundle<T: Bundle>(&mut self, bundle: T) -> &mut Self;
}

impl SpawnBundle for App {
    fn spawn_bundle<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.world_mut().spawn(bundle);
        self
    }
}

/// `SubApp` has a very similar API to `App`, so the code will usually look the same.
impl SpawnBundle for SubApp {
    fn spawn_bundle<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.world_mut().spawn(bundle);
        self
    }
}

```


* * *

### Make `AppExit` more specific about exit reason [#](#make-appexit-more-specific-about-exit-reason)

The `AppExit` event is now an enum that represents whether the code exited successfully or not. If you construct it, you must now specify `Success` or `Error`:

```
// 0.13
fn send_exit(mut writer: EventWriter<AppExit>) {
    writer.send(AppExit);
}

// 0.14
fn send_exit(mut writer: EventWriter<AppExit>) {
    writer.send(AppExit::Success);
    // Or...
    writer.send(AppExit::Error(NonZeroU8::new(1).unwrap()));
}

```


If you subscribed to this event in a system, consider `match`ing whether it was a success or an error:

```
// 0.13
fn handle_exit(mut reader: EventReader<AppExit>) {
    for app_exit in reader.read() {
        // Something interesting here...
    }
}

// 0.14
fn handle_exit(mut reader: EventReader<AppExit>) {
    for app_exit in reader.read() {
        match *app_exit {
            AppExit::Success => {
                // Something interesting here...
            },
            AppExit::Error(exit_code) => panic!("App exiting with an error! (Code: {exit_code})"),
        }
    }
}

```


Furthermore, `App::run()` now returns `AppExit` instead of the unit type `()`. Since `AppExit` implements [`Termination`](https://doc.rust-lang.org/stable/std/process/trait.Termination.html), you can now return it from the main function.

```
// 0.13
fn main() {
    App::new().run()
}

// 0.14
fn main() -> AppExit {
    App::new().run()
}

// 0.14 (alternative)
fn main() {
    // If you want to ignore `AppExit`, you can add a semicolon instead. :)
    App::new().run();
}

```


Finally, if you configured a custom `App` runner function, it will now have to return an `AppExit`.

```
let mut app = App::new();

app.set_runner(|_app| {
    // ...

    // Return success by default, though you may also return an error code.
    AppExit::Success
});

```


* * *

### Deprecate dynamic plugins [#](#deprecate-dynamic-plugins)

Dynamic plugins are now deprecated. If possible, remove all usage them from your code:

```
// 0.13
// This would be compiled into a separate dynamic library.
#[derive(DynamicPlugin)]
pub struct MyPlugin;

impl Plugin for MyPlugin {
    // ...
}

// This would be compiled into the main binary.
App::new()
    .load_plugin("path/to/plugin")
    .run()

// 0.14
// This would now be compiled into the main binary as well.
pub struct MyPlugin;

impl Plugin for MyPlugin {
    // ...
}

App::new()
    .add_plugins(MyPlugin)
    .run()

```


If you are unable to do that, you may temporarily silence the deprecation warnings by annotating all usage with `#[allow(deprecated)]`. Please note that the current dynamic plugin system will be removed by the next major Bevy release, so you will have to migrate eventually. You may be interested in these safer, related links:

*   [Bevy Assets - Scripting](https://bevy.org/assets/#scripting): Scripting and modding libraries for Bevy
*   [Bevy Assets - Development tools](https://bevy.org/assets/#development-tools): Hot reloading and other development functionality
*   [`stabby`](https://github.com/ZettaScaleLabs/stabby): Stable Rust ABI

If you truly cannot go without dynamic plugins, you may copy the code from Bevy and add it to your project locally.

* * *

### Move state initialization methods to `bevy::state` [#](#move-state-initialization-methods-to-bevy-state)

`State` has been moved to `bevy::state`. With it, `App::init_state` has been moved from a normal method to an extension trait. You may now need to import `AppExtStates` in order to use this method, if you don't use the prelude. (This trait is behind the `bevy_state` feature flag, which you may need to enable.)

```
// 0.13
App::new()
    .init_state::<MyState>()
    .run()

// 0.14
use bevy::state::app::AppExtStates as _;

App::new()
    .init_state::<MyState>()
    .run()

```


Assets [#](#assets)
-------------------

### Remove the `UpdateAssets` and `AssetEvents` schedules [#](#remove-the-updateassets-and-assetevents-schedules)

The `UpdateAssets` schedule has been removed. If you add systems to this schedule, move them to run on `PreUpdate`. (You may need to configure the ordering with `system.before(...)` and `system.after(...)`.)

```
// 0.13
App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(UpdateAssets, my_system)
    .run()

// 0.14
App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(PreUpdate, my_system)
    .run()

```


Furthermore, `AssetEvents` has been changed from a `ScheduleLabel` to a `SystemSet` within the `First` schedule.

```
// 0.13
App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(AssetEvents, my_system)
    .run()

// 0.14
App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(First, my_system.in_set(AssetEvents))
    .run()

```


* * *

### Use `async fn` in traits rather than `BoxedFuture` [#](#use-async-fn-in-traits-rather-than-boxedfuture)

In Rust 1.75, [`async fn` was stabilized for traits](https://blog.rust-lang.org/2023/12/28/Rust-1.75.0.html#async-fn-and-return-position-impl-trait-in-traits). Some traits have been switched from returning `BoxedFuture` to be an `async fn`, specifically:

*   `AssetReader`
*   `AssetWriter`
*   `AssetLoader`
*   `AssetSaver`
*   `Process`

Please update your trait implementations:

```
// 0.13
impl AssetLoader for MyAssetLoader {
    // ...

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a (),
        _load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        // Note that you had to pin the future.
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            Ok(bytes)
        })
    }
}

// 0.14
impl AssetLoader for MyAssetLoader {
    // ...

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        // No more need to pin the future, just write it!
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        Ok(bytes)
    }
}

```


Because these traits now use `async`, they are no longer object safe. If you need to receive or store `&dyn Trait`, use the `&dyn ErasedTrait` variant instead. For instance:

```
// 0.13
struct MyReader(Box<dyn AssetReader>);

// 0.14
struct MyReader(Box<dyn ErasedAssetReader>);

```


* * *

### Add `Ignore` variant to `ProcessResult` [#](#add-ignore-variant-to-processresult)

The `ProcessResult` enum, used in asset loading, has a new `Ignore` variant. You may need to update your `match` statements.

* * *

### Removed `Into<AssetId<T>>` for `Handle<T>` [#](#removed-into-assetid-t-for-handle-t)

Converting from a `Handle` to an `AssetId` using `Into` was removed because it was a footgun that could potentially drop the asset if the `Handle` was a strong reference. If you need the `AssetId`, please use `Handle::id()` instead.

```
// 0.13
let id: AssetId<T> = handle.into();

// 0.14
let id = handle.id();

```


* * *

### Add `AsyncSeek` trait to `Reader` to be able to seek inside asset loaders [#](#add-asyncseek-trait-to-reader-to-be-able-to-seek-inside-asset-loaders)

The asset loader's `Reader` type alias now requires the new `AsyncSeek` trait. Please implement `AsyncSeek` for any structures that must be a `Reader`, or use an alternative if seeking is not supported.

If this is a problem for you, please chime in at [bevy#12880](https://github.com/bevyengine/bevy/issues/12880) and help us improve the design for 0.15!

* * *

### Add error info to `LoadState::Failed` [#](#add-error-info-to-loadstate-failed)

Rust prides itself on its error handling, and Bevy has been steadily catching up. Previously, when checking if an asset was loaded using `AssetServer::load_state` (and variants), the only information returned on an error was the empty `LoadState::Failed`. Not very useful for debugging!

Now, a full `AssetLoadError` is included inside `Failed` to tell you exactly what went wrong. You may need to update your `match` and `if let` statements to handle this new value:

```
// 0.13
match asset_server.load_state(asset_id) {
    // ...
    LoadState::Failed => eprintln!("Could not load asset!"),
}

// 0.14
match asset_server.load_state(asset_id) {
    // ...
    LoadState::Failed(error) => eprintln!("Could not load asset! Error: {}", error),
}

```


Furthermore, the `Copy`, `PartialOrd`, and `Ord` implementations have been removed from `LoadState`. You can explicitly call `.clone()` instead of copying the enum, and you can manually re-implement `Ord` as a helper method if required.

* * *

### Make `AssetMetaCheck` a field of `AssetPlugin` [#](#make-assetmetacheck-a-field-of-assetplugin)

`AssetMetaCheck` is used to configure how the `AssetPlugin` reads `.meta` files. It was previously a resource, but now has been changed to a field in `AssetPlugin`. If you use `DefaultPlugins`, you can use `.set` to configure this field.

```
// 0.13
App::new()
    .add_plugins(DefaultPlugins)
    .insert_resource(AssetMetaCheck::Never)
    .run()

// 0.14
App::new()
    .add_plugins(DefaultPlugins.set(AssetPlugin {
        meta_check: AssetMetaCheck::Never,
        ..default()
    }))
    .run()

```


* * *

### Make `LoadContext` use the builder pattern [#](#make-loadcontext-use-the-builder-pattern)

`LoadContext`, used by `AssetLoader`, has been updated so all of its `load_*` methods have been merged into a builder struct.

```
// 0.13
load_context.load_direct(path);
// 0.14
load_context.loader().direct().untyped().load(path);

// 0.13
load_context.load_direct_with_reader(reader, path);
// 0.14
load_context.loader().direct().with_reader(reader).untyped().load(path);

// 0.13
load_context.load_untyped(path);
// 0.14
load_context.loader().untyped().load(path);

// 0.13
load_context.load_with_settings(path, settings);
// 0.14
load_context.loader().with_settings(settings).load(path);

```


* * *

### Use `RenderAssetUsages` to configure gLTF meshes & materials during load [#](#use-renderassetusages-to-configure-gltf-meshes-materials-during-load)

It is now possible configure whether meshes and materials should be loaded in the main world, the render world, or both with `GltfLoaderSettings`. The `load_meshes` field has been changed from a `bool` to a `RenderAssetUsages` bitflag, and a new `load_materials` field as been added.

You may need to update any gLTF `.meta` files:

```
// 0.13
load_meshes: true

// 0.14
load_meshes: ("MAIN_WORLD | RENDER_WORLD")

```


If you use `AssetServer::load_with_settings` instead when loading gLTF files, you will also have to update:

```
// 0.13
asset_server.load_with_settings("model.gltf", |s: &mut GltfLoaderSettings| {
    s.load_meshes = true;
});

// 0.14
asset_server.load_with_settings("model.gltf", |s: &mut GltfLoaderSettings| {
    s.load_meshes = RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD;
});

```


* * *

### Consolidate `RenderMaterials` and similar into `RenderAssets`, implement `RenderAsset` for destination type [#](#consolidate-rendermaterials-and-similar-into-renderassets-implement-renderasset-for-destination-type)

`RenderMaterials`, `RenderMaterials2d`, and `RenderUiMaterials` have all been replaced with the `RenderAssets` resource. If you need access a `PreparedMaterial<T>` using an `AssetId`, use `RenderAssets::get` instead.

Furthermore, the `RenderAsset` trait should now be implemented for destination types rather than source types. If you need to access the source type, use the `RenderAsset::SourceAsset` associated type.

```
// 0.13
impl RenderAsset for Image {
    type PreparedAsset = GpuImage;

    // ...
}

// 0.14
impl RenderAsset for GpuImage {
    type SourceAsset = Image;

    // ...
}

```


Audio [#](#audio)
-----------------

### Fix leftover references to children when despawning audio entities [#](#fix-leftover-references-to-children-when-despawning-audio-entities)

You can configure the behavior of spawned audio with the `PlaybackMode` enum. One of its variants, `PlaybackMode::Despawn`, would despawn the entity when the audio finished playing.

There was previously a bug where this would only despawn the entity and not its children. This has been fixed, so now `despawn_recursive()` is called when the audio finishes.

If you relied on this behavior, consider using `PlaybackMode::Remove` to just remove the audio components from the entity or `AudioSink::empty()` to check whether any audio is finished and manually `despawn()` it.

Color [#](#color)
-----------------

### Overhaul `Color` [#](#overhaul-color)

Bevy's color support has received a major overhaul, and with it the new `bevy::color` module. Buckle up, many things have been changed!

#### Color space representation [#](#color-space-representation)

Bevy's main `Color` enum is used to represent color in many different color spaces (such as RGB, HSL, and more). Before, these color spaces were all represented inline as variants:

```
enum Color {
    Rgba {
        red: f32,
        green: f32,
        blue: f32,
        alpha: f32,
    },
    Hsla {
        hue: f32,
        saturation: f32,
        lightness: f32,
        alpha: f32,
    },
    // ...
}

```


This has been changed so now each color space has its own dedicated struct:

```
struct Srgba {
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
}

struct Hsla {
    hue: f32,
    saturation: f32,
    lightness: f32,
    alpha: f32,
}

enum Color {
    Srgba(Srgba),
    Hsla(Hsla),
    // ...
}

```


This makes it easier to organize and manage different color spaces, and many more color spaces have been added too! To handle this change, you may need to update your match statements:

```
// 0.13
match color {
    Color::Rgba { red, green, blue, alpha } => {
        // Something cool here!
    },
    _ => {},
}

// 0.14
match color {
    Color::Srgba(Srgba { red, green, blue, alpha }) => {
        // Something cool here!
    },
    // If you explicitly match every possible color space, you may need to handle more variants.
    // Color::Xyza(Xyza { x, y, z, alpha }) => {
    //     // Something else even cooler here!
    // },
    _ => {}
}

```


Additionally, you must now use the `From` and `Into` implementations when converting between color spaces, as compared to the old helper methods such as `as_rgba` and `as_hsla`.

```
// 0.13
let color = Color::rgb(1.0, 0.0, 1.0).as_hsla();

// 0.14
let color: Hsla = Srgba::rgb(1.0, 0.0, 1.0).into();

```


#### `Color` methods [#](#color-methods)

Any mention of RGB has been renamed to [sRGB](https://en.wikipedia.org/wiki/SRGB). This includes the variant `Color::Rgba` turning into `Color::Srgba` as well as methods such as `Color::rgb` and `Color::rgb_u8` turning into `Color::srgb` and `Color::srgb_u8`.

Methods to access specific channels of `Color` have been removed due to causing silent, relatively expensive conversions. This includes `Color::r`, `Color::set_r`, `Color::with_r`, and all of the equivalents for `g`, `b` `h`, `s` and `l`. Convert your `Color` into the desired color space, perform your operation there, and then convert it back.

```
// 0.13
let mut color = Color::rgb(0.0, 0.0, 0.0);
color.set_b(1.0);

// 0.14
let color = Color::srgb(0.0, 0.0, 0.0);
let srgba = Srgba {
    blue: 1.0,
    ..Srgba::from(color),
};
let color = Color::from(srgba);

```


`Color::hex` has been moved to `Srgba::hex`. Call `.into()` or construct a `Color::Srgba` variant manually to convert it.

`Color::rgb_linear` and `Color::rgba_linear` have been renamed `Color::linear_rgb` and `Color::linear_rgba` to fit the naming scheme of the `LinearRgba` struct.

`Color::as_linear_rgba_f32` and `Color::as_linear_rgba_u32` have been removed. Call `LinearRgba::to_f32_array` and `LinearRgba::to_u32` instead, converting if necessary.

Several other color conversion methods to transform LCH or HSL colors into float arrays or `Vec` types have been removed. Please reimplement these externally or open a PR to re-add them if you found them particularly useful.

Vector field arithmetic operations on `Color` (add, subtract, multiply and divide by a f32) have been removed. Instead, convert your colors into `LinearRgba` space and perform your operations explicitly there. This is particularly relevant when working with emissive or HDR colors, whose color channel values are routinely outside of the ordinary 0 to 1 range.

#### Alpha [#](#alpha)

Alpha, also known as transparency, used to be referred to by the letter `a`. It is now called by its full name within structs and methods.

*   `Color::set_a`, `Color::with_a`, and `Color::a` are now `Color::set_alpha`, `Color::with_alpha`, and `Color::alpha`. These are part of the new `Alpha` trait.
*   Additionally, `Color::is_fully_transparent` is now part of the `Alpha`.

#### CSS Constants [#](#css-constants)

The various CSS color constants are no longer stored directly on `Color`. Instead, they’re defined in the `Srgba` color space, and accessed via `bevy::color::palettes`. Call `.into()` on them to convert them into a `Color` for quick debugging use.

```
// 0.13
let color = Color::BLUE;

// 0.14
use bevy::color::palettes::css::BLUE;

let color = BLUE;

```


Please note that `palettes::css` is not necessarily 1:1 with the constants defined previously as some names and colors have been changed to conform with the CSS spec. If you need the same color as before, consult the table below or use the color values from the [old constants](https://github.com/bevyengine/bevy/blob/v0.13.2/crates/bevy_render/src/color/mod.rs#L60).


|0.13      |0.14                     |
|----------|-------------------------|
|CYAN      |AQUA                     |
|DARK_GRAY |Srgba::gray(0.25)        |
|DARK_GREEN|Srgba::rgb(0.0, 0.5, 0.0)|
|GREEN     |LIME                     |
|LIME_GREEN|LIMEGREEN                |
|PINK      |DEEP_PINK                |


#### Switch to `LinearRgba` [#](#switch-to-linearrgba)

`WireframeMaterial`, `ExtractedUiNode`, `ExtractedDirectionalLight`, `ExtractedPointLight`, `ExtractedSpotLight`, and `ExtractedSprite` now store a `LinearRgba` rather than a polymorphic `Color`. Furthermore, `Color` no longer implements `AsBindGroup`. You should store a `LinearRgba` instead to avoid conversion costs.

* * *

### Move WGSL math constants and color operations from `bevy_pbr` to `bevy_render` [#](#move-wgsl-math-constants-and-color-operations-from-bevy-pbr-to-bevy-render)

Mathematical constants and color conversion functions for shaders have been moved from `bevy_pbr::utils` to `bevy_render::maths` and `bevy_render::color_operations`. If you depended on these in your own shaders, please update your import statements:

```
// 0.13
#import bevy_pbr::utils::{PI, rgb_to_hsv}

// 0.14
#import bevy_render::{maths::PI, color_operations::rgb_to_hsv}

```


* * *

### Remove old color space utilities [#](#remove-old-color-space-utilities)

The `SrgbColorSpace` trait, `HslRepresentation` struct, and `LchRepresentation` struct have been removed in favor of the specific color space structs.

For `SrgbColorSpace`, use `Srgba::gamma_function()` and `Srgba::gamma_function_inverse()`. If you used the `SrgbColorSpace` implementation for `u8`, convert it to an `f32` first:

```
// 14 is random, this could be any number.
let nonlinear: u8 = 14;

// Apply gamma function, converting `u8` to `f32`.
let linear: f32 = Srgba::gamma_function(nonlinear as f32 / 255.0);

// Convert back to a `u8`.
let linear: u8 = (linear * 255.0) as u8;

```


Note that this conversion can be costly, especially if called during the `Update` schedule. Consider just using `f32` instead.

`HslRepresentation` and `LchRepresentation` can be replaced with the `From` implementations between `Srgba`, `Hsla`, and `Lcha`.

```
// 0.13
let srgb = HslRepresentation::hsl_to_nonlinear_srgb(330.0, 0.7, 0.8);
let lch = LchRepresentation::nonlinear_srgb_to_lch([0.94, 0.66, 0.8]);

// 0.14
let srgba: Srgba = Hsla::new(330.0, 0.7, 0.8, 1.0).into();
let lcha: Lcha = Srgba::new(0.94, 0.66, 0.8, 1.0).into();

```


* * *

### Use `LinearRgba` in `ColorAttachment` [#](#use-linearrgba-in-colorattachment)

`ColorAttachment::new()` now takes `Option<LinearRgba>` instead of `Option<Color>` for the `clear_color`. You can use the `From<Color>` implementation to convert your color.

```
let clear_color: Option<LinearRgba> = Some(color.into());

```


### Remove `close_on_esc` [#](#remove-close-on-esc)

The `close_on_esc` system was removed because it was too opiniated and lacked customization. If you used this system, you may copy its contents below:

```
pub fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<(Entity, &Window)>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for (window, focus) in focused_windows.iter() {
        if !focus.focused {
            continue;
        }

        if input.just_pressed(KeyCode::Escape) {
            commands.entity(window).despawn();
        }
    }
}

```


You may be interested in using the built-in keybinds provided by the operating system instead, such as Alt+F4 and Command+Q.

Diagnostics [#](#diagnostics)
-----------------------------

### Make `sysinfo` diagnostic plugin optional [#](#make-sysinfo-diagnostic-plugin-optional)

`bevy::diagnostic` depends on the `sysinfo` to track CPU and memory usage using `SystemInformationDiagnosticsPlugin`, but compiling and polling system information can be very slow. `sysinfo` is now behind the `sysinfo_plugin` feature flag, which is enabled by default for `bevy` for _not_ for `bevy_diagnostic`.

If you depend on `bevy_diagnostic` directly, toggle the flag in `Cargo.toml`:

```
[dependencies]
bevy_diagnostic = { version = "0.14", features = ["sysinfo_plugin"] }

```


If you set `default-features = false` for `bevy`, do the same in `Cargo.toml`:

```
[dependencies]
bevy = { version = "0.14", default-features = false, features = ["sysinfo_plugin"] }

```


* * *

### Improve `tracing` layer customization [#](#improve-tracing-layer-customization)

Bevy uses `tracing` to handle logging and spans through `LogPlugin`. This could be customized with the `update_subscriber` field, but it was highly restrictive. This has since been amended, replacing the `update_subscriber` field with the more flexible `custom_layer`, which returns a `Layer`.

```
// 0.13
fn update_subscriber(_app: &mut App, subscriber: BoxedSubscriber) -> BoxedSubscriber {
    Box::new(subscriber.with(CustomLayer))
}

App::new()
    .add_plugins(LogPlugin {
        update_subscriber: Some(update_subscriber),
        ..default()
    })
    .run();

// 0.14
use bevy::log::tracing_subscriber;

fn custom_layer(_app: &mut App) -> Option<BoxedLayer> {
    // You can provide a single layer:
    return Some(CustomLayer.boxed());

    // Or you can provide multiple layers, since `Vec<Layer>` also implements `Layer`:
    Some(Box::new(vec![
        tracing_subscriber::fmt::layer()
            .with_file(true)
            .boxed(),
        CustomLayer.boxed(),
    ]))
}

App::new()
    .add_plugins(LogPlugin {
        custom_layer,
        ..default()
    })
    .run();

```


The `BoxedSubscriber` type alias has also been removed, it was replaced by the `BoxedLayer` type alias.

ECS [#](#ecs)
-------------

### Generalised ECS reactivity with Observers [#](#generalised-ecs-reactivity-with-observers)

In 0.14, ECS observers were introduced: mechanisms for immediately responding to events in the world. As part of this change, the `Event` trait was extended to require `Component`. `#[derive(Event)]` now automatically implements `Component` for the annotated type, which can break types that also `#[derive(Component)]`.

```
// 0.13
#[derive(Event, Component)]
struct MyEvent;

// 0.14
// `Component` is still implemented by the `Event` derive.
#[derive(Event)]
struct MyEvent;

```


For more information, see the [release notes](about:/news/bevy-0-14/#ecs-hooks-and-observers) on hooks and observers.

* * *

### Immediately apply deferred system params in `System::run` [#](#immediately-apply-deferred-system-params-in-system-run)

The default implementation of `System::run` will now always immediately run `System::apply_deferred`. If you were manually calling `System::apply_deferred` in this situation, you may remove it. Please note that `System::run_unsafe` still _does not_ call `apply_deferred` because it cannot guarantee it will be safe.

```
// 0.13
system.run(world);

// Sometime later:
system.apply_deferred(world);

// 0.14
system.run(world);

// `apply_deferred` no longer needs to be called!

```


* * *

### Move `Command` and `CommandQueue` into `bevy::ecs::world` [#](#move-command-and-commandqueue-into-bevy-ecs-world)

`Command` and `CommandQueue` have been moved from `bevy::ecs::system` to `bevy::ecs::world`. If you import them directly, you will need to update your import statements.

```
// 0.13
use bevy::ecs::system::{Command, CommandQueue};

// 0.14
use bevy::ecs::world::{Command, CommandQueue};

```


* * *

### Make `Component::Storage` a constant [#](#make-component-storage-a-constant)

The `Component::Storage` associated type has been replaced with the associated constant `STORAGE_TYPE`, making the `ComponentStorage` trait unnecessary. If you were manually implementing `Component` instead of using the derive macro, update your definitions:

```
// 0.13
impl Component for MyComponent {
    type Storage = TableStorage;
}

// 0.14
impl Component for MyComponent {
    const STORAGE_TYPE: StorageType = StorageType::Table;

    // ...
}

```



|Before       |After                 |
|-------------|----------------------|
|TableStorage |StorageType::Table    |
|SparseStorage|StorageType::SparseSet|


`Component` is also now no longer object safe. If you were using `dyn Component`, please consider [filing an issue](https://github.com/bevyengine/bevy/issues) describing your use-case.

* * *

### Don't store `Access<ArchetypeComponentId>` within `QueryState` [#](#don-t-store-access-archetypecomponentid-within-querystate)

`QueryState` no longer stores an `Access<ArchetypeComponentId>`, you must now pass it as an argument to each method that uses it. To account for this change:

*   `QueryState::archetype_component_access` has been removed. You can work around this by accessing the surrounding `SystemState`s instead.
*   `QueryState::new_archetype` and `QueryState::update_archetype_component_access` now require an `&mut Access<ArchetypeComponentId>` as a parameter.

* * *

### Remove `WorldCell` [#](#remove-worldcell)

`WorldCell` has been removed due to its incomplete nature, tendency to generate runtime panics, and the presence of multiple good alternatives. If you were using it to fetch multiple distinct resource values, consider using a `SystemState` instead with the `SystemState::get()` method.

If `SystemState` does not fit your use-case and `unsafe` is tolerable, you can use `UnsafeWorldCell`. It is more performant and featureful, but lacks the runtime checks.

* * *

### Return iterator instead of slice for `QueryState::matched_tables` and `QueryState::matches_archtypes` [#](#return-iterator-instead-of-slice-for-querystate-matched-tables-and-querystate-matches-archtypes)

`QueryState::matched_tables` and `QueryState::matched_archetypes` now return iterators instead of slices. If possible, use the combinators available from the [`Iterator`](https://doc.rust-lang.org/std/iter/trait.Iterator.html) trait. In a worst-case scenario you may call `Iterator::collect()` into a `Vec`, which can then be converted into a slice.

* * *

### Remove system stepping from default features [#](#remove-system-stepping-from-default-features)

The system stepping feature is now disabled by default. It generally should not be included in shipped games, and adds a small but measurable performance overhead. To enable it, add the `bevy_debug_stepping` feature to your `Cargo.toml`:

```
[dependencies]
bevy = { version = "0.14", features = ["bevy_debug_stepping"] }

```


Code using `Stepping` will still compile with the feature disabled, but will print an error message at runtime if the application calls `Stepping::enable()`.

* * *

### Optimize event updates and virtual time [#](#optimize-event-updates-and-virtual-time)

`Events::update()` has been optimized to be `O(1)` for the amount of events registered. In doing so, a few systems and run conditions have been changed.

Events are registered to a `World` using `EventRegistry` instead of the `Events` resource:

```
// 0.13
world.insert_resource(Events::<MyEvent>::default());

// 0.14
EventRegistry::register_event::<MyEvent>(&mut world);

```


A few systems and run conditions have been changed as well:

*   `event_update_system` no longer uses generics and now has different arguments.
*   `signal_event_update_system` now has different arguments.
*   `reset_event_update_signal_system` has been removed.
*   `event_update_condition` now has different arguments.

While not related to events, the `virtual_time_system` has been changed as well. It has been converted from a system to a regular function, and now takes `&T` and `&mut T` instead of `Res<T>` and `ResMut<T>`.

* * *

### Make `SystemParam::new_archetype` and `QueryState::new_archetype` unsafe [#](#make-systemparam-new-archetype-and-querystate-new-archetype-unsafe)

`QueryState::new_archetype` and `SystemParam::new_archetype` are now unsafe functions because they do not ensure that the provided `Archetype` is from the same `World` that the state was initialized from. You will need to wrap any usage inside of an `unsafe` block, and you may need to write additional assertions to verify correct usage.

* * *

### Better `SystemId` and `Entity` conversion [#](#better-systemid-and-entity-conversion)

If you need to access the underlying `Entity` for a one-shot system's `SystemId`, use the new `SystemId::entity()` method.

```
// 0.13
let system_id = world.register_system(my_system);
let entity = Entity::from(system_id);

// 0.14
let system_id = world.register_system(my_system);
let entity = system_id.entity();

```


* * *

### Make `NextState` an enum [#](#make-nextstate-an-enum)

`NextState` has been converted from a unit struct to an enum. If you accessed the internal `Option` directly, whether through `NextState::0` or matching, you will have to update your code to handle this change.

```
// 0.13
let state = next_state.0.unwrap();

// 0.14
let NextState::Pending(state) = next_state else { panic!("No pending next state!") };

```



|0.13              |0.14                 |
|------------------|---------------------|
|NextState(Some(S))|NextState::Pending(S)|
|NextState(None)   |NextState::Unchanged |


* * *

### Separate states from core ECS [#](#separate-states-from-core-ecs)

States were moved to a separate crate which is gated behind the `bevy_state` feature. Projects that use state but don't use Bevy's `default-features` will need to add this feature to their `Cargo.toml`.

Projects that use `bevy_ecs` directly and use states will need to add the `bevy_state` **crate** as a dependency.

Projects that use `bevy_app` directly and use states will need to add the `bevy_state` **feature**.

If you do not use `DefaultPlugins`, you will need to add the `StatesPlugin` manually to your app.

Users should update imports that referenced the old location.

```
// 0.13
use bevy::ecs::schedule::{NextState, OnEnter, OnExit, OnTransition, State, States};
use bevy::ecs::schedule::common_conditions::in_state;

// 0.14
use bevy::state::state::{NextState, OnEnter, OnExit, OnTransition, State, States}
use bevy::state::condition::in_state;

```


* * *

### Constrain `WorldQuery::get_state()` to only accept `Components` [#](#constrain-worldquery-get-state-to-only-accept-components)

A few methods of `WorldQuery` and `QueryState` were unsound because they were passed an `&World`. They are now restricted to just take an `&Components`. The affected methods are:

*   `WorldQuery::get_state()`
*   `QueryState::transmute()`
*   `QueryState::transmute_filtered()`
*   `QueryState::join()`
*   `QueryState::join_filtered()`

To access `Components` from a `World`, call `World::components()`.

If you manually implemented `WorldQuery`, you need to update `get_state()` to only use the information provided by `Components`.

* * *

### Unify state transition names to `exited` and `entered` [#](#unify-state-transition-names-to-exited-and-entered)

`StateTransitionEvent`'s `before` and `after` fields have been renamed to `exited` and `entered` for consistency. You will have to update your usage if you access these fields or construct `StateTransitionEvent`.

* * *

### Make `apply_state_transition` private [#](#make-apply-state-transition-private)

The `apply_state_transition` system is no longer public. The easiest way to migrate your systems that depended on it for ordering is to create a custom schedule.

```
// 0.13
App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(StateTransition, my_system.after(apply_state_transition))
    .run()

// 0.14
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct AfterStateTransition;

let mut app = App::new();

app.add_plugins(DefaultPlugins)
    .add_systems(AfterStateTransition, my_system);

// Create a new schedule and add it to the app.
let after_state_transition = Schedule::new(AfterStateTransition);
app.add_schedule(after_state_transition);

// Modify the schedule order to make this run after `StateTransition`.
app.world_mut()
    .resource_mut::<MainScheduleOrder>()
    .insert_after(StateTransition, AfterStateTransition);

app.run()

```


* * *

### Replace `FromWorld` requirement with `FromReflect` on `ReflectResource` [#](#replace-fromworld-requirement-with-fromreflect-on-reflectresource)

`#[reflect(Resource)]` now requires the `FromReflect` trait to be implemented for your resource. This is done by default if you use `#[derive(Reflect)]`, but you structs that opt-out of this behavior will have to write their own implementation. `FromReflect` was added to replace the `FromWorld` requirement, though `FromReflect` is fallible. You may wish to add `#[reflect(FromWorld)]` to your resources to maintain an infallible variant.

Finally, if you use the `ReflectResource` struct you will need to pass a `&TypeRegistry` to its `insert`, `apply_or_insert`, and `copy` methods.

* * *

### Make `ReflectComponentFns` and `ReflectBundleFns` methods work with `EntityMut` [#](#make-reflectcomponentfns-and-reflectbundlefns-methods-work-with-entitymut)

`ReflectComponentFns` and `ReflectBundleFns` have been updated to work with `EntityMut`, as compared to the more restricting `EntityWorldMut`. You will have to update your usage of `ReflectComponentFns::apply`, `ReflectComponentFns::reflect_mut`, and `ReflectBundleFns::apply`.

If you just use `ReflectComponent` and `ReflectBundle`, you will not have change your code because `EntityWorldMut` implements `Into<EntityMut>`.

* * *

### Require `TypeRegistry` in `ReflectBundle::insert()` [#](#require-typeregistry-in-reflectbundle-insert)

`ReflectBundle::insert` now requires an additional `&TypeRegistry` parameter.

* * *

### Rename `multi-threaded` feature to `multi_threaded` [#](#rename-multi-threaded-feature-to-multi-threaded)

The `multi-threaded` feature has been renamed to `multi_threaded` for `bevy`, `bevy_asset`, `bevy_ecs`, `bevy_render`, `bevy_tasks`, and `bevy_internal`. Please update your `Cargo.toml` if you manually specify Bevy features.

* * *

### Moves `intern` and `label` modules from `bevy::utils` to `bevy::ecs` [#](#moves-intern-and-label-modules-from-bevy-utils-to-bevy-ecs)

The `bevy::utils::label` and `bevy::utils::intern` modules have been moved to `bevy::ecs`, as well as the `bevy::utils::define_label` macro as part of an active effort to shrink `bevy::utils`. You will have to update your import statements to use the new paths.

Gizmos [#](#gizmos)
-------------------

### Gizmo line joints [#](#gizmo-line-joints)

Line joins have been added for gizmos, allowing for smooth or sharp corners between lines. If you manually created your own `GizmoConfig`, you will have to specify the type of line joins with the `line_joins` field.

The `Default` implementation of `GizmoLineJoint` is `None`, but you may be interested in `Miter` for sharp joints or `Round` for smooth joints.

* * *

### Gizmo line styles [#](#gizmo-line-styles)

It is now possible to configure the line style (such as solid or dotted) of gizmos using `GizmoConfig::line_style`. If you manually create a `GizmoConfig`, you will have to specify this field.

* * *

### Rename `segments()` methods to `resolution()` [#](#rename-segments-methods-to-resolution)

All gizmo methods named `segments()` have been rename to `resolution()` in order to be consistent with `bevy::render`.

* * *

### Make gizmos take primitives as a reference [#](#make-gizmos-take-primitives-as-a-reference)

`Gizmos::primitive_2d()` and `Gizmos::primitive_3d()` now take the primitive as a reference so that non-`Copy` primitives do not need to be cloned each time they are drawn.

```
// 0.13
fn draw(mut gizmos: Gizmos) {
    let polygon = Polygon {
        vertices: [
            // ...
        ],
    };

    // Since `Polygon` is not `Copy`, you would need to clone it if you use it more than once.
    gizmos.primitive_2d(polygon.clone(), Vec2::ZERO, 0.0, Color::WHITE);
    gizmos.primitive_2d(polygon, Vec2::ONE, 0.0, Color::BLACK);
}

// 0.14
fn draw(mut gizmos: Gizmos) {
    let polygon = Polygon {
        vertices: [
            // ...
        ],
    };

    // No need to clone the polygon anymore!
    gizmos.primitive_2d(&polygon, Vec2::ZERO, 0.0, Color::WHITE);
    gizmos.primitive_2d(&polygon, Vec2::ONE, 0.0, Color::BLACK);
}

```


* * *

### More gizmos builders [#](#more-gizmos-builders)

`Gizmos::primitive_2d(CIRLCE)`, `Gizmos::primitive_2d(ELLIPSE)`, `Gizmos::primitive_2d(ANNULUS)`, and `Gizmos::primitive_3d(SPHERE)` now return their corresponding builders instead of the unit type `()`. Furthermore, `SphereBuilder::circle_segments()` has been renamed to `resolution()`.

* * *

### Contextually clearing gizmos [#](#contextually-clearing-gizmos)

`App::insert_gizmo_group()` function is now named `App::insert_gizmo_config()`.

Input [#](#input)
-----------------

### Rename touchpad input to gesture [#](#rename-touchpad-input-to-gesture)

In a recent `winit` update, touchpad events can now be triggered on mobile. To account for this, touchpad-related items have been renamed to gestures:

*   `bevy::input::touchpad` has been renamed to `bevy::input::gestures`.
*   `TouchpadMagnify` has been renamed to `PinchGesture`.
*   `TouchpadRotate` has been renamed to `RotationGesture`.

* * *

### Deprecate `ReceivedCharacter` [#](#deprecate-receivedcharacter)

`ReceivedCharacter` is now deprecated due to `winit` reworking their keyboard system, switch to using `KeyboardInput` instead.

```
// 0.13
fn listen_characters(events: EventReader<ReceivedCharacter>) {
    for event in events.read() {
        info!("{}", event.char);
    }
}

// 0.14
fn listen_characters(events: EventReader<KeyboardInput>) {
    for event in events.read() {
        // Only check for characters when the key is pressed.
        if !event.state.is_pressed() {
            continue;
        }

        // Note that some keys such as `Space` and `Tab` won't be detected as a character.
        // Instead, check for them as separate enum variants.
        match &event.logical_key {
            Key::Character(character) => {
                info!("{} pressed.", character);
            },
            Key::Space => {
                info!("Space pressed.");
            },
            _ => {},
        }
    }
}

```


* * *

### Add `WinitEvent::KeyboardFocusLost` [#](#add-winitevent-keyboardfocuslost)

`WinitEvent` has a new enum variant: `WinitEvent::KeyboardFocusLost`. This was added as part of a fix where key presses would stick when losing focus of the Bevy window, such as with Alt + Tab. Please update any `match` statements.

Math [#](#math)
---------------

### Separating Finite and Infinite 3d Planes [#](#separating-finite-and-infinite-3d-planes)

The `Plane3d` primitive is now a finite plane with a `half_size` field. If you want an infinite plane, use the new `InfinitePlane3d`.

```
// 0.13
let plane = Plane3d::new(Vec3::Y);

// 0.14
let plane = Plane3d {
    normal: Dir3::Y,
    half_size: Vec2::new(10., 10.),
};
let plane = InfinitePlane3d::new(Vec3::Y);

```


* * *

### Move direction types out of `bevy::math::primitives` [#](#move-direction-types-out-of-bevy-math-primitives)

The `Direction2d`, `Direction3d`, and `InvalidDirectionError` types have been moved from `bevy::math::primitives` to `bevy::math`.

```
// 0.13
use bevy::math::primitives::{Direction2d, Direction3d, InvalidDirectionError};

// 0.14
use bevy::math::{Direction2d, Direction3d, InvalidDirectionError};

```


* * *

### Rename `Direction2d/3d` to `Dir2/3` [#](#rename-direction2d-3d-to-dir2-3)

The `Direction2d` and `Direction3d` types have been renamed to `Dir2` and `Dir3`. They have been shortened to make them easier to type, and to make them consistent with `glam`'s shorter naming scheme (e.g. `Vec2`, `Mat4`).

* * *

### Make cardinal splines include endpoints [#](#make-cardinal-splines-include-endpoints)

There was a bug in `CubicCardinalSpline` where the curve would only pass through the interior control points, not the points at the beginning and end. (For an in-depth analysis, see [this issue](https://github.com/bevyengine/bevy/issues/12570).) This has been fixed so that the curve passes through all control points, but it may break behavior you were depending on.

If you rely on the old behavior of `CubicCardinalSpline`, you will have to truncate any parametrizations you used in order to access a curve identical to the one you had previously. This can be done by chopping off a unit-distance segment from each end of the parametrizing interval. For instance, if your code looks as follows:

```
fn interpolate(t: f32) -> Vec2 {
    let points = [
        vec2(-1.0, -20.0),
        vec2(3.0, 2.0),
        vec2(5.0, 3.0),
        vec2(9.0, 8.0),
    ];
    let my_curve = CubicCardinalSpline::new(0.3, points).to_curve();
    my_curve.position(t)
}

```


Then in order to obtain similar behavior, `t` will need to be shifted up by 1 (since the output of `CubicCardinalSpline::to_curve` has introduced a new segment in the interval \[0,1\]), displacing the old segment from \[0,1\] to \[1,2\]:

```
fn interpolate(t: f32) -> Vec2 {
    let points = [
        vec2(-1.0, -20.0),
        vec2(3.0, 2.0),
        vec2(5.0, 3.0),
        vec2(9.0, 8.0),
    ];
    let my_curve = CubicCardinalSpline::new(0.3, points).to_curve();
    // Add 1 here to restore original behavior.
    my_curve.position(t + 1)
}

```


(Note that this does not provide identical output for values of `t` outside of the interval \[0,1\].)

On the other hand, any user who was specifying additional endpoint tangents simply to get the curve to pass through the right points (i.e. not requiring exactly the same output) can simply omit the endpoints that were being supplied only for control purposes.

* * *

### Replace `Point` with `VectorSpace` [#](#replace-point-with-vectorspace)

The `Point` trait has been replaced by `VectorSpace`. These traits are very similar, with a few minor changes:

*   `VectorSpace` implementations must now provide the `ZERO` constant.
*   `VectorSpace` now requires the `Div<f32, Output = Self>` and `Neg` trait bounds.
*   `VectorSpace` no longer requires the `Add<f32, Output = Self>`, `Sum`, and `PartialEq` trait bounds.

For most cases you can replace all `Point` usage with `VectorSpace`, but you may have to make further changes if you depend on anything in the list above.

* * *

### UV-mapping change for `Triangle2d` [#](#uv-mapping-change-for-triangle2d)

The UV-mapping of `Triangle2d` has changed with this PR: the main difference is that the UVs are no longer dependent on the triangle’s absolute coordinates but instead follow translations of the triangle itself in its definition. If you depended on the old UV-coordinates for `Triangle2d`, then you will have to update affected areas to use the new ones which can be briefly described as follows:

*   The first coordinate is parallel to the line between the first two vertices of the triangle.
*   The second coordinate is orthogonal to this, pointing in the direction of the third point.

Generally speaking, this means that the first two points will have coordinates `[_, 0.]`, while the third coordinate will be `[_, 1.]`, with the exact values depending on the position of the third point relative to the first two. For acute triangles, the first two vertices always have UV-coordinates `[0., 0.]` and `[1., 0.]` respectively. For obtuse triangles, the third point will have coordinate `[0., 1.]` or `[1., 1.]`, with the coordinate of one of the two other points shifting to maintain proportionality.

For example:

*   The default `Triangle2d` has UV-coordinates `[0., 0.]`, `[0., 1.]`, \[`0.5, 1.]`.
*   The triangle with vertices `vec2(0., 0.)`, `vec2(1., 0.)`, `vec2(2., 1.)` has UV-coordinates `[0., 0.]`, `[0.5, 0.]`, `[1., 1.]`.
*   The triangle with vertices `vec2(0., 0.)`, `vec2(1., 0.)`, `vec2(-2., 1.)` has UV-coordinates `[2./3., 0.]`, `[1., 0.]`, `[0., 1.]`.

* * *

### Use `Vec3A` for 3D bounding volumes and raycasts [#](#use-vec3a-for-3d-bounding-volumes-and-raycasts)

`Aabb3d`, `BoundingSphere`, and `RayCast3d` now use `Vec3A` instead of `Vec3` internally. `Vec3A` is the SIMD-accelerated form of `Vec3`, so it should provide performance improvements without visible changes in behavior.

If you manually construct any of the affected structs, you will have to convert into a `Vec3A`.

```
// 0.13
let x = Vec3::new(5.0, -2.0);

let aabb = Aabb3d {
    min: Vec3::ZERO,
    max: x,
};

// 0.14
let x = Vec3::new(5.0, -2.0);

let aabb = Aabb3d {
    // Both variants are very similar, so you can usually replace `Vec3` with `Vec3A`.
    min: Vec3A::ZERO,
    // In cases where you cannot, use the `From` and `Into` traits.
    max: x.into(),
};

```


* * *

### Update `glam` to 0.27 [#](#update-glam-to-0-27)

`glam` has been updated from 0.25 to 0.27. Please view [the changelog](https://github.com/bitshifter/glam-rs/blob/e1b521a4c8146f27b97e510d38fab489c39650d1/CHANGELOG.md#0270---2024-03-23) for both 0.26 and 0.27 to update your code.

The largest breaking change is that the `fract()` method for vector types now evaluates as `self - self.trunc()` instead of `self - self.floor()`. If you require the old behavior, use the `fract_gl()` method instead.

* * *

### Common `MeshBuilder` trait [#](#common-meshbuilder-trait)

All shape mesh builders (`ConeMeshBuilder`, `PlaneMeshBuilder`, etc.) have a method `build()` for converting into a `Mesh`. This method has been made into a common trait `MeshBuilder`. You will need to import this trait if you use `build()` but do not use the prelude.

* * *

### Add angle range to `TorusMeshBuilder` [#](#add-angle-range-to-torusmeshbuilder)

`TorusMeshBuilder` is no longer `Copy` because it contains a `RangeInclusive` (`x..=y`) for the angle range. You will need to call `clone()` manually in any scenario where it was implicitly copied before.

* * *

### Add subdivisions to `PlaneMeshBuilder` [#](#add-subdivisions-to-planemeshbuilder)

In 0.13 the `Plane` type was deprecated in favor of `Plane2d` and `Plane3d`. The new plane types did not provide a method for subdivision, which is now amended.

If you used the `Plane::subdivisions` property, you now need to convert a `Plane3d` into a `PlaneMeshBuilder`.

```
// 0.13
let plane = Plane {
    subdivisions: 10,
    ..default()
};

// 0.14
let plane = Plane3d::default().mesh().subdivisions(10);

```


* * *

### Make `Transform::rotate_axis` and `Transform::rotate_local_axis` use `Dir3` [#](#make-transform-rotate-axis-and-transform-rotate-local-axis-use-dir3)

`Transform::rotate_axis()` and `Transform::rotate_local_axis()` now require a `Dir3` instead of a `Vec3` because the axis is expected to be normalized. In general you can call `Dir3::new()` with a `Vec3`, which will automatically normalize it, though you must handle the `Result` in case the vector is invalid.

Note that most constants like `Vec3::X` have a corresponding `Dir3` variant, such as `Dir3::X`.

* * *

### Use `Dir3` for local axis methods in `GlobalTransform` [#](#use-dir3-for-local-axis-methods-in-globaltransform)

The `GlobalTransform` component's directional axis methods (`right()`, `left()`, `up()`, `down()`, `back()`, `forward()`) have been updated from returning a `Vec3` to a `Dir3`. `Dir3` implements `Deref<Target = Vec>`, but if you need mutable access you can call `Vec3::from()`.

* * *

### Fix `Ord` and `PartialOrd` differing for `FloatOrd` [#](#fix-ord-and-partialord-differing-for-floatord)

`FloatOrd`'s `PartialOrd` implementation used to differ in behavior from its `Ord` implementation, but it has since been fixed so they both now match. The current implementation of `PartialOrd` will never return `None`, as it now falls back to the `Ord` implementation. If you depended on this mismatched behavior, consider using the `PartialOrd` implementation on the inner `f32`.

* * *

### Move `FloatOrd` into `bevy::math` [#](#move-floatord-into-bevy-math)

`FloatOrd` has been moved to into the `bevy::math` module. Please update your import statements:

```
// 0.13
use bevy::utils::FloatOrd;

// 0.14
use bevy::math::FloatOrd;

```


Reflection [#](#reflection)
---------------------------

### Register missing types manually [#](#register-missing-types-manually)

Many external types are no longer registered into the type registry by Bevy's default plugin. Generally, only types used by other Bevy types (due to the new recursive registration) will be registered by default. If you were using reflection features with types from `std` or `glam` you may need to manually register them.

```
App::new().register_type::<DMat3>();

```


* * *

### Change `ReflectSerialize` trait bounds [#](#change-reflectserialize-trait-bounds)

`ReflectSerialize` now requires the `TypePath` and `FromReflect` trait bounds instead of `Reflect`. You will have to implement these traits if you previously opted-out of them. For instance, if you used `#[reflect(type_path = false)]` or `#[reflect(from_reflect = false)]`, you will have to remove them.

* * *

### Recursive registration of types [#](#recursive-registration-of-types)

It is now possible to recursively register types, but in doing so all (unignored) reflected fields need to implement `GetTypeRegistration`. This is automatically done when `Reflect` is derived, but manual implementations will need to also implement `GetTypeRegistration`.

```
#[derive(Reflect)]
struct Foo<T: FromReflect> {
    data: MyCustomType<T>
}

// 0.13
impl<T: FromReflect> Reflect for MyCustomType<T> {
    // ...
}

// 0.14
impl<T: FromReflect + GetTypeRegistration> Reflect for MyCustomType<T> {
    // ...
}

impl<T: FromReflect + GetTypeRegistration> GetTypeRegistration for MyCustomType<T> {
    // ...
}

```


* * *

### Rename `UntypedReflectDeserializer` to `ReflectDeserializer` [#](#rename-untypedreflectdeserializer-to-reflectdeserializer)

`UntypedReflectDeserializer` has been renamed to `ReflectDeserializer`. Any usage will need to be updated accordingly:

```
// 0.13
let reflect_deserializer = UntypedReflectDeserializer::new(&registry);

// 0.14
let reflect_deserializer = ReflectDeserializer::new(&registry);

```


* * *

### Implement `Reflect` for `Result` as an enum [#](#implement-reflect-for-result-as-an-enum)

`Result`'s `Reflect` implementation has been changed to make it a `ReflectKind::Enum` instead of a `ReflectKind::Value`. This increases its consistency with `Option` and allows for inspection of its contents.

Now, `Result<T, E>` no longer requires both `T` and `E` to be `Clone`, but instead requires them to implement `FromReflect`. Additionally, `<Result<T, E> as Reflect>::reflect_*` now returns the `Enum` variant instead of `Value`.

* * *

### Serialize scene with `&TypeRegistry` and rename `serialize_ron()` to `serialize()` [#](#serialize-scene-with-typeregistry-and-rename-serialize-ron-to-serialize)

`SceneSerializer` and all related serialization helpers now take `&TypeRegistry` instead of `&TypeRegistryArc`. You can access the former from the latter with `TypeRegistryArc::read()`.

Furthermore, `DynamicScene::serialize_ron()` has been renamed to `serialize()`. This has been done to highlight that this function is not about serializing into RON specifically, but rather the official Bevy scene format (`.scn` / `.scn.ron`). This leaves room to change the format in the future, if need be.

```
// 0.13
let world = World::new();
let scene = DynamicScene::from_world(&world);

let type_registry_arc: &TypeRegistryArc = &**world.resource::<AppTypeRegistry>();

let serialized_scene = scene.serialize_ron(type_registry_arc).unwrap();

// 0.14
let world = World::new();
let scene = DynamicScene::from_world(&world);

let type_registry_arc: &TypeRegistryArc = &**world.resource::<AppTypeRegistry>();

// We now need to retrieve the inner `TypeRegistry`.
let type_registry = type_registry_arc.read();

// `serialize_ron` has been renamed to `serialize`, and now takes a reference to `TypeRegistry`.
let serialized_scene = scene.serialize(&type_registry).unwrap();

```


Rendering [#](#rendering)
-------------------------

### Make default behavior for `BackgroundColor` and `BorderColor` more intuitive [#](#make-default-behavior-for-backgroundcolor-and-bordercolor-more-intuitive)

`BackgroundColor` no longer tints the color of images in `ImageBundle` or `ButtonBundle`. Set `UiImage::color` to tint images instead. Furthermore, the new default texture for `UiImage` is now a transparent white square. Use `UiImage::solid_color` to quickly draw debug images. Finally, the default value for `BackgroundColor` and `BorderColor` is now transparent. Set the color to white manually to return to previous behavior.

* * *

### Rename `Camera3dBundle::dither` to `deband_dither` [#](#rename-camera3dbundle-dither-to-deband-dither)

`Camera3dBundle::dither` has been renamed to `deband_dither` to make it consistent with `Camera2dBundle`. If you construct or access this field, you will have to update your usage.

* * *

### Rename `affine_to_square()` to `affine3_to_square()` [#](#rename-affine-to-square-to-affine3-to-square)

The `affine_to_square()` **shader** function has been renamed to `affine3_to_square`, in order to give room for `affine2_to_square`. Please update your import statements and usages accordingly. (Note that this is not Rust, but instead WGSL.)

```
// 0.13
#import bevy_render::maths::affine_to_square

// 0.14
#import bevy_render::maths::affine3_to_square

```


* * *

### Move `AlphaMode` into `bevy::render` [#](#move-alphamode-into-bevy-render)

`AlphaMode` has been moved from `bevy::pbr` to `bevy::render`. If you import them directly, you will need to update your import statements.

```
// 0.13
use bevy::pbr::AlphaMode;

// 0.14
use bevy::render::alpha::AlphaMode;

```


* * *

### Use `UVec2` when working with texture dimensions [#](#use-uvec2-when-working-with-texture-dimensions)

`GpuImage`, `TextureAtlasLayout`, `TextureAtlasBuilder`, `DynamicAtlasTextureBuilder`, and `FontAtlas` have been changed to store their dimensions in integers rather than floating point numbers, in order to increase consistency with the underlying texture data. Instances of `Vec2` and `Rect` have been replaced with `UVec2` and `URect`.

Migrating this is tricky because the conversion from `f32` to `u32` is lossy. If you work with constants, you can simply rewrite the code. If you work with user input, you could choose to simply discard the decimal (`1.4 as u32`) or round it first (`1.83.round() as u32`).

* * *

### Fix `CameraProjectionPlugin` not implementing `Plugin` in some cases [#](#fix-cameraprojectionplugin-not-implementing-plugin-in-some-cases)

There was a bug with `CameraProjectionPlugin<T>` where it would sometimes not implement `Plugin` if `T` did not implement `Component` and `GetTypeRegistration`. This has now been fixed by requiring `T: CameraProjection + Component + GetTypeRegistration`.

* * *

### Replace `random1D()` with `rand_f()` shader function [#](#replace-random1d-with-rand-f-shader-function)

The `bevy_pbr::utils::random1D()` **shader** function has been replaced by the similar `bevy_pbr::utils::rand_f()`. Note that if you convert the returned `f32` to a different data type, you may be interested in `rand_u()` which returns a `u32` and `rand_vec2f()` which returns a `vec2<f32>`.

* * *

### Intern mesh vertex buffer layouts [#](#intern-mesh-vertex-buffer-layouts)

Duplicate `MeshVertexBufferLayout`s are now combined into a single object, `MeshVertexBufferLayoutRef`, which contains an atomically-reference-counted (`Arc`) pointer to the layout. By interning these layouts, the results of `PartialEq` can be cached, resulting in a speedup in rendering. Code that was using `MeshVertexBufferLayout` may need to be updated to use `MeshVertexBufferLayoutRef` instead.

* * *

### Make `GpuArrayBufferIndex::index` a u32 [#](#make-gpuarraybufferindex-index-a-u32)

`GpuArrayBufferIndex::index` is now a `u32` instead of a `NonMaxU32`, since restricting the number isn't necessary anymore. Please update any usage to use `u32` instead.

* * *

### Allow disabling shadows through `MaterialPlugin` [#](#allow-disabling-shadows-through-materialplugin)

`MaterialPlugin` now has a `shadows_enabled` property. If you manually constructed this plugin, you may need to set it. By default it is true, but you can disable shadows entirely by setting it to false.

* * *

### Remove `SpritePipeline::COLORED` [#](#remove-spritepipeline-colored)

The `COLORED` flag of `SpritePipelineKey` has been removed, since it is no longer used. In doing so, the raw values of `HDR`, `TONEMAP_IN_SHADER`, and `DEBAND_DITHER` have changed. If you are manually converting a `u32` into `SpritePipelineKey`, you may need to update it.

* * *

### Sorted and binned render phase items, resources, and non-meshes [#](#sorted-and-binned-render-phase-items-resources-and-non-meshes)

Usage of `PhaseItem` has been split into `BinnedPhaseItem` and `SortedPhaseItem`. If you have custom `PhaseItem`s you will need to choose one of the new types. Notably some phases _must_ be Sorted (such as Transparent and Transmissive), while others can be Binned. Effectively Sorted is "what Bevy did before" and Binned is new, and the point of this change is to avoid sorting when possible for improved performance.

If you're looking for a quick migration, consider picking [`SortedPhaseItem`](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/trait.SortedPhaseItem.html) which requires the fewest code changes.

If you're looking for higher performance (and your phase doesn’t require sorting) you may want to pick [`BinnedPhaseItem`](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/trait.BinnedPhaseItem.html). Notably bins are populated based on `BinKey` and everything in the same bin is potentially batchable.

If you are only consuming these types, then a `Query` for a type like `&mut RenderPhase<Transparent2d>` will become a `Resource` as such:

```
mut transparent_render_phases: ResMut<ViewSortedRenderPhases<Transparent2d>>

```


`ViewSortedRenderPhases` and `ViewBinnedRenderPhases` are used in accordance with which phase items you're trying to access (sorted or binned).

Examples of [`SortedPhaseItems`s](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/trait.SortedPhaseItem.html#implementors):

*   Transmissive3d
*   Transparent2d
*   Transparent3d
*   TransparentUi

Examples of [`BinnedPhaseItem`s](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/trait.BinnedPhaseItem.html#implementors) include:

*   Opaque3d
*   Opaque3dPrepass
*   Opaque3dDeferred
*   AlphaMask3d
*   AlphaMask3dPrepass
*   AlphaMask3dDeferred
*   [Shadow](https://docs.rs/bevy/0.14.0/bevy/pbr/struct.Shadow.html)

If you do not have a mesh (such as for GPU-driven particles or procedural generation) and want to use the new binned behavior, the [`BinnedRenderPhase`](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/struct.BinnedRenderPhase.html) includes a `non_mesh_items` collection which correlates with the [`BinnedRenderPhaseType`](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/struct.BinnedRenderPhase.html). This type is used when [add](https://docs.rs/bevy/0.14.0/bevy/render/render_phase/struct.BinnedRenderPhase.html#method.add)ing items to the `BinnedRenderPhase`.

It may be additionally useful to checkout the new [custom\_phase\_item example](https://github.com/bevyengine/bevy/blob/5876352206d1bcea792825bf013eb212383b73d6/examples/shader/custom_phase_item.rs) which details some of the new APIs.

* * *

### GPU frustum culling [#](#gpu-frustum-culling)

For `PhaseItem`s, the `dynamic_offset: Option<NonMaxU32>` field is now `extra_index: PhaseItemExtraIndex`, which wraps a `u32`. Instead of `None`, use `PhaseItemExtraIndex::NONE`.

This change affects `AlphaMask3d`, `AlphaMask3dDeferred`, `AlphaMask3dPrepass`, `Opaque2d`, `Opaque3dDeferred`, `Opaque3dPrepass`, `Shadow`, `Transmissive3d`, `Transparent2d`, `Transparent3d`, and `TransparentUi`.

* * *

### Remove `DeterministicRenderingConfig` [#](#remove-deterministicrenderingconfig)

`DeterministicRenderingConfig` has been removed because its only property, `stable_sort_z_fighting`, is no longer needed. Z-fighting has been generally removed now that opaque items are binned instead of sorted.

* * *

### Optimize `queue_material_meshes` and remove some bit manipulation [#](#optimize-queue-material-meshes-and-remove-some-bit-manipulation)

The `primitive_topology` field on `GpuMesh` is now an getter method: `GpuMesh::primitive_topology()`.

For performance reasons, `MeshPipelineKey` has been split into `BaseMeshPipelineKey`, which lives in `bevy::render`, and `MeshPipelineKey`, which lives in `bevy::pbr`. These two may be combined with bitwise-or to produce the final `MeshPipelineKey`.

```
let base_pipeline_key = BaseMeshPipelineKey::all();
let pbr_pipeline_key = MeshPipelineKey::all();

let pipeline_key: u64 = base_pipeline_key.bits() | pbr_pipeline_key.bits();

```


* * *

### Disable `RAY_QUERY` and `RAY_TRACING_ACCELERATION_STRUCTURE` by default [#](#disable-ray-query-and-ray-tracing-acceleration-structure-by-default)

The `RAY_QUERY` and `RAY_TRACING_ACCELERATION_STRUCTURE` `wgpu` features are now disabled by default, due to some users having their program crash while initializing. (The `wgpu` issue for this can be found [here](https://github.com/gfx-rs/wgpu/issues/5488).)

If you use these features, you will need to re-enable them through `WgpuSettings::features`:

```
let mut settings = WgpuSettings::default();

// Enable `RAY_QUERY` and `RAY_TRACING_ACCELERATION_STRUCTURE`, along with the defaults.
settings.features |= WgpuFeatures::RAY_QUERY | WgpuFeatures::RAY_TRACING_ACCELERATION_STRUCTURE;

App::new()
    .add_plugins(DefaultPlugins.set(RenderPlugin {
        render_creation: settings.into(),
        ..default()
    }))
    .run()

```


Note that `WgpuSettings::default()` automatically configures good default flags for Bevy, while `WgpuFeatures::default()` is the equivalent of `WgpuFeatures::empty()`.

* * *

### Upload previous frame's `inverse_view` to GPU [#](#upload-previous-frame-s-inverse-view-to-gpu)

`PreviousViewProjection` has been renamed to `PreviousViewData` and `PreviousViewProjectionUniformOffset` has been renamed to `PreviousViewUniformOffset`. Additionally, a few systems have been renamed:

*   `update_previous_view_projections` to `update_previous_view_data`
*   `extract_camera_previous_view_projection` to `extract_camera_previous_view_data`
*   `prepare_previous_view_projection_uniforms` to `prepare_previous_view_uniforms`

* * *

### Generate `MeshUniform`s on the GPU when available. [#](#generate-meshuniforms-on-the-gpu-when-available)

Custom render phases now need multiple systems beyond just `batch_and_prepare_render_phase`. Code that was previously creating custom render phases should now add a `BinnedRenderPhasePlugin` or `SortedRenderPhasePlugin` as appropriate, instead of directly adding `batch_and_prepare_render_phase`.

* * *

### Add texture coord flipping to `StandardMaterial` [#](#add-texture-coord-flipping-to-standardmaterial)

`Quad` was deprecated in 0.13, though its replacement `Rectangle` did not provide a clear replacement for `Quad::flip`. This has been amended: now you can call `flip()` on any `StandardMaterial`.

Please note that `Quad::flip` was specifically _horizontal_ flipping, though `StandardMaterial::flip()` supports both _vertical_ and _horizontal_ flipping.

* * *

### Rename `ShadowFilteringMethod`'s `Castano13` and `Jimenez14` variants [#](#rename-shadowfilteringmethod-s-castano13-and-jimenez14-variants)

`ShadowFilteringMethod::Castano13` and `ShadowFilteringMethod::Jimenez14` have been renamed to `Gaussian` and `Temporal` respectively to leave room for expansion in the future, though the corresponding authors are still credited in the documentation.

* * *

### Store lists of `VisibleEntities` separately [#](#store-lists-of-visibleentities-separately)

`check_visibility()` and `VisibleEntities` now store the four types of renderable entities–-2D meshes, 3D meshes, lights, and UI elements-–separately. If your custom rendering code examines `VisibleEntities`, it will now need to specify which type of entity it’s interested in using the `WithMesh2d`, `WithMesh`, `WithLight`, and `WithNode` types respectively. If your app introduces a new type of renderable entity, you’ll need to add an instance of the `check_visibility` system with the appropriate query filter to the main world schedule to accommodate your new component or components. For example:

```
struct MyCustomRenderable;

App::new()
    .add_plugins(DefaultPlugins)
    .add_systems(
        PostUpdate,
        check_visibility::<With<MyCustomRenderable>>
            .in_set(VisibilitySystems::CheckVisibility)
    )
    .run();

```


* * *

### Make `Text` require `SpriteSource` [#](#make-text-require-spritesource)

`Text` now requires a `SpriteSource` marker component in order to render. This component has been added to `Text2dBundle` and may need to be specified if `..default()` isn't used.

* * *

### Expose `desired_maximum_frame_latency` [#](#expose-desired-maximum-frame-latency)

The `desired_maximum_frame_latency` field has been added to `Window` and `ExtractedWindow`. It is an `Option<NonZero<u32>>` that hints the maximum number of queued frames allowed on the GPU. Higher values may result in smoother frames and avoids freezes due to CPU-GPU data upload, but all at the cost of higher input latency. Setting `desired_maximum_frame_latency` to `None` will make it fall back to the default value, which is currently 2.

* * *

### Merge `VisibilitySystems` frusta variants [#](#merge-visibilitysystems-frusta-variants)

`VisibilitySystems`'s `UpdateOrthographicFrusta`, `UpdatePerspectiveFrusta`, and `UpdateProjectionFrusta` variants have been removed in favor of the new `VisibilitySystems::UpdateFrusta` variant.

* * *

### Expand color grading [#](#expand-color-grading)

The `ColorGrading` component has been expanded to support individually configuring the shadow, midtone, and highlight sections. If you configured the `gamma` or `pre_saturation` fields previously, you will now have to set them for all sections:

```
// 0.13
let color_grading = ColorGrading {
    gamma: 2.0,
    pre_saturation: 0.8,
    ..default()
};

// 0.14
let mut color_grading = ColorGrading::default();

for section in color_grading.all_sections_mut() {
    section.gamma = 2.0;
    // `pre_saturation` has been renamed to `saturation`.
    section.saturation = 0.8;
}

```


Additionally, the `post_saturation` and `exposure` fields have been moved specifically to the new `global` field, which is a `ColorGradingGlobal` that supports more operations for the image as a whole.

```
// 0.13
let color_grading = ColorGrading {
    post_saturation: 1.2,
    exposure: 0.4,
};

// 0.14
let color_grading = ColorGrading {
    global: ColorGradingGlobal {
        post_saturation: 1.2,
        exposure: 0.4,
        ..default()
    },
    ..default()
};

```


* * *

### Rename `BufferVec` to `RawBufferVec` [#](#rename-buffervec-to-rawbuffervec)

`BufferVec` has been renamed to `RawBufferVec` because a new implementation of `BufferVec` has taken its name. The new `BufferVec<T>` no longer requires `T: Pod`, but instead `ShaderType` from the `encase` library.

For most cases you can simply switch to using `RawBufferVec`, but if you have more complex data you may be interested in the new `BufferVec` implementation.

* * *

### Implement clearcoat [#](#implement-clearcoat)

The lighting functions in the `pbr_lighting` WGSL module now have clearcoat parameters if `STANDARD_MATERIAL_CLEARCOAT` is defined. Additionally, the `R` reflection vector parameter has been removed from some lighting functions, as it was unused.

* * *

### Split `Node2d::MainPass` [#](#split-node2d-mainpass)

`Node2d::MainPass` has been split into 3 separate phases: `StartMainPass`, `MainTransparentPass`, and `EndMainPass`. If you previously used `MainPass` to order your own custom nodes, you now need to order them relative to `StartMainPass` and `EndMainPass`.

* * *

### Remove limit on `RenderLayers` [#](#remove-limit-on-renderlayers)

There is no longer a limit on the total amount of `RenderLayers`, and so the `TOTAL_LAYERS` associated constant and `all()` constructor have been removed. Entities expecting to be visible on all layers, such as lights, should either create a constant listing all known layers used by the application or compute the active layers that are in use at runtime.

The `Copy` trait is no longer implemented on `RenderLayers`. Instead you should use the `.clone()` function from the `Clone` trait which `Renderlayers` still implements.

* * *

### Fix astronomic emissive colors required for bloom [#](#fix-astronomic-emissive-colors-required-for-bloom)

Emissive color and camera exposure now play nicely with each other. Before, the `emissive` property of a `StandardMaterial` had to be massive (in the thousands) in order for effects such as bloom to be visible. This has been scaled down, so you may have to re-adjust your emissive colors.

```
// 0.13
StandardMaterial {
    emissive: Color::linear_rgb(23000.0, 9000.0, 3000.0),
    ..default()
}

// 0.14
StandardMaterial {
    // Much more reasonable! :)
    emissive: Color::linear_rgb(13.99, 5.32, 2.0),
    ..default()
}

```


You may also be interested in the `StandardMaterial::emissive_exposure_weight` property.

* * *

### More idiomatic `TextureAtlasBuilder` [#](#more-idiomatic-textureatlasbuilder)

`TextureAtlasBuilder` has been modified to be more consistent with other builders. As part of this, most methods now return `&mut Self` instead of `Self` and `finish()` has been renamed to `build()`.

```
// 0.13
let (texture_atlas_layout, texture) = TextureAtlasBuilder::default()
    .padding(UVec2::default())
    .format(TextureFormat::bevy_default())
    .finish()
    .unwrap();

// 0.14
let (texture_atlas_layout, texture) = TextureAtlasBuilder::default()
    .padding(UVec2::default())
    .format(TextureFormat::bevy_default());
    .build() // This is now `build()`.
    .unwrap();

```


* * *

### Normalise matrix naming [#](#normalise-matrix-naming)

All matrices have been renamed to follow the convention `x_from_y` in order to decrease confusion while increasing readability.

*   `Frustum`'s `from_view_projection`, `from_view_projection_custom_far` and `from_view_projection_no_far` methods were renamed to `from_clip_from_world`, `from_clip_from_world_custom_far` and `from_clip_from_world_no_far`.
*   `ComputedCameraValues::projection_matrix` was renamed to `clip_from_view`.
*   `CameraProjection::get_projection_matrix` was renamed to `get_clip_from_view` (this affects implementations on `Projection`, `PerspectiveProjection` and `OrthographicProjection`).
*   `ViewRangefinder3d::from_view_matrix` was renamed to `from_world_from_view`.
*   `PreviousViewData`'s members were renamed to `view_from_world` and `clip_from_world`.
*   `ExtractedView`'s `projection`, `transform` and `view_projection` were renamed to `clip_from_view`, `world_from_view` and `clip_from_world`.
*   `ViewUniform`'s `view_proj`, `unjittered_view_proj`, `inverse_view_proj`, `view`, `inverse_view`, `projection` and `inverse_projection` were renamed to `clip_from_world`, `unjittered_clip_from_world`, `world_from_clip`, `world_from_view`, `view_from_world`, `clip_from_view` and `view_from_clip`.
*   `GpuDirectionalCascade::view_projection` was renamed to `clip_from_world`.
*   `MeshTransforms`' `transform` and `previous_transform` were renamed to `world_from_local` and `previous_world_from_local`.
*   `MeshUniform`'s `transform`, `previous_transform`, `inverse_transpose_model_a` and `inverse_transpose_model_b` were renamed to `world_from_local`, `previous_world_from_local`, `local_from_world_transpose_a` and `local_from_world_transpose_b` (the `Mesh` type in WGSL mirrors this, however `transform` and `previous_transform` were named `model` and `previous_model`).
*   `Mesh2dTransforms::transform` was renamed to `world_from_local`.
*   `Mesh2dUniform`'s `transform`, `inverse_transpose_model_a` and `inverse_transpose_model_b` were renamed to `world_from_local`, `local_from_world_transpose_a` and `local_from_world_transpose_b` (the `Mesh2d` type in WGSL mirrors this).
*   In WGSL, `bevy_pbr::mesh_functions`, `get_model_matrix` and `get_previous_model_matrix` were renamed to `get_world_from_local` and `get_previous_world_from_local`.
*   In WGSL, `bevy_sprite::mesh2d_functions::get_model_matrix` was renamed to `get_world_from_local`.

* * *

### Rename "point light" to "clusterable object" in cluster contexts [#](#rename-point-light-to-clusterable-object-in-cluster-contexts)

In the PBR shaders, `point_lights` is now known as `clusterable_objects`, `PointLight` is now known as `ClusterableObject`, and `cluster_light_index_lists` is now known as `clusterable_object_index_lists`. This rename generalizes over clusterable objects, which adds room for light probes and decals in the future.

* * *

### Make `Mesh::merge()` take a reference of `Mesh` [#](#make-mesh-merge-take-a-reference-of-mesh)

`Mesh::merge()` now takes `&Mesh` instead of `Mesh`. Because of this, you can now share the same `Mesh` across multiple `merge()` calls without cloning it.

* * *

### Store `ClearColorConfig` instead of `LoadOp<Color>` in `CameraOutputMode` [#](#store-clearcolorconfig-instead-of-loadop-color-in-cameraoutputmode)

`CameraOutputMode::Write` now stores a `ClearColorConfig` instead of a `LoadOp<Color>`. Use the following table to convert between the two enums:


|LoadOp<Color>|ClearColorConfig|
|-------------|----------------|
|Clear(color) |Custom(color)   |
|Load         |None            |


`ClearColorConfig` has an additional variant, `Default`, which inherits the clear color from the `ClearColor` resource.

* * *

### `wgpu` 0.20 [#](#wgpu-0-20)

Bevy now depends on `wgpu` 0.20, `naga` 0.20, and `naga_oil` 0.14. If you manually specify any of these crates in your `Cargo.toml`, make sure to update their versions to prevent them from being duplicated.

Furthermore, timestamps inside of encoders are now disallowed on WebGPU (though they still work on native). Use the `TIMESTAMP_QUERY_INSIDE_ENCODERS` feature to check for support.

* * *

### Deprecate `SpriteSheetBundle` and `AtlasImageBundle` [#](#deprecate-spritesheetbundle-and-atlasimagebundle)

`SpriteSheetBundle` has been deprecated as part of a style and maintenance-motivated move towards optional components that add functionality, rather than a proliferation of bundles. Insert the `TextureAtlas` component alongside a `SpriteBundle` instead.

```
// 0.13
commands.spawn(SpriteSheetBundle {
    texture,
    atlas: TextureAtlas {
        layout,
        ..default()
    },
    ..default()
});
// 0.14
commands.spawn((
    SpriteBundle {
        texture,
        ..default()
    },
    TextureAtlas {
        layout,
        ..default()
    },
));

```


`AtlasImageBundle` has been deprecated. Insert the `TextureAtlas` component alongside an `ImageBundle` instead.

```
// 0.13
commands.spawn(AtlasImageBundle {
    image,
    atlas: TextureAtlas {
        layout,
        ..default()
    },
    ..default()
});
// 0.14
commands.spawn((
    ImageBundle {
        image,
        ..default()
    },
    TextureAtlas {
        layout,
        ..default()
    },
));

```


* * *

### Decouple `BackgroundColor` from `UiImage` [#](#decouple-backgroundcolor-from-uiimage)

The [`BackgroundColor`](https://docs.rs/bevy/0.14.0/bevy/prelude/struct.BackgroundColor.html) component now renders a solid-color background behind [`UiImage`](https://docs.rs/bevy/0.14.0/bevy/prelude/struct.UiImage.html#structfield.color) instead of tinting its color. Use the `color` field of `UiImage` for tinting.

```
// 0.13
ButtonBundle {
    image: UiImage::new(my_texture),
    background_color: my_color_tint.into(),
    ..default()
}

// 0.14
ButtonBundle {
    image: UiImage::new(my_texture).with_color(my_color_tint),
    ..default()
}

```


Some UI systems have been split or renamed.

*   `bevy_ui::RenderUiSystem::ExtractNode` has been split into `ExtractBackgrounds`, `ExtractImages`, `ExtractBorders`, and `ExtractText`.
*   `bevy_ui::extract_uinodes` has been split into `extract_uinode_background_colors` and `extract_uinode_images`.
*   `bevy_ui::extract_text_uinodes` has been renamed to `extract_uinode_text`.

* * *

### Remove generic camera from `extract_default_ui_camera_view()` system [#](#remove-generic-camera-from-extract-default-ui-camera-view-system)

The `bevy::ui::render::extract_default_ui_camera_view()` system is now hard-wired to both the `Camera2d` and `Camera3d` components, and is no longer added twice for each type.

This change was made to fix a bug introduced after moving render phases to resources. The first thing this system does is clear out all entities from the previous frame. By having two separate systems, one was always clearing out the other, causing some entities to not be rendered.

* * *

### Rename `need_new_surfaces()` system to `need_surface_configuration()` [#](#rename-need-new-surfaces-system-to-need-surface-configuration)

The `need_new_surfaces()` system has been renamed `need_surface_configuration()` as part of a bug fix where Bevy apps would crash on iOS when the screen orientation was changed.

* * *

### Require windowing backends to store windows in `WindowWrapper` [#](#require-windowing-backends-to-store-windows-in-windowwrapper)

Windowing backends now need to store their window in the new `WindowWrapper`, so that Bevy can control when it is dropped. This fixes a number of bugs and crashes related to the window being dropped before the pipelined renderer is finished drawing to it.

Tasks [#](#tasks)
-----------------

### Add an index argument to parallel iteration helpers [#](#add-an-index-argument-to-parallel-iteration-helpers)

Closures passed as arguments to `par_chunk_map()`, `par_splat_map()`, `par_chunk_map_mut()`, and `par_splat_map_mut()` now take an additional index argument specifying which part of the slice is being processed.

```
// 0.13
items.par_chunk_map(&task_pool, 100, |chunk| {
    // ...
});

// 0.14
items.par_chunk_map(&task_pool, 100, |_index, chunk| {
    // ...
});

```


UI [#](#ui)
-----------

### Fix spawning `NodeBundle` destroying previous ones [#](#fix-spawning-nodebundle-destroying-previous-ones)

There was a regression in 0.13.1 `NodeBundle`s to destroy previous ones when spawned, and the original workaround was to add `position_type: Absolute` to all of the root nodes. This bug is now fixed, so you can remove the workaround.

* * *

### Rename `Rect::inset()` to `inflate()` [#](#rename-rect-inset-to-inflate)

`Rect::inset()`, `IRect::inset()`, and `URect::inset()` have been renamed to `inflate()` to fit the actual behavior.

* * *

### Updates default font size to 24px [#](#updates-default-font-size-to-24px)

The default font size for `TextStyle` has been increased from 12px to 24px. If you preferred the original size, you can override it using the `TextStyle::font_size` property.

Utils [#](#utils)
-----------------

### Disentangle `bevy::utils` / `bevy::core`'s re-exported crates [#](#disentangle-bevy-utils-bevy-core-s-re-exported-crates)

`bevy::utils` no longer re-exports `petgraph`, `uuid`, `nonmax`, `smallvec`, or `thiserror`. Additionally, `bevy::core` no longer re-exports `bytemuck`'s `bytes_of`, `cast_slice`, `Pod`, and `Zeroable`.

If you need any of these as dependencies, you can add them to your own `Cargo.toml`.

Windowing [#](#windowing)
-------------------------

### Re-add `Window::fit_canvas_to_parent` [#](#re-add-window-fit-canvas-to-parent)

`Window::fit_canvas_to_parent` is a property on WASM that automatically resizes the canvas element to the size of its parent, usually the screen. It was removed in 0.13, but that ended up being problematic because many users depended on its behavior when they could not customize the CSS. It has now been re-added to account for this need.

* * *

### Remove window from `WinitWindows` when it is closed [#](#remove-window-from-winitwindows-when-it-is-closed)

`WinitWindows::get_window_entity` now returns `None` after a window is closed, instead of an entity that no longer exists.

* * *

### Make window close the frame after it is requested [#](#make-window-close-the-frame-after-it-is-requested)

Windows now close a frame after their exit has been requested in order to fix several regressions. If you have custom exit logic, ensure that it does not rely on the app exiting the same frame the window is closed.

* * *

### Upgrade to `winit` 0.30 [#](#upgrade-to-winit-0-30)

The custom UserEvent is now renamed as WakeUp, used to wake up the loop if anything happens outside the app (a new [custom\_user\_event](https://github.com/bevyengine/bevy/pull/13366/files#diff-2de8c0a8d3028d0059a3d80ae31b2bbc1cde2595ce2d317ea378fe3e0cf6ef2d) shows this behavior.

The internal `UpdateState` has been removed and replaced internally by the AppLifecycle. When changed, the AppLifecycle is sent as an event.

The `UpdateMode` now accepts only two values: `Continuous` and `Reactive`, but the latter exposes 3 new properties to enable reactive to device, user or window events. The previous `UpdateMode::Reactive` is now equivalent to `UpdateMode::reactive()`, while `UpdateMode::ReactiveLowPower` to `UpdateMode::reactive_low_power()`.

The `ApplicationLifecycle` has been renamed as `AppLifecycle`, and now contains the possible values of the application state inside the event loop:

*   `Idle`: the loop has not started yet
*   `Running` (previously called `Started`): the loop is running
*   `WillSuspend`: the loop is going to be suspended
*   `Suspended`: the loop is suspended
*   `WillResume`: the loop is going to be resumed

Note: the `Resumed` state has been removed since the resumed app is just running.

Finally, now that `winit` enables this, it extends the `WinitPlugin` to support custom events.

Without area [#](#without-area)
-------------------------------

### Fix `Node2d` typo [#](#fix-node2d-typo)

`Node2d::ConstrastAdaptiveSharpening` from `bevy::core_pipeline::core_2d::graph` has been renamed to fix a typo. It was originally `Constrast`, but is now `Contrast`.

```
// 0.13
Node2D::ConstrastAdaptiveSharpening

// 0.14
Node2D::ContrastAdaptiveSharpening

```


* * *

### Update to `fixedbitset` 0.5 [#](#update-to-fixedbitset-0-5)

`Access::grow` from `bevy::ecs::query` has been removed. Many operations now automatically grow the capacity.

```
// 0.13
let mut access = Access::new();
access.grow(1);
// Other operations...

// 0.14
let mut access = Access::new();
// Other operations...

```


* * *

### Move WASM panic handler from `LogPlugin` to `PanicHandlerPlugin` [#](#move-wasm-panic-handler-from-logplugin-to-panichandlerplugin)

`LogPlugin` used to silently override the panic handler on WASM targets. This functionality has now been split out into the new `PanicHandlerPlugin`, which was added to `DefaultPlugins`.

If you want nicer error messages on WASM but don't use `DefaultPlugins`, make sure to manually add `PanicHandlerPlugin` to the app.

```
App::new()
    .add_plugins((MinimalPlugins, PanicHandlerPlugin))
    .run()

```
