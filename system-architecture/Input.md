# Bevy Input Documentation for rose-offline-client

This document provides comprehensive documentation for Bevy Input features used in the rose-offline-client project, based on Bevy 0.18.1.

## Table of Contents

1. [Overview](#overview)
2. [Bevy API References](#core-input-types)
3. [Custom Extensions](#input-integration-with-egui)
4. [Code Examples](#code-examples)
5. [Configuration Options](#key-bindings)
6. [Common Patterns](#common-pitfalls)
7. [Troubleshooting](#troubleshooting)
8. [Source File References](#source-file-references)

---

## Overview

The rose-offline-client uses Bevy's input system to handle keyboard, mouse, and UI input. The input architecture follows Bevy's event-driven model with both event-based and polling-based approaches.

### Input Flow

```
winit Events → Bevy Input Events → ButtonInput Resources → Game Systems
                    ↓
              egui Input Check
                    ↓
              Game/UI Input Routing
```

### Key Components

- **Events**: `KeyboardInput`, `MouseButtonInput`, `MouseMotion`, `MouseWheel`
- **Resources**: `ButtonInput<KeyCode>`, `ButtonInput<Key>`, `ButtonInput<MouseButton>`
- **Accumulated Resources**: `AccumulatedMouseMotion`, `AccumulatedMouseScroll`

---

## Bevy API References

### Core Input Types

### ElementState

Represents the press state of an input element.

```rust
pub enum ElementState {
    Pressed,
    Released,
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/lib.rs:179-191`

### ButtonState

Bevy's enum for button press states, used in input events.

```rust
pub enum ButtonState {
    /// The button is pressed.
    Pressed,
    /// The button is not pressed.
    Released,
}

impl ButtonState {
    /// Is this button pressed?
    pub fn is_pressed(&self) -> bool {
        matches!(self, ButtonState::Pressed)
    }
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/lib.rs:167-191`

---

## Keyboard Input

### KeyboardInput Event

The primary keyboard input event that captures key presses and releases.

```rust
pub struct KeyboardInput {
    /// The physical key code of the key (location-based)
    pub key_code: KeyCode,
    /// The logical key of the input (layout-aware)
    pub logical_key: Key,
    /// The press state of the key
    pub state: ButtonState,
    /// Contains the text produced by this keypress
    pub text: Option<SmolStr>,
    /// True if this event is from key repeat
    pub repeat: bool,
    /// Window that received the input
    pub window: Entity,
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/keyboard.rs:88-139`

### KeyCode vs Key

**KeyCode**: Physical key location (e.g., `KeyCode::KeyW` is the key in the W position regardless of layout)

**Key**: Logical key value (e.g., `Key::Character("w")` accounts for keyboard layout)

Use `KeyCode` for game controls (WASD works on QWERTY, AZERTY, etc.)
Use `Key` for text input and symbols (+, -, etc.)

### Common KeyCode Variants

```rust
// Movement
KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD
KeyCode::ArrowUp, KeyCode::ArrowLeft, KeyCode::ArrowDown, KeyCode::ArrowRight

// Modifiers
KeyCode::ShiftLeft, KeyCode::ShiftRight
KeyCode::ControlLeft, KeyCode::ControlRight
KeyCode::AltLeft, KeyCode::AltRight
KeyCode::SuperLeft, KeyCode::SuperRight  // Windows/Command key

// Function Keys
KeyCode::F1 through KeyCode::F35

// Special Keys
KeyCode::Space, KeyCode::Enter, KeyCode::Escape, KeyCode::Tab
KeyCode::Backspace, KeyCode::Delete, KeyCode::Insert
KeyCode::PageUp, KeyCode::PageDown, KeyCode::Home, KeyCode::End
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/keyboard.rs:269-737`

---

## Mouse Input

### MouseButtonInput Event

Represents mouse button press/release events.

```rust
pub struct MouseButtonInput {
    /// The mouse button assigned to the event
    pub button: MouseButton,
    /// The pressed state of the button
    pub state: ButtonState,
    /// Window that received the input
    pub window: Entity,
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/mouse.rs:29-47`

### MouseButton Enum

```rust
pub enum MouseButton {
    /// The left mouse button
    Left,
    /// The right mouse button
    Right,
    /// The middle mouse button
    Middle,
    /// The back mouse button
    Back,
    /// The forward mouse button
    Forward,
    /// Another mouse button with the associated number
    Other(u16),
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/mouse.rs:59-83`

### MouseMotion Event

Raw mouse movement delta since the last event.

```rust
pub struct MouseMotion {
    /// The change in the position of the pointing device since the last event
    pub delta: Vec2,
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/mouse.rs:94-108`

### MouseWheel Event

Mouse scroll wheel input.

```rust
pub struct MouseWheel {
    /// The mouse scroll unit (Line or Pixel)
    pub unit: MouseScrollUnit,
    /// The horizontal scroll value
    pub x: f32,
    /// The vertical scroll value
    pub y: f32,
    /// Window that received the input
    pub window: Entity,
}

pub enum MouseScrollUnit {
    /// Delta corresponds to amount of lines/rows
    Line,
    /// Delta corresponds to amount of pixels
    Pixel,
}

impl MouseScrollUnit {
    /// Conversion factor between Line and Pixel units
    pub const SCROLL_UNIT_CONVERSION_FACTOR: f32 = 100.;
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/mouse.rs:116-175`

### Accumulated Mouse Resources

These resources accumulate input across all events in a frame:

```rust
/// Tracks total mouse movement per frame
pub struct AccumulatedMouseMotion {
    pub delta: Vec2,
}

/// Tracks total mouse scroll per frame
pub struct AccumulatedMouseScroll {
    pub unit: MouseScrollUnit,
    pub delta: Vec2,
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/mouse.rs:196-249`

---

## Button State Polling

### ButtonInput Resource

Generic resource for polling button states. Provides convenient methods for checking input state.

```rust
pub struct ButtonInput<T: Clone + Eq + Hash + Send + Sync + 'static> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/button_input.rs:123-133`

### ButtonInput Methods

| Method | Description |
|--------|-------------|
| `pressed(input)` | True if button is currently held down |
| `just_pressed(input)` | True for one frame after press |
| `just_released(input)` | True for one frame after release |
| `any_pressed(inputs)` | True if any of the inputs are pressed |
| `all_pressed(inputs)` | True if all inputs are pressed |
| `any_just_pressed(inputs)` | True if any input was just pressed |
| `all_just_pressed(inputs)` | True if all inputs were just pressed |
| `get_pressed()` | Iterator over all pressed inputs |
| `get_just_pressed()` | Iterator over all just-pressed inputs |
| `get_just_released()` | Iterator over all just-released inputs |

**Source**: `bevy-0.18.1/crates/bevy_input/src/button_input.rs:144-275`

### Usage Example

```rust
fn example_system(keyboard: Res<ButtonInput<KeyCode>>) {
    // Check if W is currently held
    if keyboard.pressed(KeyCode::KeyW) {
        // Moving forward
    }
    
    // Check if W was just pressed this frame
    if keyboard.just_pressed(KeyCode::KeyW) {
        // Start movement animation
    }
    
    // Check if any modifier is held
    if keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        // Ctrl is held
    }
    
    // Get all currently pressed keys
    for key in keyboard.get_pressed() {
        println!("Key pressed: {:?}", key);
    }
}
```

---

## Custom Extensions

### Input Integration with egui

### egui Input Priority

egui has priority over game input when UI elements are active. The client checks egui's input desires before processing game input.

### egui Input Query Methods

```rust
// Check if egui wants keyboard input
egui_ctx.ctx_mut().unwrap().wants_keyboard_input()

// Check if egui wants pointer (mouse) input
egui_ctx.ctx_mut().unwrap().wants_pointer_input()

// Check if egui wants any pointer input
egui_wants_any_pointer_input()
```

### Input Guard Pattern

All game input systems should check egui input desires first:

```rust
pub fn game_input_system(
    egui_ctx: EguiContexts,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    // ... other params
) {
    // Return early if egui wants keyboard input
    if egui_ctx.ctx_mut().unwrap().wants_keyboard_input() {
        return;
    }
    
    // Process game keyboard input
    if keyboard_input.pressed(KeyCode::KeyW) {
        // ...
    }
}
```

**Example in codebase**: `src/systems/game_keyboard_input_system.rs:40-43`

---

## Input Routing Based on AppState

### AppState Enum

```rust
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

**Source**: `src/resources/app_state.rs:3-11`

### Input Routing Strategy

Game input systems check AppState before processing input:

```rust
pub fn game_keyboard_input_system(
    app_state: Res<State<AppState>>,
    // ...
) {
    // Only process input in Game state
    if *app_state.get() != AppState::Game {
        return;
    }
    
    // ... process input
}
```

**Examples**:
- `src/systems/game_keyboard_input_system.rs:37-39`
- `src/systems/game_mouse_input_system.rs:55-57`

### State-Specific Input

| State | Input Systems Active |
|-------|---------------------|
| GameLogin | UI input only (login form) |
| GameCharacterSelect | UI input (character buttons), camera rotation |
| Game | Full game input (WASD movement, mouse clicks, hotbar) |
| MapEditor | Editor-specific shortcuts, gizmo interaction |
| ModelViewer | Camera controls only |
| ZoneViewer | Camera controls only |

---

## Configuration Options

### Key Bindings

### Game Controls (AppState::Game)

| Action | Key(s) | System |
|--------|--------|--------|
| Move Forward | W | `game_keyboard_input_system` |
| Move Backward | S | `game_keyboard_input_system` |
| Move Left | A | `game_keyboard_input_system` |
| Move Right | D | `game_keyboard_input_system` |
| Thrust (Flight) | Space | `flight_movement_system` |
| Hotbar Slot 1-8 | F1-F8 | `ui_hotbar_system` |
| Minimap Toggle | Alt+M | `ui_minimap_system` |
| Admin Menu | F10 | `ui_admin_menu_system` |
| Debug Physics | P | `debug_inspector_system` |
| Debug Window | Ctrl+D | `ui_debug_window_system` |

### Mouse Controls (AppState::Game)

| Action | Button | System |
|--------|--------|--------|
| Move to Location | Left Click (terrain) | `game_mouse_input_system` |
| Attack Target | Left Click (enemy) | `game_mouse_input_system` |
| Select Target | Left Click (friendly) | `game_mouse_input_system` |
| Pickup Item | Left Click (item drop) | `game_mouse_input_system` |
| Rotate Camera | Right Click + Drag | `orbit_camera_system` |
| Zoom Camera | Mouse Wheel | `orbit_camera_system` |

### Map Editor Shortcuts

| Action | Key(s) | System |
|--------|--------|--------|
| Undo | Ctrl+Z | `undo_system` |
| Redo | Ctrl+Y | `undo_system` |
| Redo (Alt) | Ctrl+Shift+Z | `undo_system` |
| Duplicate | Ctrl+D | `keyboard_shortcuts_system` |
| Select All | Ctrl+A | `keyboard_shortcuts_system` |
| Select All (Alt) | Ctrl+Shift+A | `keyboard_shortcuts_system` |
| Find | F | `keyboard_shortcuts_system` |
| Grid Toggle | G | `keyboard_shortcuts_system` |
| New | Ctrl+N | `keyboard_shortcuts_system` |
| Open | Ctrl+O | `keyboard_shortcuts_system` |
| Delete Selection | Delete / Ctrl+Backspace | `keyboard_shortcuts_system` |
| Escape Selection | Escape | `keyboard_shortcuts_system` |
| Focus Mode | Tab | `keyboard_shortcuts_system` |
| Gizmo Mode (Translate) | G (Ctrl+G) | `transform_gizmo_system` |
| Gizmo Mode (Rotate) | E | `transform_gizmo_system` |
| Gizmo Mode (Scale) | R | `transform_gizmo_system` |
| Gizmo Mode (None) | Q | `transform_gizmo_system` |

### Free Camera Controls

| Action | Key(s) | System |
|--------|--------|--------|
| Move Forward/Back | W/S | `free_camera_system` |
| Move Left/Right | A/D | `free_camera_system` |
| Speed Boost | Shift | `free_camera_system` |
| Rotate Camera | Left/Right/Middle Click + Drag | `free_camera_system` |

---

## Code Examples

### Example 1: Basic Keyboard Movement

```rust
use bevy::input::ButtonInput;
use bevy::prelude::{KeyCode, Res};

fn movement_system(keyboard: Res<ButtonInput<KeyCode>>) {
    let mut direction = Vec2::ZERO;
    
    if keyboard.pressed(KeyCode::KeyW) {
        direction.y += 1.0;
    }
    if keyboard.pressed(KeyCode::KeyS) {
        direction.y -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyA) {
        direction.x -= 1.0;
    }
    if keyboard.pressed(KeyCode::KeyD) {
        direction.x += 1.0;
    }
    
    // Normalize for consistent diagonal speed
    let direction = direction.normalize_or_zero();
}
```

### Example 2: Just Pressed Detection

```rust
fn action_system(keyboard: Res<ButtonInput<KeyCode>>) {
    // Trigger once per key press
    if keyboard.just_pressed(KeyCode::Space) {
        // Jump action
    }
    
    // Trigger on any of multiple keys
    if keyboard.any_just_pressed([KeyCode::KeyE, KeyCode::Enter]) {
        // Interact action
    }
}
```

### Example 3: Modifier Key Combinations

```rust
fn shortcut_system(keyboard: Res<ButtonInput<KeyCode>>) {
    // Check for Ctrl+S (save)
    let ctrl_pressed = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
    
    if ctrl_pressed && keyboard.just_pressed(KeyCode::KeyS) {
        // Save action
    }
    
    // Check for Ctrl+Shift+N (new with shift)
    let shift_pressed = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
    
    if ctrl_pressed && shift_pressed && keyboard.just_pressed(KeyCode::KeyN) {
        // New with shift action
    }
}
```

### Example 4: Mouse Raycast for Click-to-Move

```rust
use bevy::input::mouse::MouseButton;
use bevy::prelude::*;
use bevy::input::ButtonInput;

fn mouse_click_system(
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    query_window: Query<&Window, With<PrimaryWindow>>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
) {
    let window = query_window.single().unwrap();
    let (camera, camera_transform) = query_camera.single().unwrap();
    
    if let Some(cursor_position) = window.cursor_position() {
        if let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) {
            // Perform raycast here
            
            // Trigger on left click
            if mouse_button_input.just_pressed(MouseButton::Left) {
                // Handle click at raycast position
            }
        }
    }
}
```

### Example 5: Mouse Motion for Camera Rotation

```rust
use bevy::prelude::*;
use bevy::input::{ButtonInput, mouse::{MouseMotion, MouseButton}};

fn camera_rotation_system(
    mut mouse_motion_events: MessageReader<MouseMotion>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut camera_yaw: Local<f32>,
    mut camera_pitch: Local<f32>,
) {
    // Only rotate when right mouse button is held
    if mouse_buttons.pressed(MouseButton::Right) {
        for event in mouse_motion_events.read() {
            let sensitivity = 0.1;
            *camera_yaw -= sensitivity * event.delta.x;
            *camera_pitch -= sensitivity * event.delta.y;
            
            // Clamp pitch to avoid gimbal lock
            *camera_pitch = camera_pitch.clamp(-89.0, 89.0);
        }
    }
}
```

### Example 6: Mouse Wheel for Zoom

```rust
use bevy::prelude::*;
use bevy::input::mouse::{MouseWheel, MouseScrollUnit};

fn camera_zoom_system(
    mut mouse_wheel_reader: MessageReader<MouseWheel>,
    mut camera_distance: Local<f32>,
) {
    for event in mouse_wheel_reader.read() {
        let zoom_multiplier = match event.unit {
            MouseScrollUnit::Line => 1.0 - event.y * 0.10,
            MouseScrollUnit::Pixel => 1.0 - event.y * 0.0005,
        };
        
        *camera_distance = (camera_distance * zoom_multiplier)
            .clamp(1.0, 100.0);
    }
}
```

### Example 7: egui Input Guard

```rust
use bevy::prelude::*;
use bevy::input::ButtonInput;
use bevy::input::keyboard::KeyCode;
use bevy_egui::EguiContexts;

fn guarded_game_input(
    egui_ctx: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    // Don't process game input if egui has focus
    if egui_ctx.ctx_mut().unwrap().wants_keyboard_input() {
        return;
    }
    
    // Safe to process game input
    if keyboard.just_pressed(KeyCode::Escape) {
        // Open pause menu
    }
}
```

---

## Common Patterns

### Reference

### Bevy Source Files

| File | Description |
|------|-------------|
| `bevy-0.18.1/crates/bevy_input/src/lib.rs` | Input plugin, ButtonState enum |
| `bevy-0.18.1/crates/bevy_input/src/keyboard.rs` | KeyboardInput, KeyCode, Key |
| `bevy-0.18.1/crates/bevy_input/src/mouse.rs` | MouseButtonInput, MouseMotion, MouseWheel |
| `bevy-0.18.1/crates/bevy_input/src/button_input.rs` | ButtonInput resource |

### Project Source Files

| File | Description |
|------|-------------|
| `src/systems/game_keyboard_input_system.rs` | WASD movement input |
| `src/systems/game_mouse_input_system.rs` | Click-to-move, attack, interact |
| `src/systems/orbit_camera_system.rs` | Camera rotation and zoom |
| `src/systems/free_camera_system.rs` | Free camera controls |
| `src/systems/flight_movement_system.rs` | Flight mode thrust |
| `src/ui/ui_hotbar_system.rs` | Hotbar F1-F8 bindings |
| `src/ui/ui_minimap_system.rs` | Minimap toggle |
| `src/map_editor/systems/keyboard_shortcuts_system.rs` | Editor shortcuts |
| `src/resources/app_state.rs` | Application states |

### Input System Execution Order

```
PreUpdate
  └─> InputSystems
      ├─> keyboard_input_system
      ├─> mouse_button_input_system
      ├─> accumulate_mouse_motion_system
      └─> accumulate_mouse_scroll_system

Update
  └─> Game Input Systems
      ├─> game_keyboard_input_system
      ├─> game_mouse_input_system
      ├─> orbit_camera_system
      ├─> flight_movement_system
      └─> UI Input Systems
```

### Performance Notes

- `ButtonInput` operations are O(1)~ for single key checks
- `any_pressed()` and `all_pressed()` are O(m)~ where m is the number of inputs checked
- Event readers clear events after reading - read early if multiple systems need same events
- Use `just_pressed()` for one-time actions, `pressed()` for continuous actions

### Common Pitfalls

1. **Not checking egui input**: Always check `wants_keyboard_input()` or `wants_pointer_input()` before processing game input
2. **Wrong AppState**: Ensure input systems only run in appropriate AppState
3. **Cursor grab mode**: Check cursor grab mode when expecting mouse input
4. **Event consumption**: MouseMotion and MouseWheel events are consumed by first reader
5. **Just pressed timing**: `just_pressed()` is only true for one frame - don't delay processing

---

## Troubleshooting

### Bevy 0.18 Migration Issues

#### Issue 1: `MessageReader` Deprecation

**Symptom**: Compilation errors about `MessageReader` being deprecated or removed.

**Cause**: Bevy 0.18 deprecated `MessageReader` in favor of `EventReader`.

**Solution**: Replace all `MessageReader` with `EventReader`:

```rust
// Bevy 0.17 (old)
use bevy::prelude::MessageReader;
fn my_system(mut mouse_motion: MessageReader<MouseMotion>) {
    for event in mouse_motion.read() {
        // ...
    }
}

// Bevy 0.18 (new)
use bevy::prelude::EventReader;
fn my_system(mut mouse_motion: EventReader<MouseMotion>) {
    for event in mouse_motion.read() {
        // ...
    }
}
```

**Affected files**: Any system using `MessageReader<MouseMotion>`, `MessageReader<MouseWheel>`, etc.

**Source**: `bevy-0.18.1/crates/bevy_input/src/lib.rs` - Plugin setup now uses `EventReader` internally

---

#### Issue 2: KeyCode vs Key Confusion

**Symptom**: Input not working on different keyboard layouts (AZERTY, Dvorak, etc.)

**Cause**: Using `Key` (logical) instead of `KeyCode` (physical) for game controls.

**Solution**: Use `KeyCode` for game controls, `Key` for text input:

```rust
// WRONG: Won't work on AZERTY keyboards
if keyboard.just_pressed(Key::Character("w")) {
    // Move forward
}

// CORRECT: Works on all layouts
if keyboard.just_pressed(KeyCode::KeyW) {
    // Move forward
}

// CORRECT: For text input symbols
if keyboard.just_pressed(Key::Character("+")) {
    // Add item
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/keyboard.rs:88-139` - KeyboardInput struct contains both `key_code` and `logical_key`

**Related**: See [KeyCode vs Key](#keyboard-input) section

---

#### Issue 3: egui Stealing All Input

**Symptom**: Game input not working when UI is visible, even when not interacting with UI elements.

**Cause**: egui's `wants_keyboard_input()` returns true when any interactive widget is visible, not just when focused.

**Solution**: Use conditional input routing based on widget focus:

```rust
// Check specific input desires
let ctx = egui_ctx.ctx_mut().unwrap();

// For text input fields specifically
if ctx.wants_text_input() {
    // Block all keyboard input
    return;
}

// For general UI interaction
if ctx.wants_pointer_input() {
    // Block mouse input only
    // Still allow keyboard for game actions
}

// For keyboard-specific UI (like hotkeys)
if ctx.wants_keyboard_input() && !ctx.wants_text_input() {
    // Block only specific keys that egui uses
    // Allow WASD, space, etc.
}
```

**Alternative**: Use `bevy_egui`'s `EguiSet` to control execution order:

```rust
// In app setup
app.add_systems(
    Update,
    game_input_system.in_set(EguiSet::Post) // Run after egui
);
```

**Source**: `bevy_egui-0.39.1/src/lib.rs` - EguiSet enum for ordering

---

#### Issue 4: Mouse Input Not Working After egui Interaction

**Symptom**: After clicking a UI button, mouse clicks stop registering for game input.

**Cause**: egui captures mouse events and doesn't release them until widget loses focus.

**Solution**: Check egui pointer desires in mouse input systems:

```rust
fn game_mouse_input_system(
    egui_ctx: EguiContexts,
    mouse_button: Res<ButtonInput<MouseButton>>,
) {
    // Check if egui wants pointer input
    if egui_ctx.ctx_mut().unwrap().wants_pointer_input() {
        return;
    }
    
    // Process game mouse input
    if mouse_button.just_pressed(MouseButton::Left) {
        // ...
    }
}
```

**Example in codebase**: `src/systems/game_mouse_input_system.rs:55-57`

---

#### Issue 5: AccumulatedMouseMotion Not Resetting

**Symptom**: Camera rotation accumulates across frames, causing jittery movement.

**Cause**: Not properly clearing accumulated mouse motion after use.

**Solution**: Bevy automatically resets `AccumulatedMouseMotion` at the start of each frame in the input plugin. Ensure you're reading it in the correct system set:

```rust
// CORRECT: Read in Update or later
fn camera_system(
    accumulated_motion: Res<AccumulatedMouseMotion>,
) {
    // This delta is for the current frame only
    let delta = accumulated_motion.delta;
}

// WRONG: Reading in a system that runs before reset
// This might get stale data
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/mouse.rs:196-249` - AccumulatedMouseMotion implementation

---

#### Issue 6: Modifier Key State Lost

**Symptom**: Ctrl+C works once but not on subsequent presses.

**Cause**: Checking `just_pressed` for modifier keys consumes the state.

**Solution**: Use `pressed()` for modifier keys, `just_pressed()` for the action key:

```rust
// WRONG
if keyboard.just_pressed(KeyCode::ControlLeft) && keyboard.just_pressed(KeyCode::KeyC) {
    // Only works on very first press
}

// CORRECT
let ctrl_pressed = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
if ctrl_pressed && keyboard.just_pressed(KeyCode::KeyC) {
    // Works every time
}
```

**Example**: See [Modifier Key Combinations](#code-examples) section

---

#### Issue 7: Window Focus Lost Input

**Symptom**: Input stops working after switching to another window and back.

**Cause**: Bevy's input system requires the window to have focus.

**Solution**: Check window focus state:

```rust
fn guarded_input_system(
    windows: Query<&Window>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let primary_window = windows.get(1).unwrap(); // Primary window has index 1
    
    // Only process input if window has focus
    if !primary_window.focused {
        return;
    }
    
    // Process input
    if keyboard.just_pressed(KeyCode::KeyW) {
        // ...
    }
}
```

**Note**: Window entity index 1 is the primary window in Bevy 0.18

---

#### Issue 8: Touch Input Coordinate System Mismatch

**Symptom**: Touch input appears at wrong screen positions.

**Cause**: Touch coordinates are in window space, need conversion to world space.

**Solution**: Use camera's `viewport_to_world` with touch position:

```rust
fn touch_input_system(
    mut touch_events: EventReader<Touch>,
    query_camera: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    query_window: Query<&Window, With<PrimaryWindow>>,
) {
    let (camera, camera_transform) = query_camera.single().unwrap();
    let window = query_window.single().unwrap();
    
    for event in touch_events.read() {
        if event.phase == TouchPhase::Started {
            let touch_position = event.position;
            
            // Convert to normalized device coordinates
            let ndc = window.physical_position() + window.physical_size() * 0.5;
            
            // Create ray from touch position
            if let Ok(ray) = camera.viewport_to_world_2d(camera_transform, touch_position) {
                // Use ray for interaction
            }
        }
    }
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/touch.rs` - Touch event structure

---

#### Issue 9: Gamepad Button Mapping Changes

**Symptom**: Gamepad buttons don't match expected actions after Bevy 0.18 upgrade.

**Cause**: Bevy 0.18 updated gamepad button mappings to match industry standards.

**Solution**: Update gamepad button references:

```rust
// Bevy 0.17 (old)
GamepadButtonType::South      // A button (Xbox) / Cross (PS)
GamepadButtonType::East       // B button (Xbox) / Circle (PS)

// Bevy 0.18 (new) - same names, but verify mapping
GamepadButtonType::South
GamepadButtonType::East

// Always test with actual controller after upgrade
```

**Debug**: Print gamepad button events to verify mapping:

```rust
fn debug_gamepad_system(
    mut gamepad_events: EventReader<GamepadButtonEvent>,
) {
    for event in game_events.read() {
        println!("Gamepad {}: {:?} - {:?}", 
            event.gamepad, 
            event.button_type, 
            event.state
        );
    }
}
```

**Source**: `bevy-0.18.1/crates/bevy_input/src/gamepad.rs` - Gamepad button types

---

#### Issue 10: Input Events Not Firing in Headless Mode

**Symptom**: Input systems don't receive events when running tests or headless rendering.

**Cause**: winit (windowing backend) doesn't generate input events in headless mode.

**Solution**: Mock input events for tests:

```rust
#[test]
fn test_movement_system() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<ButtonInput<KeyCode>>()
        .add_systems(Update, movement_system);
    
    // Manually trigger input
    let mut keyboard = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keyboard.press(KeyCode::KeyW);
    
    app.update();
    
    // Verify movement occurred
}
```

---

### egui Input Conflict Solutions

#### Solution 1: Input Priority System

Create a custom input priority system:

```rust
#[derive(Resource, Default)]
pub struct InputPriority {
    pub ui_has_focus: bool,
    pub game_can_receive_input: bool,
}

fn update_input_priority(
    egui_ctx: EguiContexts,
    mut input_priority: ResMut<InputPriority>,
) {
    let ctx = egui_ctx.ctx_mut().unwrap();
    
    input_priority.ui_has_focus = ctx.wants_keyboard_input() || ctx.wants_pointer_input();
    input_priority.game_can_receive_input = !input_priority.ui_has_focus;
}

fn guarded_game_input(
    input_priority: Res<InputPriority>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    if !input_priority.game_can_receive_input {
        return;
    }
    
    // Safe to process input
}
```

#### Solution 2: Partial Input Blocking

Allow certain keys even when egui has focus:

```rust
fn partial_input_guard(
    egui_ctx: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let ctx = egui_ctx.ctx_mut().unwrap();
    
    // Always allow these keys regardless of egui state
    let always_allowed = [
        KeyCode::Escape,      // Escape menu
        KeyCode::F1,          // Help
        KeyCode::F10,         // Admin menu
        KeyCode::F11,         // Fullscreen
    ];
    
    for key in always_allowed {
        if keyboard.just_pressed(key) {
            // Handle always-allowed key
            continue;
        }
    }
    
    // Block other input if egui wants it
    if ctx.wants_keyboard_input() {
        return;
    }
    
    // Process remaining input
}
```

#### Solution 3: UI-Game Input Bridge

Create a system that passes specific input from UI to game:

```rust
#[derive(Event)]
pub struct GameCommand {
    pub command: GameCommandType,
}

#[derive(Clone, Copy)]
pub enum GameCommandType {
    MoveForward,
    MoveBackward,
    Jump,
    // ...
}

// In UI system
fn ui_input_to_game(
    egui_ctx: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut game_commands: EventWriter<GameCommand>,
) {
    // When UI has focus but specific game keys are pressed
    if egui_ctx.ctx_mut().unwrap().wants_keyboard_input() {
        if keyboard.just_pressed(KeyCode::Space) {
            game_commands.send(GameCommand {
                command: GameCommandType::Jump,
            });
        }
        return;
    }
    
    // Normal game input when UI doesn't have focus
}

// In game system
fn process_game_commands(
    mut commands: EventReader<GameCommand>,
) {
    for cmd in commands.read() {
        match cmd.command {
            GameCommandType::Jump => {
                // Jump
            }
            // ...
        }
    }
}
```

---

### Debugging Input Issues

#### Debug System 1: Input State Logger

```rust
fn debug_input_logger(
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mut keyboard_events: EventReader<KeyboardInput>,
    mut mouse_events: EventReader<MouseButtonInput>,
) {
    // Log new keyboard events
    for event in keyboard_events.read() {
        println!("Keyboard: {:?} {:?} - repeat: {}", 
            event.logical_key, 
            event.state, 
            event.repeat
        );
    }
    
    // Log new mouse events
    for event in mouse_events.read() {
        println!("Mouse: {:?} {:?}", event.button, event.state);
    }
    
    // Log current state (spammy, use sparingly)
    // println!("Pressed keys: {:?}", keyboard.get_pressed().collect::<Vec<_>>());
}
```

#### Debug System 2: Input Flow Tracer

```rust
fn trace_input_flow(
    egui_ctx: EguiContexts,
    app_state: Res<State<AppState>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let egui_wants_kb = egui_ctx.ctx_mut().unwrap().wants_keyboard_input();
    let in_game_state = *app_state.get() == AppState::Game;
    
    if keyboard.just_pressed(KeyCode::KeyW) {
        println!("W pressed!");
        println!("  egui_wants_keyboard: {}", egui_wants_kb);
        println!("  in_game_state: {}", in_game_state);
        
        if egui_wants_kb {
            println!("  BLOCKED: egui has keyboard focus");
        } else if !in_game_state {
            println!("  BLOCKED: not in game state");
        } else {
            println!("  ALLOWED: input will be processed");
        }
    }
}
```

#### Visual Debug: Input Overlay

```rust
fn input_debug_overlay(
    egui_ctx: EguiContexts,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
) {
    egui_ctx.ctx_mut().unwrap().show_ui(
        egui::Id::new("input_debug"),
        |ui| {
            ui.heading("Input Debug");
            
            ui.label(format!("egui wants keyboard: {}", 
                ui.ctx().wants_keyboard_input()
            ));
            ui.label(format!("egui wants pointer: {}", 
                ui.ctx().wants_pointer_input()
            ));
            
            ui.separator();
            
            ui.label("Pressed Keys:");
            for key in keyboard.get_pressed() {
                ui.label(format!("{:?}", key));
            }
            
            ui.separator();
            
            ui.label("Pressed Mouse Buttons:");
            for btn in mouse_buttons.get_pressed() {
                ui.label(format!("{:?}", btn));
            }
        }
    );
}
```

---

## Source File References

### Bevy Source Files (bevy-0.18.1)

| File | Path | Description |
|------|------|-------------|
| `lib.rs` | `crates/bevy_input/src/lib.rs` | Input plugin, ButtonState enum, plugin setup |
| `keyboard.rs` | `crates/bevy_input/src/keyboard.rs` | KeyboardInput event, KeyCode enum, Key enum |
| `mouse.rs` | `crates/bevy_input/src/mouse.rs` | MouseButtonInput, MouseMotion, MouseWheel, AccumulatedMouseMotion |
| `button_input.rs` | `crates/bevy_input/src/button_input.rs` | ButtonInput resource and methods |
| `touch.rs` | `crates/bevy_input/src/touch.rs` | Touch events and coordinates |
| `gamepad.rs` | `crates/bevy_input/src/gamepad.rs` | Gamepad input and button types |
| `axis.rs` | `crates/bevy_input/src/axis.rs` | Axis input for joysticks |
| `gestures.rs` | `crates/bevy_input/src/gestures.rs` | Multi-touch gestures |
| `common_conditions.rs` | `crates/bevy_input/src/common_conditions.rs` | Input condition utilities |

### Project Source Files

| File | Path | Description |
|------|------|-------------|
| `game_keyboard_input_system.rs` | `src/systems/` | WASD movement, keyboard-based game actions |
| `game_mouse_input_system.rs` | `src/systems/` | Click-to-move, attack, interact, pickup |
| `orbit_camera_system.rs` | `src/systems/` | Orbit camera rotation and zoom |
| `free_camera_system.rs` | `src/systems/` | Free camera controls for debugging |
| `flight_movement_system.rs` | `src/systems/` | Flight mode thrust input |
| `ui_hotbar_system.rs` | `src/ui/` | Hotbar F1-F8 key bindings |
| `ui_minimap_system.rs` | `src/ui/` | Minimap toggle (Alt+M) |
| `ui_admin_menu_system.rs` | `src/ui/` | Admin menu toggle (F10) |
| `ui_debug_window_system.rs` | `src/ui/` | Debug window toggle (Ctrl+D) |
| `debug_inspector_system.rs` | `src/systems/` | Debug physics toggle (P) |
| `keyboard_shortcuts_system.rs` | `src/map_editor/systems/` | Map editor keyboard shortcuts |
| `transform_gizmo_system.rs` | `src/map_editor/systems/` | Gizmo mode switching (G/E/R/Q) |
| `app_state.rs` | `src/resources/` | Application state enum |
