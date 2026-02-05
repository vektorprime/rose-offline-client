# 0.14 to 0.15
Accessibility [#](#accessibility)
---------------------------------

### Bump `accesskit` to 0.16 [#](#bump-accesskit-to-0-16)

`accesskit`’s `Role::StaticText` variant has been renamed to `Role::Label`.

* * *

### Remove `accesskit` re-export from `bevy_a11y` [#](#remove-accesskit-re-export-from-bevy-a11y)

```
# main.rs
--    use bevy_a11y::{
--        accesskit::{Node, Rect, Role},
--        AccessibilityNode,
--    };
++    use bevy_a11y::AccessibilityNode;
++    use accesskit::{Node, Rect, Role};

# Cargo.toml
++    accesskit = "0.17"

```


*   Users will need to add `accesskit = "0.17"` to the dependencies section of their `Cargo.toml` file and update their `accesskit` use statements to come directly from the external crate instead of `bevy_a11y`.
*   Make sure to keep the versions of `accesskit` aligned with the versions Bevy uses.

Animation [#](#animation)
-------------------------

### Deprecate `is_playing_animation` [#](#deprecate-is-playing-animation)

The user will just need to replace functions named `is_playing_animation` with `animation_is_playing`.

* * *

### Fix additive blending of quaternions [#](#fix-additive-blending-of-quaternions)

This PR changes the implementation of `Quat: Animatable`, which was not used internally by Bevy prior to this release version. If you relied on the old behavior of additive quaternion blending in manual applications, that code will have to be updated, as the old behavior was incorrect.

* * *

### Implement additive blending for animation graphs. [#](#implement-additive-blending-for-animation-graphs)

*   The `animgraph.ron` format has changed to accommodate the new _additive blending_ feature. You’ll need to change `clip` fields to instances of the new `AnimationNodeType` enum.

* * *

### Implement animation masks, allowing fine control of the targets that animations affect. [#](#implement-animation-masks-allowing-fine-control-of-the-targets-that-animations-affect)

*   The serialized format of animation graphs has changed with the addition of animation masks. To upgrade animation graph RON files, add `mask` and `mask_groups` fields as appropriate. (They can be safely set to zero.)

* * *

### Impose a more sensible ordering for animation graph evaluation. [#](#impose-a-more-sensible-ordering-for-animation-graph-evaluation)

The order in which animation graphs are evaluated has been changed to be more intuitive. Please see the diagram in the linked PR for a detailed explanation.

* * *

### Make `AnimationPlayer::start` and `::play` work accordingly to documentation [#](#make-animationplayer-start-and-play-work-accordingly-to-documentation)

`AnimationPlayer::start` now correspondingly to its docs restarts a running animation. `AnimationPlayer::play` doesn’t reset the weight anymore.

* * *

### Replace `Handle<AnimationGraph>` component with a wrapper [#](#replace-handle-animationgraph-component-with-a-wrapper)

`Handle<AnimationGraph>` is no longer a component. Instead, use the `AnimationGraphHandle` component which contains a `Handle<AnimationGraph>`.

* * *

### Curve-based animation [#](#curve-based-animation)

Most user code that does not directly deal with `AnimationClip` and `VariableCurve` will not need to be changed. On the other hand, `VariableCurve` has been completely overhauled. If you were previously defining animation curves in code using keyframes, you will need to migrate that code to use curve constructors instead. For example, a rotation animation defined using keyframes and added to an animation clip like this:

```
animation_clip.add_curve_to_target(
    animation_target_id,
    VariableCurve {
        keyframe_timestamps: vec![0.0, 1.0, 2.0, 3.0, 4.0],
        keyframes: Keyframes::Rotation(vec![
            Quat::IDENTITY,
            Quat::from_axis_angle(Vec3::Y, PI / 2.),
            Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
            Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
            Quat::IDENTITY,
        ]),
        interpolation: Interpolation::Linear,
    },
);

```


would now be added like this:

```
animation_clip.add_curve_to_target(
    animation_target_id,
    AnimatableKeyframeCurve::new([0.0, 1.0, 2.0, 3.0, 4.0].into_iter().zip([
        Quat::IDENTITY,
        Quat::from_axis_angle(Vec3::Y, PI / 2.),
        Quat::from_axis_angle(Vec3::Y, PI / 2. * 2.),
        Quat::from_axis_angle(Vec3::Y, PI / 2. * 3.),
        Quat::IDENTITY,
    ]))
    .map(RotationCurve)
    .expect("Failed to build rotation curve"),
);

```


Note that the interface of `AnimationClip::add_curve_to_target` has also changed (as this example shows, if subtly), and now takes its curve input as an `impl AnimationCurve`. If you need to add a `VariableCurve` directly, a new method `add_variable_curve_to_target` accommodates that (and serves as a one-to-one migration in this regard).

**For reviewers**

The diff is pretty big, and the structure of some of the changes might not be super-obvious:

*   `keyframes.rs` became `animation_curves.rs`, and `AnimationCurve` is based heavily on `Keyframes`, with the adaptors also largely following suite.
*   The Curve API adaptor structs were moved from `bevy_math::curve::mod` into their own module `adaptors`. There are no functional changes to how these adaptors work; this is just to make room for the specialized reflection implementations since `mod.rs` was getting kind of cramped.
*   The new module `gltf_curves` holds the additional curve constructions that are needed by the glTF loader. Note that the loader uses a mix of these and off-the-shelf `bevy_math` curve stuff.
*   `animatable.rs` no longer holds logic related to keyframe interpolation, which is now delegated to the existing abstractions in `bevy_math::curve::cores`.

* * *

### Allow animation clips to animate arbitrary properties. [#](#allow-animation-clips-to-animate-arbitrary-properties)

*   Animation keyframes are now an extensible trait, not an enum. Replace `Keyframes::Translation(...)`, `Keyframes::Scale(...)`, `Keyframes::Rotation(...)`, and `Keyframes::Weights(...)` with `Box::new(TranslationKeyframes(...))`, `Box::new(ScaleKeyframes(...))`, `Box::new(RotationKeyframes(...))`, and `Box::new(MorphWeightsKeyframes(...))` respectively.

App [#](#app)
-------------

### Add features to switch `NativeActivity` and `GameActivity` usage [#](#add-features-to-switch-nativeactivity-and-gameactivity-usage)

`GameActivity` is now the default activity for Android projects, replacing `NativeActivity`. `cargo-apk` has been replaced with `cargo-ndk` since the former is not compatible with `GameActivity`.

Before:

```
rustup target add aarch64-linux-android armv7-linux-androideabi
cargo install cargo-apk

```


After:

```
rustup target add aarch64-linux-android
cargo install cargo-ndk

```


Shared object files must be now built for the target architecture before launching package builds with the Gradle wrapper.

Before:

```
cargo apk build --package bevy_mobile_example

```


After:

```
cargo ndk -t arm64-v8a -o android_example/app/src/main/jniLibs build --package bevy_mobile_example
./android_example/gradlew build

```


(replace target and project name as required). Note that build output paths have changed. APK builds can be found under `app/build/outputs/apk`).

Android Studio may also be used.

Bevy may require the `libc++_shared.so` library to run on Android. This can be manually obtained from NDK source, or NDK describes a [`build.rs`](https://github.com/bbqsrc/cargo-ndk?tab=readme-ov-file#linking-against-and-copying-libc_sharedso-into-the-relevant-places-in-the-output-directory) approach. A suggested solution is also presented in the Bevy mobile example.

Applications that still require `NativeActivity` should:

1.  disable default features in `Cargo.toml`
2.  re-enable all default features _except_ `android-game-activity`
3.  enable the `android-native-activity` feature

* * *

### Allow ordering variable timesteps around fixed timesteps [#](#allow-ordering-variable-timesteps-around-fixed-timesteps)

[run\_fixed\_main\_schedule](https://docs.rs/bevy/latest/bevy/time/fn.run_fixed_main_schedule.html) is no longer public. If you used to order against it, use the new dedicated `RunFixedMainLoopSystem` system set instead. You can replace your usage of `run_fixed_main_schedule` one for one by `RunFixedMainLoopSystem::FixedMainLoop`, but it is now more idiomatic to place your systems in either `RunFixedMainLoopSystem::BeforeFixedMainLoop` or `RunFixedMainLoopSystem::AfterFixedMainLoop`

Old:

```
app.add_systems(
    RunFixedMainLoop,
    some_system.before(run_fixed_main_schedule)
);

```


New:

```
app.add_systems(
    RunFixedMainLoop,
    some_system.in_set(RunFixedMainLoopSystem::BeforeFixedMainLoop)
);

```


* * *

### Remove deprecated `bevy_dynamic_plugin` [#](#remove-deprecated-bevy-dynamic-plugin)

Dynamic plugins were deprecated in 0.14 for being unsound, and they have now been fully removed. Please consider using the alternatives listed in the `bevy_dynamic_plugin` crate documentation, or worst-case scenario you may copy the code from 0.14.

* * *

### Remove need for EventLoopProxy to be NonSend [#](#remove-need-for-eventloopproxy-to-be-nonsend)

`EventLoopProxy` has been renamed to `EventLoopProxyWrapper` and is now `Send`, making it an ordinary resource.

Before:

```
event_loop_system(event_loop: NonSend<EventLoopProxy<MyEvent>>) {
    event_loop.send_event(MyEvent);
}

```


After:

```
event_loop_system(event_loop: Res<EventLoopProxy<MyEvent>>) {
    event_loop.send_event(MyEvent);
}

```


* * *

### Remove second generic from `.add_before`, `.add_after` [#](#remove-second-generic-from-add-before-add-after)

Removed second generic from `PluginGroupBuilder` methods: `add_before` and `add_after`.

```
// Before:
DefaultPlugins
    .build()
    .add_before::<WindowPlugin, _>(FooPlugin)
    .add_after::<WindowPlugin, _>(BarPlugin)

// After:
DefaultPlugins
    .build()
    .add_before::<WindowPlugin>(FooPlugin)
    .add_after::<WindowPlugin>(BarPlugin)

```


* * *

### Handle `Ctrl+C` in the terminal properly [#](#handle-ctrl-c-in-the-terminal-properly)

If you are overriding the `Ctrl+C` handler then you should call `TerminalCtrlCHandlerPlugin::gracefully_exit` from your handler. It will tell the app to exit.

Assets [#](#assets)
-------------------

### AssetServer LoadState API consistency [#](#assetserver-loadstate-api-consistency)

`RecursiveDependencyLoadState::Failed` now stores error information about the first encountered error, rather than being a unit struct.

* * *

### Cleanup unneeded lifetimes in bevy\_asset [#](#cleanup-unneeded-lifetimes-in-bevy-asset)

The traits `AssetLoader`, `AssetSaver` and `Process` traits from `bevy_asset` now use elided lifetimes. If you implement these then remove the named lifetime.

* * *

### Generalized `Into<AssetSourceId>` and `Into<AssetPath>` Implementations over Lifetime [#](#generalized-into-assetsourceid-and-into-assetpath-implementations-over-lifetime)

In areas where these implementations where being used, you can now add `from_static` in order to get the original specialised implementation which avoids creating an `Arc` internally.

```
// Before
let asset_path = AssetPath::from("my/path/to/an/asset.ext");

// After
let asset_path = AssetPath::from_static("my/path/to/an/asset.ext");

```


To be clear, this is only required if you wish to maintain the performance benefit that came with the specialisation. Existing code is _not_ broken by this change.

* * *

### Improve error handling for `AssetServer::add_async` [#](#improve-error-handling-for-assetserver-add-async)

`AssetServer::add_async` can now return a custom error type in its future. To return to the previous behavior, pass in an `E` generic of `AssetLoadError`.

To support these changes, `AssetLoadError` now has an additional arm that will need to be exhaustively matched against.

* * *

### Remove incorrect equality comparisons for asset load error types [#](#remove-incorrect-equality-comparisons-for-asset-load-error-types)

The types `bevy_asset::AssetLoadError` and `bevy_asset::LoadState` no longer support equality comparisons. If you need to check for an asset’s load state, consider checking for a specific variant using `LoadState::is_loaded` or the `matches!` macro. Similarly, consider using the `matches!` macro to check for specific variants of the `AssetLoadError` type if you need to inspect the value of an asset load error in your code.

`DependencyLoadState` and `RecursiveDependencyLoadState` are not released yet, so no migration needed,

* * *

### Replace `AsyncSeek` trait by `AsyncSeekForward` for `Reader` to address #12880 [#](#replace-asyncseek-trait-by-asyncseekforward-for-reader-to-address-12880)

Replace all instances of `AsyncSeek` with `AsyncSeekForward` in your asset reader implementations.

* * *

### bevy\_asset: Improve `NestedLoader` API [#](#bevy-asset-improve-nestedloader-api)

Code which uses `bevy_asset`’s `LoadContext::loader` / `NestedLoader` will see some naming changes:

*   `untyped` is replaced by `with_unknown_type`
*   `with_asset_type` is replaced by `with_static_type`
*   `with_asset_type_id` is replaced by `with_dynamic_type`
*   `direct` is replaced by `immediate` (the opposite of “immediate” is “deferred”)

* * *

### Deprecate `LoadAndSave` Asset Processor [#](#deprecate-loadandsave-asset-processor)

*   Replace `LoadAndSave<L, S>` with `LoadTransformAndSave<L, IdentityAssetTransformer<<L as AssetLoader>::Asset>, S>`
*   Replace `LoadAndSaveSettings<L, S>` with `LoadTransformAndSaveSettings<L, (), S>`

* * *

### `AssetReader::read` now returns an opaque type [#](#assetreader-read-now-returns-an-opaque-type)

The trait method `bevy_asset::io::AssetReader::read` (and `read_meta`) now return an opaque type instead of a boxed trait object. Implementors of these methods should change the type signatures appropriately:

```
impl AssetReader for MyReader {
    // Before
    async fn read<'a>(&'a self, path: &'a Path) -> Result<Box<Reader<'a>>, AssetReaderError> {
        let reader = // construct a reader
        Box::new(reader) as Box<Reader<'a>>
    }

    // After
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        // create a reader
    }
}

```


`bevy::asset::io::Reader` is now a trait, rather than a type alias for a trait object. Implementors of `AssetLoader::load` will need to adjust the method signature accordingly:

```
impl AssetLoader for MyLoader {
    async fn load<'a>(
        &'a self,
        // Before:
        reader: &'a mut bevy::asset::io::Reader,
        // After:
        reader: &'a mut dyn bevy::asset::io::Reader,
        _: &'a Self::Settings,
        load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
}

```


Additionally, implementors of `AssetReader` that return a type implementing `futures_io::AsyncRead` and `AsyncSeek` might need to explicitly implement `bevy::asset::io::Reader` for that type.

```
impl bevy::asset::io::Reader for MyAsyncReadAndSeek {}

```


* * *

### Make gLTF node children Handle instead of objects [#](#make-gltf-node-children-handle-instead-of-objects)

If accessing children, use `Assets<GltfNode>` resource to get the actual child object.

**Before**

```
fn gltf_print_first_node_children_system(gltf_component_query: Query<Handle<Gltf>>, gltf_assets: Res<Assets<Gltf>>, gltf_nodes: Res<Assets<GltfNode>>) {
    for gltf_handle in gltf_component_query.iter() {
        let gltf_root = gltf_assets.get(gltf_handle).unwrap();
        let first_node_handle = gltf_root.nodes.get(0).unwrap();
        let first_node = gltf_nodes.get(first_node_handle).unwrap();
        let first_child = first_node.children.get(0).unwrap();
        println!("First nodes child node name is {:?)", first_child.name);
    }
}

```


**After**

```
fn gltf_print_first_node_children_system(gltf_component_query: Query<Handle<Gltf>>, gltf_assets: Res<Assets<Gltf>>, gltf_nodes: Res<Assets<GltfNode>>) {
    for gltf_handle in gltf_component_query.iter() {
        let gltf_root = gltf_assets.get(gltf_handle).unwrap();
        let first_node_handle = gltf_root.nodes.get(0).unwrap();
        let first_node = gltf_nodes.get(first_node_handle).unwrap();
        let first_child_handle = first_node.children.get(0).unwrap();
        let first_child = gltf_nodes.get(first_child_handle).unwrap();
        println!("First nodes child node name is {:?)", first_child.name);
    }
}

```


* * *

### Split `TextureAtlasSources` out of `TextureAtlasLayout` and make `TextureAtlasLayout` serializable [#](#split-textureatlassources-out-of-textureatlaslayout-and-make-textureatlaslayout-serializable)

`TextureAtlasBuilder` no longer stores a mapping back to the original images in `TextureAtlasLayout`; that functionality has been added to a new struct, `TextureAtlasSources`, instead. This also means that the signature for `TextureAtlasBuilder::finish` has changed, meaning that calls of the form:

```
let (atlas_layout, image) = builder.build()?;

```


Will now change to the form:

```
let (atlas_layout, atlas_sources, image) = builder.build()?;

```


And instead of performing a reverse-lookup from the layout, like so:

```
let atlas_layout_handle = texture_atlases.add(atlas_layout.clone());
let index = atlas_layout.get_texture_index(&my_handle);
let handle = TextureAtlas {
    layout: atlas_layout_handle,
    index,
};

```


You can perform the lookup from the sources instead:

```
let atlas_layout = texture_atlases.add(atlas_layout);
let index = atlas_sources.get_texture_index(&my_handle);
let handle = TextureAtlas {
    layout: atlas_layout,
    index,
};

```


Additionally, `TextureAtlasSources` also has a convenience method, `handle`, which directly combines the index and an existing `TextureAtlasLayout` handle into a new `TextureAtlas`:

```
let atlas_layout = texture_atlases.add(atlas_layout);
let handle = atlas_sources.handle(atlas_layout, &my_handle);

```


* * *

### Export glTF skins as a Gltf struct [#](#export-gltf-skins-as-a-gltf-struct)

*   Change `GltfAssetLabel::Skin(..)` to `GltfAssetLabel::InverseBindMatrices(..)`.

* * *

### Replace `bevy_utils::CowArc` with `atomicow` [#](#replace-bevy-utils-cowarc-with-atomicow)

`bevy_utils::CowArc` has moved to a new crate called [atomicow](https://crates.io/crates/atomicow).

Audio [#](#audio)
-----------------

### Migrate audio to required components [#](#migrate-audio-to-required-components)

Replace all insertions of `AudioSourceBundle`, `AudioBundle`, and `PitchBundle` with the `AudioPlayer` component. The other components required by it will now be inserted automatically.

In cases where the generics cannot be inferred, you may need to specify them explicitly. For example:

```
commands.spawn(AudioPlayer::<AudioSource>(asset_server.load("sounds/sick_beats.ogg")));

```


Build-System [#](#build-system)
-------------------------------

### Use `en-us` locale for `typos` [#](#use-en-us-locale-for-typos)

The following methods or fields have been renamed from `*dependants*` to `*dependents*`.

*   `ProcessorAssetInfo::dependants`
*   `ProcessorAssetInfos::add_dependant`
*   `ProcessorAssetInfos::non_existent_dependants`
*   `AssetInfo::dependants_waiting_on_load`
*   `AssetInfo::dependants_waiting_on_recursive_dep_load`
*   `AssetInfos::loader_dependants`
*   `AssetInfos::remove_dependants_and_labels`

Color [#](#color)
-----------------

### Adds back in way to convert color to u8 array, implemented for the two RGB color types, also renames Color::linear to Color::to\_linear. [#](#adds-back-in-way-to-convert-color-to-u8-array-implemented-for-the-two-rgb-color-types-also-renames-color-linear-to-color-to-linear)

`Color::linear` has been renamed to `Color::to_linear` for consistency.

* * *

### Update Grid Gizmo to use Color [#](#update-grid-gizmo-to-use-color)

This shouldn’t be adding anything that isn’t already in a migration guide? I assume as it uses `impl Into<...>` in the public interfaces that any users of these APIs shouldn’t have to make any code changes.

Core [#](#core)
---------------

### Rename `bevy_core::name::DebugName` to `bevy_core::name::NameOrEntity` [#](#rename-bevy-core-name-debugname-to-bevy-core-name-nameorentity)

*   Rename usages of `bevy_core::name::DebugName` to `bevy_core::name::NameOrEntity`

Cross-Cutting [#](#cross-cutting)
---------------------------------

### Remove the `Component` trait implementation from `Handle` [#](#remove-the-component-trait-implementation-from-handle)

`Handle` can no longer be used as a `Component`. All existing Bevy types using this pattern have been wrapped in their own semantically meaningful type. You should do the same for any custom `Handle` components your project needs.

The `Handle<MeshletMesh>` component is now `MeshletMesh3d`.

The `WithMeshletMesh` type alias has been removed. Use `With<MeshletMesh3d>` instead.

* * *

### Fix floating point math [#](#fix-floating-point-math)

*   Not a breaking change
*   Projects should use bevy math where applicable

* * *

### Don't re-export `bevy_image` from `bevy_render` [#](#don-t-re-export-bevy-image-from-bevy-render)

Various types and traits are no longer re-exported from `bevy_image` in `bevy::render::texture`. Import them directly from `bevy::image` instead.

```
// 0.14
use bevy::render::texture::BevyDefault;
// 0.15
use bevy::image::BevyDefault;

```


For searchability, this is a non-comprehensive list of other types may be affected: `CompressedImageFormats`, `ExrTextureLoader`, `HdrTextureLoader`, `Image`, `ImageAddressMode`, `ImageFilterMode`, `ImageLoader`, `ImageLoaderSettings`, `ImageSampler`, `ImageSamplerDescriptor`, `ImageType`, `TextureError`, `TextureFormatPixelInfo`.

* * *

### Add custom cursors [#](#add-custom-cursors)

*   `CursorIcon` is no longer a field in `Window`, but a separate component can be inserted to a window entity. It has been changed to an enum that can hold custom images in addition to system icons.
*   `Cursor` is renamed to `CursorOptions` and `cursor` field of `Window` is renamed to `cursor_options`
*   `CursorIcon` is renamed to `SystemCursorIcon`

Diagnostics [#](#diagnostics)
-----------------------------

### Don't ignore draw errors [#](#don-t-ignore-draw-errors)

If you were using `RenderCommandResult::Failure` to just ignore an error and retry later, use `RenderCommandResult::Skip` instead.

This wasn’t intentional, but this PR should also help with https://github.com/bevyengine/bevy/issues/12660 since we can turn a few unwraps into error messages now.

ECS [#](#ecs)
-------------

### Make World::flush\_commands private [#](#make-world-flush-commands-private)

`World::flush_commands` is now private. Use `World::flush` instead.

* * *

### Add `FilteredAccess::empty` and simplify the implementation of `update_component_access` for `AnyOf`/`Or` [#](#add-filteredaccess-empty-and-simplify-the-implementation-of-update-component-access-for-anyof-or)

*   The behaviour of `AnyOf<()>` and `Or<()>` has been changed to match no archetypes rather than all archetypes to naturally match the corresponding logical operation. Consider replacing them with `()` instead.

* * *

### Add `mappings` to `EntityMapper` [#](#add-mappings-to-entitymapper)

*   If you are implementing `EntityMapper` yourself, you can use the below as a stub implementation:

```
fn mappings(&self) -> impl Iterator<Item = (Entity, Entity)> {
    unimplemented!()
}

```


*   If you were using `EntityMapper` as a trait object (`dyn EntityMapper`), instead use `dyn DynEntityMapper` and its associated methods.

* * *

### Add query reborrowing [#](#add-query-reborrowing)

*   `WorldQuery` now has an additional `shrink_fetch` method you have to implement if you were implementing `WorldQuery` manually.

* * *

### Allow `World::entity` family of functions to take multiple entities and get multiple references back [#](#allow-world-entity-family-of-functions-to-take-multiple-entities-and-get-multiple-references-back)

*   `World::get_entity` now returns `Result<_, Entity>` instead of `Option<_>`.
    
    *   Use `world.get_entity(..).ok()` to return to the previous behavior.
*   `World::get_entity_mut` and `DeferredWorld::get_entity_mut` now return `Result<_, EntityFetchError>` instead of `Option<_>`.
    
    *   Use `world.get_entity_mut(..).ok()` to return to the previous behavior.
*   Type inference for `World::entity`, `World::entity_mut`, `World::get_entity`, `World::get_entity_mut`, `DeferredWorld::entity_mut`, and `DeferredWorld::get_entity_mut` has changed, and might now require the input argument’s type to be explicitly written when inside closures.
    
*   The following functions have been deprecated, and should be replaced as such:
    
    *   `World::many_entities` -> `World::entity::<[Entity; N]>`
        
    *   `World::many_entities_mut` -> `World::entity_mut::<[Entity; N]>`
        
    *   `World::get_many_entities` -> `World::get_entity::<[Entity; N]>`
        
    *   `World::get_many_entities_dynamic` -> `World::get_entity::<&[Entity]>`
        
    *   `World::get_many_entities_mut` -> `World::get_entity_mut::<[Entity; N]>`
        
        *   The equivalent return type has changed from `Result<_, QueryEntityError>` to `Result<_, EntityFetchError>`
    *   `World::get_many_entities_dynamic_mut` -> `World::get_entity_mut::<&[Entity]>`
        
        *   The equivalent return type has changed from `Result<_, QueryEntityError>` to `Result<_, EntityFetchError>`
    *   `World::get_many_entities_from_set_mut` -> `World::get_entity_mut::<&EntityHashSet>`
        
        *   The equivalent return type has changed from `Result<Vec<EntityMut>, QueryEntityError>` to `Result<EntityHashMap<EntityMut>, EntityFetchError>`. If necessary, you can still convert the `EntityHashMap` into a `Vec`.

* * *

### Change World::inspect\_entity to return an Iterator instead of Vec [#](#change-world-inspect-entity-to-return-an-iterator-instead-of-vec)

*   `World::inspect_entity` now returns an `Iterator` instead of a `Vec`. If you need a `Vec`, immediately collect the iterator: `world.inspect_entity(entity).collect<Vec<_>>()`

* * *

### Created an EventMutator for when you want to mutate an event before reading [#](#created-an-eventmutator-for-when-you-want-to-mutate-an-event-before-reading)

Users currently using `ManualEventReader` should use `EventCursor` instead. `ManualEventReader` will be removed in Bevy 0.16. Additionally, `Events::get_reader` has been replaced by `Events::get_cursor`.

Users currently directly accessing the `Events` resource for mutation should move to `EventMutator` if possible.

* * *

### Deprecate `Events::oldest_id` [#](#deprecate-events-oldest-id)

*   Change usages of `Events::oldest_id` to `Events::oldest_event_count`
*   If `Events::oldest_id` was used to get the actual oldest `EventId::id`, note that the deprecated method never reliably did that in the first place as the buffers may contain no id currently.

* * *

### Deprecate `get_or_spawn` [#](#deprecate-get-or-spawn)

If you are given an `Entity` and you want to do something with it, use `Commands.entity(...)` or `World.entity(...)`. If instead you want to spawn something use `Commands.spawn(...)` or `World.spawn(...)`. If you are not sure if an entity exists, you can always use `get_entity` and match on the `Option<...>` that is returned.

* * *

### Enable `EntityRef::get_by_id` and friends to take multiple ids and get multiple pointers back [#](#enable-entityref-get-by-id-and-friends-to-take-multiple-ids-and-get-multiple-pointers-back)

*   The following functions now return an `Result<_, EntityComponentError>` instead of a `Option<_>`: `EntityRef::get_by_id`, `EntityMut::get_by_id`, `EntityMut::into_borrow_by_id`, `EntityMut::get_mut_by_id`, `EntityMut::into_mut_by_id`, `EntityWorldMut::get_by_id`, `EntityWorldMut::into_borrow_by_id`, `EntityWorldMut::get_mut_by_id`, `EntityWorldMut::into_mut_by_id`

* * *

### EntityRef/Mut get\_components (immutable variants only) [#](#entityref-mut-get-components-immutable-variants-only)

*   Renamed `FilteredEntityRef::components` to `FilteredEntityRef::accessed_components` and `FilteredEntityMut::components` to `FilteredEntityMut::accessed_components`.

* * *

### Fix soudness issue with Conflicts involving `read_all` and `write_all` [#](#fix-soudness-issue-with-conflicts-involving-read-all-and-write-all)

The `get_conflicts` method of `Access` now returns an `AccessConflict` enum instead of simply a `Vec` of `ComponentId`s that are causing the access conflict. This can be useful in cases where there are no particular `ComponentId`s conflicting, but instead **all** of them are; for example `fn system(q1: Query<EntityMut>, q2: Query<EntityRef>)`

* * *

### Follow up to cached `run_system` [#](#follow-up-to-cached-run-system)

*   `IntoSystem::pipe` and `IntoSystem::map` now return `IntoPipeSystem` and `IntoAdapterSystem` instead of `PipeSystem` and `AdapterSystem`. Most notably these types don’t implement `System` but rather only `IntoSystem`.

* * *

### List components for QueryEntityError::QueryDoesNotMatch [#](#list-components-for-queryentityerror-querydoesnotmatch)

*   `QueryEntityError` now has a lifetime. Convert it to a custom error if you need to store it.

* * *

### Make QueryFilter an unsafe trait [#](#make-queryfilter-an-unsafe-trait)

`QueryFilter` is now an `unsafe trait`. If you were manually implementing it, you will need to verify that the `WorldQuery` implementation is read-only and then add the `unsafe` keyword to the `impl`.

* * *

### Make `QueryState::transmute`&co validate the world of the `&Components` used [#](#make-querystate-transmute-co-validate-the-world-of-the-components-used)

*   `QueryState::transmute`, `QueryState::transmute_filtered`, `QueryState::join` and `QueryState::join_filtered` now take a `impl Into<UnsafeWorldCell>` instead of a `&Components`

* * *

### Re-name and Extend Run Conditions API [#](#re-name-and-extend-run-conditions-api)

*   The `and_then` run condition method has been replaced with the `and` run condition method.
*   The `or_else` run condition method has been replaced with the `or` run condition method.

* * *

### Remove redundant information and optimize dynamic allocations in `Table` [#](#remove-redundant-information-and-optimize-dynamic-allocations-in-table)

`Table` now uses `ThinColumn` instead of `Column`. That means that methods that previously returned `Column`, will now return `ThinColumn` instead.

`ThinColumn` has a much more limited and low-level API, but you can still achieve the same things in `ThinColumn` as you did in `Column`. For example, instead of calling `Column::get_added_tick`, you’d call `ThinColumn::get_added_ticks_slice` and index it to get the specific added tick.

* * *

### Removed Type Parameters from `Observer` [#](#removed-type-parameters-from-observer)

If you filtered for observers using `Observer<A, B>`, instead filter for an `Observer`.

* * *

### Rename Add to Queue for methods with deferred semantics [#](#rename-add-to-queue-for-methods-with-deferred-semantics)

*   `Commands::add` and `Commands::push` have been replaced with `Commands::queue`.
*   `ChildBuilder::add_command` has been renamed to `ChildBuilder::queue_command`.

* * *

### Rename `App/World::observe` to `add_observer`, `EntityWorldMut::observe_entity` to `observe`. [#](#rename-app-world-observe-to-add-observer-entityworldmut-observe-entity-to-observe)

Various observer methods have been renamed for clarity.

*   `App::observe` -> `App::add_observer`
*   `World::observe` -> `World::add_observer`
*   `Commands::observe` -> `Commands::add_observer`
*   `EntityWorldMut::observe_entity` -> `EntityWorldMut::observe`

* * *

### Rename `Commands::register_one_shot_system` -> `register_system` [#](#rename-commands-register-one-shot-system-register-system)

`Commands::register_one_shot_system` has been renamed to `register_system`.

* * *

### Rename init\_component & friends [#](#rename-init-component-friends)

*   `World::init_component` has been renamed to `register_component`.
*   `World::init_component_with_descriptor` has been renamed to `register_component_with_descriptor`.
*   `World::init_bundle` has been renamed to `register_bundle`.
*   `Components::init_component` has been renamed to `register_component`.
*   `Components::init_component_with_descriptor` has been renamed to `register_component_with_descriptor`.
*   `Components::init_resource` has been renamed to `register_resource`.
*   `Components::init_non_send` had been renamed to `register_non_send`.

* * *

### Rename observe to observe\_entity on EntityWorldMut [#](#rename-observe-to-observe-entity-on-entityworldmut)

The `observe()` method on entities has been renamed to `observe_entity()` to prevent confusion about what is being observed in some cases.

* * *

### Rename push children to add children [#](#rename-push-children-to-add-children)

Some commands and methods for adding children to an entity were renamed for consistency.


|0.14                         |0.15        |
|-----------------------------|------------|
|EntityCommands::push_children|add_children|
|PushChild                    |AddChild    |
|PushChildren                 |AddChildren |


* * *

### Require `&mut self` for `World::increment_change_tick` [#](#require-mut-self-for-world-increment-change-tick)

The method `World::increment_change_tick` now requires `&mut self` instead of `&self`. If you need to call this method but do not have mutable access to the world, consider using `world.as_unsafe_world_cell_readonly().increment_change_tick()`, which does the same thing, but is less efficient than the method on `World` due to requiring atomic synchronization.

```
fn my_system(world: &World) {
    // Before
    world.increment_change_tick();

    // After
    world.as_unsafe_world_cell_readonly().increment_change_tick();
}

```


* * *

### Simplify run conditions [#](#simplify-run-conditions)

Some run conditions have been simplified.

```
// Before:
app.add_systems(Update, (
    system_0.run_if(run_once()),
    system_1.run_if(resource_changed_or_removed::<T>()),
    system_2.run_if(resource_removed::<T>()),
    system_3.run_if(on_event::<T>()),
    system_4.run_if(any_component_removed::<T>()),
));

// After:
app.add_systems(Update, (
    system_0.run_if(run_once),
    system_1.run_if(resource_changed_or_removed::<T>),
    system_2.run_if(resource_removed::<T>),
    system_3.run_if(on_event::<T>),
    system_4.run_if(any_component_removed::<T>),
));

```


* * *

### Support more kinds of system params in buildable systems. [#](#support-more-kinds-of-system-params-in-buildable-systems)

The API for `SystemBuilder` has changed. Instead of constructing a builder with a world and then adding params, you first create a tuple of param builders and then supply the world.

```
// Before
let system = SystemBuilder::<()>::new(&mut world)
    .local::<u64>()
    .builder::<Local<u64>>(|x| *x = 10)
    .builder::<Query<&A>>(|builder| { builder.with::<B>(); })
    .build(system);

// After
let system = (
    ParamBuilder,
    LocalBuilder(10),
    QueryParamBuilder::new(|builder| { builder.with::<B>(); }),
)
    .build_state(&mut world)
    .build_system(system);

```


* * *

### Support systems that take references as input [#](#support-systems-that-take-references-as-input)

*   All current explicit usages of the following types must be changed in the way specified:
    
    *   `SystemId<I, O>` to `SystemId<In<I>, O>`
    *   `System<In = T>` to `System<In = In<T>>`
    *   `IntoSystem<I, O, M>` to `IntoSystem<In<I>, O, M>`
    *   `Condition<M, T>` to `Condition<M, In<T>>`
*   `In<Trigger<E, B>>` is no longer a valid input parameter type. Use `Trigger<E, B>` directly, instead.
    

* * *

### System param validation for observers, system registry and run once [#](#system-param-validation-for-observers-system-registry-and-run-once)

*   `RunSystemOnce::run_system_once` and `RunSystemOnce::run_system_once_with` now return a `Result<Out>` instead of just `Out`

* * *

### Track source location in change detection [#](#track-source-location-in-change-detection)

*   Added `changed_by` field to many internal ECS functions used with change detection when the `track_change_detection` feature flag is enabled. Use Location::caller() to provide the source of the function call.

* * *

### Update `trigger_observers` to operate over slices of data [#](#update-trigger-observers-to-operate-over-slices-of-data)

The `trigger_observers` method now operates on `&[ComponentId]` rather than `impl Iterator<Item=ComponentId`\>.

Try replacing `bundle_info.iter_components()` with `bundle_info.components()` or collect the iterator of component ids into a `Vec`.

* * *

### `IntoSystemConfigs::chain_ignore_deferred`'s return type fix [#](#intosystemconfigs-chain-ignore-deferred-s-return-type-fix)

`IntoSystemConfigs::chain_ignore_deferred` now correctly returns a `SystemSetConfig`.

* * *

### bevy\_ecs: Special-case `Entity::PLACEHOLDER` formatting [#](#bevy-ecs-special-case-entity-placeholder-formatting)

The `Debug` and `Display` impls for `Entity` now return `PLACEHOLDER` for the `Entity::PLACEHOLDER` constant. If you had any code relying on these values, you may need to account for this change.

* * *

### change return type of `World::resource_ref` to `Ref` [#](#change-return-type-of-world-resource-ref-to-ref)

Previously `World::get_resource_ref::<T>` and `World::resource_ref::<T>` would return a `Res<T>` which was inconsistent with the rest of the `World` API (notably `resource_scope`). This has been fixed and the methods now return `Ref<T>`.

This means it is no longer possible to get `Res<T>` from `World`. If you were relying on this, you should try using `Ref<T>` instead since it has the same functionality.

**Before**

```
let my_resource: Res<MyResource> = world.resource_ref();
function_taking_resource(my_resource);

fn function_taking_resource(resource: Res<MyResource>) { /* ... */ }

```


**After**

```
let my_resource: Ref<MyResource> = world.resource_ref();
function_taking_resource(my_resource);

fn function_taking_resource(resource: Ref<MyResource>) { /* ... */ }

```


* * *

### Minimal Bubbling Observers [#](#minimal-bubbling-observers)

*   Manual implementations of `Event` should add associated type `Traverse = TraverseNone` and associated constant `AUTO_PROPAGATE = false`;
*   `Trigger::new` has new field `propagation: &mut Propagation` which provides the bubbling state.
*   `ObserverRunner` now takes the same `&mut Propagation` as a final parameter.

* * *

### Make `ComponentTicks` field public [#](#make-componentticks-field-public)

*   Instead of using `ComponentTicks::last_changed_tick` and `ComponentTicks::added_tick` methods, access fields directly.

* * *

### Change ReflectMapEntities to operate on components before insertion [#](#change-reflectmapentities-to-operate-on-components-before-insertion)

*   Consumers of `ReflectMapEntities` will need to call `map_entities` on values prior to inserting them into the world.
*   Implementors of `MapEntities` will need to remove the `mappings` method, which is no longer needed for `ReflectMapEntities` and has been removed from the trait.

* * *

### Bubbling observers traversal should use query data [#](#bubbling-observers-traversal-should-use-query-data)

Update implementations of `Traversal`.

* * *

### Migrate bevy picking [#](#migrate-bevy-picking)

This API hasn’t shipped yet, so I didn’t bother with a deprecation. However, for any crates tracking main the changes are as follows:

Previous api:

```
commands.insert(PointerBundle::new(PointerId::Mouse));
commands.insert(PointerBundle::new(PointerId::Mouse).with_location(location));

```


New api:

```
commands.insert(PointerId::Mouse);
commands.insert((PointerId::Mouse, PointerLocation::new(location)));

```


* * *

### `ReflectBundle::remove` improvement [#](#reflectbundle-remove-improvement)

If you don’t need the returned value from `remove`, discard it.

* * *

### Use crate: `disqualified` [#](#use-crate-disqualified)

Replace references to `bevy_utils::ShortName` with `disqualified::ShortName`.

* * *

### Migrate cameras to required components [#](#migrate-cameras-to-required-components)

`Camera2dBundle` and `Camera3dBundle` have been deprecated in favor of `Camera2d` and `Camera3d`. Inserting them will now also insert the other components required by them automatically.

* * *

### Migrate fog volumes to required components [#](#migrate-fog-volumes-to-required-components)

Replace all insertions of `FogVolumeBundle` with the `Visibility` component. The other components required by it will now be inserted automatically.

* * *

### Migrate meshes and materials to required components [#](#migrate-meshes-and-materials-to-required-components)

Asset handles for meshes and mesh materials must now be wrapped in the `Mesh2d` and `MeshMaterial2d` or `Mesh3d` and `MeshMaterial3d` components for 2D and 3D respectively. Raw handles as components no longer render meshes.

Additionally, `MaterialMesh2dBundle`, `MaterialMeshBundle`, and `PbrBundle` have been deprecated. Instead, use the mesh and material components directly.

Previously:

```
commands.spawn(MaterialMesh2dBundle {
    mesh: meshes.add(Circle::new(100.0)).into(),
    material: materials.add(Color::srgb(7.5, 0.0, 7.5)),
    transform: Transform::from_translation(Vec3::new(-200., 0., 0.)),
    ..default()
});

```


Now:

```
commands.spawn((
    Mesh2d(meshes.add(Circle::new(100.0))),
    MeshMaterial2d(materials.add(Color::srgb(7.5, 0.0, 7.5))),
    Transform::from_translation(Vec3::new(-200., 0., 0.)),
));

```


If the mesh material is missing, a white default material is now used. Previously, nothing was rendered if the material was missing.

The `WithMesh2d` and `WithMesh3d` query filter type aliases have also been removed. Simply use `With<Mesh2d>` or `With<Mesh3d>`.

* * *

### Migrate motion blur, TAA, SSAO, and SSR to required components [#](#migrate-motion-blur-taa-ssao-and-ssr-to-required-components)

`MotionBlurBundle`, `TemporalAntiAliasBundle`, `ScreenSpaceAmbientOcclusionBundle`, and `ScreenSpaceReflectionsBundle` have been deprecated in favor of the `MotionBlur`, `TemporalAntiAliasing`, `ScreenSpaceAmbientOcclusion`, and `ScreenSpaceReflections` components instead. Inserting them will now also insert the other components required by them automatically.

* * *

### Migrate reflection probes to required components [#](#migrate-reflection-probes-to-required-components)

`ReflectionProbeBundle` has been deprecated in favor of inserting the `LightProbe` and `EnvironmentMapLight` components directly. Inserting them will now automatically insert `Transform` and `Visibility` components.

* * *

### Migrate visibility to required components [#](#migrate-visibility-to-required-components)

Replace all insertions of `VisibilityBundle` with the `Visibility` component. The other components required by it will now be inserted automatically.

* * *

### Synchronize removed components with the render world [#](#synchronize-removed-components-with-the-render-world)

The retained render world notes should be updated to explain this edge case and `SyncComponentPlugin`

* * *

### Deprecate SpatialBundle [#](#deprecate-spatialbundle)

`SpatialBundle` is now deprecated, insert `Transform` and `Visibility` instead which will automatically insert all other components that were in the bundle. If you do not specify these values and any other components in your `spawn`/`insert` call already requires either of these components you can leave that one out.

before:

```
commands.spawn(SpatialBundle::default());

```


after:

```
commands.spawn((Transform::default(), Visibility::default());

```


* * *

### Migrate `bevy_transform` to required components [#](#migrate-bevy-transform-to-required-components)

Replace all insertions of `GlobalTransform` and/or `TransformBundle` with `Transform` alone.

Gizmos [#](#gizmos)
-------------------

### Consistency between `Wireframe2d` and `Wireframe` [#](#consistency-between-wireframe2d-and-wireframe)

*   `Wireframe2dConfig`.`default_color` type is now `Color` instead of `Srgba`. Use `.into()` to convert between them.
*   `Wireframe2dColor`.`color` type is now `Color` instead of `Srgba`. Use `.into()` to convert between them.

* * *

### Fix Gizmos warnings and doc errors when a subset of features are selected [#](#fix-gizmos-warnings-and-doc-errors-when-a-subset-of-features-are-selected)

There shouldn’t be any reason to migrate, although if for some reason you use `GizmoMeshConfig` and `bevy_render` but not `bevy_pbr` or `bevy_sprite` (such that it does nothing), then you will get an error that it no longer exists.

* * *

### Fix `arc_2d` Gizmos [#](#fix-arc-2d-gizmos)

*   users have to adjust their usages of `arc_2d`:
    *   before:

```
arc_2d(
  pos,
  angle,
  arc_angle,
  radius,
  color
)

```


*   after:

```
arc_2d(
  // this `+ arc_angle * 0.5` quirk is only if you want to preserve the previous behavior 
  // with the new API.
  // feel free to try to fix this though since your current calls to this function most likely
  // involve some computations to counter-act that quirk in the first place
  Isometry2d::new(pos, Rot2::radians(angle + arc_angle * 0.5),
  arc_angle,
  radius,
  color
)

```


* * *

### Improve the gizmo for `Plane3d`, reusing grid [#](#improve-the-gizmo-for-plane3d-reusing-grid)

The optional builder methods on

```

gizmos.primitive_3d(&Plane3d { }, ...);


```


changed from

*   `segment_length`
*   `segment_count`
*   `axis_count`

to

*   `cell_count`
*   `spacing`

* * *

### Making `bevy_render` an optional dependency for `bevy_gizmos` [#](#making-bevy-render-an-optional-dependency-for-bevy-gizmos)

No user-visible changes needed from the users.

* * *

### Use `Isometry` in `bevy_gizmos` wherever we can [#](#use-isometry-in-bevy-gizmos-wherever-we-can)

The gizmos methods function signature changes as follows:

*   2D
    
    *   if it took `position` & `rotation_angle` before -> `Isometry2d::new(position, Rot2::radians(rotation_angle))`
    *   if it just took `position` before -> `Isometry2d::from_translation(position)`
*   3D
    
    *   if it took `position` & `rotation` before -> `Isometry3d::new(position, rotation)`
    *   if it just took `position` before -> `Isometry3d::from_translation(position)`

* * *

### Use u32 for all resolution/subdivision fields in bevy\_gizmos [#](#use-u32-for-all-resolution-subdivision-fields-in-bevy-gizmos)

*   All gizmos now take `u32` instead of `usize` for their resolution/subdivision/segment counts

* * *

### Switch rotation & translation in grid gizmos [#](#switch-rotation-translation-in-grid-gizmos)

*   Users might have to double check their already existing calls to all the `grid` methods. It should be more intuitive now though.

* * *

### Make TrackedRenderPass::set\_vertex\_buffer aware of slice size [#](#make-trackedrenderpass-set-vertex-buffer-aware-of-slice-size)

*   `TrackedRenderPass::set_vertex_buffer` function has been modified to update vertex buffers when the same buffer with the same offset is provided, but its size has changed. Some existing code may rely on the previous behavior, which did not update the vertex buffer in this scenario.

Hierarchy [#](#hierarchy)
-------------------------

### Only propagate transforms entities with GlobalTransforms. [#](#only-propagate-transforms-entities-with-globaltransforms)

*   To avoid surprising performance pitfalls, `Transform` / `GlobalTransform` propagation is no longer performed down through hierarchies where intermediate parent are missing a `GlobalTransform`. To restore the previous behavior, add `GlobalTransform::default` to intermediate entities.

Input [#](#input)
-----------------

### Gamepad improvements [#](#gamepad-improvements)

*   `Gamepad` fields are now public.
*   Instead of using `Gamepad` delegates like `Gamepad::just_pressed`, call these methods directly on the fields.

* * *

### Implement gamepads as entities [#](#implement-gamepads-as-entities)

Gamepad input is no longer accessed using resources, instead they are entities and are accessible using the Gamepad component as long as the gamepad is connected.

Gamepads resource has been deleted, instead of using an internal id to identify gamepads you can use its Entity. Disconnected gamepads will **NOT** be despawned. Gamepad components that don’t need to preserve their state will be removed i.e. Gamepad component is removed, but GamepadSettings is kept. Reconnected gamepads will try to preserve their Entity id and necessary components will be re-inserted.

GamepadSettings is no longer a resource, instead it is a component attached to the Gamepad entity.

Axis, Axis and ButtonInput methods are accessible via Gamepad component.

```
fn gamepad_system(
-   gamepads: Res<Gamepads>,
-   button_inputs: Res<ButtonInput<GamepadButton>>,
-   button_axes: Res<Axis<GamepadButton>>,
-   axes: Res<Axis<GamepadAxis>>,
+   gamepads: Query<&Gamepad>
) {
    for gamepad in gamepads.iter() {
-      if button_inputs.just_pressed(GamepadButton::new(gamepad, GamepadButtonType::South)) {
+      if gamepad.just_pressed(GamepadButton::South) {
            println!("just pressed South");
        } 
         
-      let right_trigger = button_axes
-           .get(GamepadButton::new(
-               gamepad,
-               GamepadButtonType::RightTrigger2,
-           ))
-           .unwrap();
+      let right_trigger = gamepad.get(GamepadButton::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            info!("RightTrigger2 value is {}", right_trigger);      
        }

-        let left_stick_x = axes
-           .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
-           .unwrap();
+       let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            info!("LeftStickX value is {}", left_stick_x);        
        }
    }
}

```


* * *

### Remove `ReceivedCharacter` [#](#remove-receivedcharacter)

`ReceivedCharacter` was deprecated in 0.14 due to `winit` reworking their keyboard system. It has now been fully removed. Switch to using `KeyboardInput` instead.

```
// 0.14
fn listen_characters(events: EventReader<ReceivedCharacter>) {
    for event in events.read() {
        info!("{}", event.char);
    }
}

// 0.15
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

### Use `Name` component for gamepad [#](#use-name-component-for-gamepad)

*   `GamepadInfo` no longer exists:
    *   Name now accessible via `Name` component.
    *   Other information available on `Gamepad` component directly.
    *   `GamepadConnection::Connected` now stores all info fields directly.

* * *

### Picking event ordering [#](#picking-event-ordering)

For users switching from `bevy_mod_picking` to `bevy_picking`:

*   Instead of adding an `On<T>` component, use `.observe(|trigger: Trigger<T>|)`. You may now apply multiple handlers to the same entity using this command.
*   Note that you need to add the non-default `MeshPickingPlugin` if you're using picking on meshes.
*   Pointer interaction events now have semi-deterministic ordering which (more or less) aligns with the order of the raw input stream. Consult the docs on `bevy_picking::event::pointer_events` for current information. You may need to adjust your event handling logic accordingly.
*   `PointerCancel` has been replaced with `Pointer<Canceled>`, which now has the semantics of an OS touch pointer cancel event.
*   `InputMove` and `InputPress` have been merged into `PointerInput`. The use remains exactly the same.
*   Picking interaction events are now only accessible through observers, and no `EventReader`. This functionality may be re-implemented later.

For users of `bevy_winit`:

*   The event `bevy_winit::WinitEvent` has moved to `bevy_window::WindowEvent`. If this was the only thing you depended on `bevy_winit` for, you should switch your dependency to `bevy_window`.
*   `bevy_window` now depends on `bevy_input`. The dependencies of `bevy_input` are a subset of the existing dependencies for `bevy_window` so this should be non-breaking.

Math [#](#math)
---------------

### Added `new` method to Cone 3D primitive [#](#added-new-method-to-cone-3d-primitive)

*   Addition of `new` method to the 3D primitive Cone struct.

* * *

### Basic integration of cubic spline curves with the Curve API [#](#basic-integration-of-cubic-spline-curves-with-the-curve-api)

The `RationalCurve::domain` method has been renamed to `RationalCurve::length`. Calling `.domain()` on a `RationalCurve` now returns its entire domain as an `Interval`.

* * *

### Disallow empty cubic and rational curves [#](#disallow-empty-cubic-and-rational-curves)

The `to_curve` method on Bevy’s cubic splines is now fallible (returning a `Result`), meaning that any existing calls will need to be updated by handling the possibility of an error variant.

Similarly, any custom implementation of `CubicGenerator` or `RationalGenerator` will need to be amended to include an `Error` type and be made fallible itself.

Finally, the fields of `CubicCurve` and `RationalCurve` are now private, so any direct constructions of these structs from segments will need to be replaced with the new `CubicCurve::from_segments` and `RationalCurve::from_segments` methods.

* * *

### Refactor Bounded2d/Bounded3d to use isometries [#](#refactor-bounded2d-bounded3d-to-use-isometries)

The `Bounded2d` and `Bounded3d` traits now take `Isometry2d` and `Isometry3d` parameters (respectively) instead of separate translation and rotation arguments. Existing calls to `aabb_2d`, `bounding_circle`, `aabb_3d`, and `bounding_sphere` will have to be changed to use isometries instead. A straightforward conversion is to refactor just by calling `Isometry2d/3d::new`, as follows:

```
// Old:
let aabb = my_shape.aabb_2d(my_translation, my_rotation);

// New:
let aabb = my_shape.aabb_2d(Isometry2d::new(my_translation, my_rotation));

```


However, if the old translation and rotation are 3d translation/rotations originating from a `Transform` or `GlobalTransform`, then `to_isometry` may be used instead. For example:

```
// Old:
let bounding_sphere = my_shape.bounding_sphere(shape_transform.translation, shape_transform.rotation);

// New:
let bounding_sphere = my_shape.bounding_sphere(shape_transform.to_isometry());

```


This discussion also applies to the `from_point_cloud` construction method of `Aabb2d`/`BoundingCircle`/`Aabb3d`/`BoundingSphere`, which has similarly been altered to use isometries.

* * *

### Rename `Rot2::angle_between` to `Rot2::angle_to` [#](#rename-rot2-angle-between-to-rot2-angle-to)

`Rot2::angle_between` has been deprecated, use `Rot2::angle_to` instead, the semantics of `Rot2::angle_between` will change in the future.

* * *

### Use `Dir2`/`Dir3` instead of `Vec2`/`Vec3` for `Ray2d::new`/`Ray3d::new` [#](#use-dir2-dir3-instead-of-vec2-vec3-for-ray2d-new-ray3d-new)

`Ray2d::new` and `Ray3d::new` now take a `Dir2` and `Dir3` instead of `Vec2` and `Vec3` respectively for the ray direction.

* * *

### Use a well defined type for sides in RegularPolygon [#](#use-a-well-defined-type-for-sides-in-regularpolygon)

*   `RegularPolygon` now uses `u32` instead of `usize` for the number of sides

* * *

### bevy\_reflect: Update `EulerRot` to match `glam` 0.29 [#](#bevy-reflect-update-eulerrot-to-match-glam-0-29)

The reflection implementation for `EulerRot` has been updated to align with `glam` 0.29. Please update any reflection-based usages accordingly.

* * *

### Use u32 for resolution/subdivision in primitive meshing [#](#use-u32-for-resolution-subdivision-in-primitive-meshing)

*   All primitive mesh builders now take `u32` instead of `usize` for their resolution/subdivision/segment counts

Picking [#](#picking)
---------------------

### Add flags to `SpritePlugin` and `UiPlugin` to allow disabling their picking backend (without needing to disable features). [#](#add-flags-to-spriteplugin-and-uiplugin-to-allow-disabling-their-picking-backend-without-needing-to-disable-features)

*   `UiPlugin` now contains an extra `add_picking` field if `bevy_ui_picking_backend` is enabled.
*   `SpritePlugin` is no longer a unit struct, and has one field if `bevy_sprite_picking_backend` is enabled (otherwise no fields).

* * *

### rename Drop to bevy::picking::events::DragDrop to unclash std::ops:Drop [#](#rename-drop-to-bevy-picking-events-dragdrop-to-unclash-std-ops-drop)

*   Rename `Drop` to `DragDrop`
    *   `bevy::picking::events::Drop` is now `bevy::picking::events::DragDrop`

Reflection [#](#reflection)
---------------------------

### Dedicated `Reflect` implementation for `Set`\-like things [#](#dedicated-reflect-implementation-for-set-like-things)

*   The new `Set` variants on the enums listed in the change section should probably be considered by people working with this level of the lib

**Help wanted!**

I’m not sure if this change is able to break code. From my understanding it shouldn’t since we just add functionality but I’m not sure yet if theres anything missing from my impl that would be normally provided by `impl_reflect_value!`

* * *

### Implement FromIterator/IntoIterator for dynamic types [#](#implement-fromiterator-intoiterator-for-dynamic-types)

*   Change `DynamicArray::from_vec` to `DynamicArray::from_iter`

* * *

### Make `drain` take a mutable borrow instead of `Box<Self>` for reflected `Map`, `List`, and `Set`. [#](#make-drain-take-a-mutable-borrow-instead-of-box-self-for-reflected-map-list-and-set)

*   `reflect::Map`, `reflect::List`, and `reflect::Set` all now take a `&mut self` instead of a `Box<Self>`. Callers of these traits should add `&mut` before their boxes, and implementers of these traits should update to match.

* * *

### Remove `Return::Unit` variant [#](#remove-return-unit-variant)

*   Removed the `Return::Unit` variant; use `Return::unit()` instead.

* * *

### Serialize and deserialize tuple struct with one field as newtype struct [#](#serialize-and-deserialize-tuple-struct-with-one-field-as-newtype-struct)

*   Reflection now will serialize and deserialize tuple struct with single field as newtype struct. Consider this code.

```
#[derive(Reflect, Serialize)]
struct Test(usize);
let reflect = Test(3);
let serializer = TypedReflectSerializer::new(reflect.as_partial_reflect(), &registry);
return serde_json::to_string(&serializer)

```


Old behavior will return `["3"]`. New behavior will return `"3"`. If you were relying on old behavior you need to update your logic. Especially with `serde_json`. `ron` doesn’t affect from this.

* * *

### bevy\_reflect: Add `DynamicTyped` trait [#](#bevy-reflect-add-dynamictyped-trait)

`Reflect` now has a supertrait of `DynamicTyped`. If you were manually implementing `Reflect` and did not implement `Typed`, you will now need to do so.

* * *

### bevy\_reflect: Add `ReflectDeserializerProcessor` [#](#bevy-reflect-add-reflectdeserializerprocessor)

(Since I added `P = ()`, I don’t think this is actually a breaking change anymore, but I’ll leave this in)

`bevy_reflect`’s `ReflectDeserializer` and `TypedReflectDeserializer` now take a `ReflectDeserializerProcessor` as the type parameter `P`, which allows you to customize deserialization for specific types when they are found. However, the rest of the API surface (`new`) remains the same.

Original implementation

Add `ReflectDeserializerProcessor`:

```
struct ReflectDeserializerProcessor {
    pub can_deserialize: Box<dyn FnMut(&TypeRegistration) -> bool + 'p>,
    pub deserialize: Box<
        dyn FnMut(
                &TypeRegistration,
                &mut dyn erased_serde::Deserializer,
            ) -> Result<Box<dyn PartialReflect>, erased_serde::Error>
            + 'p,
}

```


Along with `ReflectDeserializer::new_with_processor` and `TypedReflectDeserializer::new_with_processor`. This does not touch the public API of the existing `new` fns.

This is stored as an `Option<&mut ReflectDeserializerProcessor>` on the deserializer and any of the private `-Visitor` structs, and when we attempt to deserialize a value, we first pass it through this processor.

Also added a very comprehensive doc test to `ReflectDeserializerProcessor`, which is actually a scaled down version of the code for the `bevy_animation_graph` loader. This should give users a good motivating example for when and why to use this feature.

**Why `Box<dyn ..>`?**

When I originally implemented this, I added a type parameter to `ReflectDeserializer` to determine the processor used, with `()` being “no processor”. However when using this, I kept running into rustc errors where it failed to validate certain type bounds and led to overflows. I then switched to a dynamic dispatch approach.

The dynamic dispatch should not be that expensive, nor should it be a performance regression, since it’s only used if there is `Some` processor. (Note: I have not benchmarked this, I am just speculating.) Also, it means that we don’t infect the rest of the code with an extra type parameter, which is nicer to maintain.

**Why the `'p` on `ReflectDeserializerProcessor<'p>`?**

Without a lifetime here, the `Box`es would automatically become `Box<dyn FnMut(..) + 'static>`. This makes them practically useless, since any local data you would want to pass in must then be `'static`. In the motivating example, you couldn’t pass in that `&mut LoadContext` to the function.

This means that the `'p` infects the rest of the Visitor types, but this is acceptable IMO. This PR also elides the lifetimes in the `impl<'de> Visitor<'de> for -Visitor` blocks where possible.

**Future possibilities**

I think it’s technically possible to turn the processor into a trait, and make the deserializers generic over that trait. This would also open the door to an API like:

```
type Seed;

fn seed_deserialize(&mut self, r: &TypeRegistration) -> Option<Self::Seed>;

fn deserialize(&mut self, r: &TypeRegistration, d: &mut dyn erased_serde::Deserializer, s: Self::Seed) -> ...;

```


A similar processor system should also be added to the serialization side, but that’s for another PR. Ideally, both PRs will be in the same release, since one isn’t very useful without the other.

* * *

### bevy\_reflect: Add `Type` type [#](#bevy-reflect-add-type-type)

Certain type info structs now only return their item types as `Type` instead of exposing direct methods on them.

The following methods have been removed:

*   `ArrayInfo::item_type_path_table`
*   `ArrayInfo::item_type_id`
*   `ArrayInfo::item_is`
*   `ListInfo::item_type_path_table`
*   `ListInfo::item_type_id`
*   `ListInfo::item_is`
*   `SetInfo::value_type_path_table`
*   `SetInfo::value_type_id`
*   `SetInfo::value_is`
*   `MapInfo::key_type_path_table`
*   `MapInfo::key_type_id`
*   `MapInfo::key_is`
*   `MapInfo::value_type_path_table`
*   `MapInfo::value_type_id`
*   `MapInfo::value_is`

Instead, access the `Type` directly using one of the new methods:

*   `ArrayInfo::item_ty`
*   `ListInfo::item_ty`
*   `SetInfo::value_ty`
*   `MapInfo::key_ty`
*   `MapInfo::value_ty`

For example:

```
// BEFORE
let type_id = array_info.item_type_id();

// AFTER
let type_id = array_info.item_ty().id();

```


* * *

### bevy\_reflect: Nested `TypeInfo` getters [#](#bevy-reflect-nested-typeinfo-getters)

All active fields for reflected types (including lists, maps, tuples, etc.), must implement `Typed`. For the majority of users this won’t have any visible impact.

However, users implementing `Reflect` manually may need to update their types to implement `Typed` if they weren’t already.

Additionally, custom dynamic types will need to implement the new hidden `MaybeTyped` trait.

* * *

### bevy\_reflect: Refactor `serde` module [#](#bevy-reflect-refactor-serde-module)

The fields on `ReflectSerializer` and `TypedReflectSerializer` are now private. To instantiate, the corresponding constructor must be used:

```
// BEFORE
let serializer = ReflectSerializer {
    value: &my_value,
    registry: &type_registry,
};

// AFTER
let serializer = ReflectSerializer::new(&my_value, &type_registry);

```


Additionally, the following types are no longer public:

*   `ArraySerializer`
*   `EnumSerializer`
*   `ListSerializer`
*   `MapSerializer`
*   `ReflectValueSerializer` (fully removed)
*   `StructSerializer`
*   `TupleSerializer`
*   `TupleStructSerializer`

As well as the following traits:

*   `DeserializeValue` (fully removed)

* * *

### bevy\_reflect: Replace "value" terminology with "opaque" [#](#bevy-reflect-replace-value-terminology-with-opaque)

The reflection concept of “value type” has been replaced with a clearer “opaque type”. The following renames have been made to account for this:

*   `ReflectKind::Value` → `ReflectKind::Opaque`
*   `ReflectRef::Value` → `ReflectRef::Opaque`
*   `ReflectMut::Value` → `ReflectMut::Opaque`
*   `ReflectOwned::Value` → `ReflectOwned::Opaque`
*   `TypeInfo::Value` → `TypeInfo::Opaque`
*   `ValueInfo` → `OpaqueInfo`
*   `impl_reflect_value!` → `impl_reflect_opaque!`
*   `impl_from_reflect_value!` → `impl_from_reflect_opaque!`

Additionally, declaring your own opaque types no longer uses `#[reflect_value]`. This attribute has been replaced by `#[reflect(opaque)]`:

```
// BEFORE
#[derive(Reflect)]
#[reflect_value(Default)]
struct MyOpaqueType(u32);

// AFTER
#[derive(Reflect)]
#[reflect(opaque)]
#[reflect(Default)]
struct MyOpaqueType(u32);

```


Note that the order in which `#[reflect(opaque)]` appears does not matter.

* * *

### reflect: implement the unique reflect rfc [#](#reflect-implement-the-unique-reflect-rfc)

*   Most instances of `dyn Reflect` should be changed to `dyn PartialReflect` which is less restrictive, however trait bounds should generally stay as `T: Reflect`.
*   The new `PartialReflect::{as_partial_reflect, as_partial_reflect_mut, into_partial_reflect, try_as_reflect, try_as_reflect_mut, try_into_reflect}` methods as well as `Reflect::{as_reflect, as_reflect_mut, into_reflect}` will need to be implemented for manual implementors of `Reflect`.

* * *

### Use `FromReflect` when extracting entities in dynamic scenes [#](#use-fromreflect-when-extracting-entities-in-dynamic-scenes)

The `DynamicScene` format is changed to use custom serialize impls so old scene files will need updating:

Old:

```
(
  resources: {},
  entities: {
    4294967299: (
      components: {
        "bevy_render::camera::projection::OrthographicProjection": (
          near: 0.0,
          far: 1000.0,
          viewport_origin: (
            x: 0.5,
            y: 0.5,
          ),
          scaling_mode: WindowSize(1.0),
          scale: 1.0,
          area: (
            min: (
              x: -1.0,
              y: -1.0,
            ),
            max: (
              x: 1.0,
              y: 1.0,
            ),
          ),
        ),
      },
    ),
  },
)

```


New:

```
(
  resources: {},
  entities: {
    4294967299: (
      components: {
        "bevy_render::camera::projection::OrthographicProjection": (
          near: 0.0,
          far: 1000.0,
          viewport_origin: (0.5, 0.5),
          scaling_mode: WindowSize(1.0),
          scale: 1.0,
          area: (
            min: (-1.0, -1.0),
            max: (1.0, 1.0),
          ),
        ),
      },
    ),
  },
)

```


* * *

### move ShortName to bevy\_reflect [#](#move-shortname-to-bevy-reflect)

*   References to `bevy_utils::ShortName` should instead now be `bevy_reflect::ShortName`.

Rendering [#](#rendering)
-------------------------

### Retained Rendering [#](#retained-rendering)

With the advent of the retained render world, entities are no longer despawned at the end of every frame in the render world. Extracted entities with the `TemporaryRenderEntity` component will be despawned at the end of every frame like before.

In order to make this possible, the `Entity` identifiers in the main and the extracted version in render world are no longer guaranteed to line up. As a result:

*   all tools to spawn entities with a precise `Entity` id are in the process of being deprecated and will be removed
*   collections that contain references to `Entity` that are extracted into the render world have been changed to contain `MainEntity` in order to prevent errors where a render world entity id is used to look up an item by accident. Custom rendering code may need to be changed to query for `&MainEntity` in order to look up the correct item from such a collection
    *   users who implement their own extraction logic for collections of main world entity should strongly consider extracting into a different collection that uses `MainEntity` as a key.
*   render phases now require specifying both the `Entity` and `MainEntity` for a given `PhaseItem`. Custom render phases should ensure `MainEntity` is available when queuing a phase item

Renderers can now check `RenderVisibleEntities` to avoid rendering items that are not visible from a view. `RenderVisibleMeshEntities`, `RenderCubemapVisibleEntities`, and `RenderCascadeVisibleEntities` are also available for more fine-grained control.

To guide you further, let's take a look at a few common patterns. For every example, we specify in which world the code is run.

#### Spawning entities in the render world [#](#spawning-entities-in-the-render-world)

Previously, if you spawned an entity with `world.spawn(...)`, `commands.spawn(...)` or some other method in the rendering world, it would be despawned at the end of each frame. In 0.15, this is no longer the case and so your old code could leak entities. This can be mitigated by either re-architecting your code to no longer continuously spawn entities (like you're used to in the main world), or by adding the `bevy_render::world_sync::TemporaryRenderEntity` component to the entity you're spawning. Entities tagged with `TemporaryRenderEntity` will be removed at the end of each frame (like before).

#### Extract components with `ExtractComponentPlugin` [#](#extract-components-with-extractcomponentplugin)

```
// main world
app.add_plugins(ExtractComponentPlugin::<ComponentToExtract>::default());

```


`ExtractComponentPlugin` has been changed to automatically sync entities with `ComponentToExtract`. This is done via the new `WorldSyncPlugin`. Any code using `ExtractComponentPlugin` will not require any changes.

#### Manual extraction using `Extract<Query<(Entity, ...)>>` [#](#manual-extraction-using-extract-query-entity)

```
// in render world, inspired by bevy_pbr/src/cluster/mod.rs
pub fn extract_clusters(
    mut commands: Commands,
    views: Extract<Query<(Entity, &Clusters, &Camera)>>,
) {
    for (entity, clusters, camera) in &views {
        // some code
        commands.get_or_spawn(entity).insert(...);
    }
}

```


An extract query in the render world queries for entities and components in the main world. Here `entity` is a main world entity and `get_or_spawn(main_world_entity).insert(...)` potentially inserts components on the wrong entity. Remember, there is no longer a one-to-one correspondence between the main and render world entities. Moreover `get_or_spawn` has been deprecated.

In 0.15, you should use `RenderEntity` in place of `Entity` to get the correct entity in the render world. For entities to have a `RenderEntity` they need to be synced first. This can be done either via `WorldSyncPlugin` or adding the `SyncToRenderWorld` to the main world entity.

This results in the following code:

```
// in render world, inspired by bevy_pbr/src/cluster/mod.rs
pub fn extract_clusters(
    mut commands: Commands,
    views: Extract<Query<(RenderEntity, &Clusters, &Camera)>>,
) {
    for (render_entity, clusters, camera) in &views {
        // some code
        // After the sync step, all main world entities with a &RenderEntity have a corresponding (empty) render world entity. This should never panic.
        commands.entity(render_entity).insert(...);
    }
}

// in main world, when spawning
world.spawn((Clusters::default(), Camera::default(), SyncToRenderWorld))

```


#### Looking up main world entities in the render world [#](#looking-up-main-world-entities-in-the-render-world)

In order to get the main world entity from a render world entity. It works much the same. Every synced render world entity has a `MainEntity` component you can query for that returns the correct main world entity.

```
// in the render world
pub fn inspect_clusters(
    views: Query<(MainEntity, &Clusters, &Camera)>
) {
    for (main_entity, clusters, camera in &views) {
        // do something
    }
}

```


#### General advice for working with main and render world entities [#](#general-advice-for-working-with-main-and-render-world-entities)

When working with entities from both worlds it can be confusing. If you are every in a scenario where this isn't entirely clear (for example, when working on custom extraction code in the render world), we advise that you use `RenderEntity` and `MainEntity` as simple wrappers around `Entity`. Mixing these up can become a real headache and lead to some non-obvious errors.

```
// render world 0.14
pub instances: Vec<(Entity, RenderLayers, bool)>,

// render world 0.15
pub instances: Vec<(MainEntity, RenderLayers, bool)>,

```


There are also other ways to disambiguate between the two worlds.

```
// render world 0.14
pub(crate) render_lightmaps: EntityHashMap<RenderLightmap>,

// render world 0.15
pub(crate) render_lightmaps: MainEntityHashMap<RenderLightmap>,

```


* * *

### Add 2d opaque phase with depth buffer [#](#add-2d-opaque-phase-with-depth-buffer)

*   `ColorMaterial` now contains `AlphaMode2d`. To keep previous behaviour, use `AlphaMode::BLEND`. If you know your sprite is opaque, use `AlphaMode::OPAQUE`

* * *

### Add `RenderSet::FinalCleanup` for `World::clear_entities` [#](#add-renderset-finalcleanup-for-world-clear-entities)

`World::clear_entities` is now part of `RenderSet::PostCleanup` rather than `RenderSet::Cleanup`. Your cleanup systems should likely stay in `RenderSet::Cleanup`.

* * *

### Add feature requirement info to image loading docs [#](#add-feature-requirement-info-to-image-loading-docs)

Image format related entities are feature gated, if there are compilation errors about unknown names there are some of features in list (`exr`, `hdr`, `basis-universal`, `png`, `dds`, `tga`, `jpeg`, `bmp`, `ktx2`, `webp` and `pnm`) should be added.

* * *

### Add support for environment map transformation [#](#add-support-for-environment-map-transformation)

*   Since we have added a new filed to the `EnvironmentMapLight` struct, users will need to include `..default()` or some rotation value in their initialization code.

* * *

### Add support for skybox transformation [#](#add-support-for-skybox-transformation)

*   Since we have added a new filed to the Skybox struct, users will need to include `..Default::default()` or some rotation value in their initialization code.

* * *

### Added feature switch to default Standard Material's new anisotropy texture to off [#](#added-feature-switch-to-default-standard-material-s-new-anisotropy-texture-to-off)

*   Add feature pbr\_anisotropy\_texture if you are using that texture in any standard materials.

* * *

### Added visibility bitmask as an alternative SSAO method [#](#added-visibility-bitmask-as-an-alternative-ssao-method)

SSAO algorithm was changed from GTAO to VBAO (visibility bitmasks). A new field, `constant_object_thickness`, was added to `ScreenSpaceAmbientOcclusion`. `ScreenSpaceAmbientOcclusion` also lost its `Eq` and `Hash` implementations.

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

### Adds `ShaderStorageBuffer` asset [#](#adds-shaderstoragebuffer-asset)

The `AsBindGroup` `storage` attribute has been modified to reference the new `Handle<Storage>` asset instead. Usages of Vec\` should be converted into assets instead.

* * *

### Allow mix of hdr and non-hdr cameras to same render target [#](#allow-mix-of-hdr-and-non-hdr-cameras-to-same-render-target)

Change `CameraOutputMode` to use `ClearColorConfig` instead of `LoadOp`.

* * *

### Allow volumetric fog to be localized to specific, optionally voxelized, regions. [#](#allow-volumetric-fog-to-be-localized-to-specific-optionally-voxelized-regions)

*   A `FogVolume` is now necessary in order to enable volumetric fog, in addition to `VolumetricFogSettings` on the camera. Existing uses of volumetric fog can be migrated by placing a large `FogVolume` surrounding the scene.

* * *

### Attempt to remove component from render world if not extracted. [#](#attempt-to-remove-component-from-render-world-if-not-extracted)

Components that implement `ExtractComponent` and return `None` will cause the extracted component to be removed from the render world.

* * *

### Bind only the written parts of storage buffers. [#](#bind-only-the-written-parts-of-storage-buffers)

*   Fixed a bug with StorageBuffer and DynamicStorageBuffer binding data from the previous frame(s) due to caching GPU buffers between frames.

* * *

### Changed `Mesh::attributes*` functions to return `MeshVertexAttribute` [#](#changed-mesh-attributes-functions-to-return-meshvertexattribute)

*   When using the iterator returned by `Mesh::attributes` or `Mesh::attributes_mut` the first value of the tuple is not the `MeshVertexAttribute` instead of `MeshVertexAttributeId`. To access the `MeshVertexAttributeId` use the `MeshVertexAttribute.id` field.

* * *

### Expose Pipeline Compilation Zero Initialize Workgroup Memory Option [#](#expose-pipeline-compilation-zero-initialize-workgroup-memory-option)

*   add `zero_initialize_workgroup_memory: false,` to `ComputePipelineDescriptor` or `RenderPipelineDescriptor` structs to preserve 0.14 functionality, add `zero_initialize_workgroup_memory: true,` to restore bevy 0.13 functionality.

* * *

### Feature-gate all image formats [#](#feature-gate-all-image-formats)

Image formats that previously weren’t feature-gated are now feature-gated, meaning they will have to be enabled if you use them:

*   `avif`
*   `ff` (Farbfeld)
*   `gif`
*   `ico`
*   `tiff`

Additionally, the `qoi` feature has been added to support loading QOI format images.

Previously, these formats appeared in the enum by default, but weren’t actually enabled via the `image` crate, potentially resulting in weird bugs. Now, you should be able to add these features to your projects to support them properly.

* * *

If you were individually configuring the `bevy_render` crate, the feature flags for the general image formats were moved to `bevy_image` instead. For example, `bevy_render/png` no longer exists, and `bevy_image/png` is the new location for this. The texture formats are still available on `bevy_render`, e.g. `bevy_render/ktx2` is needed to fully enable `ktx2` support, and this will automatically enable `bevy_image/ktx2` for loading the textures.

* * *

### Fix Mesh allocator bug and reduce Mesh data copies by two [#](#fix-mesh-allocator-bug-and-reduce-mesh-data-copies-by-two)

*   `Mesh::get_vertex_buffer_data` has been renamed `Mesh::create_packed_vertex_buffer_data` to reflect the fact that it copies data and allocates.

* * *

### Improve API for scaling orthographic cameras [#](#improve-api-for-scaling-orthographic-cameras)

`ScalingMode` has been refactored for clarity, especially on how to zoom orthographic cameras and their projections:

*   `ScalingMode::WindowSize` no longer stores a float, and acts as if its value was 1. Divide your camera’s scale by any previous value to achieve identical results.
*   `ScalingMode::FixedVertical` and `FixedHorizontal` now use named fields.

* * *

### Lighting Should Only hold `Vec<Entity>` instead of `TypeId<Vec<Entity>>` [#](#lighting-should-only-hold-vec-entity-instead-of-typeid-vec-entity)

`SpotLight`, `CascadesVisibleEntities` and `CubemapVisibleEntities` now use `VisibleMeshEntities` instead of `VisibleEntities`

* * *

### Migrate bevy\_sprite to required components [#](#migrate-bevy-sprite-to-required-components)

Replace all uses of `SpriteBundle` with `Sprite`. There are several new convenience constructors: `Sprite::from_image`, `Sprite::from_atlas_image`, `Sprite::from_color`.

WARNING: use of `Handle<Image>` and `TextureAtlas` as components on sprite entities will NO LONGER WORK. Use the fields on `Sprite` instead. I would have removed the `Component` impls from `TextureAtlas` and `Handle<Image>` except it is still used within ui. We should fix this moving forward with the migration.

* * *

### Migrate lights to required components [#](#migrate-lights-to-required-components)

`PointLightBundle`, `SpotLightBundle`, and `DirectionalLightBundle` have been deprecated. Use the `PointLight`, `SpotLight`, and `DirectionalLight` components instead. Adding them will now insert the other components required by them automatically.

* * *

### Move `ImageLoader` and `CompressedImageSaver` to `bevy_image`. [#](#move-imageloader-and-compressedimagesaver-to-bevy-image)

*   `ImageLoader` can no longer be initialized directly through `init_asset_loader`. Now you must use `app.register_asset_loader(ImageLoader::new(supported_compressed_formats))` (check out the implementation of `bevy_render::ImagePlugin`). This only affects you if you are initializing the loader manually and does not affect users of `bevy_render::ImagePlugin`.
*   The asset loader name must be updated in `.meta` files for images. Change: `loader: "bevy_render::texture::image_loader::ImageLoader",` to: `loader: "bevy_image::image_loader::ImageLoader",`

This will fix the following error:

> `no` AssetLoader `found with the name 'bevy_render::texture::image_loader::ImageLoader`

* * *

### Move `Msaa` to component [#](#move-msaa-to-component)

`Msaa` is no longer configured as a global resource, and should be specified on each spawned camera if a non-default setting is desired.

* * *

### Only use the AABB center for mesh visibility range testing if specified. [#](#only-use-the-aabb-center-for-mesh-visibility-range-testing-if-specified)

*   The `VisibilityRange` component now has an extra field, `use_aabb`. Generally, you can safely set it to false.

* * *

### Pack multiple vertex and index arrays together into growable buffers. [#](#pack-multiple-vertex-and-index-arrays-together-into-growable-buffers)

*   Vertex and index buffers for meshes may now be packed alongside other buffers, for performance.
*   `GpuMesh` has been renamed to `RenderMesh`, to reflect the fact that it no longer directly stores handles to GPU objects.
*   Because meshes no longer have their own vertex and index buffers, the responsibility for the buffers has moved from `GpuMesh` (now called `RenderMesh`) to the `MeshAllocator` resource. To access the vertex data for a mesh, use `MeshAllocator::mesh_vertex_slice`. To access the index data for a mesh, use `MeshAllocator::mesh_index_slice`.

* * *

### Reduce the clusterable object UBO size below 16384 for WebGL 2. [#](#reduce-the-clusterable-object-ubo-size-below-16384-for-webgl-2)

The maximum number of clusterable objects on `WebGL2` is now 204, to keep us within our 16 kB memory budget. Modify your scenes or use WebGPU if you are running into this.

* * *

### Refactor `AsBindGroup` to use a associated `SystemParam`. [#](#refactor-asbindgroup-to-use-a-associated-systemparam)

`AsBindGroup` now allows the user to specify a `SystemParam` to be used for creating bind groups.

* * *

### Remove AVIF feature [#](#remove-avif-feature)

AVIF images are no longer supported. They never really worked, and require system dependencies (libdav1d) to work correctly, so, it’s better to simply offer this support via an unofficial plugin instead as needed. The corresponding types have been removed from Bevy to account for this.

* * *

### Remove OrthographicProjection.scale (adopted) [#](#remove-orthographicprojection-scale-adopted)

Replace all uses of `scale` with `scaling_mode`, keeping in mind that `scale` is (was) a multiplier. For example, replace

```
    scale: 2.0,
    scaling_mode: ScalingMode::FixedHorizontal(4.0),


```


with

```
    scaling_mode: ScalingMode::FixedHorizontal(8.0),

```


* * *

### Rename rendering components for improved consistency and clarity [#](#rename-rendering-components-for-improved-consistency-and-clarity)

Many rendering components have been renamed for improved consistency and clarity.

*   `AutoExposureSettings` → `AutoExposure`
*   `BloomSettings` → `Bloom`
*   `BloomPrefilterSettings` → `BloomPrefilter`
*   `ContrastAdaptiveSharpeningSettings` → `ContrastAdaptiveSharpening`
*   `DepthOfFieldSettings` → `DepthOfField`
*   `FogSettings` → `DistanceFog`
*   `SmaaSettings` → `Smaa`
*   `TemporalAntiAliasSettings` → `TemporalAntiAliasing`
*   `ScreenSpaceAmbientOcclusionSettings` → `ScreenSpaceAmbientOcclusion`
*   `ScreenSpaceReflectionsSettings` → `ScreenSpaceReflections`
*   `VolumetricFogSettings` → `VolumetricFog`

* * *

### Replace the `wgpu_trace` feature with a field in `bevy_render::settings::WgpuSettings` [#](#replace-the-wgpu-trace-feature-with-a-field-in-bevy-render-settings-wgpusettings)

The `bevy/wgpu_trace` and `bevy_render/wgpu_trace` features have been removed, as WGPU tracing is now enabled during the creation of `bevy_render::RenderPlugin`.

Note: At the time of writing, WGPU has not reimplemented tracing support, so WGPU tracing will not currently work. However, once WGPU has reimplemented tracing support, the steps below should be sufficient to continue generating WGPU traces.

You can track the progress of WGPU tracing being reimplemented at [gfx-rs/wgpu#5974](https://github.com/gfx-rs/wgpu/issues/5974).

To continue generating WGPU traces:

1.  Remove any instance of the `bevy/wgpu_trace` or `bevy_render/wgpu_trace` features you may have in any of your `Cargo.toml` files.
2.  Follow the instructions in [`docs/debugging.md`, under the WGPU Tracing section](https://github.com/bevyengine/bevy/blob/release-0.15.0/docs/debugging.md#wgpu-tracing).

* * *

### Replaced implicit emissive weight with default. [#](#replaced-implicit-emissive-weight-with-default)

The behaviour of emissive materials when using deferred rendering has been changed to match forward rendering. Tweak the emissive values of your materials to achieve the desired effect.

* * *

### Return `Result`s from `Camera`'s world/viewport conversion methods [#](#return-results-from-camera-s-world-viewport-conversion-methods)

The following methods on `Camera` now return a `Result` instead of an `Option` so that they can provide more information about failures:

*   `world_to_viewport`
*   `world_to_viewport_with_depth`
*   `viewport_to_world`
*   `viewport_to_world_2d`

Call `.ok()` on the `Result` to turn it back into an `Option`, or handle the `Result` directly.

* * *

### Rewrite screenshots. [#](#rewrite-screenshots)

`ScreenshotManager` has been removed. To take a screenshot, spawn a `Screenshot` entity with the specified render target and provide an observer targeting the `ScreenshotCaptured` event. See the `window/screenshot` example to see an example.

* * *

### Split OrthographicProjection::default into 2d & 3d (Adopted) [#](#split-orthographicprojection-default-into-2d-3d-adopted)

*   In initialization of `OrthographicProjection`, change `..default()` to `..OrthographicProjection::default_2d()` or `..OrthographicProjection::default_3d()`

Example:

```
--- a/examples/3d/orthographic.rs
+++ b/examples/3d/orthographic.rs
@@ -20,7 +20,7 @@ fn setup(
         projection: OrthographicProjection {
             scale: 3.0,
             scaling_mode: ScalingMode::FixedVertical(2.0),
-            ..default()
+            ..OrthographicProjection::default_3d()
         }
         .into(),
         transform: Transform::from_xyz(5.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),

```


* * *

### Split out bevy\_mesh from bevy\_render [#](#split-out-bevy-mesh-from-bevy-render)

`bevy_render::mesh::morph::inherit_weights` has been moved to `bevy_render::mesh::inherit_weights`.

`Mesh::compute_aabb` has been moved to the new `MeshAabb` trait. You may need to import it.

```
use bevy::render::mesh::MeshAabb;

```


* * *

### Using Cas instead of CAS #14341 [#](#using-cas-instead-of-cas-14341)

`CASNode`, `DenoiseCAS` `CASPipeline`, and `CASUniform` have been renamed to `CasNode` (and so on) to follow standard Rust naming conventions.

* * *

### Virtual Geometry changes [#](#virtual-geometry-changes)

*   Runtime
    *   This feature now requires that your GPU supports `WgpuFeatures::SHADER_INT64_ATOMIC_MIN_MAX`.
    *   `MeshletPlugin` now requires a `cluster_buffer_slots` field. Read the rustdoc for more details.
    *   `MaterialMeshletMeshBundle` has been deprecated. Instead of using `Handle<MeshletMesh>` and `Handle<M: Material>` as components directly, use the `MeshletMesh3d` and `MeshMaterial3d` components.
*   Asset Conversion
    *   Regenerate your `MeshletMesh` assets, as the asset format has changed, and `MESHLET_MESH_ASSET_VERSION` has been bumped. Old assets are not compatible.
    *   When using `MeshletMesh::from_mesh()`, the provided mesh must no longer have tangents as a vertex attribute.
    *   When using `MeshletMesh::from_mesh()`, you must now supply a `vertex_position_quantization_factor` argument. Use `MESHLET_DEFAULT_VERTEX_POSITION_QUANTIZATION_FACTOR`, and adjust as needed. See the docs for more info.
*   Misc
    *   `MeshletMeshSaverLoad` has been split into `MeshletMeshSaver` and `MeshletMeshLoader`.
    *   Renamed `MeshletMeshSaveOrLoadError` to `MeshToMeshletMeshConversionError`.
    *   The `MeshletMeshSaveOrLoadError::SerializationOrDeserialization` enum variant has been removed.
    *   Added `MeshToMeshletMeshConversionError::WrongFileType`, match on this variant if you match on `MeshToMeshletMeshConversionError`
    *   `MeshletMesh` fields are now private.
    *   The `Meshlet`, `MeshletBoundingSpheres`, and `MeshletBoundingSphere` types are now private.

* * *

### Wgpu 0.20 [#](#wgpu-0-20)

*   Updated to `wgpu` 0.20, `naga` 0.20, and `naga_oil` 0.14
*   All of Naga’s [`Capabilities`](https://docs.rs/naga/latest/naga/valid/struct.Capabilities.html) should now be properly detected and supported.
*   Timestamps inside encoders are now disallowed on WebGPU to follow the spec (they still work on native). Use the `TIMESTAMP_QUERY_INSIDE_ENCODERS` wgpu feature to check for support.
*   You can now use many numeric built-ins in `const` contexts (eg. `abs`, `cos`, `floor`, `max`, etc, see https://github.com/gfx-rs/wgpu/blob/v0.20/CHANGELOG.md#wgsl-const-evaluation-for-many-more-built-ins for the whole list)
*   You can now use Subgroup operations in shaders on supported hardware (see https://github.com/gfx-rs/wgpu/blob/v0.20/CHANGELOG.md#subgroup-operations for limitations and which features to check)
*   `u64` and `i64` are now supported in shaders on supported hardware (requires the `SHADER_INT64` feature, supported on desktop Vulkan, DX12 with DXC, and Metal with MSL 2.3+)

* * *

### check sampler type in as\_bind\_group derives [#](#check-sampler-type-in-as-bind-group-derives)

Instead of panicking, the `AsBindGroup` derive can now fail. To accommodate this, `PrepareAssetError` now has another arm: `PrepareAssetError::AsBindGroupError`. If you were exhaustively matching, you now need to handle this failure mode.

* * *

### cleanup bevy\_render/lib.rs [#](#cleanup-bevy-render-lib-rs)

`RenderCreation::Manual` variant fields are now wrapped in a struct called `RenderResources`

* * *

### Fix UI texture atlas with offset [#](#fix-ui-texture-atlas-with-offset)

```
let ui_node = ExtractedUiNode {
                    stack_index,
                    transform,
                    color,
                    rect,
                    image,
-                   atlas_size: Some(atlas_size * scale_factor),      
+                   atlas_scaling: Some(Vec2::splat(scale_factor)),
                    clip,
                    flip_x,
                    flip_y,
                    camera_entity,
                    border,
                    border_radius,
                    node_type,
                },

```


```
let computed_slices = ComputedTextureSlices {
    slices,
-    image_size,
}

```


* * *

### Make default behavior for `BackgroundColor` and `BorderColor` more intuitive [#](#make-default-behavior-for-backgroundcolor-and-bordercolor-more-intuitive)

*   `BackgroundColor` no longer tints the color of images in `ImageBundle` or `ButtonBundle`. Set `UiImage::color` to tint images instead.
*   The default texture for `UiImage` is now a transparent white square. Use `UiImage::solid_color` to quickly draw debug images.
*   The default value for `BackgroundColor` and `BorderColor` is now transparent. Set the color to white manually to return to previous behavior.

* * *

### Optional UI rendering [#](#optional-ui-rendering)

`UiPlugin` has a new field `enable_rendering`. If set to false, the UI’s rendering systems won’t be added to the `RenderApp` and no UI elements will be drawn. The layout and interaction components will still be updated as normal.

* * *

### use precomputed border values [#](#use-precomputed-border-values)

The `logical_rect` and `physical_rect` methods have been removed from `Node`. Use `Rect::from_center_size` with the translation and node size instead.

The types of the fields border and border\_radius of `ExtractedUiNode` have been changed to `BorderRect` and `ResolvedBorderRadius` respectively.

* * *

### Inverse bevy\_render bevy\_winit dependency and move cursor to bevy\_winit [#](#inverse-bevy-render-bevy-winit-dependency-and-move-cursor-to-bevy-winit)

`CursorIcon` and `CustomCursor` previously provided by `bevy::render::view::cursor` is now available from `bevy::winit`. A new feature `custom_cursor` enables this functionality (default feature).

Scenes [#](#scenes)
-------------------

### Align `Scene::write_to_world_with` to match `DynamicScene::write_to_world_with` [#](#align-scene-write-to-world-with-to-match-dynamicscene-write-to-world-with)

`Scene::write_to_world_with` no longer returns an `InstanceInfo`.

Before

```
scene.write_to_world_with(world, &registry)

```


After

```
let mut entity_map = EntityHashMap::default();
scene.write_to_world_with(world, &mut entity_map, &registry)

```


* * *

### Align `Scene::write_to_world_with` to match `DynamicScene::write_to_world_with` [#](#align-scene-write-to-world-with-to-match-dynamicscene-write-to-world-with-1)

`Scene::write_to_world_with` no longer returns an `InstanceInfo`.

Before

```
scene.write_to_world_with(world, &registry)

```


After

```
let mut entity_map = EntityHashMap::default();
scene.write_to_world_with(world, &mut entity_map, &registry)

```


* * *

### Change `SceneInstanceReady` to trigger an observer. [#](#change-sceneinstanceready-to-trigger-an-observer)

If you have a system which reads `SceneInstanceReady` events, it must be rewritten as an observer or entity observer.

```
// 0.14
fn ready_system(ready_events: EventReader<'_, '_, SceneInstanceReady>) {
    // ...
}

// 0.15
commands.observe(|trigger: Trigger<SceneInstanceReady>| {
    // ...
});
commands.entity(entity).observe(|trigger: Trigger<SceneInstanceReady>| {
    // ...
});

```


* * *

### Migrate scenes to required components [#](#migrate-scenes-to-required-components)

Asset handles for scenes and dynamic scenes must now be wrapped in the `SceneRoot` and `DynamicSceneRoot` components. Raw handles as components no longer spawn scenes.

Additionally, `SceneBundle` and `DynamicSceneBundle` have been deprecated. Instead, use the scene components directly.

Previously:

```
let model_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("model.gltf"));

commands.spawn(SceneBundle {
    scene: model_scene,
    transform: Transform::from_xyz(-4.0, 0.0, -3.0),
    ..default()
});

```


Now:

```
let model_scene = asset_server.load(GltfAssetLabel::Scene(0).from_asset("model.gltf"));

commands.spawn((
    SceneRoot(model_scene),
    Transform::from_xyz(-4.0, 0.0, -3.0),
));

```


* * *

### Send `SceneInstanceReady` when spawning any kind of scene [#](#send-sceneinstanceready-when-spawning-any-kind-of-scene)

*   `SceneInstanceReady { parent: Entity }` is now `SceneInstanceReady { id: InstanceId, parent: Option<Entity> }`.

* * *

### explicitly mention `component` in methods on `DynamicSceneBuilder` [#](#explicitly-mention-component-in-methods-on-dynamicscenebuilder)

`DynamicSceneBuilder::allow_all` and `deny_all` now set resource accesses, not just components. To return to the previous behavior, use the new `allow_all_components` or `deny_all_components` methods.

The following methods for `DynamicSceneBuilder` have been renamed:

*   `with_filter` -> `with_component_filter`
*   `allow` -> `allow_component`
*   `deny` -> `deny_component`

Tasks [#](#tasks)
-----------------

### Support `on_thread_spawn` and `on_thread_destroy` for `TaskPoolPlugin` [#](#support-on-thread-spawn-and-on-thread-destroy-for-taskpoolplugin)

*   `TaskPooolThreadAssignmentPolicy` now has two additional fields: `on_thread_spawn` and `on_thread_destroy`. Please consider defaulting them to `None`.

Text [#](#text)
---------------

### Text Rework cleanup [#](#text-rework-cleanup)

Doubles as #15591 migration guide.

Text bundles (`TextBundle` and `Text2dBundle`) were removed in favor of `Text` and `Text2d`. Shared configuration fields were replaced with `TextLayout`, `TextFont` and `TextColor` components. Just `TextBundle`’s additional field turned into `TextNodeFlags` component, while `Text2dBundle`’s additional fields turned into `TextBounds` and `Anchor` components.

Text sections were removed in favor of hierarchy-based approach. For root text entities with `Text` or `Text2d` components, child entities with `TextSpan` will act as additional text sections. To still access text spans by index, use the new `TextUiReader`, `Text2dReader` and `TextUiWriter`, `Text2dWriter` system parameters.

* * *

### Text rework [#](#text-rework)

The `Text` API in Bevy has been overhauled in several ways as part of Bevy 0.15. There are several major changes to consider:

*   `ab_glyph` has been replaced with `cosmic-text`. These changes are mostly internal and the majority of users will not interact with either text backend directly.
*   each text section is now stored as a distinct entity within the standard hierarchy, rather than as a `Vec<TextSection>` on the `Text` component. Children of `Text`/`Text2d` entities with `TextSpan` components will act as additional text sections.
*   like other aspects of Bevy's API, required components have replaced bundles

#### `TextBundle` and text styling [#](#textbundle-and-text-styling)

`TextBundle` has been removed. Add the `Text` component to set the string displayed.

`TextLayout`, `TextFont` and `TextColor` are required components for `Text`, and are automatically added whenever `Text` is. Set those values to change the text section's style.

Like elsewhere in Bevy, there is no style inheritance. Consider [writing your own abstraction for this](https://github.com/viridia/thorium_ui/blob/main/crates/thorium_ui_controls/src/text_styles.rs) if this is something you'd like to use.

To control the layout of a `Text` section, modify the properties of its `Node`.

#### Accessing text spans by index [#](#accessing-text-spans-by-index)

Previously, text sections were elements of a vector stored within `Text`. Now, they are stored as distinct entities under the same `Parent`. You can use the new `TextUiReader` and `TextUiWriter` system parameters to conveniently access text spans by index.

Before:

```
fn refresh_text(mut query: Query<&mut Text, With<TimeText>>, time: Res<Time>) {
    let text = query.single_mut();
    text.sections[1].value = format_time(time.elapsed());
}

```


After:

```
fn refresh_text(
    query: Query<Entity, With<TimeText>>,
    mut writer: TextUiWriter,
    time: Res<Time>
) {
    let entity = query.single();
    *writer.text(entity, 1) = format_time(time.elapsed());
}

```


2D equivalents (`Text2dReader` and `Text2dWriter`) also exist.

#### Internal layout information [#](#internal-layout-information)

`TextBundle` additional fields have been moved into the `TextNodeFlags` component, while `Text2dBundle`'s additional fields turned into the `TextBounds` and `Anchor` components.

* * *

### Uncouple `DynamicTextureAtlasBuilder` from assets [#](#uncouple-dynamictextureatlasbuilder-from-assets)

*   Replace the `glyph_id` and `subpixel_offset` of a few text atlas APIs by a single `place_glyph: PlacedGlyph` parameter trivially combining the two.
*   `DynamicTextureAtlasBuilder::add_texture` now takes a `&mut Image`, rather than a `Handle<Image>`. To access this, fetch the underlying image using `Assets<Image>::get_mut`.

* * *

### split up `TextStyle` [#](#split-up-textstyle)

`TextStyle` has been renamed to `TextFont` and its `color` field has been moved to a separate component named `TextColor` which newtypes `Color`.

* * *

### Add the ability to control font smoothing [#](#add-the-ability-to-control-font-smoothing)

*   `Text` now contains a `font_smoothing: FontSmoothing` property, make sure to include it or add `..default()` when using the struct directly;
*   `FontSizeKey` has been renamed to `FontAtlasKey`, and now also contains the `FontSmoothing` setting;
*   The following methods now take an extra `font_smoothing: FontSmoothing` argument:
    *   `FontAtlas::new()`
    *   `FontAtlasSet::add_glyph_to_atlas()`
    *   `FontAtlasSet::get_glyph_atlas_info()`
    *   `FontAtlasSet::get_outlined_glyph_texture()`

* * *

### Cosmic text [#](#cosmic-text)

*   `Text2dBounds` has been replaced with `TextBounds`, and it now accepts `Option`s to the bounds, instead of using `f32::INFINITY` to indicate lack of bounds
*   Textsizes should be changed, dividing the current size with 1.2 will result in the same size as before.
*   `TextSettings` struct is removed
*   Feature `subpixel_glyph_atlas` has been removed since cosmic-text already does this automatically
*   TextBundles and things rendering texts requires the `CosmicBuffer` Component on them as well

Time [#](#time)
---------------

### aligning public apis of Time,Timer and Stopwatch [#](#aligning-public-apis-of-time-timer-and-stopwatch)

The APIs of `Time`, `Timer` and `Stopwatch` have been cleaned up for consistency with each other and the standard library’s `Duration` type. The following methods have been renamed:

*   `Stowatch::paused` -> `Stopwatch::is_paused`
*   `Time::elapsed_seconds` -> `Time::elapsed_secs` (including `_f64` and `_wrapped` variants)

UI [#](#ui)
-----------

### Add UI `GhostNode` [#](#add-ui-ghostnode)

Any code that previously relied on `Parent`/`Children` to iterate UI children may now want to use `bevy_ui::UiChildren` to ensure ghost nodes are skipped, and their first descendant Nodes included.

UI root nodes may now be children of ghost nodes, which means `Without<Parent>` might not query all root nodes. Use `bevy_ui::UiRootNodes` where needed to iterate root nodes instead.

* * *

### Clean up UiSystem system sets [#](#clean-up-uisystem-system-sets)

`UiSystem` system set adjustments.

*   The `UiSystem::Outline` system set is now strictly ordered after `UiSystem::Layout`, rather than overlapping it.

* * *

### Migrate UI bundles to required components [#](#migrate-ui-bundles-to-required-components)

`NodeBundle` has been replaced with `Node` (and its associated required components). Simultaneously, the fields and behavior of `Style` have been moved to `Node`, and the largely internal values previously stored there are now found on `ComputedNode`.

It will be easiest to migrate if you replace `Node` with `ComputedNode` first, then `Style` with `Node`, and finally `NodeBundle` with `Node`.

#### `Node` -> `ComputedNode` [#](#node-computednode)

For any usage of the “computed node properties” that used to live on `Node`, use `ComputedNode` instead. This is a trivial find-and-replace rename.

If you were ever explicitly adding `Node` (now `ComputedNode`) to your UI bundles, you can remove this, as it is now required by `Node` (previously `Style`).

Before:

```
fn system(nodes: Query<&Node>) {
    for node in &nodes {
        let computed_size = node.size();
    }
}

```


After:

```
fn system(computed_nodes: Query<&ComputedNode>) {
    for computed_node in &computed_nodes {
        let computed_size = computed_node.size();
    }
}

```


#### `Style` -> `Node` [#](#style-node)

All of the values of `Style` now live on `Node`. This is a find-and-replace rename.

Before:

```
Style {
    width: Val::Px(100.),
    ..default()
}

```


After:

```
Node {
    width: Val::Px(100.),
    ..default()
}

```


#### `NodeBundle` -> `Node` [#](#nodebundle-node)

Finally, replace all uses of `NodeBundle` with `Node`. All other components in `NodeBundle` are now added implicitly via required components. Adding them to your bundles manually will overwrite the default values.

Before:

```
     commands
        .spawn(NodeBundle {
            style: Style {
                 width: Val::Percent(100.),
                 align_items: AlignItems::Center,
                 justify_content: JustifyContent::Center,
                 ..default()
             },
            ..default()
        });

```


After:

```
     commands
        .spawn(Node {
            width: Val::Percent(100.),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        });

```


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

### Only use physical coords internally in `bevy_ui` [#](#only-use-physical-coords-internally-in-bevy-ui)

`ComputedNode`’s fields and methods now use physical coordinates. `ComputedNode` has a new field `inverse_scale_factor`. Multiplying the physical coordinates by the `inverse_scale_factor` will give the logical values.

* * *

### Overflow clip margin [#](#overflow-clip-margin)

Style has a new field `OverflowClipMargin`. It allows users to set the visible area for clipped content when using overflow-clip, -hidden, or -scroll and expand it with a margin.

There are three associated constructor functions `content_box`, `padding_box` and `border_box`:

*   `content_box`: elements painted outside of the content box area (the innermost part of the node excluding the padding and border) of the node are clipped. This is the new default behaviour.
*   `padding_box`: elements painted outside outside of the padding area of the node are clipped.
*   `border_box`: elements painted outside of the bounds of the node are clipped. This matches the behaviour from Bevy 0.14.

There is also a `with_margin` method that increases the size of the visible area by the given number in logical pixels, negative margin values are clamped to zero.

`OverflowClipMargin` is ignored unless overflow-clip, -hidden or -scroll is also set on at least one axis of the UI node.

* * *

### Remove custom rounding [#](#remove-custom-rounding)

`UiSurface::get_layout` now also returns the final sizes before rounding. Call `.0` on the `Ok` result to get the previously returned `taffy::Layout` value.

* * *

### Remove useless `Direction` field [#](#remove-useless-direction-field)

`Style` no longer has a `direction` field, and `Direction` has been deleted. They didn’t do anything, so you can delete any references to them as well.

* * *

### Rename BreakLineOn to LineBreak [#](#rename-breaklineon-to-linebreak)

`BreakLineOn` was renamed to `LineBreak`, and parameters named `linebreak_behavior` were renamed to `linebreak`.

* * *

### Replace `Handle<M: UiMaterial>` component with `UiMaterialHandle` wrapper [#](#replace-handle-m-uimaterial-component-with-uimaterialhandle-wrapper)

Let’s defer the migration guide to the required component port. I just want to yeet the `Component` impl on `Handle` in the meantime :)

* * *

### Simplified `ui_stack_system` [#](#simplified-ui-stack-system)

The `ZIndex` enum has been split into two separate components `ZIndex` (which replaces `ZIndex::Local`) and `GlobalZIndex` (which replaces `ZIndex::Global`). An entity can have both a `ZIndex` and `GlobalZIndex`, in comparisons `ZIndex` breaks ties if two `GlobalZindex` values are equal.

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


* * *

### Explicitly order `CameraUpdateSystem` before `UiSystem::Prepare` [#](#explicitly-order-cameraupdatesystem-before-uisystem-prepare)

`CameraUpdateSystem` is now explicitly ordered before `UiSystem::Prepare` instead of being ambiguous with it.

Utils [#](#utils)
-----------------

### Allow `bevy_utils` in `no_std` Contexts [#](#allow-bevy-utils-in-no-std-contexts)

If you were importing `bevy_utils` and setting `default_features` to `false`, but relying on elements which are now gated behind the `std` or `alloc` features, include the relevant feature in your `Cargo.toml`.

* * *

### Remove allocation in `get_short_name` [#](#remove-allocation-in-get-short-name)

**For `format!`, `dbg!`, `panic!`, etc.**

```
// Before
panic!("{} is too short!", get_short_name(name));

// After
panic!("{} is too short!", ShortName(name));

```


**Need a `String` Value**

```
// Before
let short: String = get_short_name(name);

// After
let short: String = ShortName(name).to_string();

```


* * *

### Remove remnant `EntityHash` and related types from `bevy_utils` [#](#remove-remnant-entityhash-and-related-types-from-bevy-utils)

*   Uses of `bevy::utils::{EntityHash, EntityHasher, EntityHashMap, EntityHashSet}` now have to be imported from `bevy::ecs::entity`.

* * *

### Remove unused type parameter in `Parallel::drain()` [#](#remove-unused-type-parameter-in-parallel-drain)

The type parameter of `Parallel::drain()` was unused, so it is now removed. If you were manually specifying it, you can remove the bounds.

```
// 0.14
// Create a `Parallel` and give it a value.
let mut parallel: Parallel<Vec<u8>> = Parallel::default();
*parallel.borrow_local_mut() = vec![1, 2, 3];

for v in parallel.drain::<u8>() {
    // ...
}

// 0.15
let mut parallel: Parallel<Vec<u8>> = Parallel::default();
*parallel.borrow_local_mut() = vec![1, 2, 3];

// Remove the type parameter.
for v in parallel.drain() {
    // ...
}

```


Windowing [#](#windowing)
-------------------------

### Add `bevy_window::Window` options for MacOS [#](#add-bevy-window-window-options-for-macos)

`bevy_window::Window` now has extra fields for configuring MacOS window settings:

```
    pub movable_by_window_background: bool,
    pub fullsize_content_view: bool,
    pub has_shadow: bool,
    pub titlebar_shown: bool,
    pub titlebar_transparent: bool,
    pub titlebar_show_title: bool,
    pub titlebar_show_buttons: bool,

```


Using `Window::default` keeps the same behaviour as before.

* * *

### Expose winit's `MonitorHandle` [#](#expose-winit-s-monitorhandle)

*   `WindowMode` variants now take a `MonitorSelection`, which can be set to `MonitorSelection::Primary` to mirror the old behavior.

* * *

### Remove unused `default` feature from `bevy_window` [#](#remove-unused-default-feature-from-bevy-window)

`bevy_window` had an empty default feature flag that did not do anything, so it was removed. You may have to remove any references to it if you specified it manually.

```
# 0.14
[dependencies]
bevy_window = { version = "0.14", default-features = false, features = ["default"] }

# 0.15
[dependencies]
bevy_window = { version = "0.15", default-features = false }

```


* * *

### move ANDROID\_APP to bevy\_window [#](#move-android-app-to-bevy-window)

If you use the `android_activity` reexport from `bevy::winit::android_activity`, it is now in `bevy::window::android_activity`. Same for the `ANDROID_APP` static

Without area [#](#without-area)
-------------------------------

### Add Display implementation to DebugName. [#](#add-display-implementation-to-debugname)

*   In code which uses DebugName you should now use the Display implementation rather than the Debug implementation (ie {} instead of {:?} if you were printing it out).