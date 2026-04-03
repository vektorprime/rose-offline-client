# Window Features Documentation

Comprehensive documentation for window management features used in rose-offline-client with Bevy 0.18.1.

## Overview

This document covers window management in rose-offline-client using Bevy 0.18.1's `bevy_window` crate. The window system handles:

- Window creation and configuration
- Display modes (windowed, borderless fullscreen, exclusive fullscreen)
- Resolution and scale factor management
- VSync and present modes
- Multi-monitor support
- Cursor visibility and grab modes
- Custom cursor icons
- Window lifecycle events

---

## Bevy API References

### WindowPlugin Configuration

The `WindowPlugin` is part of `DefaultPlugins` and provides window management infrastructure.

**Source File:** `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\lib.rs`

```rust
pub struct WindowPlugin {
    /// Settings for the primary window
    pub primary_window: Option<Window>,
    
    /// Settings for the cursor on the primary window
    pub primary_cursor_options: Option<CursorOptions>,
    
    /// Exit condition for the application
    pub exit_condition: ExitCondition,
    
    /// Whether to close windows when requested
    pub close_when_requested: bool,
}
```

**ExitCondition Variants**

| Variant | Description |
|---------|-------------|
| `OnPrimaryClosed` | Exit when primary window closes |
| `OnAllClosed` | Exit when all windows close (default) |
| `DontExit` | Keep running headless (requires manual exit) |

**rose-offline-client Configuration**

The project configures `WindowPlugin` in `src/lib.rs:805-825`:

```rust
.set(bevy::window::WindowPlugin {
    primary_window: Some(Window {
        title: "rose-offline-client".to_string(),
        present_mode: if config.graphics.disable_vsync {
            bevy::window::PresentMode::Immediate
        } else {
            bevy::window::PresentMode::Fifo
        },
        resolution: bevy::window::WindowResolution::new(
            window_width as u32,
            window_height as u32,
        ),
        mode: if matches!(config.graphics.mode, GraphicsModeConfig::Fullscreen) {
            WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Primary)
        } else {
            WindowMode::Windowed
        },
        ..Default::default()
    }),
    ..Default::default()
})
```

---

### Window Component

The `Window` component defines window properties and behavior.

**Source File:** `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\window.rs`

**Core Properties**

| Field | Type | Description |
|-------|------|-------------|
| `present_mode` | `PresentMode` | VSync/presentation mode |
| `mode` | `WindowMode` | Windowed/Fullscreen mode |
| `position` | `WindowPosition` | Window position on screen |
| `resolution` | `WindowResolution` | Window size settings |
| `title` | `String` | Window title bar text |
| `name` | `Option<String>` | Application ID (Wayland/X11) |
| `resizable` | `bool` | Allow user resizing (default: true) |
| `decorations` | `bool` | Show window chrome (default: true) |
| `transparent` | `bool` | Transparent background |
| `focused` | `bool` | Window focus state |
| `visible` | `bool` | Window visibility |

**Platform-Specific Properties**

| Field | Platform | Description |
|-------|----------|-------------|
| `skip_taskbar` | Windows | Hide from taskbar |
| `clip_children` | Windows | Draw over child windows |
| `titlebar_shown` | macOS | Show titlebar |
| `titlebar_transparent` | macOS | Transparent titlebar |
| `fullsize_content_view` | macOS | Content behind titlebar |
| `has_shadow` | macOS | Window drop shadow |
| `borderless_game` | macOS | Hide dock in fullscreen |
| `prefers_home_indicator_hidden` | iOS | Hide home indicator |
| `prefers_status_bar_hidden` | iOS | Hide status bar |

**Helper Methods**

```rust
// Get window dimensions
pub fn width(&self) -> f32                    // Logical width
pub fn height(&self) -> f32                   // Logical height
pub fn size(&self) -> Vec2                    // Logical size
pub fn physical_width(&self) -> u32           // Physical width
pub fn physical_height(&self) -> u32          // Physical height
pub fn physical_size(&self) -> UVec2          // Physical size
pub fn scale_factor(&self) -> f32             // Scale factor

// Cursor position
pub fn cursor_position(&self) -> Option<Vec2>              // Logical
pub fn physical_cursor_position(&self) -> Option<Vec2>     // Physical
pub fn set_cursor_position(&mut self, position: Option<Vec2>)
pub fn set_physical_cursor_position(&mut self, position: Option<DVec2>)

// Window state
pub fn set_maximized(&mut self, maximized: bool)
pub fn set_minimized(&mut self, minimized: bool)
pub fn start_drag_move(&mut self)
pub fn start_drag_resize(&mut self, direction: CompassOctant)
```

