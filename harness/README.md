# Harness Workflow

Use this harness to run repeatable import + render validation without manual UI interaction.
Default harness setup includes:
- one configurable light (directional by default)
- AdamsPlace environment (`assets/environments/AdamsPlace`)

## Quick Start

```powershell
pwsh -File .\harness\run_asset_probe.ps1 `
  -AssetPath "C:\repos\glTF-Sample-Assets\Models\FlightHelmet\glTF\FlightHelmet.gltf" `
  -Name "flighthelmet"
```

Outputs:
- `harness/out/<name>.png` window-only screenshot from the app render output
- `harness/out/<name>.report.json` machine-readable result report

Planning docs:
- `harness/MATERIAL_VALIDATION_PLAN.md` material-focused model list and execution status
- `harness/LIGHT_VALIDATION_PLAN.md` light/shadow implementation plan and status

## CLI Flags

- `-AssetPath` required asset path (`.gltf` / `.glb`)
- `-Name` output prefix, default `asset_probe`
- `-SettleFrames` frames to wait before capture, default `90`
- `-MaxFrames` max frames before timeout/fail, default `1500`
- `-NoUi` optional capture without ImGui overlay
- `-NoLight` optional disable default directional light
- `-ExtraAssetPaths` optional additional assets to import after the main asset
- `-LightType` one of `directional`, `sun`, `point`, `spot`, `focused_spot`
- `-NoShadows` optional disable light shadow casting
- `-LightPosition` light position as `x,y,z`
- `-LightDirection` light direction as `x,y,z`
- `-LightRange` local-light range/falloff distance
- `-LightSpotInner` spot inner cone degrees
- `-LightSpotOuter` spot outer cone degrees
- `-LightIntensity` light intensity value
- `-StartMinimized` start harness window minimized (default `true`, set `-StartMinimized:$false` to disable)
- `-Environment` one of `adamsplace`, `artistworkshop`, `none` (default `adamsplace`)
- `-EnvironmentHdr` optional path to `.hdr`; if set, harness generates KTX and uses it

## Direct Command

```powershell
cargo run -- `
  --harness-import "<asset path>" `
  --harness-screenshot "harness\out\<name>.png" `
  --harness-report "harness\out\<name>.report.json" `
  --harness-settle-frames 90 `
  --harness-max-frames 1500 `
  --harness-env adamsplace `
  --harness-light-type directional `
  --harness-light-shadows on `
  --harness-start-minimized
```

## Light Sweep

Run the automated light validation sweep (multi-angle, all supported light types, with transparent extra asset when available):

```powershell
pwsh -File .\harness\run_light_suite.ps1 `
  -BaseAssetPath "assets\gltf\DamagedHelmet.gltf" `
  -TransparentAssetPath "C:\repos\glTF-Sample-Assets\Models\TransmissionRoughnessTest\glTF\TransmissionRoughnessTest.gltf" `
  -OutputNamePrefix "light_sweep_round1"
```

Outputs:
- `harness/out/<OutputNamePrefix>-*.png`
- `harness/out/<OutputNamePrefix>-*.report.json`
- `harness/out/<OutputNamePrefix>/summary.csv`
