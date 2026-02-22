# Rules

## Commands
Use only Windows commands, not linux commands

Do NOT try to parse output with "head" like this "cargo build 2>&1 | head -100"

Do NOT try to parse cargo build with select-string like this "cargo build 2>&1 | Select-Object -First 100"

Do NOT run "cargo build --release" only run "cargo build"

Do NOT run "cargo run" ask the user for output of the compiled software

Do NOT run "cargo clean" without asking the user for permission.

## Source Code For Bevy 0.16.1 We Use

Before you begin troubleshooting, make note of the features that are involved in this interaction, then search the bevy source code located here for those features and read their .rs files to make sure we fully understand how they work.
C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.16.1

### Source Code For WGPU v24
C:\Users\vicha\RustroverProjects\bevy-collection\wgpu-24.0.5

## What To Do When Stuck

### Old Working Version Of The Game
An older, working version of the game that uses Bevy 0.11 is here for reference
C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client

### Web Search
Use your web search tools if you're stuck and the source code review is not helping


## Task Difficulty

If something is too difficult, break it down into as many steps as needed steps. NEVER give up on a task due to difficulty.

## Lessons Learned in pitfalls.md

When you fix an issue AND the user confirms it's resolved, note the interaction and details in pitfalls.md so that future work can benefit from the lessons learned. Do not edit pitfalls.md until the user confirms the issue is fixed. Your notes should be short and concise.
