# Chat Bubble and Name Tag System Architecture

## Status: RESOLVED
**Last Updated**: 2026-08-03

## Overview
The chat bubble and name tag systems provide world-space UI elements that follow characters and monsters. Both systems utilize a custom rendering pipeline designed for high performance and billboard behavior, bypassing the standard `bevy_ui` for elements that need to exist within the 3D world.

---

## Core Components

### 1. WorldUiRect
**File:** `src/render/world_ui.rs`
The primary rendering component for all world-space UI. It defines a single quad to be rendered as a billboard.
```rust
pub struct WorldUiRect {
    pub image: Handle<Image>,    // The generated text or background texture
    pub screen_offset: Vec2,     // Offset in pixels from the projected world position
    pub screen_size: Vec2,       // Size in pixels on the screen
    pub uv_min: Vec2,            // UV coordinates for the texture
    pub uv_max: Vec2,
    pub color: Color,            // Vertex color (used for tinting and fading)
    pub order: u8,               // Sorting order for overlapping elements
}
```

### 2. ChatBubble
**File:** `src/components/chat_bubble.rs`
Tracks the state and lifetime of a chat bubble.
- `target_entity`: The entity the bubble is following.
- `text`: The text being displayed.
- `remaining_time`: Time remaining before fade-out starts.
- `total_time`: Initial duration for fade calculations.
- `fade_start_fraction`: Fraction of the total time at which fading begins (default `0.2`).

### 3. MonsterChatter
**File:** `src/components/chat_bubble.rs` (same module as `ChatBubble`)
Enables NPCs to periodically "speak" random phrases.
- `time_until_next_chat`: Randomized timer.
- `min_interval` / `max_interval`: Bounds for the random timer.

### 4. ModelHeight
**File:** `src/components/model_height.rs`
Stores the calculated height of a character's model, used to position UI elements above the head.
- **Calculation**: Derived from the AABB of all model parts (Body, Hands, Feet, Head, Face, Hair).

---

## Rendering Pipeline

### Custom Render Phase: `Transparent3d`
World UI elements are rendered in the `Transparent3d` phase to support alpha blending and proper depth sorting with other transparent objects in the scene.

### Vertex Shader (`world_ui.wgsl`)
The vertex shader is responsible for the "billboard" effect. It projects the world position of the parent entity to screen space and then applies pixel-perfect offsets.
- **Projection**: `clip_pos = view.clip_from_world * vec4(world_position, 1.0)`
- **NDC Offset**: `ndc_offset = vec2(offset.x, -offset.y) / viewport_size * 2.0`
- **Final Position**: `out.clip_position = vec4((clip_pos.xy / clip_pos.w + ndc_offset) * clip_pos.w, clip_pos.z, clip_pos.w)`
- **Note**: The Y-axis negation is critical because screen space is top-down (Y increases down) while NDC is bottom-up (Y increases up).

### Fragment Shader (`world_ui.wgsl`)
- **Texture Sampling**: Samples the `base_texture` using the provided UVs.
- **Alpha Blending**: Multiplies the sampled color by the vertex color.
- **Fog Suppression**: Fog is disabled for UI elements to maintain readability regardless of distance or environmental conditions.

---

## System Logic

### 1. Texture Generation (The `egui` Pattern)
Both systems use `egui` for high-quality text layout:
1. **Layout**: Create an `egui::Galley` using `LayoutJob`.
2. **Texture Allocation**: Allocate a Bevy `Image` with a power-of-two size large enough for the text.
3. **Glyph Copying**: Iterate through the `egui` font texture and copy individual glyph pixels into the Bevy `Image` buffer.
4. **Tinting**: Glyph pixels are copied as white + alpha (glyph coverage = max of the RGBA channels) and tinted at render time via the `WorldUiRect` vertex color; chat bubbles also receive a procedurally generated rounded-rectangle background texture.

### 2. Spawn Systems
- **`chat_bubble_spawn_system`**: Listens for `ChatBubbleEvent`. It caches pending bubbles, forces `egui` font glyphs to be uploaded by rendering a hidden `egui::Area` (required in bevy_egui 0.39), and reads the CPU-side font atlas directly to build the text and background textures.
- **`name_tag_system`**: Automatically detects new entities with `ClientEntityName` and generates name tags. It caches textures by name to avoid redundant work.

### 3. Update and Cleanup
- **`chat_bubble_update_system`**: Ticks down the `remaining_time`. In the last 20% of the bubble's life, it linearly fades the alpha of the `WorldUiRect` color.
- **`chat_bubble_cleanup_system`**: Uses `RemovedComponents<ClientEntityName>` to detect when a character is despawned and immediately removes its associated chat bubbles.

---

## Key Fixes (Deep Dive)

### The "Gray Box" Bug
- **Symptom**: UI elements appeared as solid gray boxes.
- **Root Cause**: The shader was using `zone_lighting.fog_params.y` (min density) and `.z` (max density) as `fog_near` and `fog_far` distances. In the game, these values are `0.0` and `0.75`. Any UI element further than 0.75 units from the camera was being 100% fogged to the default gray fog color.
- **Fix**: Commented out the fog calculation in the fragment shader.

### The "Upside Down" Bug
- **Symptom**: Text and backgrounds were flipped vertically.
- **Root Cause**: The vertex shader was adding the screen-space Y offset directly to the NDC Y coordinate. Since screen Y increases downwards and NDC Y increases upwards, this caused a vertical flip and incorrect positioning.
- **Fix**: Negated the Y offset in the vertex shader's NDC transformation.

### Player Positioning
- **Symptom**: Player name tags appeared at the waist.
- **Root Cause**: The AABB calculation in `character_model_add_collider_system.rs` was only considering the Body, Hands, and Feet parts. It was missing the Head, Face, and Hair parts, which are separate entities in the player's skinned mesh.
- **Fix**: Expanded the AABB calculation to include all head-related parts and increased the base vertical offset to `1.8` (the height is computed as `ModelHeight::new(1.8 + half_extents.y * 2.0)` in `character_model_add_collider_system.rs`).

---

## Technical Considerations for Bevy 0.18
- **Depth Testing**: The pipeline uses `CompareFunction::Always` with `depth_write_enabled: false`; world UI quads are screen-space overlays anchored to world positions and render on top regardless of scene depth.
- **VisibilityClass**: `WorldUiRect` is registered with `#[require(VisibilityClass)]` so it integrates with Bevy's standard visibility systems.
- **Rendering Architecture**: World UI does not use Bevy's mesh rendering components (`Mesh3d` / `MeshMaterial3d`). It is a fully custom pipeline: quads are emitted as `WorldUiBatch` entities in the render app with a raw vertex buffer, a per-image bind group, and a specialized `WorldUiPipeline` in the `Transparent3d` phase.
