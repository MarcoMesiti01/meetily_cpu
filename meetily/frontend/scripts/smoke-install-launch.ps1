param(
    [Parameter(Mandatory = $true)][string]$InstallerPath,
    [string]$InstallDir = (Join-Path $env:TEMP "meetily-smoke-install"),
    [string]$LogDir = (Join-Path $env:TEMP "meetily-smoke-logs"),
    [int]$LaunchSeconds = 20
)

$ErrorActionPreference = "Stop"

function Write-SmokeLog {
    param([Parameter(Mandatory = $true)][string]$Message)

    $line = "$(Get-Date -Format o) $Message"
    Write-Host $line
    Add-Content -LiteralPath (Join-Path $LogDir "smoke.log") -Value $line
}

if (-not (Test-Path -LiteralPath $InstallerPath -PathType Leaf)) {
    throw "Installer not found: $InstallerPath"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $LogDir | Out-Null

$installer = (Resolve-Path -LiteralPath $InstallerPath).Path
$installRoot = (Resolve-Path -LiteralPath $InstallDir).Path
$installerLog = Join-Path $LogDir "installer-exit.txt"

Write-SmokeLog "Installing $installer into $installRoot"
$installArgs = @("/S", "/D=$installRoot")
$installProcess = Start-Process -FilePath $installer -ArgumentList $installArgs -Wait -PassThru -WindowStyle Hidden
"Installer exit code: $($installProcess.ExitCode)" | Set-Content -LiteralPath $installerLog -Encoding UTF8

if ($installProcess.ExitCode -ne 0) {
    throw "Silent installer failed with exit code $($installProcess.ExitCode)"
}

$candidatePaths = @(
    (Join-Path $installRoot "meetily.exe"),
    (Join-Path $installRoot "Meetily.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\meetily\meetily.exe"),
    (Join-Path $env:LOCALAPPDATA "Programs\Meetily\Meetily.exe"),
    (Join-Path $env:ProgramFiles "Meetily\Meetily.exe")
)

$installedExe = $candidatePaths |
    Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } |
    Select-Object -First 1

if (-not $installedExe) {
    $installedExe = Get-ChildItem -LiteralPath $installRoot -Filter "meetily.exe" -File -Recurse -ErrorAction SilentlyContinue |
        Select-Object -First 1 -ExpandProperty FullName
}

if (-not $installedExe) {
    throw "Installed meetily.exe was not found under $installRoot or the standard per-user install locations"
}

Write-SmokeLog "Launching $installedExe"
$appProcess = Start-Process -FilePath $installedExe -PassThru -WindowStyle Hidden

try {
    Start-Sleep -Seconds $LaunchSeconds
    $running = Get-Process -Id $appProcess.Id -ErrorAction SilentlyContinue
    if (-not $running) {
        throw "Meetily process exited before the $LaunchSeconds second smoke window completed"
    }

    [pscustomobject]@{
        ProcessId = $running.Id
        ProcessName = $running.ProcessName
        StartTime = $running.StartTime
        Path = $installedExe
        InstallDir = $installRoot
    } | ConvertTo-Json -Depth 3 | Set-Content -LiteralPath (Join-Path $LogDir "process-evidence.json") -Encoding UTF8

    Write-SmokeLog "Meetily stayed running for $LaunchSeconds seconds"
}
finally {
    $running = Get-Process -Id $appProcess.Id -ErrorAction SilentlyContinue
    if ($running) {
        Write-SmokeLog "Stopping Meetily process $($running.Id)"
        Stop-Process -Id $running.Id -Force -ErrorAction SilentlyContinue
    }
}

$knownLogRoots = @(
    (Join-Path $env:APPDATA "meetily"),
    (Join-Path $env:LOCALAPPDATA "meetily")
) | Where-Object { Test-Path -LiteralPath $_ -PathType Container }

foreach ($root in $knownLogRoots) {
    $destination = Join-Path $LogDir ("appdata-" + (Split-Path $root -Leaf))
    Copy-Item -LiteralPath $root -Destination $destination -Recurse -Force -ErrorAction SilentlyContinue
}

Write-SmokeLog "Smoke install and launch completed"
