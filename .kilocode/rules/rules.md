**Rules**
Use only Windows commands, not linux commands

Do NOT try to parse output with "head" like this "cargo build 2>&1 | head -100"

Do NOT try to parse cargo build with select-string like this "cargo build 2>&1 | Select-Object -First 100"

Do NOT run "cargo build --release" only run "cargo build"

Do NOT run "cargo run" ask the user for output of the compiled software



**Source Code For Bevy 0.15.4 We Use**

Before you begin troubleshooting, make note of the features that are involved in this interaction, then search the bevy source code located here for those features and read their .rs files to make sure we fully understand how they work.
C:\Users\vicha\RustroverProjects\bevy-collection\bevy-0.15.4


**What To Do When Stuck**
***Old Working Version Of The Game***
An older, working version of the game that uses Bevy 0.11 is here for reference
C:\Users\vicha\RustroverProjects\exjam-rose-offline-client\rose-offline-client

***Web Search***
Use your web search tools if you're stuck and the source code review is not helping