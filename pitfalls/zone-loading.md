# Zone Loading Pitfalls

This document records zone loading-related issues encountered during development.

---

## Crash on Startup: `SpawnZoneParams::zone_loader_assets` Resource Does Not Exist (Fixed 2026-07-31)

### Problem
After a code-simplification cleanup pass, the game panicked on startup with:

```
Encountered an error in system `rose_offline_client::zone_loader::systems::zone_loader_system`:
Parameter `SpawnZoneParams<'_, '_>::zone_loader_assets` failed validation: Resource does not exist
```

### Root Cause
The cleanup removed the legacy `ZoneLoader` AssetServer path (the `AssetLoader` impl and `load_zone`), keeping only the direct VFS path (`load_zone_direct` → renamed `load_zone`). Along with the `AssetLoader`, the `app.init_asset::<zone_loader::ZoneLoaderAsset>()` call was removed from `lib.rs`. But `init_asset::<T>()` was the thing that implicitly created the `Assets<T>` resource — and the kept direct path still uses it:

- `systems.rs:51` — `spawn_zone_params.zone_loader_assets.add(zone_asset)`
- `systems.rs:526` — `zone_loader_assets.remove(&zone_handle)`
- `systems.rs:538` — `zone_loader_assets.get(&current_zone.handle)`

So `SpawnZoneParams::zone_loader_assets` (a `ResMut<Assets<ZoneLoaderAsset>>`) failed validation at schedule time.

### Solution
Re-add the resource initialization in `src/lib.rs` (near the zone load channel registration):

```rust
app.init_resource::<Assets<ZoneLoaderAsset>>();
```

and import `ZoneLoaderAsset` in the `use zone_loader::{...}` block.

### Lesson Learned
When deleting a dead `AssetLoader`/asset path, remember that `init_asset::<T>()` (or `register_asset_loader` + `init_asset`) also initializes the `Assets<T>` resource. If a live system still holds `Res<Assets<T>>`/`ResMut<Assets<T>>` (or a `SystemParam` containing one), the resource must be initialized via `init_resource::<Assets<T>>()` — otherwise the system fails validation at runtime with "Resource does not exist", even though the crate compiles fine.

---

## Login Screen Sky Intermittently Wrong Colors or Missing (Fixed 2026-02-24)

### Problem
On the login screen, the sky would sometimes appear with strange colors or not load at all. This was an intermittent issue that didn't happen on every launch, suggesting a race condition.

### Root Cause
The [`spawn_skybox()`](src/zone_loader.rs:2182) function was loading skybox assets (mesh and textures) via `asset_server.load()` but **was not tracking these handles** in `zone_loading_assets`. This meant:

1. The zone could be marked as "loaded" before skybox assets were actually ready
2. When rendering started, the skybox texture might not be loaded yet, causing:
  - Strange colors (rendering with unloaded/fallback texture data)
  - Missing sky (mesh not loaded yet)

The zone loader system waits for all assets in `zone_loading_assets` to reach `LoadState::Loaded` before sending `ZoneEvent::Loaded`. Without the skybox assets in this list, the zone was considered ready prematurely.

### Solution
Modified [`spawn_skybox()`](src/zone_loader.rs:2182) to return both the entity AND a list of asset handles that need to be tracked:

```rust
// Before (broken - skybox assets not tracked):
fn spawn_skybox(...) -> Entity {
    let mesh_handle = asset_server.load::<Mesh>(&mesh_path);
    let texture_handle = asset_server.load::<Image>(&texture_day_path);
    // ... spawn entity ...
    entity
}

// After (fixed - skybox assets tracked):
fn spawn_skybox(...) -> (Entity, Vec<UntypedHandle>) {
    let mut loading_assets: Vec<UntypedHandle> = Vec::new();
    
    let mesh_handle = asset_server.load::<Mesh>(&mesh_path);
    loading_assets.push(mesh_handle.clone().untyped());  // Track mesh!
    
    let texture_handle = asset_server.load::<Image>(&texture_day_path);
    loading_assets.push(texture_handle.clone().untyped());  // Track texture!
    
    // ... spawn entity ...
    (entity, loading_assets)
}
```

Then in [`spawn_zone()`](src/zone_loader.rs:1930), the skybox assets are added to `zone_loading_assets`:

```rust
let (skybox_entity, skybox_assets) = spawn_skybox(...);
zone_loading_assets.extend(skybox_assets);  // CRITICAL: Wait for skybox to load!
```

### Files Modified
- `src/zone_loader.rs` - `spawn_skybox()` now returns `(Entity, Vec<UntypedHandle>)` and tracks all loaded assets
- `src/zone_loader.rs` - `spawn_zone()` adds skybox assets to `zone_loading_assets`

### Lesson Learned
When spawning entities with asynchronously loaded assets (via `asset_server.load()`):
1. **Track all asset handles** - Any asset loaded via `asset_server.load()` should have its handle added to a tracking list if other systems need to wait for it
2. **Use `UntypedHandle` for heterogeneous collections** - When tracking multiple asset types (Mesh, Image, etc.), use `.untyped()` to convert to `UntypedHandle`
3. **Zone readiness depends on asset tracking** - The zone loader only waits for assets in `zone_loading_assets`; untracked assets can cause race conditions
4. **Intermittent bugs often indicate timing issues** - If a bug doesn't happen every time, suspect a race condition in asset loading or initialization order

---

## `--zone-viewer` Flag Shows Login Screen Instead (Fixed 2026-02-24)

### Problem
Running the game with `--zone-viewer` flag would show the login screen instead of the zone viewer. No egui menus for changing or viewing the zone appeared.

### Root Cause
In [`lib.rs:877`](src/lib.rs:877), the code used `init_state::<AppState>()` which **always initializes to the default value** (`GameLogin`), completely ignoring the `app_state` parameter passed to `run_client()`.

```rust
// BROKEN CODE - app_state parameter is ignored!
fn run_client(config: &Config, app_state: AppState, ...) {
    // ...
    app.init_state::<AppState>();  // Always uses AppState::GameLogin (the #[default] value)
}
```

The old working code (Bevy 0.11) used:
```rust
app.add_state::<AppState>()
    .insert_resource(State::new(app_state));  // Properly set initial state
```

### Solution
In Bevy 0.16, use `insert_state()` instead of `init_state()` to set a specific initial state:

```rust
// FIXED CODE - app_state parameter is used
fn run_client(config: &Config, app_state: AppState, ...) {
    // ...
    app.insert_state(app_state);  // Properly sets the initial state
}
```

### Files Modified
- `src/lib.rs` (line 877) - Changed `init_state::<AppState>()` to `insert_state(app_state)`

### Lesson Learned
In Bevy 0.16:
- `init_state::<S>()` - Initializes state to `S::default()` (requires `FromWorld` trait)
- `insert_state::<S>(value)` - Initializes state to a specific value (does NOT require `FromWorld` trait)

When you need to set a specific initial state value, use `insert_state()`, not `init_state()`.
