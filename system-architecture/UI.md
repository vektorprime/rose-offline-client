# System Architecture: UI

## 1. Overview
The UI system is built upon the **bevy_egui 0.39** integration, providing a powerful immediate-mode interface for both game menus and in-world elements. It features a custom **Widget System** that allows for declarative UI definition via XML, enabling rapid iteration on dialogs and menus without recompiling the entire engine.

## 2. EguiPlugin Configuration
The UI utilizes `bevy_egui` with specific configurations to ensure compatibility across different rendering backends:
- **Bindless Mode**: Disabled to maintain compatibility with the **WGPU** backend.
- **Font and Scale**: Custom font settings and scaling are managed through `UiResources` and `egui`'s native context to ensure consistent text rendering across various screen resolutions.

## 3. bevy_egui 0.39 Integration
Integration is handled through the `EguiContexts` system parameter.
- **Context Management**: Systems access the egui context using `egui_context.ctx_mut()`.
- **Primary Context Pass**: For systems requiring standard egui interaction, the `EguiPrimaryContextPass` is utilized to ensure proper rendering order.
- **Layering**: Specialized rendering (like tooltips or world-space UI) uses custom `LayerId`s to control depth and visibility.

## 4. Custom Widget System
The custom widget system provides a layer of abstraction over raw `egui` calls.
- **Widget Types**: A central `Widget` enum (defined in `src/ui/widgets/mod.rs`) encompasses all supported UI elements (e.g., `Button`, `Caption`, `Checkbox`, `Listbox`, `Scrollbox`, `Skill`).
- **Architecture**:
    - `DrawWidget` trait: Handles the rendering logic for each widget.
    - `LoadWidget` trait: Manages the loading of assets (like images) associated with a widget.
    - `DataBindings`: Facilitates two-way communication between the UI and game state (e.g., linking an `Editbox` to a `String` in a resource).
- **Serialization**: Widgets are serialized/deserialized using `serde`. Each widget type is mapped to a specific XML tag via `#[serde(rename = "...")]`.
- **Discriminant Serialization**: Uses `std::mem::discriminant` during the loading phase to identify widget types and manage resources effectively.

## 5. XML Dialog Loading
Dialogs are defined in XML files and loaded at runtime via the `DialogLoader`.
- **`DialogLoader`**: A Bevy `AssetLoader` that parses XML content into `Dialog` assets using `quick_xml`.
- **XML Parsing**: The parser converts XML tags into a tree of `Widget` variants.
- **Widget Tree Construction**: The `Dialog` struct contains a `Vec<Widget>`, which can contain nested widgets (e.g., a `Pane` containing other widgets).
- **Asset References**: Dialogs can reference sprites and other assets which are resolved through `UiResources` during the `load_widget` phase.

## 6. UI State Management
UI visibility and state are tracked through various dedicated resources:
- `UiStateWindows`: Manages core windows like `inventory_open`, `skill_tree_open`, and `settings_open` (`src/ui/mod.rs:56`).
- `UiStateDebugWindows`: Controls the visibility of various debugging tools (`src/ui/mod.rs:102`).
- `UiStateAdminMenu`: Manages the administrative interface.
- `UiStateDragAndDrop`: Tracks current dragged items for inventory and store interactions (`src/ui/ui_drag_and_drop_system.rs:12`).

## 7. Key UI Systems
- **Login & Server Selection**: Handled by `ui_login_system` and `ui_server_select_system`.
- **Character Selection**: Manages character lists and selection states.
- **Game HUD**:
    - **Hotbar**: `ui_hotbar_system`.
    - **Minimap**: `ui_minimap_system`.
    - **Player Info**: `ui_player_info_system`.
- **Debugging Tools**:
    - **Entity Inspector**: `ui_debug_entity_inspector_system`.
    - **Zone Lighting Debugger**: `ui_debug_zone_lighting_system`.
    - **Render Debugger**: `ui_debug_render_system`.

## 8. World UI Rendering
Elements that must appear in the 3D world are rendered using specialized techniques:
- **Name Tags**: Uses `world_to_viewport` to project 3D positions into 2D screen space, then renders an egui `Tooltip` at that location (`src/ui/ui_character_select_name_tag_system.rs:19`).
- **Chat Bubbles**: Temporary text overlays positioned above entities.
- **Item Drop Names**: Uses `world_to_ndc` to calculate screen positions and renders text using `egui::LayerPainter` on a background layer (`src/ui/ui_item_drop_name_system.rs:44`).

## 9. Code Examples

### Widget Drawing Implementation
```rust
// src/ui/widgets/mod.rs:158
impl DrawWidget for Widget {
    fn draw_widget(&self, ui: &mut egui::Ui, bindings: &mut DataBindings) {
        match self {
            Widget::Button(this) => this.draw_widget(ui, bindings),
            // ...
        }
    }
}
```

### Dialog Loading via XML
```rust
// src/ui/dialog_loader.rs:61
let dialog: Dialog = quick_xml::de::from_str(bytes_str)?;
```

### Projecting 3D to 2D (Name Tags)
```rust
// src/ui/ui_character_select_name_tag_system.rs:19
if let Ok(screen_pos) = camera.world_to_viewport(
    camera_transform,
    game_data.character_select_positions[index].translation
        + Vec3::new(0.0, 4.0, 0.0),
) { ... }
```

## 10. Troubleshooting
- **UI Not Rendering**: Verify that the system is using `EguiContexts` and that a valid `EguiPrimaryContextPass` is present in the render graph.
- **Text/Font Issues**: Check `UiResources` for correct font loading and ensure `bevy_egui` is correctly initialized with the target scale.
- **Input Conflicts**: When UI elements are overlapping game world interactions, ensure `egui`'s `wants_pointer_input()` or `wants_keyboard_input()` is checked before processing game-world input.

## 11. Source File References
- **Bevy_egui Source**: `C:\Users\vicha\RustroverProjects\bevvy-collection\bev_egui-0.39.1\src\`
- **Project Source**:
    - `src/ui/mod.rs`: Core module and state definitions.
    - `src/ui/widgets/mod.rs`: Widget enum and traits.
    - `src/ui/dialog_loader.rs`: XML asset loading.
    - `src/ui/name_tag.rs`: (See `ui_character_select_name_tag_system.rs`) 3D positioned UI.
    - `src/ui/chat_bubble.rs`: (See `ui_chatbox_system.rs`) Chat interface.