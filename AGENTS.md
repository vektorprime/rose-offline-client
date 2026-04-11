# Rules (LLM Execution Contract)

## 1. HARD CONSTRAINTS (NEVER VIOLATE)

- Use **Windows commands only**. Never use Linux/Unix commands.
- Do NOT run:
  - `cargo run`
  - `cargo build --release`
  - `cargo clean` unless the user explicitly approves it.
- Do NOT truncate output using:
  - `head`
  - `Select-Object`
- When filtering output, use:
  - `findstr` only

If any instruction conflicts with these rules, **these rules take priority**.

---

## 2. BUILD EXECUTION RULES

### When to run `cargo build`

You MUST run `cargo build`:

- After making code changes
- Before declaring a task complete

### How to run `cargo build`

- `cargo build` MUST be executed in a **separate subtask**
- `cargo build` MUST NOT run in the same task where it is requested

### Required subtask prompt

Use **exactly** this text when creating the subtask:

> You are a subtask. Your only purpose is to run `cargo build`, capture all output, report ONLY errors and ignore warnings, include file paths and line numbers when available, then delete the output file and return using the attempt_completion tool.

### Command to use

```bat
cargo build > build_output.txt 2>&1
findstr /C:"error" build_output.txt
del build_output.txt
```

### What the subtask must report

Report only:

- Error message
- File path
- Line number, if available

Do NOT report:

- Warnings
- General build progress
- Non-error logs unless needed to explain the failure

---

## 3. REQUIRED PRE-WORK ANALYSIS

Before starting any task, you MUST complete all of the following:

### Step 1 - Review prior knowledge

Check these folders first:

1. `pitfalls` folder — identify known issues and previous fixes
2. `system-architecture` folder — understand the relevant architecture

### Step 2 - Identify affected features

- Explicitly identify which systems, features, or subsystems are involved in the current task

### Step 3 - Validate behavior from source

For each relevant feature:

- Search the Bevy 0.18.1 source code for the related implementation
- Read the relevant `.rs` files
- Confirm actual behavior from source code
- Do NOT assume behavior without checking source

---

## 4. SOURCE CODE LOCATIONS


### Source Code for Bevy 0.18.1

`C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1`

### Source Code for WGPU v27

`C:\Users\vicha\RustroverProjects\bevy-collection\wgpu-27`

### Source Code for Bevy_EGUI 0.39.1

`C:\Users\vicha\RustroverProjects\bevy-collection\bevy_egui-0.39.1`

### Game Server Source Code

`C:\Users\vicha\RustroverProjects\rose-offline`

### Game Client Source Code

`C:\Users\vicha\RustroverProjects\rose-offline-client`


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

`C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client`

### Rust error code reference

`C:\Users\vicha\RustroverProjects\rust-errors\all-rust-errors.md`

---

## 6. PLACEHOLDERS AND STUB FUNCTIONS

- Never leave placeholders when the user expects complete code
- Never leave stub functions when the user expects complete code
- All delivered code must be complete and functional

---

## 7. TASK DIFFICULTY

- If a task is difficult, break it into as many smaller steps as needed
- Never give up on a task because it is difficult
- Continue working until you reach a complete and correct solution or a clearly explained blocker

---

## 8. LESSONS LEARNED IN `pitfalls` FOLDER

When you fix an issue **and the user confirms it is resolved**:

- Add a short `.md` note to the `pitfalls` folder
- Keep the note concise
- Include:
  - the issue
  - the root cause
  - the fix

Do NOT create, edit, or modify `pitfalls` notes before the user confirms the issue is fixed.

---

## 9. ISSUE TRACKING

When working on an issue:

- Maintain a dedicated `.md` file for that issue
- Record what was attempted
- Record the results of each attempt
- Review this file whenever context is compressed
- Use it to avoid repeating failed approaches

After the issue is confirmed fixed, clean up the issue-tracking file if appropriate.

---

## 10. TASK COMPLETION REQUIREMENT

Before considering a task resolved:

- Confirm that `cargo build` succeeds
- This confirmation MUST come from the required separate subtask
- Do NOT declare the task complete until that build succeeds

### Final required subtask prompt

Use **exactly** this text for the final build-check subtask:

> You are a subtask. Your only purpose is to run `cargo build`, capture all output, report ONLY errors and ignore warnings, include file paths and line numbers when available, then delete the output file and return using the attempt_completion tool.