---

### Window Mode

Defines how the window is displayed on screen.

**Source File:** `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\window.rs:1330-1367`

**Variants**

| Variant | Description |
|---------|-------------|
| `Windowed` | Normal window with specified resolution |
| `BorderlessFullscreen(MonitorSelection)` | Borderless window filling monitor |
| `Fullscreen(MonitorSelection, VideoModeSelection)` | Exclusive fullscreen with specific video mode |

**Usage**

```rust
// Windowed mode (default)
window.mode = WindowMode::Windowed;

// Borderless fullscreen on primary monitor
window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Primary);

// Exclusive fullscreen with current video mode
window.mode = WindowMode::Fullscreen(MonitorSelection::Primary, VideoModeSelection::Current);

// Exclusive fullscreen with specific resolution
window.mode = WindowMode::Fullscreen(
    MonitorSelection::Primary,
    VideoModeSelection::Specific(VideoMode {
        physical_size: UVec2::new(1920, 1080),
        bit_depth: 32,
        refresh_rate_millihertz: 60000,
    })
);
```

**rose-offline-client Usage**

The project uses `GraphicsModeConfig` in `src/lib.rs:427-459`:

```rust
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum GraphicsModeConfig {
    #[serde(rename = "window")]
    Window { width: f32, height: f32 },
    #[serde(rename = "fullscreen")]
    Fullscreen,
}
```

---

### Window Resolution

Controls window size with support for physical and logical pixels.

**Source File:** `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\window.rs:886-1044`

**Physical vs Logical Pixels**

- **Physical pixels**: Actual pixels on the monitor
- **Logical pixels**: Scaled pixels for consistent UI across DPI settings
- **Scale factor**: Ratio of physical to logical pixels

```
physical_pixels = logical_pixels * scale_factor
```

**Properties**

| Field | Type | Description |
|-------|------|-------------|
| `physical_width` | `u32` | Width in physical pixels |
| `physical_height` | `u32` | Height in physical pixels |
| `scale_factor` | `f32` | OS-provided scale factor |
| `scale_factor_override` | `Option<f32>` | Manual scale factor override |

**Methods**

```rust
// Create resolution
pub fn new(physical_width: u32, physical_height: u32) -> Self

// Get sizes
pub fn width(&self) -> f32                    // Logical width
pub fn height(&self) -> f32                   // Logical height
pub fn size(&self) -> Vec2                    // Logical size
pub fn physical_width(&self) -> u32           // Physical width
pub fn physical_height(&self) -> u32          // Physical height
pub fn physical_size(&self) -> UVec2          // Physical size
pub fn scale_factor(&self) -> f32             // Effective scale factor
pub fn base_scale_factor(&self) -> f32        // OS scale factor only

// Set resolution
pub fn set(&mut self, width: f32, height: f32)
pub fn set_physical_resolution(&mut self, width: u32, height: u32)
pub fn set_scale_factor(&mut self, scale_factor: f32)
pub fn set_scale_factor_override(&mut self, override: Option<f32>)

// Builder pattern
pub fn with_scale_factor_override(self, scale_factor: f32) -> Self
```

**Resize Constraints**

```rust
pub struct WindowResizeConstraints {
    pub min_width: f32,   // Default: 180.0
    pub min_height: f32,  // Default: 120.0
    pub max_width: f32,   // Default: f32::INFINITY
    pub max_height: f32,  // Default: f32::INFINITY
}
```

---

### Present Mode

Controls frame presentation and VSync behavior.

**Source File:** `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\window.rs:1210-1288`

**Variants**

| Variant | Description | Tearing | Latency | Support |
|---------|-------------|---------|---------|---------|
| `AutoVsync` | FIFO Relaxed → FIFO | No | Medium | All |
| `AutoNoVsync` | Immediate → Mailbox → FIFO | Possible | Low | All |
| `Fifo` | Traditional VSync | No | Medium | All |
| `FifoRelaxed` | Adaptive VSync | Possible | Medium | AMD Vulkan |
| `Immediate` | No VSync, lowest latency | Yes | Lowest | Most platforms |
| `Mailbox` | Fast VSync | No | Low | DX11/12, NVidia Vulkan, Wayland |

