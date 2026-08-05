# Optimization Review 06 — UI: egui Layer (bevy_egui 0.39.1), Widgets, Dialogs, Minimap, Chatbox, DnD Slots, Tooltips, Debug Windows

**Repo**: `rose-offline-client` (single crate, Bevy 0.18.1, bevy_egui 0.39.1 / egui 0.33.0 / egui_extras 0.33.0)
**Date**: 2026-08-03
**Type**: Research + report only. No `.rs` files were modified, no build was run, no game was launched.

---

## 1. Scope Summary

Architecture docs in `system-architecture/` before this review:

| Doc | Status | Relevance |
|---|---|---|
| `system-architecture/UI.md` | existed, read | egui plugin setup, bindless mode, `PrimaryEguiContext`, custom `Widget`/`DrawWidget`/XML dialog system (authoritative for the plugin facts below) |
| `system-architecture/chat-bubble-and-name-tag-architecture.md` | existed, read | WorldUiRect pipeline, egui galley text, `WorldUiBatch` render path |
| `system-architecture/README.md`, `ECS.md`, `Render.md`, `Input.md`, `Window.md` | existed, read | Bevy 0.18 change-detection semantics, render/input flow for egui context wiring |
| Remaining docs (`Lighting.md`, `Physics.md`, `Assets.md`, etc.) | existed, not read | Out of scope |

Pitfalls read: `pitfalls/index.md`, `pitfalls/skill-bar-ui.md` (`ui_drag_and_drop_system` must run `.after()` all drop-target systems), `pitfalls/personal-store-drag-buy.md` (`PersonalStoreEvent::RequestBuyItem` + `NumberInputDialogEvent::Show`), `pitfalls/performance-memory.md`.

Bevy 0.18.1 / bevy_egui 0.39.1 source consulted at `C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1` and `...\bevy-collection\bevy_egui-0.39.1` (change-detection semantics; repaint callback wiring at `bevy_egui-0.39.1/src/input.rs:200`).

Files analyzed (all in `src/`): `lib.rs` (lines 800–834, 1000–1031, 1560–1640), `ui/mod.rs`, `resources/ui_resources.rs`, `ui/widgets/{dialog,draw,button,listbox,zlistbox,editbox,gauge,data_bindings,skill}.rs`, `ui/ui_window_sound_system.rs`, `ui/ui_sound_event_system.rs`, `ui/ui_chatbox_system.rs`, `ui/ui_inventory_system.rs`, `ui/ui_minimap_system.rs`, `ui/ui_settings_system.rs`, `ui/ui_admin_menu_system.rs`, `ui/ui_skill_tree_system.rs`, `ui/ui_skill_list_system.rs` (header), `ui/ui_party_system.rs` (header), `ui/ui_character_create_system.rs`, `ui/ui_character_select_name_tag_system.rs`, `ui/ui_debug_window_system.rs`, `ui/ui_quest_scroll_system.rs`, `ui/ui_selected_target_system.rs`, `ui/ui_status_effects_system.rs`, `ui/ui_sailing_hud_system.rs`, `ui/drag_and_drop_slot.rs`, `ui/tooltips.rs`, `systems/name_tag_system.rs`, `systems/chat_bubble_spawn_system.rs`, `systems/boat_spawn_system.rs` (`find_nearest_shore_position`, called from sailing HUD), `render/world_ui.rs`.

Key architecture facts confirmed from source:

- **egui plugin runs with bindless disabled and no auto-created context**: `EguiPlugin { bindless_mode_array_size: None, ..Default }` (`src/lib.rs:809`) and `EguiGlobalSettings { auto_create_primary_context: false }` (`src/lib.rs:818-821`); the main camera carries `PrimaryEguiContext` explicitly and the UI passes are scheduled around `EguiPreUpdateSet::InitContexts` (`src/lib.rs:1014`). Custom font `fonts/Ubuntu-M.ttf` via `setup_egui_fonts` (`src/lib.rs:2041`).
- **No `request_repaint` / repaint-policy abuse exists anywhere in `src/`** — verified by grep. bevy_egui 0.39.1 is repaint-driven through egui's own mechanism: `WindowToEguiContextMap::on_egui_context_added_system` registers a repaint callback via `set_request_repaint_callback` (`bevy_egui-0.39.1/src/input.rs:200`). Repaint cost therefore follows egui's edge-triggered "needs repaint" flags only; this review found no app-side calls that force repaints.
- **UI is entirely egui**: every game window (inventory, minimap, chatbox, store, skill tree…) is an `egui::Window` / `egui::Area` drawn every frame inside `EguiPrimaryContextPass`; there is no retained UI scene graph — everything is rebuilt per frame in egui immediate mode, so the per-frame allocations below are the core of the UI CPU cost.
- **Custom sprite widget system**: `UiSprite::draw` / `draw_stretched` (`resources/ui_resources.rs:27-38`) build a 6-vertex/4-index `Mesh` and upload it via `ui.allocate_painter` **on every call** — callers pay a fresh `Mesh` allocation + `alloc_freelist` per sprite per frame.
- **World-space text (name tags, chat bubbles) uses egui `Galley`s** baked into textures (`systems/name_tag_system.rs`, `systems/chat_bubble_spawn_system.rs`) and rendered by the custom `world_ui.rs` pipeline (already reviewed as a render-path finding in report 05; cross-referenced here only).
- **A set of correct caching patterns already exists** and is used as the reference in several fixes below: one-time sprite lookup with cached handles (`ui_selected_target_system.rs:59-60`), zone-name galley cache invalidated on zone/DPI change (`ui_minimap_system.rs:294-307`), name-tag galley cache keyed by string with zone/DPI invalidation (`name_tag_system.rs:383-397`), dialog instance caching in `dialog_loader`, admin-menu filtered caches with early return (`ui_admin_menu_system.rs:103`), debug windows gated behind `debug_ui_open`.

Cost notation: **per-frame** counts assume a populated zone and default settings; estimates are static-analysis relative ordering, not benchmarked numbers. The `EguiPrimaryContextPass` runs after `Update` and before rendering, so every UI system listed below runs once per frame while the game world is open.

---

## 2. Methodology

1. Read `system-architecture/UI.md` and the pitfall entries above; read `pitfalls/index.md` for the UI-related entries.
2. Read every file in the file list above end-to-end (line-numbered); where a file was too large for a single read, it was read in offset chunks until fully covered (`ui_chatbox_system.rs`, `ui_inventory_system.rs`, `ui_minimap_system.rs`, `ui_settings_system.rs`, `ui_admin_menu_system.rs`, `ui_drag_and_drop_slot.rs`, `tooltips.rs`, `name_tag_system.rs`, `ui_debug_window_system.rs`, `ui_quest_scroll_system.rs`).
3. Traced every caller of the shared hot functions to quantify per-frame impact: `UiSprite::draw`/`draw_stretched` (7 call sites), `DragAndDropSlot::new`/`with_item` (8 call sites), `get_sprite_by_index` (minimap + admin), `DataBindings` getters (all widget draws).
4. Verified the egui/bevy_egui interaction surface from the vendored crate source: repaint wiring (`input.rs:200`), `PrimaryEguiContext` registration, `EguiPrimaryContextPass` scheduling — to rule out repaint-abuse and confirm that all UI work is per-frame immediate-mode.
5. Cross-checked every `is_changed()`-gated system against Bevy 0.18.1 change-detection semantics (write = marked changed even if identical), since several UI systems re-write values every frame.
6. Confirmed the sail HUD's per-frame shore search cost by reading its implementation (`boat_spawn_system.rs:64-103`) and its caller (`ui_sailing_hud_system.rs:261-267`).
7. No builds, no runs, no edits.

