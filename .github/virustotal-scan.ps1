[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Path,
    [Parameter(Mandatory)][string] $Sha256,
    [switch] $Installer,
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

$binaryReason = @"
An unsigned tray app that writes an autostart key, launches ``npm`` and replaces its own executable
trips machine-learning heuristics. The
[README](https://github.com/TOR968/Globlin#install) explains each detection and how to verify this
build yourself.
"@

$installerReason = @"
An unsigned Inno Setup installer unpacks a program and starts it, which is also what a dropper does,
and a few machine-learning engines flag that wrapper on every release. The portable ``.exe`` in this
release is the same program without it. The
[README](https://github.com/TOR968/Globlin#install) explains how to verify either download yourself.
"@

function Format-Section([string] $verdict, [string] $reason) {
    $lines = @("## Virus scan — $(Split-Path $Path -Leaf)", '', "[VirusTotal report]($reportUrl) — $verdict")
    if ($reason) {
        $lines += @('', $reason)
    }
    $lines -join "`n"
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
foreach ($name in 'malicious', 'suspicious', 'undetected', 'harmless') {
    $total += [int] $stats.$name
}

if ($flagged -eq 0) {
    $verdict = "no engine out of $total flagged this build."
    $reason = ''
} else {
    $verdict = "$flagged of $total engines flagged this build."
    $reason = if ($Installer) { $installerReason } else { $binaryReason }
}

Write-Host "verdict: $verdict"
Format-Section $verdict $reason