**Detailed Descriptions**

**Fifo (Default)**
- Frames queued in FIFO order (~3 frames)
- Presents on vbblank
- Blocks when queue full
- Traditionally called "VSync On"

**Immediate**
- No queueing, immediate present
- Can cause tearing
- Lowest latency
- Traditionally called "VSync Off"

**Mailbox**
- Single-frame queue with replacement
- No tearing, low latency
- Traditionally called "Fast VSync"

**rose-offline-client Configuration**

```rust
present_mode: if config.graphics.disable_vsync {
    bevy::window::PresentMode::Immediate
} else {
    bevy::window::PresentMode::Fifo
}
```

---

### Monitor Selection

Specifies which monitor to use for window positioning and fullscreen.

**Source Files:**
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\window.rs:1141-1168`
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\monitor.rs`

**Variants**

| Variant | Description |
|---------|-------------|
| `Current` | Current monitor of the window |
| `Primary` | Primary system monitor |
| `Index(usize)` | Monitor by index |
| `Entity(Entity)` | Specific monitor entity |

**Monitor Component**

```rust
pub struct Monitor {
    pub name: Option<String>,
    pub physical_width: u32,
    pub physical_height: u32,
    pub physical_position: IVec2,
    pub refresh_rate_millihertz: Option<u32>,
    pub scale_factor: f64,
    pub video_modes: Vec<VideoMode>,
}
```

**VideoMode**

```rust
pub struct VideoMode {
    pub physical_size: UVec2,
    pub bit_depth: u16,
    pub refresh_rate_millihertz: u32,
}
```

**Usage**

```rust
// Center on primary monitor
window.position = WindowPosition::Centered(MonitorSelection::Primary);

// Center on current monitor
window.position = WindowPosition::Centered(MonitorSelection::Current);

// Fullscreen on specific monitor index
window.mode = WindowMode::BorderlessFullscreen(MonitorSelection::Index(0));
```

---

### Cursor Management

Controls cursor visibility, grab mode, and custom cursors.

**Source Files:**
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\cursor\mod.rs`
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\cursor\custom_cursor.rs`
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\cursor\system_cursor.rs`

**CursorOptions Component**

```rust
pub struct CursorOptions {
    pub visible: bool,        // Default: true
    pub grab_mode: CursorGrabMode,
    pub hit_test: bool,       // Default: true
}
```

**CursorGrabMode**

| Variant | Description | Platform Notes |
|---------|-------------|----------------|
| `None` | Cursor can leave window | All platforms |
| `Confined` | Cursor confined to window | Not supported on macOS |
| `Locked` | Cursor locked in place | Not supported on X11 |

**System Cursor Icons**

Based on CSS UI Level 3 specification:

```rust
pub enum SystemCursorIcon {
    Default,        // Platform default (arrow)
    ContextMenu,    // Arrow with menu graphic
    Help,           // Question mark/balloon
    Pointer,        // Hand with index finger
    Progress,       // Progress indicator
    Wait,           // Hourglass/watch
    Cell,           // Thick plus with dot
    Crosshair,      // Simple crosshair
    Text,           // I-beam
    VerticalText,   // Horizontal I-beam
    Alias,          // Arrow with curved arrow
    Copy,           // Arrow with plus
    Move,           // Move indicator
    NoDrop,         // Hand with circle/slash
    NotAllowed,     // Circle with line
    Grab,           // Open hand
    Grabbing,       // Closed hand
    EResize, NResize, NeResize, NwResize,
    SResize, SeResize, SwResize, WResize,
    EwResize, NsResize, NeswResize, NwseResize,
    ColResize,      // Horizontal resize
    RowResize,      // Vertical resize
    AllScroll,      // Four-way scroll
    ZoomIn,         // Magnifying glass with +
    ZoomOut,        // Magnifying glass with -
}
```

**Custom Cursors**

Requires `custom_cursor` feature flag.

```rust
pub enum CursorIcon {
    Custom(CustomCursor),
    System(SystemCursorIcon),
}

pub enum CustomCursor {
    Image(CustomCursorImage),
    Url(CustomCursorUrl),  // Web only
}