---

## 3. Findings

### F1 — `UiSprite::draw` / `draw_stretched` allocate a `Mesh` on every call; hot widgets call it several times per frame
**File**: `src/resources/ui_resources.rs:27-38`; callers: `src/ui/widgets/button.rs:135`, `src/ui/widgets/gauge.rs:69,76`, `src/ui/drag_and_drop_slot.rs:340-350,353-362`, `src/ui/widgets/skill.rs:62-68`, `src/ui/ui_minimap_system.rs:560-562,763-787,807-812,915-920`, `src/ui/ui_selected_target_system.rs:87-124`

Both methods construct a `Mesh` from raw vertex/index data and upload it through `ui.allocate_painter(...)` **on every invocation, every frame**. Nothing is cached: each call allocates a fresh `Mesh` (vertex + index `Vec`s) and a new paint primitive with texture id + UV rect. Sprites drawn this way are re-uploaded every frame even though the texture/UV never change.

Per-frame cost in a typical game screen: inventory = up to 48 slot sprites (several per slot), minimap = ~8–12 sprites + 100–300 NPC icons, DnD slots = 3–4 sprites each, party window = 4–8 sprites. That is roughly 150–400 `Mesh` allocations and uploads per frame in normal gameplay, and more with skill tree (hundreds of nodes) or stores open.

**Suggested fix**: cache the mesh+galley once per sprite handle and reuse the cached `epaint` primitive for the common full-UV case. The codebase already has the reference pattern — `ui_selected_target_system.rs:59-60` caches `(texture_id, uv, size)` per sprite and only looks up once:

```rust
// in UiSprite: add
#[derive(Clone)]
struct SpriteMeshCache { mesh: Mesh, rect_size: Vec2 } // built lazily
// in draw():
if let Some(cached) = self.mesh_cache.as_ref() {
    if cached.rect_size == rect_size { reuse cached mesh & painter }
}
```

Even simpler for the ~95 % of call sites that use full UV + fixed size: store a prebuilt `Mesh` in `UiSprite` (built when the texture is loaded, e.g. in the loader `LoadWidget`/`LoadComponent` path) and only recompute when the sprite changes. Fallback: at minimum avoid re-allocating `Vec`s by reusing a per-`UiSprite` scratch `Mesh` struct.

**Impact estimate**: 150–400 small heap allocations + vertex/UV re-upload per frame; one of the top UI CPU costs. Fix is localized to one file with zero call-site changes.

---

### F2 — `DragAndDropSlot` allocates a `Box<dyn Fn>` closure every frame, for every slot
**Files**: `src/ui/drag_and_drop_slot.rs:34,62,162,207` (field + `accepts` boxing in `new`/`with_item`); callers: `src/ui/ui_inventory_system.rs:230`, `src/ui/ui_hotbar_system.rs:84,99,111`, `src/ui/ui_skill_tree_system.rs:82`, plus quest list / bank / npc store / personal store slots

`DragAndDropSlot::new` stores `accepts: Box<dyn Fn(&dyn DragAndDropPayload) -> bool>` and `on_drop: Box<dyn Fn(&mut World, &dyn DragAndDropPayload) -> bool>`. The constructor is called **every frame** inside each slot's draw (e.g. `ui_inventory_system.rs:230` for each of the 48 inventory slots, hotbar 12–15 slots, skill tree hundreds of nodes). Each call does `Box::new(closure)` — a heap allocation — and the closures capture `&mut Messages<...>` and `&mut Assets<...>` by re-borrowing system params, so the closures themselves are per-frame fresh. The `DragAndDropSlot` (including both boxes) is dropped right after `draw_drag_and_drop_slot` returns, unless a drag is active.

Per-frame cost: inventory alone = 48 slots × 2 box allocations = ~96 allocs; with hotbar + skill tree open it is 300–1000 allocs/frame.

**Suggested fix**: split the per-frame state from the policy. Make `DragAndDropSlot` store policy as an enum (the codebase already has `DragAndDropPayload` enum variants — inventory item / hotbar skill / skill-tree skill / item sell/buy / trade etc.), and keep the `Box<dyn Fn>` only for the *active drag* (a single `Local<Option<ActiveDrag>>` in `ui_drag_and_drop_system`), not for every slot:

```rust
enum SlotAccept { InventoryItem, HotbarSkill, SkillTreeSkill, … }
struct DragAndDropSlot {
    rect, id, sprite, …,
    accept: SlotAccept,               // Copy, no heap
    on_drop: SlotDrop,                // enum dispatch in ui_drag_and_drop_system
}
```

The active-drag closure (needed for the `request_drop` message path) is created once per drag start in the input system instead of once per slot per frame. This also removes the `Box<dyn Fn(&mut World, …)>` from the slot struct so slots can be `Copy`/cheaply rebuilt.

**Impact estimate**: removes ~100–1000 heap allocations/frame depending on open windows; also shrinks the per-frame struct churn on every slot draw.

---

### F3 — Chatbox clones the entire egui `Style` (and the whole UI style stack) every frame — dead code
**File**: `src/ui/ui_chatbox_system.rs:177-183`

```rust
let mut ui_style = (*ui.style()).clone();   // :177 — full Style clone every frame
// … several mutation blocks …
ui.set_style(ui_style);                     // :183
```

The clone's mutations (`.visuals.widgets.*`, `item_spacing`…) are applied but the resulting style is **never read by any widget afterwards** — the chatbox's custom drawing (chat text galley + background painter) doesn't consume the mutated style, and egui `Ui::set_style` pushes a modified style only for subsequent widgets, of which there are none in this branch. Net effect: one `Style` clone (dozens of `Vec`s, `Color32`s, fonts handles) + full style stack push/pop **per frame, wasted**.

**Suggested fix**: delete lines 177–183. If some drawing does depend on a style value (e.g. a stroke color), read it via `ui.visuals()`/`ui.style()` by reference instead of cloning the whole style:

```rust
let stroke = ui.visuals().widgets.inactive.fg_stroke;
```

**Impact estimate**: one full `Style` clone per frame (~2–5 µs + several small heap allocations, a dozen+ `Vec` copies). Small absolute, but it is pure dead code — zero-risk fix, belongs in the quick-win list.

---

### F4 — Chatbox clones its whole `LayoutJob` per entry, per frame
**File**: `src/ui/ui_chatbox_system.rs:69-75` (struct), `:259` (`ui.label(job.clone())`)

Each of the up to 100 visible chat entries (`MAX_CHATBOX_ENTRIES`) holds a `LayoutJob` (vector of `TextFormat`-tagged sections). Every frame the draw loop calls `ui.label(job.clone())` — cloning the entire job including all text sections — because `egui::Label` takes ownership. With 100 entries × multiple sections each, that is ~100 clones of multi-section jobs per frame just to hand them to egui.

