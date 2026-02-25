# Map Editor and Zone Viewer Implementation Analysis

## Overview

This document analyzes the current implementations of both the `--map-editor` and `--zone-viewer` features in the Rose Online client, identifying differences and providing recommendations for fixes.

---

## 1. Map Editor Architecture Summary

### Location: [`src/map_editor/`](src/map_editor/)

### Plugin Structure

The map editor is organized as a Bevy plugin with the following submodules:

| Module | Purpose |
|--------|---------|
| [`mod.rs`](src/map_editor/mod.rs:98-131) | Main `MapEditorPlugin` registration |
| [`components.rs`](src/map_editor/components.rs) | Editor marker components |
| [`resources.rs`](src/map_editor/resources.rs) | State management resources |
| [`systems/`](src/map_editor/systems/) | Selection, grid, gizmo, keyboard shortcuts |
| [`ui/`](src/map_editor/ui/) | egui panels |
| [`save/`](src/map_editor/save/) | Zone export functionality |

### Key Components

- `SelectedInEditor` - Marks selected entities
- `EditorGizmo` - Transform manipulation handles
- `EditorGrid` - Visual grid
- `EditorSelectable` - Marks entities that can be selected
- `EditorPreview` - Preview entities during placement

### Key Resources

- `MapEditorState` - Main editor state (selection, mode, undo/redo)
- `EditorGridSettings` - Grid configuration
- `SelectedModel` - Currently selected model for placement
- `AvailableModels` - Model database organized by category

### Enter System

```rust
// src/map_editor/mod.rs:138-158
pub fn map_editor_enter_system(
    mut commands: Commands,
    mut map_editor_state: ResMut<MapEditorState>,
    query_cameras: Query<Entity, With<Camera3d>>,
) {
    map_editor_state.enabled = true;
    
    let camera_position = Vec3::new(5120.0, 50.0, -5120.0);
    let camera_yaw: f32 = -45.0;
    let camera_pitch: f32 = -20.0;
    
    for entity in query_cameras.iter() {
        commands.entity(entity)
            .remove::<CameraAnimation>()
            .insert(FreeCamera::new(camera_position, camera_yaw, camera_pitch));
    }
}
```

### Registration

```rust
// src/lib.rs:1162-1163
app.add_systems(OnEnter(AppState::MapEditor), map_editor::map_editor_enter_system);
app.add_systems(OnExit(AppState::MapEditor), map_editor::map_editor_exit_system);
```

---

## 2. Zone Viewer Architecture Summary

### Location: [`src/systems/zone_viewer_system.rs`](src/systems/zone_viewer_system.rs)

### Enter System

```rust
// src/systems/zone_viewer_system.rs:13-68
pub fn zone_viewer_enter_system(
    mut commands: Commands,
    query_cameras: Query<Entity, With<Camera3d>>,
    mut ui_state_debug_windows: ResMut<UiStateDebugWindows>,
) {
    let camera_position = Vec3::new(5120.0, 50.0, -5120.0);
    let camera_yaw: f32 = -45.0;
    let camera_pitch: f32 = -20.0;

    for entity in query_cameras.iter() {
        commands
            .entity(entity)
            .remove::<OrbitCamera>()
            .remove::<CameraAnimation>()
            .insert(FreeCamera::new(
                camera_position,
                camera_yaw,
                camera_pitch,
            ));
    }

    // KEY DIFFERENCE: Opens debug windows automatically
    ui_state_debug_windows.camera_info_open = true;
    ui_state_debug_windows.debug_ui_open = true;
    ui_state_debug_windows.zone_list_open = true;
}
```

### Registration

```rust
// src/lib.rs:1159
app.add_systems(OnEnter(AppState::ZoneViewer), zone_viewer_enter_system);
```

---

## 3. Free Camera System Analysis

### Location: [`src/systems/free_camera_system.rs`](src/systems/free_camera_system.rs)

The `FreeCamera` component and system are shared between both modes:

```rust
// src/systems/free_camera_system.rs:15-40
#[derive(Component)]
pub struct FreeCamera {
    pub rig: CameraRig<LeftHanded>,
    pub move_speed: f32,
    pub drag_speed: f32,
}

impl FreeCamera {
    pub fn new(position: Vec3, yaw_degrees: f32, pitch_degrees: f32) -> Self {
        // Creates camera rig with Position, YawPitch, and Smooth drivers
    }
}
```

### System Registration

```rust
// src/lib.rs:921-926
app.add_systems(
    Update,
    (free_camera_system, orbit_camera_system)
        .in_set(GameSystemSets::UpdateCamera)
        .after(bevy_egui::EguiPreUpdateSet::InitContexts),
);
```

