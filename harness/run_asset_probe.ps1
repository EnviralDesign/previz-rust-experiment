param(
    [Parameter(Mandatory = $true)]
    [string]$AssetPath,
    [string[]]$ExtraAssetPaths = @(),
    [string]$Name = "asset_probe",
    [int]$SettleFrames = 90,
    [int]$MaxFrames = 1500,
    [switch]$NoUi,
    [switch]$NoLight,
    [ValidateSet("directional", "sun", "point", "spot", "focused_spot")]
    [string]$LightType = "directional",
    [switch]$NoShadows,
    [string]$LightPosition = "0,2,2",
    [string]$LightDirection = "0,-1,-0.5",
    [double]$LightRange = 10.0,
    [double]$LightSpotInner = 25.0,
    [double]$LightSpotOuter = 35.0,
    [double]$LightIntensity = 100000.0,
    [bool]$StartMinimized = $true,
    [ValidateSet("adamsplace", "artistworkshop", "none")]
    [string]$Environment = "adamsplace",
    [string]$EnvironmentHdr = ""
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$outDir = Join-Path $PSScriptRoot "out"
New-Item -ItemType Directory -Path $outDir -Force | Out-Null

$screenshotPath = Join-Path $outDir "$Name.png"
$reportPath = Join-Path $outDir "$Name.report.json"

$args = @(
    "run",
    "--",
    "--harness-import", $AssetPath,
    "--harness-screenshot", $screenshotPath,
    "--harness-report", $reportPath,
    "--harness-settle-frames", "$SettleFrames",
    "--harness-max-frames", "$MaxFrames",
    "--harness-env", $Environment,
    "--harness-light-type", $LightType,
    "--harness-light-position", $LightPosition,
    "--harness-light-direction", $LightDirection,
    "--harness-light-range", "$LightRange",
    "--harness-light-spot-inner", "$LightSpotInner",
    "--harness-light-spot-outer", "$LightSpotOuter",
    "--harness-light-intensity", "$LightIntensity"
)

if ($NoUi) {
    $args += "--harness-no-ui"
}
if ($NoLight) {
    $args += "--harness-no-light"
}
if ($NoShadows) {
    $args += @("--harness-light-shadows", "off")
}
if ($StartMinimized) {
    $args += "--harness-start-minimized"
}
if (-not [string]::IsNullOrWhiteSpace($EnvironmentHdr)) {
    $args += @("--harness-env-hdr", $EnvironmentHdr)
}
foreach ($extraPath in $ExtraAssetPaths) {
    if (-not [string]::IsNullOrWhiteSpace($extraPath)) {
        $args += @("--harness-import-extra", $extraPath)
    }
}

Write-Host "Running harness for: $AssetPath"
Push-Location $repoRoot
try {
    & cargo @args
    $exitCode = $LASTEXITCODE
} finally {
    Pop-Location
}

if ($exitCode -ne 0) {
    throw "Harness run failed with exit code $exitCode"
}

Write-Host "Harness report: $reportPath"
Get-Content $reportPath
