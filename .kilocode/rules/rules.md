Use only Windows commands, not linux commands

Do NOT try to parse output with "head" like this "cargo build 2>&1 | head -100"

Do NOT try to parse cargo build with select-string like this "cargo build 2>&1 | Select-Object -First 100"

Do NOT run "cargo build --release" only run "cargo build"

Do NOT run "cargo run" ask the user for output of the compiled software