**Important**: The `free_camera_system` runs globally on every frame, not conditionally based on app state. It will work for any entity with a `FreeCamera` component.

---

## 4. Key Differences Between Implementations

### 4.1 Camera Component Removal

| Feature | Zone Viewer | Map Editor |
|---------|-------------|------------|
| Removes `OrbitCamera` | ✅ Yes | ❌ No |
| Removes `CameraAnimation` | ✅ Yes | ✅ Yes |
| Inserts `FreeCamera` | ✅ Yes | ✅ Yes |

**Issue**: Map Editor does not remove `OrbitCamera`. If the camera has both `OrbitCamera` and `FreeCamera`, the systems may conflict.

### 4.2 Debug Windows / UI State

| Feature | Zone Viewer | Map Editor |
|---------|-------------|------------|
| Opens camera info | ✅ Auto | ❌ No |
| Opens debug UI | ✅ Auto | ❌ No |
| Opens zone list | ✅ Auto | ⚠️ Manual |

**Issue**: Map Editor has a zone list panel but it's not opened by default. Users must manually open it from the menu.

### 4.3 Zone List Implementation

**Zone Viewer** uses [`ui_debug_zone_list_system.rs`](src/ui/ui_debug_zone_list_system.rs):
- Part of debug windows system
- Has "Despawn other zones" checkbox
- Shows "Load" button for ZoneViewer mode

**Map Editor** uses [`zone_list_panel.rs`](src/map_editor/ui/zone_list_panel.rs):
- Separate panel system
- No "Despawn other zones" option
- Shows "Load" button to switch zones
- Based on zone viewer's implementation but simplified

### 4.4 Hierarchy Panel

**Current State**: The hierarchy panel in [`hierarchy_panel.rs`](src/map_editor/ui/hierarchy_panel.rs:52-85) shows **placeholder content**:

```rust
// Placeholder content - in a full implementation this would show
// actual zone objects from a query
ui.collapsing("Zone 1", |ui| {
    ui.collapsing("Block 0_0", |ui| {
        object_list_item(ui, "Deco Object 1", false);
        object_list_item(ui, "Deco Object 2", false);
        // ...
    });
});
```

**Issue**: The hierarchy panel is not connected to actual zone entities.

---

## 5. Root Cause Analysis

### 5.1 Free Camera Not Working

**Root Cause**: The map editor's `map_editor_enter_system` does NOT remove `OrbitCamera` component before adding `FreeCamera`. If both components exist on the camera, both systems will try to control the camera transform, causing conflicts.

**Evidence**:
- Zone viewer removes `OrbitCamera`: Line 44 in [`zone_viewer_system.rs`](src/systems/zone_viewer_system.rs:44)
- Map editor does NOT remove it: Lines 152-154 in [`mod.rs`](src/map_editor/mod.rs:152-154)

### 5.2 Zone List Not Visible by Default

**Root Cause**: The map editor's enter system does not open the zone list panel. The `ZoneListPanelState` resource has `is_open: false` by default.

**Evidence**:
- [`zone_list_panel.rs`](src/map_editor/ui/zone_list_panel.rs:27) - `is_open: bool` defaults to `false`
- No code in `map_editor_enter_system` sets it to `true`

### 5.3 Hierarchy Panel Shows Placeholder Data

**Root Cause**: The hierarchy panel UI code uses hardcoded placeholder content instead of querying actual `ZoneObject` entities.

**Evidence**:
- [`hierarchy_panel.rs`](src/map_editor/ui/hierarchy_panel.rs:52-85) - Contains static placeholder content
- No entity query is passed to `editor_hierarchy_panel()`

---

## 6. Recommendations

### 6.1 Fix Free Camera in Map Editor

**File**: [`src/map_editor/mod.rs`](src/map_editor/mod.rs:152-154)

**Change**: Add `OrbitCamera` removal in `map_editor_enter_system`:

```rust
// Before:
for entity in query_cameras.iter() {
    commands.entity(entity)
        .remove::<CameraAnimation>()
        .insert(FreeCamera::new(camera_position, camera_yaw, camera_pitch));
}

// After:
for entity in query_cameras.iter() {
    commands.entity(entity)
        .remove::<OrbitCamera>()      // ADD THIS LINE
        .remove::<CameraAnimation>()
        .insert(FreeCamera::new(camera_position, camera_yaw, camera_pitch));
}
```

### 6.2 Open Zone List Panel by Default

