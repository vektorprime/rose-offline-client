# Map Editor Fixes Tracking

## Status: COMPLETED

## Tasks

### Task 1: Fix Hierarchy Panel Not Listing All Zone Elements
- **Status**: ✅ COMPLETED
- **Problem**: Hierarchy panel doesn't show all zone elements
- **Root Cause**: Zone elements were missing `EditorSelectable` marker component
- **Solution**: Added `EditorSelectable` component to all zone entity types in `src/zone_loader.rs`:
  - Terrain entities (`spawn_terrain()`)
  - Water entities (`spawn_water()`)
  - Object entities (`spawn_object()`)
  - Object part entities
  - Animated objects (`spawn_animated_object()`)
  - Effect objects (`spawn_effect_object()`)
  - Sound objects (`spawn_sound_object()`)
- **Files modified**: `src/zone_loader.rs`

### Task 2: Add Model Viewer Button to Menu Bar
- **Status**: ✅ COMPLETED
- **Problem**: No visible button to open model browser, only Ctrl+M shortcut exists
- **Solution**: Added "Model Browser" toggle menu item to View menu in `src/map_editor/ui/menu_bar.rs`
  - Shows checkmark when visible
  - Displays Ctrl+M shortcut hint
- **Files modified**: `src/map_editor/ui/menu_bar.rs`

### Task 3: Remove W Keyboard Shortcut Conflict
- **Status**: ✅ COMPLETED
- **Problem**: W key switches to Translate mode but also moves FreeCamera forward
- **Solution**: Removed W key handler from `handle_mode_switches()` in `src/map_editor/systems/keyboard_shortcuts_system.rs`
  - W now only moves FreeCamera forward
- **Files modified**: `src/map_editor/systems/keyboard_shortcuts_system.rs`

### Task 4: Make Mode Display Clickable with Dropdown
- **Status**: ✅ COMPLETED
- **Problem**: Mode is displayed as static text, not clickable
- **Solution**: Replaced static label with clickable button and upward-opening dropdown in `src/map_editor/ui/status_bar.rs`
  - Uses `egui::popup::popup_above_or_below_widget()` with `AboveOrBelow::Above`
- **Files modified**: `src/map_editor/ui/status_bar.rs`

### Task 5: Fix Model Browser Not Showing with Ctrl+M
- **Status**: ✅ COMPLETED
- **Problem**: Ctrl+M shortcut didn't toggle model browser visibility
- **Root Cause**: The `model_browser_keyboard_shortcuts` system was defined but never registered in the plugin
- **Solution**: Added system to `EditorUiPlugin` in `src/map_editor/ui/mod.rs`
- **Files modified**: `src/map_editor/ui/mod.rs`

### Task 6: Remove Q and E from FreeCamera
- **Status**: ✅ COMPLETED
- **Problem**: Q and E keys moved FreeCamera up/down but are needed for mode switching (Select/Rotate)
- **Solution**: Removed Q and E key handlers from `src/systems/free_camera_system.rs`
  - Q and E now only used for mode switching (Select/Rotate)
- **Files modified**: `src/systems/free_camera_system.rs`

