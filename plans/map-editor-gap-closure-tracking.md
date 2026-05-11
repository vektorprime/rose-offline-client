# Map Editor Gap Closure Tracking

## Goal
Close map editor functional gaps for new-zone bootstrap, terrain/water authoring, and persistence.

## Attempt Log

### 2026-04-11 - Attempt 1
- Added metadata components for editable terrain/water:
  - `MapEditorWaterPlane`
  - `MapEditorTerrainBlock`
- Wired metadata components into zone spawning for terrain and water entities.
- Added menu/dialog scaffolding for:
  - Save Version custom path
  - New Zone options (output path + default block bootstrap)

### 2026-04-11 - Attempt 2
- Extended `NewZoneEvent` payload and UI resource wiring.
- Added initial file bootstrap implementation for default `0_0.HIM`, `0_0.TIL`, `0_0.IFO`.

### 2026-04-11 - Current Focus
- Implement in-editor water/terrain authoring controls.
- Persist water planes to IFO and terrain data to HIM/TIL in save pipeline.
- Validate with required standalone `cargo build` subtask.

### 2026-04-12 - Attempt 3
- Reviewed current map editor new-zone + save pipeline behavior against latest summary.
- Confirmed water creation behavior already sets surface 20m above selected terrain average (`+2000 cm`) so depth below is present by default.
- Expanded new-zone bootstrap from only `0_0` to full flat `64x64` terrain scaffold (`HIM/TIL/IFO` for every block) so new custom zones start fully flat.
- Updated new-zone UI wording to match actual behavior (full flat zone scaffold).
- Implemented shared world->block coordinate conversion in save and delete-tracking paths to reduce block misclassification risk from ad-hoc formulas.

### 2026-04-12 - Attempt 4
- Reproduced reported mismatch where File > Save logged fallback loaded zone id (`1`) while editing custom zone id (`64`).
- Root cause: menu save event used `CurrentZone` id directly; for fallback editing flows this reflects loaded source zone, not intended custom zone id.
- Fix: editor effective zone id now prioritizes `CustomZonePath.zone_id` when active, then falls back to `CurrentZone.id`.
