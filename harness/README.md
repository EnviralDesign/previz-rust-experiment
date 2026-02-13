# Harness Workflow

Use this harness to run repeatable import + render validation without manual UI interaction.
Default harness setup includes:
- one directional light
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

## CLI Flags

- `-AssetPath` required asset path (`.gltf` / `.glb`)
- `-Name` output prefix, default `asset_probe`
- `-SettleFrames` frames to wait before capture, default `90`
- `-MaxFrames` max frames before timeout/fail, default `1500`
- `-NoUi` optional capture without ImGui overlay
- `-NoLight` optional disable default directional light
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
  --harness-env adamsplace
```