pub struct CustomCursorImage {
    pub handle: Handle<Image>,      // 8-bit int or 32-bit float RGBA
    pub texture_atlas: Option<TextureAtlas>,
    pub flip_x: bool,
    pub flip_y: bool,
    pub rect: Option<URect>,
    pub hotspot: (u16, u16),        // Hotspot in pixels
}

pub struct CustomCursorUrl {
    pub url: String,
    pub hotspot: (u16, u16),
}
```

**Setting Custom Cursor**

```rust
use bevy::window::{CursorIcon, CustomCursor, CustomCursorImage};

// Load cursor image
let cursor_handle = asset_server.load("cursor.png");

// Create custom cursor
let custom_cursor = CustomCursor::Image(CustomCursorImage {
    handle: cursor_handle,
    texture_atlas: None,
    flip_x: false,
    flip_y: false,
    rect: None,
    hotspot: (0, 0),
});

// Apply to window
commands.entity(window_entity).insert(CursorIcon::Custom(custom_cursor));
```

---

### Window Events

Events for window state changes and user input.

**Source File:** `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\event.rs`

**Window Lifecycle Events**

| Event | Description |
|-------|-------------|
| `WindowCreated` | New window created |
| `WindowCloseRequested` | Close button pressed |
| `WindowClosing` | Window in process of closing |
| `WindowClosed` | Window closed (entity despawned) |
| `WindowDestroyed` | Window destroyed by OS |

**Window State Events**

| Event | Description | Fields |
|-------|-------------|--------|
| `WindowResized` | Window size changed | `window`, `width`, `height` |
| `WindowMoved` | Window position changed | `window`, `position` |
| `WindowFocused` | Focus gained/lost | `window`, `focused` |
| `WindowOccluded` | Window hidden | `window`, `occluded` |
| `WindowScaleFactorChanged` | Scale factor changed | `window`, `scale_factor` |
| `WindowBackendScaleFactorChanged` | OS scale changed | `window`, `scale_factor` |
| `WindowThemeChanged` | System theme changed | `window`, `theme` |

**Cursor Events**

| Event | Description | Fields |
|-------|-------------|--------|
| `CursorMoved` | Cursor moved in window | `window`, `position`, `delta` |
| `CursorEntered` | Cursor entered window | `window` |
| `CursorLeft` | Cursor left window | `window` |

**Other Events**

| Event | Description |
|-------|-------------|
| `RequestRedraw` | Request redraw of all windows |
| `FileDragAndDrop` | File dragged/dropped |
| `Ime` | Input Method Editor events |
| `AppLifecycle` | App lifecycle (Idle, Running, Suspended) |

**Reading Events**

```rust
use bevy::prelude::*;
use bevy::window::*;

// Individual event readers
fn handle_resize(mut events: EventReader<WindowResized>) {
    for event in events.read() {
        println!("Window {} resized to {}x{}", event.window, event.width, event.height);
    }
}

// Combined WindowEvent enum
fn handle_all_events(mut events: EventReader<WindowEvent>) {
    for event in events.read() {
        match event {
            WindowEvent::WindowResized(e) => println!("Resized: {}x{}", e.width, e.height),
            WindowEvent::WindowFocused(e) => println!("Focused: {}", e.focused),
            WindowEvent::CloseRequested(e) => println!("Close requested for {}", e.window),
            // ... handle other events
        }
    }
}
```

---

### Multi-Window Support

Bevy supports multiple windows through entity spawning.

**Creating Additional Windows**

```rust
use bevy::window::{Window, WindowResolution, WindowMode};

fn create_secondary_window(mut commands: Commands) {
    commands.spawn(Window {
        title: "Secondary Window".to_string(),
        resolution: WindowResolution::new(800, 600),
        mode: WindowMode::Windowed,
        position: WindowPosition::At(IVec2::new(100, 100)),
        ..Default::default()
    });
}
```

**Window References**

```rust
pub enum WindowRef {
    Primary,              // Default primary window
    Entity(Entity),       // Specific window entity
}
```

**Camera-Window Binding**

```rust
use bevy::window::WindowRef;

// Bind camera to specific window
commands.spawn((
    Camera3d::default(),
    Camera::default(),
    WindowRef::Entity(window_entity),
));

