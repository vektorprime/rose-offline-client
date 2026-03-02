# LLM Troubleshooting Guide

## Overview

The Rose Online game client logs all operations to timestamped folders in the `logs/` directory. Each game session creates a new folder named with the start time (e.g., `logs/2026-03-02_11-24-10/`). These logs are designed to be easily queried and analyzed by LLMs for troubleshooting.

## Log File Structure

Each session folder contains:

| File | Description |
|------|-------------|
| `session.json` | Session metadata (command line, config, versions) |
| `structured.jsonl` | Primary log file in JSON Lines format |
| `game.log` | Human-readable log (optional) |

## Log Format (JSON Lines)

The `structured.jsonl` file uses JSON Lines format where each line is a valid JSON object:

```json
{"ts":"2026-03-02T11:24:11.123456-06:00","level":"INFO","target":"rose_offline_client::zone_loader","tag":"ZONE LOADER","msg":"Loading zone 1...","span":null,"kvs":{"zone_id":1}}
{"ts":"2026-03-02T11:24:11.456789-06:00","level":"WARN","target":"rose_offline_client::vfs","tag":"VFS","msg":"File not found: 3DDATA/MODELS/TEST.ZMS","span":null,"kvs":{"path":"3DDATA/MODELS/TEST.ZMS"}}
{"ts":"2026-03-02T11:24:12.001234-06:00","level":"ERROR","target":"rose_offline_client::memory","tag":"MEMORY","msg":"Failed to allocate buffer","span":{"name":"spawn_zone","zone_id":1},"kvs":{"size":1048576}}
```

### Field Definitions

| Field | Type | Description | Example |
|-------|------|-------------|---------|
| `ts` | string | ISO 8601 timestamp with timezone | `2026-03-02T11:24:11.123456-06:00` |
| `level` | string | Log level: `TRACE`, `DEBUG`, `INFO`, `WARN`, `ERROR` | `ERROR` |
| `target` | string | Rust module path | `rose_offline_client::zone_loader` |
| `tag` | string\|null | Extracted tag from message | `ZONE LOADER` |
| `msg` | string | Human-readable message | `Loading zone 1...` |
| `span` | object\|null | Tracing span context | `{"name":"spawn_zone","zone_id":1}` |
| `kvs` | object | Additional key-value pairs | `{"zone_id":1,"duration_ms":150}` |

### Common Tags

The codebase uses consistent tag prefixes in log messages:

| Tag | Description |
|-----|-------------|
| `[ZONE LOADER]` | Zone loading operations |
| `[VFS]` | Virtual filesystem operations |
| `[SPAWN ZONE]` | Zone entity spawning |
| `[MEMORY]` | Memory tracking |
| `[BIRD]`, `[FISH]` | Entity systems |
| `[UI LOGIN]` | Login UI operations |
| `[AUDIO]` | Audio system |
| `[ANIMATION]` | Animation system |

## Querying Logs with PowerShell


The base directory for cargo is rose-offline-client
The logs are located in rose-offline-client\target\debug\logs

### Basic Filtering

```powershell
# Get all ERROR level messages
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.level -eq "ERROR" }

# Get all WARN and ERROR messages
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.level -in @("WARN", "ERROR") }

# Filter by tag
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.tag -eq "ZONE LOADER" }
```

### Searching Messages

```powershell
# Search for text in messages (case-insensitive)
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.msg -like "*zone 1*" }

# Search using regex
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.msg -match "failed|error|missing" }
```

### Extracting Specific Fields

```powershell
# Get all unique tags used in the log
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Select-Object -ExpandProperty tag -Unique

# Get all error messages with timestamps
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.level -eq "ERROR" } | Select-Object ts, msg

# Count messages by level
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Group-Object level
```

### Working with Key-Value Pairs

```powershell
# Filter by key-value data
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.kvs.zone_id -eq 1 }

# Extract specific kvs fields
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.kvs } | Select-Object msg, kvs
```

## Common Troubleshooting Scenarios

### Finding All Errors in a Session

```powershell
# Display all errors with context
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.level -eq "ERROR" } | Format-Table ts, tag, msg -Wrap
```

### Investigating Zone Loading Issues

```powershell
# All zone loader messages
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.tag -eq "ZONE LOADER" }

# Zone loading errors only
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.tag -eq "ZONE LOADER" -and $_.level -in @("WARN", "ERROR") }
```

### Finding Missing Files

```powershell
# All VFS file-not-found warnings
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.tag -eq "VFS" -and $_.msg -like "*not found*" }
```

### Memory-Related Issues

```powershell
# All memory-related logs
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.tag -eq "MEMORY" }
```


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

## Working with LLMs

### Providing Logs to an LLM

When asking an LLM to help troubleshoot:

1. **Read the structured.jsonl file** - This is the primary log file
2. **Include session.json for context** - Provides command line args and configuration
3. **Filter relevant logs** - Don't dump the entire file; filter by level or tag first

### Example: Preparing Logs for LLM Analysis

```powershell
# Export errors and warnings to a file for LLM analysis
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.level -in @("WARN", "ERROR") } | ConvertTo-Json -Depth 10 | Out-File issues.json
```

### Session Metadata (session.json)

The `session.json` file contains valuable context:

```json
{
  "session_id": "2026-03-02_11-24-10",
  "start_time_utc": "2026-03-02T17:24:10.980Z",
  "start_time_local": "2026-03-02T11:24:10.980-06:00",
  "hostname": "GAMING-PC",
  "command_line": "rose-offline-client.exe --auto-login --character-name tesrrr --username ryko --password ryko",
  "mode": "Game",
  "bevy_version": "0.16.1",
  "rust_version": "1.76.0",
  "os": "Windows 11",
  "config": {
    "zone_id": 1,
    "data_version": "irose"
  }
}
```

This information helps LLMs understand:
- What command was run
- What version of the game/engine was used
- What configuration was active

### Sample LLM Prompt

```
Analyze the following game logs and identify:
1. All ERROR level messages and their likely causes
2. Any asset loading failures (missing files, parse errors)
3. The sequence of operations leading up to any errors

Session info:
[Paste session.json contents here]

Log entries:
[Paste filtered structured.jsonl contents here]
```

## Tips for Effective Troubleshooting

1. **Start with errors** - Filter by `level == "ERROR"` first
2. **Check warnings** - Warnings often indicate problems that become errors later
3. **Look at timestamps** - Correlate events by time to understand sequences
4. **Use tags** - Filter by tag to focus on specific subsystems
5. **Check kvs** - Additional structured data often has important context
6. **Compare sessions** - If something works in one session but not another, diff the logs

## Quick Reference

```powershell
# Find latest log folder
Get-ChildItem logs | Sort-Object LastWriteTime -Descending | Select-Object -First 1

# Count total log entries
(Get-Content logs\2026-03-02_11-24-10\structured.jsonl).Count

# Export specific entries for analysis
Get-Content logs\2026-03-02_11-24-10\structured.jsonl | ConvertFrom-Json | Where-Object { $_.level -eq "ERROR" } | ConvertTo-Json | clip
```
