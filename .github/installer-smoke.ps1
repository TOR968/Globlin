[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Setup,
    [int] $TimeoutSeconds = 60
)

$ErrorActionPreference = 'Stop'

$uninstallKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{380BD341-81C1-4AC3-AA2E-BC70BE5CC8F7}_is1'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$app = Join-Path $env:LOCALAPPDATA 'Programs\Globlin'
$exe = Join-Path $app 'globlin.exe'
$version = (Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value

function Wait-Until([scriptblock] $Condition, [string] $What) {
    $deadline = (Get-Date).AddSeconds($TimeoutSeconds)
    while (-not (& $Condition)) {
        if ((Get-Date) -gt $deadline) {
            throw "timed out waiting for $What"
        }
        Start-Sleep -Milliseconds 500
    }
}

Start-Process $Setup -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/TASKS=autostart' -Wait

if (-not (Test-Path $exe)) {
    throw "setup did not install $exe"
}
$entry = Get-ItemProperty $uninstallKey
if ($entry.InstallLocation.TrimEnd('\') -ne $app) {
    throw "InstallLocation is '$($entry.InstallLocation)', expected '$app'"
}
if ($entry.DisplayVersion -ne $version) {
    throw "DisplayVersion is '$($entry.DisplayVersion)', expected '$version'"
}
$autostart = (Get-ItemProperty $runKey -ErrorAction SilentlyContinue).globlin
$expected = "`"$exe`" --background"
if ($autostart -ne $expected) {
    throw "the Run value is '$autostart', expected '$expected'"
}
"installed $version into $app"

Start-Process (Join-Path $app 'unins000.exe') -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait
Wait-Until { -not (Test-Path $exe) } 'the uninstaller to remove globlin.exe'
Wait-Until { -not (Test-Path $uninstallKey) } 'the uninstaller to remove its Apps entry'

if ((Get-ItemProperty $runKey -ErrorAction SilentlyContinue).globlin) {
    throw 'the Run value survived the uninstall'
}
"uninstalled cleanly"
