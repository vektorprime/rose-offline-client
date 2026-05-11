# How to Use the New Ocean Map (Zone 200)

## Current status

- Ocean zone scaffolding exists at [`3DDATA/MAPS/OCEAN`](../3DDATA/MAPS/OCEAN).
- Runtime ocean water tuning is active when zone 200 is loaded via [`game_zone_change_system()`](../src/systems/game_system.rs:73).
- The folder currently contains scaffold docs only; it does **not** yet contain exported playable zone binaries (`ZON/HIM/TIL/IFO`).

## Prerequisites

1. Export real map data into [`3DDATA/MAPS/OCEAN`](../3DDATA/MAPS/OCEAN):
   - `OCEAN.ZON`
   - block files (`HIM`, `TIL`, `IFO`)
2. Ensure zone 200 is registered in server/game data (zone list and server-side warp/NPC wiring).

## Run modes and commands

### 1) Zone Viewer (client-side map check)

Use the CLI flags parsed in [`src/main.rs`](../src/main.rs:53):

- [`--zone-viewer`](../src/main.rs:59)
- [`--zone=<N>`](../src/main.rs:53)

Example:

```text
rose-offline-client --zone-viewer --zone=200 --data-idx=path/to/data.idx
```

### 2) Map Editor (author/edit zone 200)

Use [`--map-editor`](../src/main.rs:69) with [`--zone=<N>`](../src/main.rs:53):

```text
rose-offline-client --map-editor --zone=200 --data-idx=path/to/data.idx
```

Then save/export zone files into [`3DDATA/MAPS/OCEAN`](../3DDATA/MAPS/OCEAN).

### 3) In-game usage (server-integrated)

After server-side zone registration is complete, load/teleport to zone 200 through your normal GM tooling (for example server teleport command flow). If zone 200 is not registered server-side, client-only scaffolding is not enough for game-mode travel.

## Notes

- The ocean runtime settings are intentionally split from map content so rendering behavior can be tested as soon as zone 200 loads.
- If zone 200 fails to load, verify both zone-list registration and presence of exported map assets.

