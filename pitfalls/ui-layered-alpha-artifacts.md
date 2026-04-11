# UI Layered Alpha Artifacts (White Boxes)

## Issue
Some UI elements showed white/opaque boxes when layered (for example, login input area backgrounds and socket overlays on top of item icons).

## Root Cause
UI textures were being rendered in [`bevy_egui` rendering flow](../bevy-collection/bevy_egui-0.39.1/src/render/mod.rs:397) with premultiplied-alpha expectations, but loaded game UI textures were not premultiplied.

## Fix
- Added `premultiplied_alpha` tracking to [`UiTexture`](src/resources/ui_resources.rs:60).
- Added one-time alpha premultiplication in [`premultiply_image_alpha()`](src/resources/ui_resources.rs:346).
- Applied conversion during texture readiness in [`update_ui_resources()`](src/resources/ui_resources.rs:363), including skill-tree textures.
- Preserved transparent editbox visuals in [`Editbox::draw_widget()`](src/ui/widgets/editbox.rs:55).

## Validation
Confirmed by user: layered white-box artifact no longer appears.
