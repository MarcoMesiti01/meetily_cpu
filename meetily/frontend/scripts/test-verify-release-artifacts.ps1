$ErrorActionPreference = "Stop"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$Verifier = Join-Path $ScriptDir "verify-release-artifacts.ps1"
$Root = Join-Path $env:TEMP ("meetily-release-test-" + [guid]::NewGuid().ToString("N"))
$NsisDir = Join-Path $Root "nsis"
$MsiDir = Join-Path $Root "msi"
$UpdaterDir = Join-Path $Root "updater"
$RuntimeDir = Join-Path $Root "runtime"
$ModelDir = Join-Path $Root "model"

New-Item -ItemType Directory -Force -Path $NsisDir, $MsiDir, $UpdaterDir, $RuntimeDir, $ModelDir | Out-Null

Set-Content -Path (Join-Path $NsisDir "meetily_1.2.3_x64-setup.exe") -Value "fake exe"
Set-Content -Path (Join-Path $MsiDir "meetily_1.2.3_x64_en-US.msi") -Value "fake msi"
Set-Content -Path (Join-Path $UpdaterDir "meetily_1.2.3_x64-setup.nsis.zip") -Value "fake updater"
Set-Content -Path (Join-Path $UpdaterDir "meetily_1.2.3_x64-setup.nsis.zip.sig") -Value "fake signature"
Set-Content -Path (Join-Path $RuntimeDir "python.exe") -Value "fake python"
Set-Content -Path (Join-Path $ModelDir "config.json") -Value "{}"

$UnsignedFailed = $false
try {
    & $Verifier `
        -NsisDir $NsisDir `
        -MsiDir $MsiDir `
        -UpdaterDir $UpdaterDir `
        -RuntimeDir $RuntimeDir `
        -ModelDir $ModelDir `
        -OutputDir $Root `
        -RequireAuthenticode
} catch {
    $UnsignedFailed = $_.Exception.Message -match "Authenticode"
}

if (-not $UnsignedFailed) {
    throw "Expected unsigned artifact verification to fail with an Authenticode error"
}

& $Verifier `
    -NsisDir $NsisDir `
    -MsiDir $MsiDir `
    -UpdaterDir $UpdaterDir `
    -RuntimeDir $RuntimeDir `
    -ModelDir $ModelDir `
    -OutputDir $Root

if (-not (Test-Path (Join-Path $Root "SHA256SUMS.txt"))) {
    throw "Expected SHA256SUMS.txt to be generated"
}

Remove-Item -LiteralPath $Root -Recurse -Force
Write-Host "release artifact verification test passed"