// Bind camera to primary window
commands.spawn((
    Camera3d::default(),
    Camera::default(),
    WindowRef::Primary,
));
```

**Querying Windows**

```rust
// Query all windows
fn query_all_windows(windows: Query<(&Window, &WindowRef)>) {
    for (window, _) in windows.iter() {
        println!("Window: {}x{}", window.width(), window.height());
    }
}

// Query primary window only
fn query_primary_window(
    primary: Query<&Window, With<PrimaryWindow>>
) {
    if let Ok(window) = primary.get_single() {
        println!("Primary window: {}x{}", window.width(), window.height());
    }
}
```

---

## Custom Extensions

### rose-offline-client Cursor Implementation

The project defines custom cursor types and loading mechanisms in `src/resources/ui_resources.rs:73-104`:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default, Enum)]
pub enum UiCursorType {
    #[default]
    Default,
    Attack,
    Inventory,
    PickupItem,
    Left,
    Right,
    Npc,
    User,
    Wheel,
    NoUi,
    Repair,
    Appraisal,
}

#[derive(Default, Clone)]
pub struct UiCursor {
    pub handle: Handle<ExeResourceCursor>,
    pub loaded: bool,
}
```

Cursors are loaded from the game binary (`trose.exe`):

```rust
cursors: enum_map! {
    UiCursorType::Default => UiCursor::new(asset_server.load("trose.exe#cursor_196")),
    UiCursorType::Attack => UiCursor::new(asset_server.load("trose.exe#cursor_190")),
    UiCursorType::Inventory => UiCursor::new(asset_server.load("trose.exe#cursor_195")),
    // ... more cursors
}
```

**Implementation Guide for Custom Cursors:**
To implement the actual visual change in Bevy 0.18:
1. Enable `custom_cursor` feature in `Cargo.toml`.
2. Load cursor images as Bevy `Image` assets.
3. Create `CustomCursorImage` structs.
4. Insert `CursorIcon::Custom` on window entities.

---

## Code Examples

### Basic Window Configuration
Refer to `src/lib.rs:805-825` for the actual project implementation.

```rust
use bevy::prelude::*;
use bevy::window::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "My Game".to_string(),
                resolution: WindowResolution::new(1920, 1080),
                present_mode: PresentMode::Fifo,
                mode: WindowMode::Windowed,
                ..Default::default()
            }),
            ..Default::default()
        }))
        .run();
}
```

### Toggle Fullscreen
Example of switching between windowed and borderless fullscreen.

```rust
use bevy::window::{Window, WindowMode, MonitorSelection, PrimaryWindow};
use bevy::prelude::*;

fn toggle_fullscreen(
    keyboard: Res<Input<KeyCode>>,
    mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
    if keyboard.just_pressed(KeyCode::F11) {
        for mut window in windows.iter_mut() {
            window.mode = match window.mode {
                WindowMode::Windowed => {
                    WindowMode::BorderlessFullscreen(MonitorSelection::Primary)
                }
                _ => WindowMode::Windowed,
            };
        }
    }
}
```

### Handle Window Close
```rust
use bevy::window::WindowCloseRequested;
use bevy::prelude::*;

fn on_close_requested(mut close_events: EventReader<WindowCloseRequested>) {
    for event in close_events.read() {
        println!("Window {} requested to close", event.window);
        // Handle cleanup before window closes
    }
}
```

### Custom Cursor with System Icon
```rust
use bevy::window::{CursorIcon, SystemCursorIcon, PrimaryWindow, Window};
use bevy::prelude::*;

fn set_cursor_icon(
    mut query: Query<&mut CursorIcon, (With<Window>, With<PrimaryWindow>)>,
) {
    if let Ok(mut cursor) = query.get_single_mut() {
        *cursor = CursorIcon::System(SystemCursorIcon::Crosshair);
    }
}
```

### Responsive Window Resize
Refer to `src/lib.rs` for camera projection updates.

```rust
use bevy::window::WindowResized;
use bevy::prelude::*;

fn handle_resize(
    mut resize_events: EventReader<WindowResized>,
    mut camera: Query<&mut OrthographicProjection, With<Camera2d>>,
) {
    for event in resize_events.read() {
        for mut projection in camera.iter_mut() {
            // Adjust camera scale based on window size
            projection.scale = 100.0 / event.height;
        }
    }
}
```

