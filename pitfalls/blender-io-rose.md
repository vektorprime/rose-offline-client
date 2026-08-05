# Blender io_rose Addon Round-Trip Pitfalls

## Importer dropped texture alpha (wings rendered as black cards in Blender)

**Problem:** Imported ZSC items with alpha-cutout textures (e.g. BACK_WING12) showed the
texture's black background in Blender, although alpha works in game.

**Root cause:** `create_material` only linked the texture Alpha socket when
`zsc_mat.alpha != 1.0`. The game shaders (e.g. `wing_material.wgsl`, `alpha_discard`)
always sample the texture alpha channel; the ZSC flag `alpha_enabled` with `alpha == 1.0`
never triggered the link.

**Fix:** In `import_zsc.py` / `import_map.py` / `import_combined_zone.py`, always wire
`Image Texture.Alpha -> BSDF.Alpha` when `image.channels == 4`, multiply by material alpha
via a Math node when `alpha != 1.0`, and pick `CLIP` (+`alpha_threshold`) when
`alpha_test` is set, else `HASHED`; set `show_transparent_back = False`.

**Files:** `io_rose/import_zsc.py`, `io_rose/import_map.py`, `io_rose/import_combined_zone.py`

## ZMS export crash: `_PropertyDeferred` and str vs int version

**Problem:** `rose.export_zms` failed with
`TypeError: '<=' not supported between instances of '_PropertyDeferred' and 'int'`.

**Root cause:** Two bugs. (1) Properties were declared old-style
(`export_version = EnumProperty(...)`) which Blender 4.5 no longer registers as RNA, so
`self.export_version` returned the deferred property object. (2) Even registered, the enum
yields the string `'8'`, compared against ints.

**Fix:** Use annotation style (`export_version: EnumProperty(...)`) and coerce
`version = int(version)` in `export_zms_mesh_object`. Same annotation fix in
`import_zms.py`.

**Files:** `io_rose/export_zms.py`, `io_rose/import_zms.py`

## ZMS export wrote wrong orientation in game (Y-flip round trip)

**Problem:** After exporting the reworked wings from Blender, they attached to the
character with the wrong orientation in game. Original file was fine.

**Root cause:** Importers read ZMS vertices verbatim, but `zms_from_mesh_data` wrote them
Y-flipped (`x, -y, z`). Round trip therefore applied a flip the game never had; the client
also applies its own `(y,z)->(z,-y)` vertex transform on load, so the stored space must be
kept verbatim.

**Fix:** Added `apply_world_transform` / `convert_coordinates` parameters to
`export_zms_mesh_object`. The manual `ExportZMS` operator passes both `False` (faithful
local round trip); `AddZoneObject` keeps the defaults (world-space Blender meshes).

**Files:** `io_rose/export_zms.py`

## Lesson learned

- For asset round trips, export must be the exact inverse of import. Check both ends
  (and the game client's vertex transform in `zms_asset_loader.rs`) before assuming a
  coordinate convention.
- Blender 4.5 requires annotation-style operator properties; old-style assignments fail
  silently and surface as `_PropertyDeferred` at runtime.
- Also fixed in `io_rose/__init__.py`: the reload block skipped several submodules
  (`import_zsc`, `import_zms`, ...), so edits never took effect on addon reload.
