param(
    [switch]$RequireModel
)

$ErrorActionPreference = "Stop"

$FrontendDir = Split-Path -Parent $PSScriptRoot
$RuntimeRoot = Join-Path $FrontendDir "src-tauri\resources\runtime\windows-x64"
$PythonExe = Join-Path $RuntimeRoot "python\python.exe"
$BackendApp = Join-Path $RuntimeRoot "backend\app\main.py"
$FwsExe = Join-Path $RuntimeRoot "python\Scripts\faster-whisper-server.exe"
$ModelDir = Join-Path $FrontendDir "src-tauri\resources\models\faster-whisper-base"
$env:PYTHONPATH = @(
    (Join-Path $RuntimeRoot "backend\app")
    (Join-Path $RuntimeRoot "python\Lib")
    (Join-Path $RuntimeRoot "python\Lib\site-packages")
) -join ";"

function Assert-Path($Path, $Label) {
    if (-not (Test-Path $Path)) {
        throw "$Label missing at $Path"
    }
}

Assert-Path $PythonExe "Bundled Python"
Assert-Path $BackendApp "Bundled backend app"
Assert-Path $FwsExe "faster-whisper-server launcher"

if ($RequireModel) {
    Assert-Path $ModelDir "Bundled faster-whisper base model"
}

& $PythonExe -c "import fastapi, uvicorn, faster_whisper, faster_whisper_server; import main; print('runtime imports ok')" `
    2>&1 | Write-Host

if ($LASTEXITCODE -ne 0) {
    throw "Bundled runtime import smoke check failed"
}

Write-Host "[meetily runtime] bundled runtime smoke check passed"
