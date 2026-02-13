param(
    [Parameter(Mandatory = $true)]
    [string]$AssetPath,
    [string]$Name = "asset_probe",
    [int]$SettleFrames = 90,
    [int]$MaxFrames = 1500,
    [switch]$NoUi,
    [switch]$NoLight,
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
    "--harness-env", $Environment
)

if ($NoUi) {
    $args += "--harness-no-ui"
}
if ($NoLight) {
    $args += "--harness-no-light"
}
if (-not [string]::IsNullOrWhiteSpace($EnvironmentHdr)) {
    $args += @("--harness-env-hdr", $EnvironmentHdr)
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
