param(
    [string]$PythonVersion = "3.11.9",
    [string]$FasterWhisperServerVersion = "0.0.2",
    [switch]$SkipModelDownload
)

$ErrorActionPreference = "Stop"

$FrontendDir = Split-Path -Parent $PSScriptRoot
$RepoRoot = Split-Path -Parent $FrontendDir
$RuntimeRoot = Join-Path $FrontendDir "src-tauri\resources\runtime\windows-x64"
$PythonDir = Join-Path $RuntimeRoot "python"
$BackendTarget = Join-Path $RuntimeRoot "backend\app"
$ModelsTarget = Join-Path $FrontendDir "src-tauri\resources\models\faster-whisper-base"
$DownloadDir = Join-Path $FrontendDir ".runtime-cache"
$PythonZip = Join-Path $DownloadDir "python-$PythonVersion-embed-amd64.zip"
$GetPip = Join-Path $DownloadDir "get-pip.py"
$RequirementsFile = Join-Path $DownloadDir "runtime-requirements.txt"

function Write-Step($Message) {
    Write-Host "[meetily runtime] $Message"
}

function Reset-Directory($Path) {
    if (Test-Path $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $Path | Out-Null
}

New-Item -ItemType Directory -Force -Path $DownloadDir | Out-Null
Reset-Directory $RuntimeRoot

Write-Step "Downloading Python $PythonVersion embeddable runtime"
$PythonUrl = "https://www.python.org/ftp/python/$PythonVersion/python-$PythonVersion-embed-amd64.zip"
Invoke-WebRequest -Uri $PythonUrl -OutFile $PythonZip
Expand-Archive -Path $PythonZip -DestinationPath $PythonDir -Force

Write-Step "Enabling site-packages in embedded Python"
$PthFile = Get-ChildItem -Path $PythonDir -Filter "python*._pth" | Select-Object -First 1
if (-not $PthFile) {
    throw "Could not find Python ._pth file in $PythonDir"
}
$PthContent = Get-Content $PthFile.FullName
$PthContent = $PthContent | ForEach-Object {
    if ($_ -eq "#import site") { "import site" } else { $_ }
}
$PthContent = @($PthContent) + @("Lib", "Lib\site-packages", "..\backend\app")
Set-Content -Path $PthFile.FullName -Value $PthContent -Encoding ASCII
New-Item -ItemType Directory -Force -Path (Join-Path $PythonDir "Lib\site-packages") | Out-Null

Write-Step "Installing pip into embedded Python"
Invoke-WebRequest -Uri "https://bootstrap.pypa.io/get-pip.py" -OutFile $GetPip
& (Join-Path $PythonDir "python.exe") $GetPip --no-warn-script-location

Write-Step "Preparing runtime requirements"
$BackendRequirements = Get-Content (Join-Path $RepoRoot "backend\requirements.txt")
$RuntimeRequirements = @(
    $BackendRequirements
    "faster-whisper-server==$FasterWhisperServerVersion"
)
Set-Content -Path $RequirementsFile -Value $RuntimeRequirements -Encoding ASCII

Write-Step "Installing backend and faster-whisper-server dependencies"
& (Join-Path $PythonDir "python.exe") -m pip install `
    --disable-pip-version-check `
    --no-warn-script-location `
    -r $RequirementsFile

if ($LASTEXITCODE -ne 0) {
    throw "Failed to install bundled runtime Python dependencies"
}

Write-Step "Adding faster-whisper-server wheel metadata shim"
$SitePackagesDir = Join-Path $PythonDir "Lib\site-packages"
$FasterWhisperServerPyproject = Join-Path $SitePackagesDir "pyproject.toml"
if (-not (Test-Path -LiteralPath $FasterWhisperServerPyproject -PathType Leaf)) {
    @(
        "[project]"
        "version = `"$FasterWhisperServerVersion`""
    ) | Set-Content -LiteralPath $FasterWhisperServerPyproject -Encoding ASCII
}

Write-Step "Copying backend application source"
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $BackendTarget) | Out-Null
Copy-Item -Path (Join-Path $RepoRoot "backend\app") -Destination (Split-Path -Parent $BackendTarget) -Recurse -Force

if (-not $SkipModelDownload) {
    Write-Step "Downloading Systran/faster-whisper-base model"
    New-Item -ItemType Directory -Force -Path $ModelsTarget | Out-Null
    & (Join-Path $PythonDir "python.exe") -c "from huggingface_hub import snapshot_download; snapshot_download(repo_id='Systran/faster-whisper-base', local_dir=r'$ModelsTarget')"
} else {
    Write-Step "Skipping model download by request"
}

Write-Step "Validating staged Python imports"
& (Join-Path $PythonDir "python.exe") -c "import fastapi, uvicorn, faster_whisper, faster_whisper_server; import sys; print(sys.version)"

if ($LASTEXITCODE -ne 0) {
    throw "Bundled runtime import validation failed"
}

Write-Step "Runtime staged at $RuntimeRoot"
