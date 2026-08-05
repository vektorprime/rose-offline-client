# UI Tooltips Pitfalls

This document records UI tooltip-related issues.

---

## Inventory Tooltip Delay on Hover (Fixed 2026-08-04)

### Problem
Hovering over an item in the inventory (or any slot) took noticeably long for the item description tooltip to appear.

### Root Cause
The client used egui's default tooltip behavior. In egui 0.33.3, `style.interaction.tooltip_delay` defaults to **0.5 seconds**, and `show_tooltips_only_when_still` defaults to `true`. The tooltip only appears after the pointer has been still for the full delay, which feels sluggish in a game UI.

### Solution
In `src/lib.rs`, `setup_egui_fonts` (global egui init), reduce the delay to near-instant:
```rust
let ctx = egui_context.ctx_mut().unwrap();
let mut style = (*ctx.style()).clone();
style.interaction.tooltip_delay = 0.05;
ctx.set_style(style);
```
Keeping `show_tooltips_only_when_still = true` prevents flicker while sweeping the mouse across slots; 0.05s still feels instant once the pointer stops.

### Files Modified
- `src/lib.rs` - Global egui style: `interaction.tooltip_delay = 0.05` in `setup_egui_fonts`

### Lesson Learned
"Tooltip takes a while to show" in an egui app is almost always the built-in hover delay (`style.interaction.tooltip_delay`, default 0.5s), not slow tooltip rendering. Set the global style once at egui init; it applies to every `on_hover_ui` / `show_tooltip` call.
