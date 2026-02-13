param(
    [string]$BaseAssetPath = "assets/gltf/DamagedHelmet.gltf",
    [string]$TransparentAssetPath = "C:\repos\glTF-Sample-Assets\Models\TransmissionRoughnessTest\glTF\TransmissionRoughnessTest.gltf",
    [string]$OutputNamePrefix = "light_sweep_round1",
    [int]$SettleFrames = 140,
    [int]$MaxFrames = 2200,
    [bool]$StartMinimized = $true,
    [ValidateSet("adamsplace", "artistworkshop", "none")]
    [string]$Environment = "adamsplace"
)

$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$outDir = Join-Path $PSScriptRoot "out\$OutputNamePrefix"
New-Item -ItemType Directory -Path $outDir -Force | Out-Null

$transparentExtras = @()
if (-not [string]::IsNullOrWhiteSpace($TransparentAssetPath) -and (Test-Path $TransparentAssetPath)) {
    $transparentExtras += $TransparentAssetPath
}

$cases = @(
    @{ Name = "directional_shadow_on_a"; LightType = "directional"; Shadows = $true;  LightDirection = "-0.30,-1.00,-0.40"; LightPosition = "0,3,3" },
    @{ Name = "directional_shadow_off";  LightType = "directional"; Shadows = $false; LightDirection = "-0.30,-1.00,-0.40"; LightPosition = "0,3,3" },
    @{ Name = "directional_shadow_on_b"; LightType = "directional"; Shadows = $true;  LightDirection = "0.50,-1.00,-0.25";  LightPosition = "1,3,2" },
    @{ Name = "sun_shadow_on_a";         LightType = "sun";         Shadows = $true;  LightDirection = "-0.20,-1.00,-0.55"; LightPosition = "0,4,3" },
    @{ Name = "sun_shadow_off";          LightType = "sun";         Shadows = $false; LightDirection = "-0.20,-1.00,-0.55"; LightPosition = "0,4,3" },
    @{ Name = "sun_shadow_on_b";         LightType = "sun";         Shadows = $true;  LightDirection = "0.35,-1.00,-0.30";  LightPosition = "0,4,2" },
    @{ Name = "point_shadow_off";        LightType = "point";       Shadows = $false; LightDirection = "0,-1,0";            LightPosition = "0,2,2" },
    @{ Name = "point_shadow_off_side";   LightType = "point";       Shadows = $false; LightDirection = "0,-1,0";            LightPosition = "2,2,1" },
    @{ Name = "spot_shadow_on_a";        LightType = "spot";        Shadows = $true;  LightDirection = "-0.25,-0.95,-0.35"; LightPosition = "1.5,2.5,2.5" },
    @{ Name = "spot_shadow_off";         LightType = "spot";        Shadows = $false; LightDirection = "-0.25,-0.95,-0.35"; LightPosition = "1.5,2.5,2.5" },
    @{ Name = "spot_shadow_on_b";        LightType = "spot";        Shadows = $true;  LightDirection = "0.30,-0.95,-0.10";  LightPosition = "-1.5,2.0,2.0" },
    @{ Name = "focused_spot_shadow_on";  LightType = "focused_spot";Shadows = $true;  LightDirection = "-0.20,-0.98,-0.20"; LightPosition = "1.0,2.2,2.0" },
    @{ Name = "focused_spot_shadow_off"; LightType = "focused_spot";Shadows = $false; LightDirection = "-0.20,-0.98,-0.20"; LightPosition = "1.0,2.2,2.0" },
    @{ Name = "focused_spot_shadow_on_b";LightType = "focused_spot";Shadows = $true;  LightDirection = "0.28,-0.95,-0.12";  LightPosition = "-1.2,2.0,2.2" }
)

$summary = @()
$probeScript = Join-Path $PSScriptRoot "run_asset_probe.ps1"

foreach ($case in $cases) {
    $name = $case.Name
    $runName = "$OutputNamePrefix-$name"
    Write-Host "Running case: $name"

    $probeArgs = @{
        AssetPath = $BaseAssetPath
        ExtraAssetPaths = $transparentExtras
        Name = $runName
        SettleFrames = $SettleFrames
        MaxFrames = $MaxFrames
        Environment = $Environment
        LightType = $case.LightType
        LightPosition = $case.LightPosition
        LightDirection = $case.LightDirection
        LightIntensity = 100000.0
        LightRange = 10.0
        LightSpotInner = 20.0
        LightSpotOuter = 35.0
        StartMinimized = $StartMinimized
    }
    if (-not $case.Shadows) {
        $probeArgs["NoShadows"] = $true
    }

    Push-Location $repoRoot
    try {
        & $probeScript @probeArgs
        $exitCode = $LASTEXITCODE
    } finally {
        Pop-Location
    }

    $reportPath = Join-Path $PSScriptRoot "out\$runName.report.json"
    $pngPath = Join-Path $PSScriptRoot "out\$runName.png"
    $status = "fail"
    $importSuccess = $false
    $screenshotSuccess = $false
    $frameCount = 0
    $errorMessage = ""
    if (Test-Path $reportPath) {
        try {
            $report = Get-Content $reportPath -Raw | ConvertFrom-Json
            $importSuccess = [bool]$report.import_success
            $screenshotSuccess = [bool]$report.screenshot_success
            $frameCount = [int]$report.frame_count
            if ($importSuccess -and $screenshotSuccess -and $exitCode -eq 0) {
                $status = "pass"
            } else {
                $status = "fail"
                $errorMessage = [string]$report.screenshot_error
                if ([string]::IsNullOrWhiteSpace($errorMessage)) {
                    $errorMessage = [string]$report.import_error
                }
            }
        } catch {
            $status = "fail"
            $errorMessage = "failed parsing report"
        }
    } else {
        $errorMessage = "missing report file"
    }

    $summary += [PSCustomObject]@{
        case = $name
        status = $status
        exit_code = $exitCode
        light_type = $case.LightType
        shadows = [bool]$case.Shadows
        import_success = $importSuccess
        screenshot_success = $screenshotSuccess
        frame_count = $frameCount
        report_path = $reportPath
        screenshot_path = $pngPath
        notes = $errorMessage
    }
}

$summaryPath = Join-Path $outDir "summary.csv"
$summary | Export-Csv -NoTypeInformation -Path $summaryPath -Encoding UTF8
Write-Host "Light suite summary: $summaryPath"
