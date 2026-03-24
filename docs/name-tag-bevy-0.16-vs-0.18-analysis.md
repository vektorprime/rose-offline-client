# Name Tag Behavior: Bevy 0.16 Working Version vs Bevy 0.18 Current

## Scope
This report compares the working Bevy 0.16 implementation (`C:\Users\vicha\RustroverProjects\bevy-0.16-rose-offline-client\src`) against the current Bevy 0.18 implementation in this workspace (`src/`), and identifies likely breakage introduced during the upgrade.

## Files Reviewed

### Working 0.16 project
- `src/systems/name_tag_system.rs`
- `src/components/name_tag_entity.rs`
- `src/components/chat_bubble.rs`
- `src/systems/chat_bubble_spawn_system.rs`

### Current 0.18 project
- `src/systems/name_tag_system.rs`
- `src/components/name_tag_entity.rs`
- `src/components/chat_bubble.rs`
- `src/systems/chat_bubble_spawn_system.rs`
- `src/render/world_ui.rs`
- `src/render/shaders/world_ui.wgsl`

### External references
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1\crates\bevy_camera\src\camera.rs`
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy_egui-0.39.1\src\lib.rs`
- `C:\Users\vicha\RustroverProjects\bevy-collection\bevy_egui-0.39.1\src\render\mod.rs`
- `bevy-0.16-to-0.17-migration-guide.md`
- `bevy-0.17-to-0.18-migration-guide.md`

---

## How Name Tags Work in the Working Bevy 0.16 Version

1. **Entity selection and type detection**
   - `name_tag_system` queries all entities with `ClientEntityName` and `ModelHeight` that do not yet have `NameTagEntity`.
   - It classifies each target as Character/Monster/Npc (`NameTagType`) using NPC/team/game-data checks.

2. **Text layout via egui**
   - `create_pending_nametag` builds an egui `LayoutJob` and `Galley` for one or two rows (NPC job+name split).
   - Per-row text colors are captured for later tinting.

3. **Texture generation**
   - `create_nametag_data` reads glyphs from `EguiManagedTextures` (0.16 path uses `(window_entity, texture_id)` key).
   - Glyph pixels are copied into a runtime `Image` buffer and outlined.
   - One or two `WorldUiRect` quads are built from this texture with UV slices.

4. **World placement and rendering**
   - A parent nametag entity is spawned at `Transform::from_translation(Vec3::new(0.0, model_height.height, 0.0))`.
   - Child `WorldUiRect` entities are attached for text/target marks/HP bars.
   - `WorldUiRect` uses pixel offsets (`screen_offset`) from projected world position.

5. **Projection behavior**
   - In the custom world UI pipeline, world position is projected to clip/NDC and converted to screen coordinates.
   - Y inversion is intentionally applied (NDC up vs screen down).

---

## What Changed in Current Bevy 0.18 Implementation

### Confirmed migration-level/API updates
1. **egui texture key changed conceptually to camera context entity**
   - Current code correctly queries camera with `PrimaryEguiContext`.
   - `EguiManagedTextures` in bevy_egui 0.39 is keyed by `(Entity, u64)` where entity is the egui context owner.

2. **Scheduling/context integration changed**
   - `name_tag_system` now runs in `EguiPrimaryContextPass`.
   - This aligns with bevy_egui 0.39’s rendering lifecycle.

3. **Hierarchy/event API migrations**
   - `EventReader` -> `MessageReader`
   - Parenting style moved from `set_parent` calls to `ChildOf` inserts in these code paths.

4. **Camera/RenderTarget migration context**
   - 0.17->0.18 migration notes that `RenderTarget` moved off `Camera` field to component.
   - Relevant camera API changes were reviewed; world projection behavior itself remains consistent.

### Important runtime-behavior changes from bevy_egui 0.39
1. **Managed textures are updated from rendered egui output (`textures_delta`)**
   - Font atlas updates only exist after egui output actually produces the texture changes.

2. **Font atlas channel assumptions are less stable for manual glyph extraction**
   - Chat bubble code in this repo already contains a compatibility fix for this.
   - Name tag code still uses old direct RGBA copy logic.

---

## Why Name Tags Likely Broke After Upgrade

### Primary cause
**Name tag glyph extraction still uses pre-upgrade channel assumptions, while chat bubble extraction was updated for bevy_egui 0.39 behavior.**

- Current `name_tag_system.rs` copies glyph pixels directly:
  - `dst.r = pixel[0]`, `dst.g = pixel[1]`, `dst.b = pixel[2]`, `dst.a = pixel[3]`
- Current `chat_bubble_spawn_system.rs` instead computes coverage robustly as `max(r,g,b,a)` and stores white+alpha.

If the atlas channel carrying glyph coverage is not always `alpha` in the expected way, name tag textures can become transparent/incorrect while chat bubbles still work due to robust extraction.

### Secondary robustness gap
**Name tag font-upload forcing uses a single constant egui `Area` id (`"nametag_font_upload"`) for all entities.**

In frames where many tags are pending, this can cause less reliable per-entity glyph upload behavior compared to unique IDs. Chat bubble upload forcing already uses per-target IDs.

---

## Transform / Coordinate Findings

- Camera projection logic (`world_to_viewport`/NDC conversion) in Bevy 0.18 still flips Y in viewport conversion, matching expected screen-space semantics.
- The project’s `WorldUiRect` pipeline also explicitly handles Y inversion in shader/math.
- No critical transform-coordinate regression was found that uniquely explains name tags failing while related world-space UI paths still function.

---

## Fix Strategy

1. **Align name tag glyph copy path with chat bubble’s 0.39-safe extraction path**
   - Use `coverage = max(r, g, b, a)`
   - Write output glyph pixels as white RGB + coverage alpha
   - Keep tinting via `WorldUiRect.color`

2. **Make forced font-upload egui Area ID entity-specific**
   - Use ID tuple including target entity to avoid collisions during mass spawn.

This fix preserves existing world-space positioning, rendering order, and health/mark layout behavior while addressing 0.18-specific text texture generation fragility.
