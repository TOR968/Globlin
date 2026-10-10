[CmdletBinding()]
param(
    [Parameter(Mandatory)][string] $Exe,
    [Parameter(Mandatory)][ValidateSet('x64', 'arm64')][string] $Arch,
    [Parameter(Mandatory)][string] $Icon,
    [Parameter(Mandatory)][string] $OutDir
)

$ErrorActionPreference = 'Stop'

$iscc = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'
if (-not (Test-Path $iscc)) {
    choco install innosetup -y --no-progress | Out-Host
}

$version = (Select-String -Path Cargo.toml -Pattern '^version = "([^"]+)"').Matches[0].Groups[1].Value
$output = (New-Item -ItemType Directory -Force $OutDir).FullName

& $iscc "/DAppVersion=$version" "/DArch=$Arch" "/DSourceExe=$((Resolve-Path $Exe).Path)" `
    "/DIconFile=$((Resolve-Path $Icon).Path)" "/O$output" installer/globlin.iss | Out-Host
if ($LASTEXITCODE -ne 0) {
    throw "ISCC failed with exit code $LASTEXITCODE"
}

Join-Path $output "globlin-setup-$Arch.exe"
