# Markdown Document Cleanup Tracking

## Scope
- Task: examine project `.md` documents and delete those with no useful project value.
- Affected systems: documentation organization, project rule docs, architecture docs, pitfalls, old implementation plans, generated/dependency markdown.
- Runtime systems affected: none.

## Pre-Work
- Reviewed `pitfalls/index.md` and pitfall headings to identify known issue documentation that should generally be preserved.
- Reviewed `system-architecture/README.md` and architecture headings to understand project documentation structure.
- No Bevy 0.18.1 source validation needed because this task does not depend on Bevy runtime behavior.

## Attempts
- Initial inventory listed markdown under root, `.kilocode`, `3DDATA`, `docs`, `pitfalls`, `plans`, `system-architecture`, `.kilocode/node_modules`, and `target`.
- Classified documents by whether they are project-authored current references, active/valuable history, generated dependency/build output, duplicates, or stale branch-specific notes.
- Confirmed `docs/sailing-zone-features-*.md` references a removed `src/sailing/...` module tree.
- Confirmed `docs/how-to-run-game.md` says ocean zone files do not exist, which conflicts with the current `3DDATA/MAPS/OCEAN` scaffold and generated zone files.
- Updated `plans/sailing-system-implementation-tracking.md` so it no longer links to the removed ocean run guide.
- Follow-up request restored the generated `target/debug/3Ddata/MAPS/OCEAN/OCEAN-zone-scaffold.md` copy and added a root-level `OCEAN-zone-scaffold.md` copy.
- Follow-up request updated `AGENTS.md` so `cargo build` is required only after code-file changes, not markdown-only changes.

## Results
- Delete as no-value/stale:
  - `test.md`: outdated duplicate of `AGENTS.md`.
  - `docs/how-to-run-game.md`: stale ocean-zone instructions superseded by the current ocean scaffold/status notes.
  - `docs/sailing-zone-features-bevy-compile-issues.md`: obsolete branch compile notes for removed files.
  - `docs/sailing-zone-features-rust-compile-issues.md`: obsolete branch compile notes for removed files.
  - `plans/cargo-fmt-fix-tracking.md`: completed transient formatting-fix log with no lasting feature/architecture value.
  - `.kilocode/node_modules/zod/README.md`: generated dependency package README, not project documentation.
  - `target/doc/static.files/SourceSerif4-LICENSE-a2cfd9d5.md`: generated cargo-doc static asset license in ignored build output.
- Keep:
  - `AGENTS.md`, `.kilocode/rules/*`, `README.md`, `pitfalls/*`, `system-architecture/*`, active sailing plans/tracking, server-authority docs, and current ocean scaffold docs.
  - `OCEAN-zone-scaffold.md` copies in the project root, `3DDATA/MAPS/OCEAN`, and `target/debug/3Ddata/MAPS/OCEAN`.
- Required separate `cargo build` subtask result: `BUILD_SUCCESS_NO_ERRORS`.
- Follow-up changed only `.md` files, so `cargo build` was intentionally skipped under the updated `AGENTS.md` rule.
