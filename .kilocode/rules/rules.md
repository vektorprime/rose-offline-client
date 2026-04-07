# Rules

## Commands
Use only Windows commands, not linux commands

Do NOT try to parse output with "head" like this "cargo build 2>&1 | head -100"

Do NOT try to parse output with select-string like this "cargo build 2>&1 | Select-Object -First 100" instead prefer findstr.

Do NOT run "cargo build --release" only run "cargo build"

Do NOT run "cargo run" ask the user for output of the compiled software

Do NOT run "cargo clean" without asking the user for permission.

You MUST run "Cargo build" inside of a subtask where it was requested. It should not run in the same task it was requested on. Report back the errors or failures, not warnings. Use this text when invoking the subtask "You are a subtask, your purpose is to run "cargo build" and report back all errors and ignore warnings. you should list the error and the location and then use the attempt_completion tool to return to the parent task."



## Before you start work
Before working on an issue you must consult the two below folders:
1. "pitfalls" folder -  to identify previous issues and resolutions.
2. "system-architecture" folder - understand the architecture


You MUST also make note of the features that are involved in this interactions, then search the bevy 0.18.1 source code for those features and read their .rs files to make sure we fully understand how they work.

## Source Code locations

### Game Server Source Code
C:\Users\vicha\RustroverProjects\rose-offline

## Game Client Source Code

C:\Users\vicha\RustroverProjects\rose-offline-client

### Source Code For Bevy 0.18.1
Bevy 0.18
C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1


### Source Code For WGPU v27
C:\Users\vicha\RustroverProjects\bevy-collection\wgpu-27

### Source Code For Bevy_EGUI 0.39.1
C:\Users\vicha\RustroverProjects\bevy-collection\bevy_egui-0.39.1


## What To Do When Stuck
When stuck, research the issue with your search and fetch content tools.

When stuck, reference an older version of the game. An older, working verison of the game developed in C++ is here for reference
E:\cpp\client\src

An older, working version of the game that uses Bevy 0.11 is here for reference
C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client

When troubleshooting compilation errors, reference the rust error code in the document C:\Users\vicha\RustroverProjects\rust-errors\all-rust-errors.md

## Place holders and stub functions
Never leave any place holders or stub functions when the user is expecting complete code.

## Task Difficulty
If something is too difficult, break it down into as many steps as needed steps. NEVER give up on a task due to difficulty.


## Lessons Learned in pitfalls folder
When you fix an issue AND the user confirms it's resolved, note the interaction and details in the pitfalls folder in a .md file so that future work can benefit from the lessons learned. Do not edit create or modify pitfall .md documents until the user confirms the issue is fixed. Your notes should be short and concise.


## Issue Tracking

When working on an issue, note what you attempted in a .md file dedicated to the issue. This should be reviewed everytime context is compressed to prevent repeatedly trying the same thing. The file should be cleaned up when the issue is confirmed as fixed.



## Before you consider all of the issues resolved

Before ending a task, confirm that "cargo build" is successful.
Always run "cargo build" in a separate task and report the progress back, use this text for the subtask  "You are a subtask, your purpose is to run "cargo build" and output it to a file, and then report back all errors and ignore warnings. you should list the error and the location and then use the attempt_completion tool to return to the parent task."
