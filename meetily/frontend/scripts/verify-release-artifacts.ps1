param(
    [string]$NsisDir = "target\release\bundle\nsis",
    [string]$MsiDir = "target\release\bundle\msi",
    [string]$UpdaterDir = "target\release\bundle",
    [string]$RuntimeDir = "src-tauri\resources\runtime\windows-x64",
    [string]$ModelDir = "src-tauri\resources\models\faster-whisper-base",
    [string]$SidecarDir = "src-tauri\binaries",
    [string]$OutputDir = ".",
    [switch]$RequireAuthenticode
)

$ErrorActionPreference = "Stop"

function Resolve-RequiredDirectory {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Container)) {
        throw "$Label directory is missing: $Path"
    }

    return (Resolve-Path -LiteralPath $Path).Path
}

function Get-RequiredFile {
    param(
        [Parameter(Mandatory = $true)][string]$Directory,
        [Parameter(Mandatory = $true)][string]$Filter,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $file = Get-ChildItem -LiteralPath $Directory -Filter $Filter -File -Recurse |
        Where-Object { $_.Length -gt 0 } |
        Sort-Object FullName |
        Select-Object -First 1

    if (-not $file) {
        throw "$Label artifact is missing or empty in $Directory ($Filter)"
    }

    return $file
}

function Assert-DirectoryHasFiles {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Label
    )

    $hasFiles = Get-ChildItem -LiteralPath $Path -File -Recurse | Select-Object -First 1
    if (-not $hasFiles) {
        throw "$Label directory has no files: $Path"
    }
}

function Assert-AuthenticodeValid {
    param([Parameter(Mandatory = $true)][System.IO.FileInfo]$File)

    $signature = Get-AuthenticodeSignature -LiteralPath $File.FullName
    if ($signature.Status -eq "NotSigned" -or -not $signature.SignerCertificate) {
        throw "Authenticode signature is missing for $($File.FullName): $($signature.Status)"
    }

    if ($signature.Status -ne "Valid") {
        Write-Warning "Authenticode signature is present but not chain-trusted for $($File.FullName): $($signature.Status)"
    }
}

function Add-Sha256Line {
    param(
        [Parameter(Mandatory = $true)][System.IO.FileInfo]$File,
        [System.Collections.Generic.List[string]]$Lines
    )

    $hash = Get-FileHash -LiteralPath $File.FullName -Algorithm SHA256
    $Lines.Add("$($hash.Hash)  $($File.Name)")
}

$nsisRoot = Resolve-RequiredDirectory -Path $NsisDir -Label "NSIS"
$msiRoot = Resolve-RequiredDirectory -Path $MsiDir -Label "MSI"
$updaterRoot = Resolve-RequiredDirectory -Path $UpdaterDir -Label "Updater"
$runtimeRoot = Resolve-RequiredDirectory -Path $RuntimeDir -Label "Bundled runtime"
$modelRoot = Resolve-RequiredDirectory -Path $ModelDir -Label "Bundled faster-whisper model"

$nsisInstaller = Get-RequiredFile -Directory $nsisRoot -Filter "*.exe" -Label "NSIS installer"
$msiInstaller = Get-RequiredFile -Directory $msiRoot -Filter "*.msi" -Label "MSI installer"

$updaterArtifacts = Get-ChildItem -LiteralPath $updaterRoot -File -Recurse |
    Where-Object {
        $_.Length -gt 0 -and (
            $_.Name -eq "latest.json" -or
            $_.Name.EndsWith(".sig", [StringComparison]::OrdinalIgnoreCase) -or
            $_.Name.EndsWith(".zip", [StringComparison]::OrdinalIgnoreCase) -or
            $_.Name.EndsWith(".tar.gz", [StringComparison]::OrdinalIgnoreCase) -or
            $_.Name.EndsWith(".nsis.zip", [StringComparison]::OrdinalIgnoreCase)
        )
    } |
    Sort-Object FullName

if (-not $updaterArtifacts -or $updaterArtifacts.Count -eq 0) {
    throw "Updater artifacts are missing or empty in $updaterRoot"
}

Assert-DirectoryHasFiles -Path $runtimeRoot -Label "Bundled runtime"
Assert-DirectoryHasFiles -Path $modelRoot -Label "Bundled faster-whisper model"

$runtimePython = @(
    Join-Path $runtimeRoot "python\python.exe"
    Join-Path $runtimeRoot "python.exe"
) | Where-Object { Test-Path -LiteralPath $_ -PathType Leaf } | Select-Object -First 1

if (-not $runtimePython) {
    throw "Bundled runtime does not include python.exe under $runtimeRoot"
}

if ($RequireAuthenticode) {
    Assert-AuthenticodeValid -File $nsisInstaller
    Assert-AuthenticodeValid -File $msiInstaller

    if (Test-Path -LiteralPath $SidecarDir -PathType Container) {
        Get-ChildItem -LiteralPath $SidecarDir -Filter "*.exe" -File -Recurse |
            ForEach-Object { Assert-AuthenticodeValid -File $_ }
    }
}

New-Item -ItemType Directory -Force -Path $OutputDir | Out-Null

$publishedArtifacts = Get-ChildItem -LiteralPath $OutputDir -File -ErrorAction SilentlyContinue |
    Where-Object {
        $_.Name -ne "SHA256SUMS.txt" -and
        $_.Name -ne "RELEASE_NOTES.md" -and
        $_.Length -gt 0
    } |
    Sort-Object Name

$hashTargets = if ($publishedArtifacts -and $publishedArtifacts.Count -gt 0) {
    $publishedArtifacts
} else {
    @($nsisInstaller, $msiInstaller) + @($updaterArtifacts)
}

$hashLines = [System.Collections.Generic.List[string]]::new()
foreach ($artifact in $hashTargets) {
    Add-Sha256Line -File $artifact -Lines $hashLines
}

$shaFile = Join-Path $OutputDir "SHA256SUMS.txt"
$hashLines | Set-Content -LiteralPath $shaFile -Encoding UTF8

Write-Host "Verified release artifacts:"
Write-Host "  NSIS: $($nsisInstaller.FullName)"
Write-Host "  MSI: $($msiInstaller.FullName)"
Write-Host "  Updater artifacts: $($updaterArtifacts.Count)"
Write-Host "  Runtime: $runtimeRoot"
Write-Host "  Model: $modelRoot"
Write-Host "  Checksums: $shaFile"
