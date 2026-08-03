# Rules (LLM Execution Contract)

You MUST follow all of the instructions in this document. Consider this document the authority on every topic. Assume everything in this document is correct, even when it conflicts with your understanding.

## 1. HARD CONSTRAINTS (NEVER VIOLATE)

- Use **Windows commands only**. Never use Linux/Unix commands.
- Do NOT run:
  - `cargo run`
  - `cargo build --release`
  - `cargo clean` unless the user explicitly approves it.
- Do NOT truncate output using:
  - `head`


If any instruction conflicts with these rules, **these rules take priority**.

---

## 2. BUILD EXECUTION RULES

### When to run `cargo build`

You MUST run `cargo build` only when code files were changed:

- After making code changes
- Before declaring a task complete when code files were changed

Do NOT run `cargo build` for documentation-only changes, including changes limited to `.md` files.

### How to run `cargo build`

- `cargo build` MUST be executed in a **separate subtask**
- `cargo build` MUST NOT run in the same task where it is requested

### Required subtask prompt

Use **exactly** this text when creating the subtask:

> You are a subtask. Your only purpose is to run `cargo build`, capture all output, report ONLY errors and ignore warnings, include file paths and line numbers when available, then delete the output file and return using the attempt_completion tool.


### What the subtask must report

Report only:

- Error message
- File path
- Line number, if available

Do NOT report:

- Warnings
- General build progress
- Non-error logs unless needed to explain the failure

Note: dev builds are slow — `[profile.dev.package."*"] opt-level = 3` in Cargo.toml builds all dependencies optimized. Do not treat long build times as a hang.

---

## 3. REQUIRED PRE-WORK ANALYSIS

Before starting any task, you MUST complete all of the following:

### Step 1 - Review prior knowledge

Check these folders first:

1. `pitfalls` folder — start with `pitfalls/index.md` (table of contents by component and by Bevy version), then read the entries for the component you are touching
2. `system-architecture` folder — start with `system-architecture/README.md`, then read the doc for the subsystem you are touching

### Step 2 - Identify affected features

- Explicitly identify which systems, features, or subsystems are involved in the current task

### Step 3 - Validate behavior from source

IF YOU ARE UNSURE ABOUT THE BEVY 0.18 DOCUMENTATION:

For each relevant feature:

- Search the Bevy 0.18.1 source code for the related implementation - `C:\Users\%USERNAME%\RustroverProjects\bevy-collection\bevy-0.18.1`
- Read the relevant `.rs` files
- Confirm actual behavior from source code
- Do NOT assume behavior without checking source

---

## 4. SOURCE CODE LOCATIONS

### Dependency crates (build requirement)

`Cargo.toml` uses **absolute-path dependencies**. `cargo build` fails if these are missing or moved:

- `C:/Users/%USERNAME%/RustroverProjects/rose-offline/` — `rose-data`, `rose-data-irose`, `rose-file-readers`, `rose-game-common`, `rose-game-irose`, `rose-network-common`, `rose-network-irose`
- `../bevy_procedural_grass` (i.e. `C:/Users/%USERNAME%/RustroverProjects/bevy_procedural_grass`)

### Source Code for Bevy 0.18.1

`C:\Users\%USERNAME%\RustroverProjects\bevy-collection\bevy-0.18.1`

### Source Code for WGPU v27

`C:\Users\%USERNAME%\RustroverProjects\bevy-collection\wgpu-27`

### Source Code for Bevy_EGUI 0.39.1

`C:\Users\%USERNAME%\RustroverProjects\bevy-collection\bevy_egui-0.39.1`

### Game Server Source Code

`C:\Users\%USERNAME%\RustroverProjects\rose-offline`

### Game Client Source Code

`C:\Users\%USERNAME%\RustroverProjects\rose-offline-client`

---

## 5. WHEN STUCK (MANDATORY ACTIONS)

If progress stalls, uncertainty remains, or the issue is not understood well enough to proceed confidently, you MUST do all of the following:

1. Research the issue using your search and fetch content tools
2. Compare against older working references
3. Check Rust compiler error documentation when dealing with compilation failures

### Older working references

**Older C++ version of the game**

`E:\cpp\client\src`

**Older working version using Bevy 0.11**

`C:\Users\%USERNAME%\RustroverProjects\exjam-rose-offline-client\rose-offline-client`

### Rust error code reference

`C:\Users\%USERNAME%\RustroverProjects\rust-errors\all-rust-errors.md`

---

## 6. REPOSITORY FACTS

- Single crate; entrypoint `src/main.rs` (clap CLI) dispatches to `src/lib.rs` functions `run_game`, `run_model_viewer`, `run_zone_viewer`, `run_map_editor`. Mode is selected by CLI flags: `--model-viewer`, `--zone-viewer` / `--zone=<N>`, `--map-editor`.
- The client loads game data from `data.idx` in the current directory, or via `--data-idx=<path>` / `--data-path=<path>`. It cannot start without game data. Only the `irose` data version exists (`--data-version irose`, likewise for network/UI versions).
- Actual engine version is **Bevy 0.18.1**.
- The user launches the client (you must not run it). Every session writes structured logs to `logs/<session-timestamp>/` relative to the working directory (`structured.jsonl`, `session.json`).
- `simplification/` — research and cleanup reports for the dead-code removal pass (historical, do not re-apply).

---

## 7. PLACEHOLDERS AND STUB FUNCTIONS

- Never leave placeholders when the user expects complete code
- Never leave stub functions when the user expects complete code
- All delivered code must be complete and functional

---

## 8. TASK DIFFICULTY

- If a task is difficult, break it into as many smaller steps as needed
- Never give up on a task because it is difficult
- Continue working until you reach a complete and correct solution or a clearly explained blocker

---

## 9. LESSONS LEARNED IN `pitfalls` FOLDER

When you fix an issue **and the user confirms it is resolved**:

- Add a short `.md` note to the `pitfalls` folder
- Keep the note concise
- Include:
  - the issue
  - the root cause
  - the fix

Do NOT create, edit, or modify `pitfalls` notes before the user confirms the issue is fixed.

---

## 10. ISSUE TRACKING

The `plans/` and `docs/` folders have been removed; do not create new files there. Track progress within the current session's todo list instead.

---

## 11. TASK COMPLETION REQUIREMENT

Before considering a task resolved:

- If code files were changed, confirm that `cargo build` succeeds
- This confirmation MUST come from the required separate subtask
- Do NOT declare a code-change task complete until that build succeeds
- If changes are limited to documentation files such as `.md`, do NOT run `cargo build`; state that the build was skipped because no code files changed

### Final required subtask prompt

Use **exactly** this text for the final build-check subtask:

> You are a subtask. Your only purpose is to run `cargo build`, capture all output, report ONLY errors and ignore warnings, include file paths and line numbers when available, then delete the output file and return using the attempt_completion tool.
