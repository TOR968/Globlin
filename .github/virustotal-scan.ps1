[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Path,
    [Parameter(Mandatory)][string] $Sha256,
    [int] $TimeoutSeconds = 360,
    [int] $PollSeconds = 20
)

$ErrorActionPreference = 'Stop'

$key = $env:VIRUSTOTAL_API_KEY
if (-not $key) {
    Write-Host 'no VIRUSTOTAL_API_KEY secret; skipping the scan'
    return
}

$reportUrl = "https://www.virustotal.com/gui/file/$($Sha256.ToLower())"
$headers = @{ 'x-apikey' = $key }

function Format-Section([string] $verdict) {
    @"
## Virus scan

[VirusTotal report]($reportUrl) — $verdict

An unsigned tray app that writes an autostart key, launches ``npm`` and replaces its own executable
trips machine-learning heuristics. The
[README](https://github.com/TOR968/Globlin#install) explains each detection and how to verify this
build yourself.
"@
}

try {
    $upload = Invoke-RestMethod -Method Post -Uri 'https://www.virustotal.com/api/v3/files' `
        -Headers $headers -Form @{ file = Get-Item $Path }
    $analysisId = $upload.data.id
    Write-Host "uploaded, analysis $analysisId"
} catch {
    Write-Host "upload failed: $($_.Exception.Message)"
    return
}

$analysisUrl = "https://www.virustotal.com/api/v3/analyses/$analysisId"
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
$stats = $null

while ((Get-Date) -lt $deadline) {
    Start-Sleep -Seconds $PollSeconds
    try {
        $analysis = Invoke-RestMethod -Uri $analysisUrl -Headers $headers
    } catch {
        Write-Host "poll failed: $($_.Exception.Message)"
        continue
    }
    $status = $analysis.data.attributes.status
    Write-Host "status $status"
    if ($status -eq 'completed') {
        $stats = $analysis.data.attributes.stats
        break
    }
}

if (-not $stats) {
    Write-Host 'analysis did not complete in time; linking the report without a verdict'
    Format-Section 'the analysis was still running when this release was published.'
    return
}

$flagged = [int] $stats.malicious + [int] $stats.suspicious
$total = 0
foreach ($name in 'malicious', 'suspicious', 'undetected', 'harmless', 'timeout', 'failure') {
    $total += [int] $stats.$name
}

$verdict = if ($flagged -eq 0) {
    "no engine out of $total flagged this build."
} else {
    "$flagged of $total engines flagged this build."
}

Write-Host "verdict: $verdict"
Format-Section $verdict
