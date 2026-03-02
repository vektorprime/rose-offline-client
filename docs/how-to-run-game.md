# How to Run the Game for Testing

This is a quick reference guide for building and running the Rose Online game client for testing purposes.

## Build the Game

```powershell
cargo build
```

This compiles the game in debug mode. The executable will be at `.\target\debug\rose-offline-client.exe`.

> **Note:** Do not use `cargo build --release` unless specifically needed. Debug builds are faster to compile and provide better error messages.

## Run with Auto-Login

The game supports command-line arguments for automatic login, useful for testing:

```powershell
.\target\debug\rose-offline-client.exe --auto-login --character-name tesrrr --username ryko --password ryko
```

### Auto-Login Arguments

| Argument | Description | Example |
|----------|-------------|---------|
| `--auto-login` | Enable automatic login | `--auto-login` |
| `--username` | Login username | `--username ryko` |
| `--password` | Login password | `--password ryko` |
| `--character-name` | Character to select | `--character-name tesrrr` |

## Run with Auto-Close (Timed Testing)

For automated testing, you can run the game and automatically close it after a specified duration:

### Auto-close after 15 seconds:

```powershell
$proc = Start-Process -FilePath ".\target\debug\rose-offline-client.exe" -ArgumentList "--auto-login","--character-name","tesrrr","--username","ryko","--password","ryko" -PassThru; Start-Sleep -Seconds 15; Stop-Process -Id $proc.Id -Force
```

### Breakdown of the command:

1. `Start-Process` - Launches the game executable
2. `-ArgumentList` - Passes command-line arguments (comma-separated, each argument quoted)
3. `-PassThru` - Returns the process object so we can access its ID
4. `Start-Sleep -Seconds 15` - Waits 15 seconds
5. `Stop-Process -Id $proc.Id -Force` - Kills the process

### Adjust the duration:

Change `Start-Sleep -Seconds 15` to any number of seconds you need.

## Check Logs
### How to check logs
You MUST create a subtask for another sub agent to:
1. Review the logs
2. Reduce deduplication of logs

3. Remove preamble from the log:
Example:
Original log line:
{"kvs":{"log.file":"src\\render\\starry_sky_material.rs","log.line":340,"log.module_path":"rose_offline_client::render::starry_sky_material","log.target":"rose_offline_client::render::starry_sky_material"},"level":"INFO","msg":"Updated 1 material(s)","span":null,"tag":"STARRY SKY UPDATE","target":"log","ts":"2026-03-02T13:30:50.772516300-06:00"}

Should be reduced to:
"level":"INFO","msg":"Updated 1 material(s)","span":null,"tag":"STARRY SKY UPDATE","target":"log","ts":"2026-03-02T13:30:50.772516300-06:00"}

4. Remove time stamps from logs
Original (after removing preamble):
"level":"INFO","msg":"Updated 1 material(s)","span":null,"tag":"STARRY SKY UPDATE","target":"log","ts":"2026-03-02T13:30:50.772516300-06:00"}

Should be reduced to:
"level":"INFO","msg":"Updated 1 material(s)","span":null,"tag":"STARRY SKY UPDATE","target":"log",

### Log commands
Logs are automatically saved to timestamped folders in the `logs/` directory:

```powershell
# List all log sessions
Get-ChildItem logs

# View the latest session's logs
$latest = Get-ChildItem logs | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Get-Content $latest.FullName\structured.jsonl
```

### Log Files

| File | Purpose |
|------|---------|
| `session.json` | Session metadata (command line, versions, config) |
| `structured.jsonl` | Primary log in JSON Lines format |
| `game.log` | Human-readable log (if enabled) |

For detailed log querying instructions, see [`llm-troubleshooting-guide.md`](llm-troubleshooting-guide.md).

## Common Testing Workflows

### Quick Smoke Test

```powershell
# Build, run for 15 seconds, auto-close
cargo build
$proc = Start-Process   -FilePath '.\target\debug\rose-offline-client.exe'   -WorkingDirectory '.\target\debug'   -ArgumentList @(    '--auto-login',    '--character-name', 'tesrrr',    '--username', 'ryko',    '--password', 'ryko'  ) -PassThru ; Start-Sleep -Seconds 15; Stop-Process -Id $proc.Id -Force `
```

### Check for Errors After Run

```powershell
# Get errors from latest log session
$latest = Get-ChildItem logs | Sort-Object LastWriteTime -Descending | Select-Object -First 1
Get-Content "$($latest.FullName)\structured.jsonl" | ConvertFrom-Json | Where-Object { $_.level -eq "ERROR" }
```

### Interactive Testing

```powershell
# Run normally and test manually
.\target\debug\rose-offline-client.exe --auto-login --character-name tesrrr --username ryko --password ryko
```

## Troubleshooting

### Game Won't Start

1. Check if build succeeded: `cargo build` should complete without errors
2. Check logs in the latest session folder for startup errors
3. Verify the executable exists: `Test-Path .\target\debug\rose-offline-client.exe`

### Game Crashes

1. Check the latest log session for ERROR entries
2. Look for panic messages in `structured.jsonl`
3. Check Windows Event Viewer for additional crash information

### Auto-Login Not Working

1. Verify the username and password are correct
2. Check if the character name exists on the account
3. Look for authentication errors in the logs