### Monitor Information
```rust
use bevy::window::Monitor;
use bevy::prelude::*;

fn list_monitors(monitors: Query<&Monitor>) {
    for (index, monitor) in monitors.iter().enumerate() {
        println!("Monitor {}: {}x{} at {:?}", 
            index, 
            monitor.physical_width, 
            monitor.physical_height,
            monitor.physical_position
        );
        println!("  Scale factor: {}", monitor.scale_factor);
        println!("  Video modes: {}", monitor.video_modes.len());
    }
}
```

---

## Configuration Options

### config.toml Example
The project uses a custom configuration file to set window properties.

```toml
[graphics]
mode = { type = "window", width = 1920.0, height = 1080.0 }
# or
# mode = { type = "fullscreen" }
disable_vsync = false
```

### Cargo.toml Features
Custom cursor support must be enabled in `Cargo.toml`.

```toml
[dependencies]
bevy = { version = "0.18", features = [
    "custom_cursor",  # Enable custom cursor support
] }
```

---

## Common Patterns

### DPI-Aware Sizing
To ensure consistent UI across different monitor scales, use logical pixels for UI elements and physical pixels for window resolution. The scale factor can be accessed via `window.scale_factor()`.

### Window Centering
The project centers the window on the primary monitor by default using:
`window.position = WindowPosition::Centered(MonitorSelection::Primary);`

### Dynamic Fullscreen Toggle
Switching between `WindowMode::Windowed` and `WindowMode::BorderlessFullscreen(MonitorSelection::Primary)` is the preferred way to implement a fullscreen toggle without changing the physical resolution of the monitor.

### Resolution Constraints
To prevent the window from becoming too small, use `WindowResizeConstraints` to set `min_width` and `min_height`.

---

## Troubleshooting

### Bevy 0.18 Migration Issues

| Issue | Symptom | Solution |
|-------|----------|----------|
| **WindowMode Changes** | `WindowMode::Fullscreen` now requires `MonitorSelection` and `VideoModeSelection` | Use `BorderlessFullscreen(MonitorSelection)` for most "fullscreen" needs, or provide specific `VideoMode` for exclusive fullscreen. |
| **PresentMode Panics** | App panics on startup with `PresentMode::Immediate` | Ensure the platform supports the chosen `PresentMode`. Use `AutoNoVsync` for a safer "no-vsync" option. |
| **Cursor Visibility** | Cursor is hidden or not behaving as expected | Visibility is now managed via the `CursorOptions` component on the window entity. Check `CursorOptions.visible`. |
| **Logical vs Physical Size** | Window size is unexpected on High-DPI screens | `WindowResolution::new()` takes physical pixels. Use `window.width()` and `window.height()` for logical size. |
| **Exclusive Fullscreen Flicker** | Screen flickers or resolution changes on toggle | Use `BorderlessFullscreen` instead of `Fullscreen` to avoid triggering a hardware display mode change. |
| **Custom Cursor Not Loading** | Cursor remains default despite `CursorIcon::Custom` | Verify `custom_cursor` feature is enabled in `Cargo.toml` and the image asset is fully loaded before applying. |
| **Cursor Grab Mode Limits** | Grab mode not working on certain OS | `CursorGrabMode::Confined` is not supported on macOS. `CursorGrabMode::Locked` is not supported on X11. |
| **DPI Scale Confusion** | UI elements shifted or incorrectly sized | `window.scale_factor()` returns the effective scale (including overrides). Use `window.base_scale_factor()` for the raw OS value. |
| **Event Reader Changes** | Resize/Move events not firing | Ensure you are reading from `WindowResized` / `WindowMoved` events rather than generic `WindowEvent` if specific types are needed. |

---

## Source File References

### Bevy Source Files

| Component | Path |
|-----------|------|
| WindowPlugin | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\lib.rs` |
| Window | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\window.rs` |
| Monitor | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\monitor.rs` |
| Events | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\event.rs` |
| System Cursor | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\cursor\system_cursor.rs` |
| Custom Cursor | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_window\src\cursor\custom_cursor.rs` |
| Winit Integration | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_winit\src\lib.rs` |
| Winit Cursor | `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_winit\src\cursor\mod.rs` |

### Project Files

| Component | Path |
|-----------|------|
| Window Setup | `src/lib.rs:805-825` |
| Graphics Config | `src/lib.rs:427-459` |
| UI Cursors | `src/resources/ui_resources.rs` |