# Sailing Plan Expansion Tracking

## Goal

Expand the existing sailing system planning documents with implementation-level detail grounded in the current Bevy 0.18.1 client codebase.

## Affected Systems

- Sailing and boat gameplay components/systems
- Player movement, command, camera, and input integration
- Water, wake, buoyancy, and zone rendering
- UI/HUD and debug diagnostics
- Asset/model loading and animation
- Networking/server-authority migration boundaries

## Attempts

- 2026-05-29: Started repository review. Located existing sailing plans, current sailing/boat code, relevant pitfalls, and architecture documentation.
- 2026-05-29: Reviewed existing plan drift against current source. Confirmed the client already has local boat state, procedural visuals, wind, movement, buoyancy, sail deformation, wake/spray, HUD, camera behavior, disembark flow, collision gating, and ocean-zone water tuning.
- 2026-05-29: Validated related Bevy 0.18.1 behavior from source: messages, hierarchy/recursive despawn, mutable mesh assets, input, time/timers, schedules, state run conditions, and query disjointness.
- 2026-05-29: Expanded `sailing-system-plan.md` with a current baseline, implementation handoff, updated file inventory, revised risks, and new priority order.
- 2026-05-29: Expanded `sailing-system-detailed-expansion.md` with a current architecture snapshot, source-validated Bevy rules, updated section statuses, server-authority plan, ocean-zone MVP checklist, audio implementation plan, remote boat rendering plan, and production boarding/disembark notes.

## Results

- Documentation edits complete.
- Required separate `cargo build` subtask completed with no compilation errors.