**File**: [`src/map_editor/mod.rs`](src/map_editor/mod.rs:138)

**Change**: Add `ZoneListPanelState` parameter and set `is_open = true`:

```rust
pub fn map_editor_enter_system(
    mut commands: Commands,
    mut map_editor_state: ResMut<MapEditorState>,
    mut zone_list_state: ResMut<ZoneListPanelState>,  // ADD
    query_cameras: Query<Entity, With<Camera3d>>,
) {
    map_editor_state.enabled = true;
    zone_list_state.is_open = true;  // ADD - Open zone list by default
    
    // ... rest of function
}
```

**Also update imports**:
```rust
use crate::map_editor::ui::zone_list_panel::ZoneListPanelState;
```

### 6.3 Connect Hierarchy Panel to Zone Entities

**File**: [`src/map_editor/ui/hierarchy_panel.rs`](src/map_editor/ui/hierarchy_panel.rs)

**Change**: Pass entity query to the panel function:

```rust
// Updated function signature
pub fn editor_hierarchy_panel(
    ctx: &egui::Context,
    map_editor_state: &MapEditorState,
    zone_objects: &Query<(Entity, &Name, &Transform, Option<&ZoneObject>), With<EditorSelectable>>,
    deco_objects: &Query<(Entity, &Name), With<DecoObject>>,
    cnst_objects: &Query<(Entity, &Name), With<CnstObject>>,
    event_objects: &Query<(Entity, &Name), With<EventObject>>,
) {
    // Query actual entities instead of showing placeholder content
}
```

### 6.4 Add Despawn Option to Map Editor Zone List

**File**: [`src/map_editor/ui/zone_list_panel.rs`](src/map_editor/ui/zone_list_panel.rs)

**Change**: Add "Despawn other zones" checkbox like zone viewer has:

```rust
// In ZoneListPanelState struct, add:
pub despawn_other_zones: bool,

// In editor_zone_list_panel function, add checkbox:
ui.horizontal(|ui| {
    ui.label("Despawn other zones:");
    ui.checkbox(&mut state.despawn_other_zones, "Despawn");
});

// Update LoadZoneEvent to include despawn flag:
load_zone_events.write(LoadZoneEvent {
    id: zone_data.id,
    despawn_other_zones: state.despawn_other_zones,
});
```

---

## 7. Code Snippets Reference

### Zone Viewer Free Camera Setup (Working)

```rust
// src/systems/zone_viewer_system.rs:41-52
for entity in query_cameras.iter() {
    commands
        .entity(entity)
        .remove::<OrbitCamera>()           // <-- KEY: Removes OrbitCamera
        .remove::<CameraAnimation>()
        .insert(FreeCamera::new(
            camera_position,
            camera_yaw,
            camera_pitch,
        ));
}
```

### Map Editor Free Camera Setup (Missing OrbitCamera Removal)

```rust
// src/map_editor/mod.rs:151-155
for entity in query_cameras.iter() {
    commands.entity(entity)
        .remove::<CameraAnimation>()        // <-- Missing: .remove::<OrbitCamera>()
        .insert(FreeCamera::new(camera_position, camera_yaw, camera_pitch));
}
```

### Zone Viewer Zone List with Despawn Option

```rust
// src/ui/ui_debug_zone_list_system.rs:61-66
if matches!(app_state.get(), AppState::ZoneViewer) {
    ui.label("Despawn other zones:");
    ui.checkbox(&mut ui_state.despawn_other_zones, "Despawn");
    ui.end_row();
}

// src/ui/ui_debug_zone_list_system.rs:144-149
AppState::ZoneViewer => {
    if ui.button("Load").clicked() {
        load_zone_events.write(LoadZoneEvent {
            id: zone_data.id,
            despawn_other_zones: ui_state.despawn_other_zones,
        });
    }
}
```

---

## 8. Summary

| Issue | Root Cause | Fix Location | Priority |
|-------|------------|--------------|----------|
| Free camera not working | Missing `OrbitCamera` removal | [`mod.rs:152`](src/map_editor/mod.rs:152) | High |
| Zone list not visible | `is_open` defaults to false | [`mod.rs:138`](src/map_editor/mod.rs:138) | Medium |
| Hierarchy shows placeholder | No entity query connected | [`hierarchy_panel.rs`](src/map_editor/ui/hierarchy_panel.rs) | Medium |
| No despawn option | Feature not implemented | [`zone_list_panel.rs`](src/map_editor/ui/zone_list_panel.rs) | Low |

The most critical fix is adding `OrbitCamera` removal to the map editor's enter system, which should immediately restore free camera functionality.
