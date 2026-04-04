# Rules

## Commands
Use only Windows commands, not linux commands

Do NOT try to parse output with "head" like this "cargo build 2>&1 | head -100"

Do NOT try to parse output with select-string like this "cargo build 2>&1 | Select-Object -First 100" instead prefer findstr.

Do NOT run "cargo build --release" only run "cargo build"

Do NOT run "cargo run" ask the user for output of the compiled software

Do NOT run "cargo clean" without asking the user for permission.

You MUST run "Cargo build" inside of a subtask with the instructions "You must run cargo build and report back the errors or failures, not warnings."

## Reading files
Prefer to read the whole file if you've never read the file before. You have read the file before, then read only the relevant lines.

## Source Code For Bevy

Before you begin troubleshooting, make note of the features that are involved in this interaction, then search the bevy source code located here for those features and read their .rs files to make sure we fully understand how they work.

Bevy 0.18.1 source
C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.18.1



### Source Code For WGPU v27
C:\Users\vicha\RustroverProjects\bevy-collection\wgpu-27

### Source Code For Bevy_EGUI 0.39.1
C:\Users\vicha\RustroverProjects\bevy-collection\bevy_egui-0.39.1

## What To Do When Stuck

### Old Working Version Of The Game
An older, working verison of the game developed in C++ is here for reference
E:\cpp\client\src

An older, working version of the game that uses Bevy 0.11 is here for reference
C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client



## Task Difficulty

If something is too difficult, break it down into as many steps as needed steps. NEVER give up on a task due to difficulty.

## Lessons Learned in pitfalls folder

When you fix an issue AND the user confirms it's resolved, note the interaction and details in the pitfalls folder in a .md file so that future work can benefit from the lessons learned. Do not edit create or modify pitfall .md documents until the user confirms the issue is fixed. Your notes should be short and concise.


## Issue Tracking

When working on an issue, note what you attempted in a .md file dedicated to the issue. This should be reviewed everytime context is compressed to prevent repeatedly trying the same thing. The file should be cleaned up when the issue is confirmed as fixed.

## Finishing

Before ending a task, confirm that "cargo build" is successful.
Always run "cargo build" in a separate task and report the progress back in max of 1 sentence