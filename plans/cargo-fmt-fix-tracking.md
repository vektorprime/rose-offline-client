# Cargo Fmt Fix Tracking

## Task
Fix repository issues that prevent `cargo fmt` from completing.

## Affected systems
- Rust source formatting across files reported by `rustfmt`.
- No intended gameplay, rendering, ECS scheduling, asset loading, or networking behavior changes.

## Pre-work results
- Reviewed `pitfalls/index.md`; no formatting-specific pitfall found.
- Reviewed `system-architecture/ECS.md` because the active working-tree changes include Bevy systems and system params.
- Checked Bevy 0.18.1 source for relevant system-param behavior:
  - `Commands` is a deferred `SystemParam`.
  - `Local<T>` derefs to per-system local state.
  - Function systems accept `SystemParam` inputs.

## Attempts
- Attempt 1: Ran `cargo fmt` and captured trailing-whitespace failures.
  - Reported files: `src/ui/ui_settings_system.rs`, `src/systems/game_connection_system.rs`, `src/map_editor/ui/zone_list_panel.rs`, `src/ui/ui_chatbox_system.rs`.
- Attempt 2: Mechanically stripped trailing whitespace from the reported files.
  - Result: `cargo fmt` completed successfully.
- Attempt 3: Let `cargo fmt` apply normal repository formatting after blockers were removed.
  - Result: formatter changed many existing Rust files, primarily line wrapping/normalization across the project.
- Verification:
  - `cargo fmt --check` succeeded.
  - Required separate `cargo build` subtask reported no errors.
