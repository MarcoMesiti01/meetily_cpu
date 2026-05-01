param(
    [Parameter(Mandatory = $true)]
    [string]$FilePath
)

$ErrorActionPreference = "Stop"

function Fail($Message) {
    throw "[meetily signing] $Message"
}

if (-not (Test-Path -LiteralPath $FilePath)) {
    Fail "File to sign does not exist: $FilePath"
}

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_CODESIGN_CERTIFICATE_P12_BASE64)) {
    Fail "WINDOWS_CODESIGN_CERTIFICATE_P12_BASE64 is required for Windows release signing"
}

if ([string]::IsNullOrWhiteSpace($env:WINDOWS_CODESIGN_CERTIFICATE_PASSWORD)) {
    Fail "WINDOWS_CODESIGN_CERTIFICATE_PASSWORD is required for Windows release signing"
}

$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if (-not $signtool) {
    $candidate = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin" -Recurse -Filter signtool.exe -ErrorAction SilentlyContinue |
        Where-Object { $_.FullName -match "\\x64\\signtool.exe$" } |
        Sort-Object FullName -Descending |
        Select-Object -First 1
    if (-not $candidate) {
        Fail "signtool.exe was not found. Install the Windows SDK on the release runner."
    }
    $signtoolPath = $candidate.FullName
} else {
    $signtoolPath = $signtool.Source
}

$certPath = Join-Path $env:TEMP ("meetily-codesign-" + [guid]::NewGuid().ToString("N") + ".p12")
try {
    [IO.File]::WriteAllBytes($certPath, [Convert]::FromBase64String($env:WINDOWS_CODESIGN_CERTIFICATE_P12_BASE64))

    & $signtoolPath sign `
        /f $certPath `
        /p $env:WINDOWS_CODESIGN_CERTIFICATE_PASSWORD `
        /fd SHA256 `
        /td SHA256 `
        /tr "http://timestamp.digicert.com" `
        /v `
        $FilePath

    if ($LASTEXITCODE -ne 0) {
        Fail "signtool failed for $FilePath"
    }

    $signature = Get-AuthenticodeSignature -LiteralPath $FilePath
    if ($signature.Status -eq "NotSigned" -or -not $signature.SignerCertificate) {
        Fail "Authenticode signature was not applied for $FilePath. Status: $($signature.Status)"
    }

    if ($signature.Status -ne "Valid") {
        Write-Warning "[meetily signing] Authenticode signature is present but not chain-trusted on this runner. Status: $($signature.Status)"
    }

    Write-Host "[meetily signing] signed $FilePath"
} finally {
    if (Test-Path -LiteralPath $certPath) {
        Remove-Item -LiteralPath $certPath -Force
    }
}