**Suggested fix**: replace `Label::new(job.clone())` with egui's text-primitive path that borrows:

```rust
let galley = ui.fonts(|f| f.layout_job(job));       // jobs are &'static once cached
ui.painter().galley(pos, galley, Color32::WHITE);    // or push_text/allocate galley
```

Even better: keep the `LayoutJob` in the entry struct but cache the **laid-out `Arc<Galley>`** on the entry (re-layout only when the job's text/width changes — i.e. when the entry is appended). Entries are append-only, so a galley computed at insert time stays valid forever (width fixed by the chatbox inner width; invalidate on width change). This removes both the per-frame clone and the per-frame layout of unchanged text.

**Impact estimate**: ~100 job clones + ~100 layout passes per frame at a full chatbox; the layout pass is the more expensive part (font shaping per section). Fix removes both.

---

### F5 — Chatbox formats the timestamp string every frame
**File**: `src/ui/ui_chatbox_system.rs:112-113`

```rust
let now = chrono::Local::now();
let timestamp = format!("[{:02}:{:02}] ", now.hour(), now.minute());
```

Computed unconditionally each frame, even though chat entries are append-only and the timestamp only changes once per minute. The resulting string is re-sent to `ui.text_edit_singleline(&mut timestamp)`-style editing below (line ~114) — meaning the **user's in-progress typed text is also rebuilt from this string every frame**.

**Suggested fix**: compute lazily — cache `(last_minute, timestamp_string)` in the `UiStateChatbox` resource (or `Local`) and only reformat when `(hour, minute)` changes. When the string hasn't changed and the user is editing, the cached value also avoids re-triggering edit-state churn.

**Impact estimate**: one `Local::now()` + `format!` per frame, plus an edit-buffer rebuild; cheap individually, but it is a steady per-frame alloc and it interacts badly with text editing (see F4's cousin). One-minute caching removes it entirely.

---

### F6 — Button widget re-lays-out its label galley every frame
**File**: `src/ui/widgets/button.rs:146-178`

`Button::get_label` builds a `LayoutJob` (font id, color, alignment — the "NOT READY" / quantity strings) and calls `ui.fonts(|f| f.layout_job(job))` (line ~171) **on every frame for every button** — even when the label text is a constant like "NOT READY". Many dialogs (character creation, login, stores) hold dozens of buttons.

**Suggested fix**: cache the `Arc<Galley>` on the button widget keyed by the label string + width; invalidate when `label_text` or width changes. Since `Widget::draw` receives the same `Dialog`-owned `Widget` instance each frame (dialogs are cached in `Assets<Dialog>`), adding a `galley_cache: Option<(String, f32, Arc<Galley>)>` to `Widget` works without any caller changes.

**Impact estimate**: dozens of text-layout passes/frame across dialogs; each layout pass is font-shaping work. Cache removes all but the first.

---

### F7 — Minimap re-does sprite lookups for every map icon and iterates every NPC in the zone every frame
**File**: `src/ui/ui_minimap_system.rs:615-689`

The NPC/player icon loop runs every frame while the minimap is open:
- per icon: `get_sprite_by_index(sprite_index)` (`:672-677`) — a **linear scan over `ui_resources.ui_sprites`** (hundreds–thousands of entries) with per-candidate string allocation (`format!("{sprite_name}.tga")` + `(map_name, sprite_index)` tuple construction), building a `HashMap` key per icon;
- `get_minimap_player_sprite` (`:724`) does the same per frame for the player marker;
- the outer loop (`:663-690`) iterates **all NPCs in the zone** (`npcs.iter()`) — no culling to map bounds or viewport; every NPC pays the distance check and (if inside) the lookup above.

**Suggested fix**: (1) cache the sprite handle for each `(map_name, sprite_index)` in a `HashMap<(u32, u32), (TextureId, Rect)>` built lazily once per zone load (the codebase's own one-time-lookup pattern at `ui_selected_target_system.rs:59-60`); (2) pre-filter NPCs into the map bounds before the per-NPC work, and skip NPCs whose map position cannot be within the minimap viewport; (3) re-use the existing per-frame `is_minimap_visible`/zoom gate to early-out the loop when the minimap is closed.

**Impact estimate**: 100–300 HashMap-key allocations + linear sprite-scan per NPC per frame; the sprite scan is the dominant cost (it is O(NPCs × sprites) in the worst case). Both fix items remove it.

---

### F8 — Minimap rebuilds its zoom and coordinate strings every frame
**File**: `src/ui/ui_minimap_system.rs:878` (zoom label), `:924` (coordinates label)

```rust
format!("{:.1}x", minimap_state.zoom)          // :878 — changes only on zoom input
format!("{:?}", player_transform.translation)  // :924 — full Debug of Vec3, changes every frame
```

`format!("{:?}", Vec3)` per frame is a real but small cost; the zoom string is pure waste on frames without zoom input. Additionally the zoom label should show the *rounded* zoom the user perceives.

**Suggested fix**: cache the zoom string in `UiStateMinimap` updated only when `zoom` changes; for the coordinates, format with fixed precision (`format!("({:.0}, {:.0}, {:.0})")`) — same output, cheaper formatting and stable width (a fixed-width label also stops the window from jittering as numbers grow/shrink).

**Impact estimate**: 1–2 `format!` allocations/frame; trivial CPU but an easy quick win and a nice UX side-effect (stable window width).

---

### F9 — Cooldown radial mesh is re-created every frame for every hotbar slot
**File**: `src/ui/drag_and_drop_slot.rs:214-302` (build), called from `:365-369`

`draw_drag_and_drop_slot` builds the cooldown "sweep" visual as a full vertex fan: `Vec::with_capacity(fan_steps)` + per-vertex `sin`/`cos` + push — a fresh mesh per slot per frame while any cooldown is active (hotbar, skill list, inventory pots). No reuse of the vertex buffer across frames even when the cooldown fraction is unchanged.

**Suggested fix**: keep a cached `Mesh` on the slot/state keyed by `(skill_id, cooldown_fraction)` rounded to e.g. 1 % buckets — the visual difference below 1 % is imperceptible, so the mesh is rebuilt at most 100 times per skill lifetime instead of every frame. Simpler alternative: cache the previous fraction and skip rebuilding when `abs(new - old) < 0.01` (the common case while a long cooldown ticks).

**Impact estimate**: N slots × ~30–60 trig calls + mesh alloc per frame during cooldowns; with 15 hotbar slots + skill list open ≈ 20–60 meshes/frame. Removed by the fraction-delta check.

---

### F10 — Item quantity galley is re-laid-out every frame per slot
**File**: `src/ui/drag_and_drop_slot.rs:372-405`

For every slot with `quantity > 1`, the code formats `"{quantity}"` and runs a `LayoutJob` + `layout_job` pass every frame, then draws the galley. Quantity changes rarely (only on pickup/split/use).

**Suggested fix**: cache the galley on the slot keyed by `(quantity_string, slot_width)`; re-layout only when the string or width changes. The format itself should also skip the allocation when quantity is unchanged.

**Impact estimate**: ~10–60 small layout passes/frame (inventory + hotbar); each is a font-shaping call. Removed by caching.

---

### F11 — Every inventory/hotbar slot clones the full `Item` every frame to draw it
**Files**: `src/ui/drag_and_drop_slot.rs:79-150` (`with_item`/`item` field), clone at `src/ui/ui_inventory_system.rs:181` (per slot per frame)

The per-frame slot builder does `item: Some(item.clone())` — a full `Item` struct clone (String names, `EquipmentData` enum with model strings, socket lists, etc.) for each of up to 48 inventory slots + hotbar, and `with_item` performs sprite lookups per slot (`get_item_sprite` at drag_and_drop_slot.rs:~100). None of the drawn data (icon, name) changes between frames.

**Suggested fix**: store `&Item`-derived data instead: `slot.item = item` requires the drag system to own the item only while dragging (it already needs the item at drop time — move the clone to the drag-start path). For drawing, cache `(sprite_id, uv, quantity)` per slot id in a `Local<HashMap<u32, SlotVisual>>` invalidated when the inventory version changes — the codebase has no inventory version counter, so either add a tiny `ItemChangeStamp` incremented on inventory mutations, or key the cache by `(slot_id, item.id(), item.quantity)`.

**Impact estimate**: 30–60 `Item` clones + sprite lookups per frame; clones include heap strings. Fix removes the clone and moves lookups to change-time.

---

### F12 — Admin menu recompiles the filter regex and recomputes the filtered list every frame
**File**: `src/ui/ui_admin_menu_system.rs:434-443` (recompute), `:614-631` (regex)

The filter logic (`filter_items` / `filter_skills`) rebuilds the filtered `Vec<u16>`/`Vec<SkillId>` **every frame** the filter text is non-empty, and constructs `Regex::new(...)` for the name match on every recompute (`:614-631`). Regex compilation is a heavyweight step (builds the DFA); doing it per frame while typing is measurable. The `filtered_*` caches already exist in `UiStateAdminMenu` (struct at lines 18–50) — but the recompute is unconditional rather than keyed on the filter string.

**Suggested fix**: cache `(filter_string, last_filtered_vec)` in `UiStateAdminMenu`; recompute only when the string changes. Move `Regex` construction out of the loop: build it once when the filter changes (store `Option<Regex>` in the resource), and reuse it for the whole frame (also for F13's per-row calls).

**Impact estimate**: a regex build + full-list scan every frame while filtering; with ~2–5k items and skills this is several hundred µs/frame of pure waste between keystrokes. Change-driven recompute drops it to keystroke-time.

---

### F13 — Admin menu popups render the entire catalog per frame with no virtualization
**File**: `src/ui/ui_admin_menu_system.rs:446-467` (item popup), `:524-569` (item rows), `:571-612` (skill rows)

The two `SelectableLabel` loops (`for (i, item) in filtered_items.iter().enumerate()`) render **every** row every frame — for the unfiltered item catalog that is ~2–3k rows, each doing `get_item_static_reference`, per-row `format!` of name + level string, and a full row layout. The skill list likewise renders all skills. There is no `scroll_to`/virtualized visible-window logic; the whole list is drawn into the `ScrollArea`'s `show_rows`-less loop.

**Suggested fix**: use `ScrollArea::show_rows` (egui's built-in virtualization: takes `total_rows` and renders only visible rows — it exists precisely for this) or cap with `ScrollArea::vertical().max_height(...).show_rows(ui, row_height, total, |ui, range| …)`. Combined with F12's cached filter, row rendering only touches the ~20–40 visible rows per frame.

**Impact estimate**: thousands of SelectableLabels + formats + layouts per frame while the admin menu is open; virtualization cuts it to ~visible-row count. This is the single largest single-frame cost in the admin window.

---

### F14 — Status effects window: `Instant::now()` per active effect + unused `Time` param
**File**: `src/ui/ui_status_effects_system.rs:19` (unused `Time` param), `:29` (window title), `:45` (`Instant::now()` inside the effect loop)

The system takes `time: Res<Time>` but uses `Instant::now()` per effect per frame instead (`:45`), paying a syscall/clock-read per active effect (up to ~20 status icons). The window title string also contains a typo (`Player Status Effects}` — mismatched brace) — cosmetic, but while touching this file it is a one-line fix.

**Suggested fix**: use the already-available `time.elapsed()` once per frame (or `Duration` from `time`), pass it to the expire/alpha computations; drop `Instant::now()`. Fix the title string.

**Impact estimate**: ~1–20 clock reads/frame; negligible CPU but the fix is trivial and removes a needless per-effect syscall in a window that runs always-on (see F19).

---

### F15 — Quest scroll dialog re-lays-out its title and description every frame
**File**: `src/ui/ui_quest_scroll_system.rs:159-168`

`ActiveQuestScrollDialog` already caches two `LayoutJob`s (`title_layout_job`, `description_layout_job` in the struct), but the draw code runs `ui.fonts(|f| f.layout_job(job))` on both **every frame** instead of at job construction, and calls `dialog_assets.get_mut(&ui_resources.dialog_message_box)` per frame (a mutable borrow of the asset just to read a rect). The layout results are the same every frame (fixed dialog size).

**Suggested fix**: lay out both jobs once when the dialog opens (store `Arc<Galley>`s alongside the jobs) and draw the cached galleys; change `get_mut` to `get` (immutable read is sufficient for drawing).

**Impact estimate**: 2 font-shaping passes + a mut-asset lookup per frame while the scroll is open; trivial absolute, but free and localized.

---

### F16 — Sailing HUD performs a full shore search every frame while in a boat
**Files**: `src/ui/ui_sailing_hud_system.rs:261-267` (caller), `src/systems/boat_spawn_system.rs:64-103` (`find_nearest_shore_position`)

While sailing, the HUD calls `find_nearest_shore_position` every frame. That function walks `SHORE_SEARCH_STEP_CM=200` to `SHORE_SEARCH_MAX_CM=2000` (10 steps) × 8 directions (`boat_spawn_system.rs:69-78`) = **80 terrain-height samples per frame**, each a `get_terrain_height` lookup. Boats are slow; the result changes rarely.

**Suggested fix**: cache the result in `UiStateSailingHud` and recompute at most every ~0.25–0.5 s, or only when the player moved more than ~100 cm since the last computation (hysteresis so the arrow doesn't flicker). Trivial: store `(last_position, last_sample_time, last_direction)` on the HUD state.

**Impact estimate**: 80 terrain samples/frame → ~0. That is a real per-frame cost while sailing (terrain heightmap access is not free) and the fix is 5 lines.

---

### F17 — Listbox/zlistbox draw every row every frame, with per-row string/mesh allocations
**Files**: `src/ui/widgets/listbox.rs:69-105`, `src/ui/widgets/zlistbox.rs:50-61`

`Listbox::draw_list_items` loops over the full extent (bank items, npc store sell list, quest lists) and per row calls `get_item_text(i)` (string per row per frame), `selectable_label` or painter text with `allocate_painter` (a mesh per row), plus per-row `format!`s. `ZListbox` (character creation) does the same for its 10+ rows. There is **no row virtualization** — every row of a 100-item list is laid out and drawn each frame.

Also noted: `listbox.rs` calls `get_item_text(i)` with the **loop index** (`i`), not `scroll_index + i`, while `zlistbox.rs` correctly passes `scroll_index + i` — if `get_item_text` is ever intended to index into a scrollable data set, the listbox version is an off-by-scroll bug waiting to happen (currently the bank/store bindings return full lists and the scroll index is applied inside `draw_list_items`, so behavior is correct — but the pairing is fragile).

**Suggested fix**: switch the loop to `ScrollArea::show_rows` (virtualization) or, for the always-short lists, at least reuse a single scratch `String`/mesh per draw pass. Add a comment at `get_item_text(i)` documenting that `i` must be `scroll_index + row` if the data set scrolls (or normalize the contract so both widgets pass the same index).

**Impact estimate**: ~50–150 rows × (string + mesh + layout) per frame with stores open; virtualization cuts it to ~10 visible rows.

---

### F18 — Editbox builds an unbound-string `format!` every frame
**File**: `src/ui/widgets/editbox.rs:60`

```rust
let mut unbound_buffer = format!("<{} unbound>", self.id);
```

Re-formatted every frame even though the string only changes when the editbox's binding appears/disappears. The string is also passed by value into the text-edit widget, forcing an edit-state rebuild.

**Suggested fix**: compute it once (on the `Widget::draw` for that id — the widget instance is cached in the `Dialog`) and reuse; or cache in the dialog's instance state.

**Impact estimate**: 1 small alloc/frame per editbox (character creation has ~8); trivial but an easy win and avoids edit-buffer churn while typing.

---

### F19 — Several always-on UI systems fetch heavy `SystemParam`s and draw every frame with no run condition
**File**: `src/lib.rs:1576-1620` (registrations in `EguiPrimaryContextPass`); worst case `src/ui/ui_settings_system.rs:176-201` (25-parameter `SystemParam` struct, run every frame in Game state) and `src/ui/ui_status_effects_system.rs` (always draws its window)

The egui systems are registered without `run_if` conditions on visibility: `ui_settings_system` (a 25-param system: `ResMut<Assets<...>>` ×6, `ResMut` settings ×6, etc. — all fetched and partially read every frame), `ui_status_effects_system`, `ui_window_sound_system`, `ui_sound_event_system`, `ui_debug_menu_system` and the tooltip/name-tag systems run **every frame** even when their windows are closed. `ui_settings_system` in particular mutably borrows six settings resources each frame for the always-drawn "menu" head (settings menu closed = still pays resource fetch + initial if-checks; its actual menu only draws when open, but the system-wide `SystemParam` fetch and the sound/input plumbing happen regardless).

**Suggested fix**: add cheap run conditions so closed windows don't pay system-param assembly:
- `ui_settings_system` → run only when the game menu / settings window is open (`run_if(resource_exists_and_changed<UiStateGameMenu>)` or a `local_window_open`-style check — the pattern exists for the debug windows which gate on `debug_ui_open`),
- `ui_status_effects_system` → run only when the status window is open,
- `ui_window_sound_system` → `run_if` any of its dialogs is open (it already diffs `UiStateWindows` every frame; a `Changed`-style condition on the resource would do),
- keep `ui_sound_event_system` always-on (it is the cheap event relay).

The 25-param `SystemParam` fetch itself is the notable cost: `ResMut<Assets<Dialog>>` etc. trigger archetype/resource lock acquire per frame. Gating removes it.

**Impact estimate**: several unused-but-fetched resource locks + always-drawn windows; gating is free and idiomatic Bevy (the debug windows already prove the pattern works in this codebase).

---

### F20 — Item/skill tooltips re-run their full content builder on every hover frame
**File**: `src/ui/tooltips.rs:606-613` (`on_hover_ui` wrapper), content builders `:286-604` (item), `:1152-1269` (skill)

`ui_add_item_tooltip` runs `response.on_hover_ui(|ui| { … dozens of `format!`s, per-line sprite lookups, `get_item_static_reference` string constructions … })` — the closure re-executes **every frame the mouse hovers**, and it is invoked not just from the inventory but from every slot draw (hotbar, stores, personal store, quest rewards). Static item tooltips (name, description, stats) produce identical content each frame; only the few dynamic lines (weight, ability check colors, `get_ability_value`) change with player state.

**Suggested fix**: two-tier cache keyed by `(item.id(), item.quantity)`:
- static galley: lay out name + description + stat lines once per item into cached galleys (invalidated on inventory version change — same stamp as F11);
- dynamic lines: only the 2–4 player-dependent rows are re-formatted per frame.
In egui 0.33 the standard lightweight alternative for pure-text tooltips is `response.on_hover_text(Arc::from("…"))` whose galley egui caches internally — use it where the tooltip is one string, and reserve the custom builder for rich tooltips.

**Impact estimate**: per-hover-frame, roughly 20–40 `format!`s + several font layouts + sprite lookups; with hotbar/inventory visible the hover target changes rarely, but each frame while hovering pays the whole build. Caching reduces it to the dynamic lines only.

---

### F21 — Name-tag system scans all entities to check pending work every frame
**File**: `src/systems/name_tag_system.rs:410-416`

When the pending cache is non-empty, the system iterates `query_add` (`Query<&NameTagName, Added<NameTagName>>`) and calls `iter().len()` — a full scan over all added names — to decide whether to update `NameTagCache`. The len() is only used as a count; the actual insert loop then iterates the same query. When many NPCs/players have tags, `iter().len()` walks every matching entity once, then the insert loop walks the same entities again (the scan is `Added`-gated, so steady state is empty — the cost is bounded to tag-insertion frames, but doubled work on those frames).

**Suggested fix**: `for name in query_add.iter()` in one pass: count and insert in the same loop (or use `query_add.iter_mut().count()` only if the count is truly needed before inserting — it isn't; the later loop re-iterates anyway).

**Impact estimate**: one extra archetype-wide iteration per entity at spawn time; Low — but a one-line fix (`for` over `iter()` and insert inline).

---

### F22 — Skill tree re-creates every node slot (Box closure + sprite lookups + mesh) every frame
**File**: `src/ui/ui_skill_tree_system.rs:32-117` (node draw), recursive `draw_skill_slots` `:119-150`

The skill tree is a recursive walk over the full skill tree data with a `DragAndDropSlot::new(...)` per node per frame (see F2), per-node `get_sprite_by_index` lookups (F1/F7 pattern), and per-frame `format!` for level strings (`:100-110`). For a 150–300 node tree that is the same count of Box allocations + sprite linear scans + mesh allocs per frame, all while the tree is open.

**Suggested fix**: apply F1/F2 fixes (cached sprites, enum-based slot policy) which cover the tree automatically; add a `Local<HashMap<skill_id, (sprite, level_string)>>` cache if the tree stays expensive. Tree topology changes only on server data, so per-frame rebuild is entirely redundant.

**Impact estimate**: hundreds of allocs + linear sprite scans per frame while the tree is open; fixed by the shared-slot refactor (F2) + sprite cache (F1).

---

### F23 — (Cross-reference, no new work) World-space UI text re-uploads every frame
**Files**: `src/render/world_ui.rs` (extract/queue/render), `src/systems/name_tag_system.rs`, `src/systems/chat_bubble_spawn_system.rs`

Name tags and chat bubbles are baked from egui `Arc<Galley>`s into textures and drawn by the custom `WorldUiBatch` pipeline, which re-uploads vertex data and recreates the view bind group per frame (covered in detail in report 05, findings F~World UI). The galley *creation* itself is already cached (keyed by string with zone/DPI invalidation, `name_tag_system.rs:383-397`) — good. Only noted here for completeness of the UI review; no duplicate recommendation.

---

### Positive patterns to preserve (verified in source)

- **No repaint abuse**: zero `request_repaint`/`ctx.request_repaint` calls in `src/`; repaints flow only from egui's own change-driven flags via the callback registered at `bevy_egui-0.39.1/src/input.rs:200`. Keep it that way; never call `request_repaint` from the per-frame systems.
- **One-time sprite caching**: `ui_selected_target_system.rs:59-60` (sprite handles cached in `Local`), `ui_minimap_system.rs:294-307` (zone-name galley cached with zone/DPI invalidation), `name_tag_system.rs:383-397` (string-keyed galley cache with invalidation).
- **Dialog instance caching** in `dialog_loader` — dialogs are loaded once per XML asset and reused; `DialogInstance::get_mut` caches instance data.
- **Admin menu early-return** (`ui_admin_menu_system.rs:103`) and **debug windows gated by `debug_ui_open`** — the gating model F19 asks to extend.
- **`ui_resources.rs` update early-return** — the sprite-table rebuild runs only when the texture asset actually changed.
- **`get_mut`-free drawing**: most dialog draws read assets immutably; the few `get_mut` call sites are F15's.
- **DataBindings as a compact query-local struct** with small fixed-size arrays (`ui/widgets/data_bindings.rs:11-43`) — linear scans over ≤ ~15 entries per widget are fine; do not "optimize" these into hash maps.

---

## 4. Priority Table

| # | Finding | File:line | Impact | Effort | Priority |
|---|---|---|---|---|---|
| F2 | DnD slot Box-closure alloc per frame | `ui/drag_and_drop_slot.rs:34,62` | High (100–1000 allocs/frame) | Med (touches 8 call sites) | **High** |
| F7 | Minimap per-frame sprite scans + full-zone NPC loop | `ui_minimap_system.rs:615-689` | High | Med | **High** |
| F13 | Admin popup renders whole catalog per frame | `ui_admin_menu_system.rs:446-612` | High (2–3k rows) | Low (show_rows) | **High** |
| F1 | Per-call Mesh alloc in sprite draw | `ui_resources.rs:27-38` | High (150–400 meshes/frame) | Low (cache in UiSprite) | **High** |
| F19 | Always-on systems + 25-param fetch | `lib.rs:1576-1620`, `ui_settings_system.rs:176-201` | Med | Low (run_if) | **High** |
| F3 | Chatbox dead Style clone | `ui_chatbox_system.rs:177-183` | Med (pure dead work) | Trivial (delete) | **High (quick win)** |
| F4 | Chatbox LayoutJob clone+layout per entry | `ui_chatbox_system.rs:259` | Med | Med | Med |
| F9 | Cooldown fan mesh per frame | `drag_and_drop_slot.rs:214-302` | Med | Low (delta check) | Med |
| F11 | Per-slot Item clone + lookups | `drag_and_drop_slot.rs:79-150`, `ui_inventory_system.rs:181` | Med | Med (stamp cache) | Med |
| F12 | Admin regex recompile per frame | `ui_admin_menu_system.rs:614-631` | Med (while typing) | Low (cache) | **Med (quick win)** |
| F16 | Shore search per frame while sailing | `ui_sailing_hud_system.rs:261-267`, `boat_spawn_system.rs:64-103` | Med (80 terrain samples/frame) | Low (time/pos gate) | Med |
| F17 | Listbox rows all rendered + per-row allocs | `listbox.rs:69-105`, `zlistbox.rs:50-61` | Med | Low (show_rows) | Med |
| F20 | Tooltips rebuilt every hover frame | `tooltips.rs:606-613` | Med | Med (two-tier cache) | Med |
| F22 | Skill tree per-node slot/lookup/mesh | `ui_skill_tree_system.rs:32-150` | Med | Low (inherits F1/F2) | Med |
| F6 | Button label re-layout per frame | `button.rs:146-178` | Low | Low (cache galley) | Low |
| F10 | Quantity galley per frame | `drag_and_drop_slot.rs:372-405` | Low | Low (cache) | Low |
| F5 | Chatbox timestamp format per frame | `ui_chatbox_system.rs:112-113` | Low | Trivial (cache minute) | **Low (quick win)** |
| F8 | Minimap zoom/coord format per frame | `ui_minimap_system.rs:878,924` | Low | Trivial | **Low (quick win)** |
| F18 | Editbox unbound format per frame | `editbox.rs:60` | Low | Trivial | **Low (quick win)** |
| F14 | Instant::now per effect + title typo | `ui_status_effects_system.rs:19,29,45` | Low | Trivial | **Low (quick win)** |
| F15 | Quest scroll re-layout + get_mut | `ui_quest_scroll_system.rs:159-168` | Low | Low | Low |
| F21 | Name-tag double query iteration | `name_tag_system.rs:410-416` | Low | Trivial | Low |

## 5. Quick Wins

1. **F3** — delete the dead `Style` clone in the chatbox (`ui_chatbox_system.rs:177-183`). No behavior change, saves a full style-stack clone per frame.
2. **F5** — cache the chatbox timestamp string per minute (`ui_chatbox_system.rs:112-113`); also stabilizes the in-progress edit buffer.
3. **F12** — cache `(filter, Regex, filtered_vec)` in `UiStateAdminMenu`; recompute on string change only.
4. **F8** — cache minimap zoom label; fixed-precision coordinate format.
5. **F18** — hoist the `<{id} unbound>` string out of the per-frame editbox path.
6. **F14** — replace per-effect `Instant::now()` with `time.elapsed()` (the `Time` param is already in the signature, currently unused) and fix the title brace.
7. **F21** — fold the `iter().len()` pass into the insert loop.

## 6. Risks & Validation Notes

- **Performance claims are static-analysis estimates** — none of the above was benchmarked. Before/after profiling (e.g. `tracy`/`perf`/`optick` on the UI pass) is required before shipping any of the High items; the report is research-only by design.
- **Galley caching invalidation**: any cache of laid-out text must account for (a) DPI/window scale changes, (b) font scale / `pixels_per_point` changes via the settings UI, (c) dialog width changes. The existing `name_tag`/minimap caches already implement zone/DPI invalidation — reuse those exact patterns. A stale-name-tag-width bug in chat bubbles has been fixed before (per `chat-bubble-and-name-tag-architecture.md`); the chatbox (F4) cache must not reintroduce it.
- **Virtualization correctness (F13/F17)**: `show_rows` requires a constant row height and correct `total_rows`; the admin rows and store lists currently vary row height (name + level on separate lines). Must either fix row heights first or virtualize with a known height. Also keep drag-and-drop start offsets consistent with the virtualized index math (the DnD system reads the slot rects, which are only created for visible rows — invisible-row drops must remain impossible, matching current behavior).
- **F2 enum refactor touches every drop target** (inventory, hotbar, skill tree, bank, npc store, personal store, quest list). The `accepts` closures currently encode per-window rules; move them into `ui_drag_and_drop_system` dispatch and port the existing `#[cfg(test)]` slot tests (the DnD module has test helpers) before removing `Box<dyn Fn>`.
- **Tooltip caching (F20)** must not cache player-dependent content (ability colors, weight, usable-state text) — cache only the static galley and re-run dynamic lines per frame. A cached-but-stale stat tooltip is a worse bug than the per-frame cost it removes.
- **Minimap NPC cache (F7)** must invalidate on NPC despawn/respawn and visibility changes (NPCs are despawned/spawned per zone region); keying by `(npc_id, sprite_index)` with a zone-load reset is safe.
- **Do not "optimize" the small DataBindings linear scans or `Dialog::get_widget` linear search** — arrays are ≤ ~15 entries and the scan is dwarfed by the allocations above; the codebase convention is fine.
- **Keep the repaint policy as-is** (no app-side `request_repaint`). If the UI ever needs event-driven refreshes (e.g. after a long delay without egui events), that is a deliberate feature decision, not a fix — egui only repaints when something requests it, so a dormant frame costs nothing today.
- The chatbox timestamp cache (F5) must not skip the minute rollover while the game is minimized; a wall-clock comparison on repaint is sufficient.

---

## 7. Verification Update (2026-08-04)

Independent sub-agent scrutiny of every finding (F1–F23) against the actual source, egui 0.33 / bevy_egui 0.39.1, and the previous implementation attempt on `wip/local-changes-2026-08-04` (validated in `12-validation.md`). Verdicts per finding:

| Finding | Verdict | Scrutiny result / action |
|---|---|---|
| F1 | PARTIAL (defer) | Mechanism real (fresh `Mesh` = 2 small Vec allocs per call, ~25 draw sites + ~10 inline builds; ~300–800 tiny allocs/frame ≈ 20–60 µs) — but **not** "one of the top UI costs" (tessellator merges shapes; bevy_egui uploads ONE vertex+index buffer per frame — no per-sprite GPU upload). The fix as designed is **infeasible**: `UiSprite` is a `Copy` value type re-fetched per frame (no place for a cache); a rect-size key is broken (vertex positions vary per call/frame); `Painter::add` consumes the mesh so a cached `Mesh` must be cloned anyway (same allocs). Doc corrections: 4 verts/6 indices (not 6/4); no `allocate_painter`. Only viable design (batch per (clip_rect, texture)) is high-effort/high-regression for <0.5% of a frame — **defer; the wip `reserve(4)/reserve(6)` tweak is a harmless no-op**. |
| F2 | PARTIAL (already fixed on wip) | `accepts: Box<dyn Fn>` exists, but the doc describes an architecture that never existed: no `on_drop` field, no `DragAndDropPayload`, no `request_drop` path; closures are `'static` with zero captures except the inventory closure (8 bytes × 44 slots ≈ 44 allocs/frame — not 100–1000; ZST-closure Boxes don't allocate). **Already implemented on wip** as `enum SlotAccept` (validated OK, byte-for-byte verified) — merge it; drop the "active-drag closure" half (unnecessary — accepts is invoked inline during draw). Correct the doc's F2 text. |
| F3 | PARTIAL (already fixed on wip) | Dead clone confirmed (mutations never consumed by anything; `chatbox_style` referenced only at 177–181) — deletion cannot change output. Doc errors: there is **no `set_style` call in the file**; egui 0.33 `set_style` is an Arc swap, not a stack push. **Deleted on wip** — merge. |
| F4 | PARTIAL (downgrade + reject as written) | Doc structurally wrong: `UiStateChatbox` holds **one** `LayoutJob` for all entries (not ~100); egui 0.33's `GalleyCache` retains across frames, so identical chat text is not re-shaped (real cost ≈ 2–4 µs/frame: one clone + one hash). The doc's snippet **doesn't compile** (`layout_job` needs `&mut`, takes owned job; the painter path breaks ScrollArea sizing/stick-to-bottom). Viable form if ever wanted: one cached `Arc<Galley>` + `ui.label(WidgetText::Galley(...))`, invalidated on (job append, width, ppp). Downgrade to Low; not a priority. |
| F5 | PARTIAL (already fixed on wip — differently) | `Local::now()` per frame TRUE; but the code uses `%H:%M:%S` (changes every second — the minute-cache is **unsafe**, would freeze seconds), and there is **no `text_edit_singleline`** (the edit-buffer claim is false). **Already fixed on wip** by moving the clock read into the event loop (per-event) — strictly better and behavior-identical; merge. Do NOT implement the minute-cache. |
| F6 | PARTIAL (reject) | Mechanism real, but egui 0.33's galley cache is per-frame (full re-layout per frame per button — one agent disputed this vs F4's cache-hit claim; resolution: F6's frame-scoped-flush reading is correct, F4's chatbox win is the retained-previous-frame case) — regardless, the scope is ~10× overstated: only 3 systems set button labels (max 9 labeled buttons; "NOT READY" doesn't exist in the repo; character creation/login set zero labels). Worst case ≈ 10–50 µs/frame. Also "without any caller changes" is false — the draw chain is `&self`; a cache needs `RefCell` interior mutability. **Reject** as an optimization. |
| F7 | PARTIAL (already fixed on wip; impact downgraded) | The core mechanism is **refuted**: `get_sprite_by_index` is O(1) direct `Vec::get` — no linear scan, no per-candidate `format!`, no `(map_name, sprite_index)` key; the cost is O(NPCs), not O(NPCs×sprites). The all-NPC iteration is real (gated only by `!minimised`). **Already implemented on wip** (NpcId-keyed icon cache + bounds pre-cull + zone reset + `loaded_all_textures` guard — the guard is REQUIRED to avoid permanently caching `None` before textures load). Merge wip; correct the doc (impact High → low). |
| F8 | PARTIAL (zoom on wip; coords refuted) | Zoom half accurate and **already cached on wip** — but the wip file contains **mojibake** (`"≡ƒöì {:.1}x"` instead of `"🔍 {:.1}x"`) — repair on merge. Coords half **refuted**: the code already formats fixed-width `{:0>4}` i32s; there is no `{:?}` Debug — the suggested `({:.0}, {:.0}, {:.0})` change would alter visible output. Do not apply the coords change. |
| F9 | PARTIAL (defer) | Per-frame rebuild TRUE; the "sin/cos + with_capacity fan" description is **false** (fixed 9-vertex octagon, zero trig); impact ~10× overstated (single-digit µs/frame; 0.167%/frame fraction change, not 1.7%). If ever touched: per-slot delta guard (`|Δ| ≥ 0.01`) building the mesh origin-relative + translate per frame (screen-space staleness trap), rect size in the key, `Hash/Eq` derives on `DragAndDropId` for the `Local<HashMap>`. Defer. |
| F10 | PARTIAL (reject) | Mechanics real but egui 0.33 **memoizes layout** (shaping runs once per unique quantity string — the "font-shaping call per frame" claim is wrong); `slot_width` is a dead key component (`layout_no_wrap` wraps at infinity); no persistent per-slot storage exists; a safe key must include `pixels_per_point`. Fix duplicates egui's own cache — **reject/defer indefinitely**. |
| F11 | PARTIAL (implement-with-changes, Low) | The clone exists but "full Item with Strings" is **false**: `Item` is a ~24-byte Copy enum (`ItemReference` + primitives), zero heap; strings live only in the static DB. Worst case ~52 slots, 0 when the window is closed — a few µs/frame. The SlotVisual cache is **unsound** (key misses life/socket/gem — stale icons after repair/gem; `ItemReference` lacks `Hash`; no version stamp exists in the codebase). Correct minimal fix: borrow-elimination via a `GetItem` borrowed view (~5 call sites). Low priority; deferring is defensible. |
| F12 | PARTIAL (already fixed on wip) | Doc misdescribes: recompute is **emptiness-gated** — it only runs every frame when the filter matches *nothing* (the empty vec can't distinguish "not computed" from "zero results" — the real bug); regex is recompiled per frame only in that state (~10–50 µs + 50–200 µs scan). **Already implemented on wip** (filter-key cache keyed on (ItemType, String) — the ItemType part is required to avoid stale lists on tab switch). Regex hoist unnecessary. Merge wip. |
| F13 | PARTIAL (already fixed on wip) | Core confirmed — and the skill popup renders **6,307 rows/frame** (measured from data STB headers); item popup is per-tab 217–1,212. Details wrong: Grid cells (not SelectableLabels), no level strings. **Already implemented on wip** with `egui_extras::TableBuilder` + `body.rows(34.0)` (correct constant-height virtualization; the doc's `show_rows` + Grid sketch would break on spacing drift). Add `RichText::truncate()` for the name column (fixed-width Column overlaps long names — real risk in the wip). Merge wip + truncation. |
| F14 | PARTIAL (already fixed on wip — better form) | Claims confirmed (up to 36 effect-type slots; title typo; unused `Time`). **Already fixed on wip** by hoisting `Instant::now()` once per frame + title fix + param removal. The doc's `time.elapsed()` form is inferior: type mismatch vs `Instant` expire_times, and `Res<Time>` diverges from wall clock during >250 ms stalls or future pause — use the wip form. Merge. |
| F15 | PARTIAL (implement-with-changes, Low) | Per-frame layout confirmed (egui's cache is per-frame only — full re-layout+shaping each frame). The `get_mut → get` suggestion is **refuted**: the code genuinely mutates widget positions every frame — and the doc missed the real cost: the system mutates the **shared MSGBOX.XML asset** every frame → `AssetEvent::Modified` → `load_dialog_sprites_system` re-runs the whole dialog sprite reload every frame (the biggest cost here). Correct fix: clone the dialog into an owned `DialogInstance` at open (existing pattern) + cache galleys with a ppp guard. Also present in `ui_message_box_system` and `conversation_dialog_system` — fix the pattern, not just the quest scroll. |
| F16 | CONFIRM (implement-with-changes) | Accurate (80 samples/frame; used only as `.is_some()` for an advisory prompt). `UiStateSailingHud` **does not exist** — use `Local<SailingShoreCache>` keyed on (zone id, position moved >100 cm, water-height delta >100 cm, 0.5 s). Verified safe: 0.5 s staleness = 5 m ≪ 20 m detection radius; disembark recomputes fresh on keypress. Optional: early-return once a shore ring is found in the search loop. |
| F17 | PARTIAL (reject as written + separate bug found) | Core claim **wrong**: listbox already draws only the visible window (`extent` = widget's configured visible rows; breaks at `scroll_range.end`); bank/NPC-store are fixed grids, not listboxes; the only listbox user is server select (1–20 rows); quests use ZListbox (~8–12 rows). Impact ~10× overstated. `show_rows` rejected (already visible-window-only; would break row alignment). **Live bug found instead**: the quest ZListbox **double-applies the scroll offset** (`ui_quest_list_system.rs:175` `nth(index + current_scroll_index)` + absolute row y) — wrong quests/blank rows whenever the quest list scrolls. Fix that as a bug fix (not an optimization); add the index-contract comment at `listbox.rs:75`. |
| F18 | CONFIRM (already fixed on wip) | `format!` runs every frame even when bound (result discarded) — true. "Edit-state churn" claim **false** (egui `TextEditState` stores only cursor/undoer; identical content causes no churn). **Already fixed on wip** (format moved into the `None` branch) — merge. |
| F19 | PARTIAL (settings gate on wip; rest rejected) | 23 params (not 25), zero `Assets` params; the always-drawn body is inside the window closure (menu closed = empty head draw + param fetch). `UiStateGameMenu` **does not exist** — the settings gate keys on `UiStateWindows.settings_open` and is **already on wip** (correct). Status-effects gating is **unimplementable** — it is a deliberate always-on HUD with no open flag (reject; optional no-effects early-out). Window-sound gating rejected (saves 11 bool compares; change-gate defeatable). Merge wip settings gate; correct the doc. |
| F20 | PARTIAL (reject two-tier cache; defer) | Core confirmed (closure re-runs every hover frame; 8 item + 4 skill call sites; residual ~15–40 µs/hover frame). Errors: no sprite lookups in tooltips; `get_item_static_reference` doesn't exist; weight is static (only equip-requirement colors are player-dependent); egui's GalleyCache already makes identical strings cheap. The two-tier cache is **unsound as keyed**: `(item.id(), quantity)` misses grade/durability/life/gem state (mid-combat staleness — the doc's own risk note); the skill tooltip is mostly dynamic; no version stamp exists. `on_hover_text` gains nothing. Optional micro-win only: static-line string cache with colors re-evaluated per frame. |
| F21 | PARTIAL (already fixed on wip) | Filter misquoted (`Without<NameTagEntity>`, not `Added<NameTagName>` — steady state includes failed-texture retry entities); `iter().len()` is an **O(matched tables) metadata sum**, not an entity scan (Bevy 0.18); the pending-clear is a correctness guard. **Already fixed on wip** (fold-count + clear-after-loop, semantically equivalent). Merge; correct the doc (impact "negligible"). |
| F22 | PARTIAL (fold into F1/F2; already half-done) | Recursion + per-node `DragAndDropSlot::new` TRUE; but no `format!` exists in the file, sprite lookup is O(1) (not a "linear scan"), node counts are 29–49 per class (not 150–300), and only ~14–20 slots draw per frame. The F2 half (SlotAccept) is **already on wip**; the F1 mesh half is still pending. **Reject** the `Local<HashMap<skill_id,...>>` cache (stale with leveled skill ids — the displayed icon is `skill.id + learned_level - 1`). No independent tree work. |
| F23 | CONFIRM (cross-ref only) | Accurate: galley creation cached (string-keyed + zone/ppp invalidation verified); per-frame upload/bind-group churn belongs to 05-F9 (BufferId cache on wip, batch-entity leak, `bevy_default()` format fix — none are 06's scope). No work here. |

**Cross-cutting note for the whole doc set:** the vendored source folder `bevy-collection\bevy-0.18.1` is actually **0.19.0-dev** (its `Cargo.toml` declares `version = "0.19.0-dev"`); the client builds against crates.io Bevy 0.18.1. API citations verified only against that folder must be re-checked against 0.18.1 before implementing.
