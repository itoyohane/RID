param(
  [switch]$RequireSignature
)

$ErrorActionPreference = "Stop"

$repositoryRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$releaseRoot = Join-Path $repositoryRoot "src-tauri\target\release"
$tauriConfig = Get-Content (Join-Path $repositoryRoot "src-tauri\tauri.conf.json") -Raw |
  ConvertFrom-Json
$version = $tauriConfig.version
$files = @(
  (Join-Path $releaseRoot "rid.exe"),
  (Join-Path $releaseRoot "bundle\nsis\RID_${version}_x64-setup.exe"),
  (Join-Path $releaseRoot "bundle\msi\RID_${version}_x64_en-US.msi")
)

$missing = @($files | Where-Object { -not (Test-Path -LiteralPath $_ -PathType Leaf) })
if ($missing.Count -gt 0) {
  throw "Missing release files: $($missing -join ', ')"
}

$nsisScript = Join-Path $releaseRoot "nsis\x64\installer.nsi"
if (-not (Test-Path -LiteralPath $nsisScript -PathType Leaf)) {
  throw "Generated NSIS script was not found: $nsisScript"
}

$nsisSource = Get-Content -LiteralPath $nsisScript -Raw
$requiredNsisMarkers = @(
  '!define INSTALLMODE "currentUser"',
  '${Silent}',
  'RequestExecutionLevel user'
)

foreach ($marker in $requiredNsisMarkers) {
  if (-not $nsisSource.Contains($marker)) {
    throw "Generated NSIS script is missing required marker: $marker"
  }
}

$checksumLines = foreach ($file in $files) {
  $signature = Get-AuthenticodeSignature -LiteralPath $file
  $name = Split-Path -Leaf $file
  Write-Host "$name Authenticode status: $($signature.Status)"

  if ($RequireSignature -and $signature.Status -ne "Valid") {
    throw "$name does not have a valid Authenticode signature."
  }

  $hash = Get-FileHash -LiteralPath $file -Algorithm SHA256
  "$($hash.Hash.ToLowerInvariant())  $name"
}

$checksumPath = Join-Path $releaseRoot "bundle\SHA256SUMS.txt"
[System.IO.File]::WriteAllLines(
  $checksumPath,
  $checksumLines,
  [System.Text.UTF8Encoding]::new($false)
)

Write-Host "NSIS silent-install markers verified."
Write-Host "Checksums written to $checksumPath"
