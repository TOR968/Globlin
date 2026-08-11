[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Version,
    [string] $Path = 'CHANGELOG.md'
)

if (-not (Test-Path $Path)) {
    return
}

$collected = [System.Collections.Generic.List[string]]::new()
$inside = $false

foreach ($line in Get-Content $Path) {
    if ($line -match '^##\s') {
        if ($inside) { break }
        if ($line -match ('\[?' + [regex]::Escape($Version) + '\]?')) { $inside = $true }
        continue
    }
    if ($inside) { $collected.Add($line) }
}

if ($collected.Count -eq 0) {
    return
}

($collected -join "`n").Trim()
