# PowerShell script to filter log lines starting with "2026"
# - Remove first 28 characters (timestamp prefix)
# - Find "INFO", "DEBUG", "WARN", or "ERROR" in the first 6 chars (after trimming)
# - Delete from that word to the next space
# - Then trim everything up to and including the first ": "
# - Optional: filter by match string and limit max items

param(
    [string]$InputFile = "output.txt",
    [string]$Match = "",
    [int]$MaxItems = -1
)

# Get the directory where this script is located
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$InputPath = Join-Path $ScriptDir $InputFile

if (-not (Test-Path $InputPath)) {
    Write-Error "Input file not found: $InputPath"
    exit 1
}

# Read and filter lines
$counter = 0
Get-Content $InputPath | Where-Object {
    $_.StartsWith("2026")
} | ForEach-Object {
    $line = $_.Substring(28).TrimStart()

    # Check for log level keywords in first 6 characters
    foreach ($level in @("INFO", "DEBUG", "WARN", "ERROR")) {
        if ($line.StartsWith($level)) {
            # Find the space after the log level
            $spaceIndex = $line.IndexOf(" ", [ StringComparison]::Ordinal)
            if ($spaceIndex -ge 0) {
                # Remove from log level to first space (inclusive)
                $line = $line.Substring($spaceIndex + 1)
                break
            } else {
                # No space found, return empty
                return ""
            }
        }
    }

    # Trim everything up to and including the first ": "
    $colonSpaceIndex = $line.IndexOf(": ")
    if ($colonSpaceIndex -ge 0) {
        $line = $line.Substring($colonSpaceIndex + 2)
    }

    # Check if line matches the match string (if specified)
    if ($Match -and -not $line.Contains($Match)) {
        return
    }

    # Check if we've reached max items limit
    $counter++
    if ($MaxItems -gt 0 -and $counter -gt $MaxItems) {
        return
    }

    return $line
}