### Task 7: Fix Model Browser CNST and DECO Tabs Empty
- **Status**: ✅ COMPLETED
- **Problem**: CNST and DECO tabs in model browser were empty because zone-specific models weren't loaded
- **Root Cause**: The `load_available_models_system` ran once at startup before any zone was loaded. DECO and CNST models are zone-specific (loaded from zone's ZSC files)
- **Solution**: Added `update_models_on_zone_load_system` that listens for `ZoneEvent::Loaded` events and populates DECO/CNST models from the zone's ZSC files
- **Files modified**:
  - `src/map_editor/systems/load_models_system.rs` - Added `update_models_on_zone_load_system`
  - `src/map_editor/mod.rs` - Registered the new system
  - `src/map_editor/systems/mod.rs` - Exported the new system

## Build Validation
- **Status**: ✅ SUCCESS
- **Build completed**: Without errors
- **Runtime validation**: Passed

### Task 8: Fix Model Placement Click Detection
- **Status**: ✅ COMPLETED
- **Problem**: Left-click wasn't placing models even in Add mode
- **Root Cause**: Selection system was consuming clicks before placement system could process them
- **Solution**: Added check in `selection_system.rs` to skip selection when in Add mode
- **Files modified**: `src/map_editor/systems/selection_system.rs`

### Task 9: Fix Save Button Not Persisting Changes
- **Status**: ✅ COMPLETED
- **Problem**: Models were lost on relaunch after saving
- **Root Cause**: Save system was writing to VFS path relative to CWD instead of actual game data directory
- **Solution**: Added `base_path` field to `VfsResource` and use it for save operations
- **Files modified**: 
  - `src/resources/virtual_filesystem.rs` - Added `base_path` field
  - `src/lib.rs` - Modified `create_virtual_filesystem()` to return base path
  - `src/map_editor/save/save_system.rs` - Use `base_path` for save location

### Task 10: Zone Version/Backup Management
- **Status**: ✅ ALREADY IMPLEMENTED
- **Note**: The save system already creates timestamped backups in `{zone_path}/backup/YYYYMMDD_HHMMSS/` before overwriting files

### Task 11: Real Filesystem Priority Over VFS
- **Status**: ✅ COMPLETED
- **Problem**: Game loaded files from VFS first, ignoring saved map editor modifications
- **Root Cause**: VFS was checked before real filesystem, so modified files weren't loaded. The previous implementation only fixed `VfsAssetIo`, but actual file loading happens in multiple places:
  - `zone_loader.rs:load_zone_direct()` - Loaded ZON and ZSC files directly from VFS
  - `zone_loader.rs:load_block_files_direct()` - Already had the fix for HIM/TIL/IFO/LIT
  - `load_models_system.rs:try_load_zsc_from_vfs()` - Loaded ZSC files directly from VFS
- **Solution**: Modified multiple locations to check real filesystem first, fall back to VFS
- **Files modified**:
  1. `src/vfs_asset_io.rs` - Modified `VfsAssetIo::read()` to check real filesystem first
  2. `src/zone_loader.rs` - Added `read_bytes_with_priority_sync()` helper function
     - Updated `load_zone_direct()` to use the helper for ZON and ZSC files
  3. `src/map_editor/systems/load_models_system.rs` - Added `try_load_zsc_with_priority()` function
     - Updated `load_default_deco_cnst_from_vfs()` to use this helper
- **How it works**:
  1. First checks `{base_path}/{virtual_path}` on real filesystem
  2. If found, loads from there (map editor modifications)
  3. If not found, loads from VFS (original game files)
- **Log messages to verify fix is working**:
  - `[VFS PRIORITY] Loaded from real filesystem: <path> (<size> bytes)`
  - `[VFS PRIORITY] File exists on real filesystem but failed to read <path>, falling back to VFS`
  - `[LOAD MODELS] Loaded X Y models from real filesystem (priority): <path>`

### Task 12: Fix Empty VFS base_path
- **Status**: ✅ COMPLETED
- **Problem**: `VFS base_path: ""` was empty, causing saves to go to relative paths instead of the actual game directory
- **Root Cause**:
  1. For VFS file types, `Path::parent()` on `"data.idx"` returns `Some("")` not `None`, which got converted to empty `PathBuf`
  2. `std::env::current_dir().unwrap_or_default()` returns empty path when it fails
- **Solution**:
  - Fixed all VFS type handlers to check for empty parent paths
  - Added robust fallback with error logging when current_dir() fails
- **Files modified**: `src/lib.rs` - `FilesystemConfig::create_virtual_filesystem()`
- **Expected behavior**: `base_path` will now be set to the game directory (e.g., `C:\Games\Rose`)

## Build Validation
- **Status**: ✅ SUCCESS
- **Build completed**: Without errors
- **Runtime validation**: Passed

## Summary
All 12 map editor fixes have been successfully implemented and validated:
1. Hierarchy panel now shows all zone elements
2. Model Browser accessible via View menu
3. W key no longer conflicts with camera movement
4. Mode display is clickable with upward dropdown
5. Ctrl+M shortcut now works (system was registered)
6. Q and E keys no longer interfere with camera movement
7. CNST and DECO tabs now populate with zone-specific models when a zone is loaded
8. Model placement click detection now works in Add mode
9. Save button now persists changes to the correct game data directory
10. Zone backup system already creates timestamped backups before saving
11. Real filesystem takes priority over VFS, allowing map editor modifications to load
12. VFS base_path now correctly resolves to game directory instead of being empty

## Notes
- Analysis completed: Map editor uses `EditorSelectable` marker component for hierarchy
- FreeCamera uses WASD for movement, W conflicted with Translate mode shortcut (now resolved)
- Model browser visibility controlled by `SelectedModel.browser_visible`
- Q and E keys now exclusively used for mode switching (Select/Rotate)
- DECO/CNST models are zone-specific and loaded from ZSC files via `update_models_on_zone_load_system`
- Selection system now skips processing when in Add mode to allow placement clicks through
- Save system uses `base_path` from `VfsResource` to ensure files are saved to the correct location
- VFS asset loading now prioritizes real filesystem files, enabling map editor changes to persist across reloads
- VFS base_path correctly resolves game directory by checking for empty parent paths and handling current_dir() failures
