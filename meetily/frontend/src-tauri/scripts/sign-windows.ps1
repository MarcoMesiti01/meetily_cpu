param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath
)

$ErrorActionPreference = "Stop"

$releaseSigningScript = Join-Path $PSScriptRoot "..\..\scripts\sign-windows.ps1"
if (-not (Test-Path -LiteralPath $releaseSigningScript -PathType Leaf)) {
    throw "[meetily signing] Release signing script not found: $releaseSigningScript"
}

& $releaseSigningScript -FilePath $FilePath
exit 0
