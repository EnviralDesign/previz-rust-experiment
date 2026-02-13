# Material Validation Plan

This is the working list for validating material and shading correctness in the harness.

Scope for this pass:
- Focus on PBR/material behavior.
- Prefer static models and comparison grids.
- Avoid animation, morph, skinning, camera-only, and compression-only test assets.

Common harness setup for every run:
- Add directional light.
- Add environment (`adamsplace` unless otherwise noted).
- Use window-only screenshot capture.

Asset root:
- `C:\repos\glTF-Sample-Assets\Models`

Status legend:
- `todo` not run yet
- `pass` visually acceptable
- `fail` incorrect result, needs investigation

## Latest Run

- Date: `2026-02-13`
- Harness profile: `--harness-no-ui --harness-env adamsplace --harness-settle-frames 90 --harness-max-frames 1500`
- Output folder: `harness/out/material_sweep_round2`
- Final machine summary: `harness/out/material_sweep_round2/summary_round2_final.csv`
- Result: all listed assets passed import + screenshot harness validation in this run.

## Priority Set (Broad Coverage First)

| Status | Model | Primary coverage | Asset path |
|---|---|---|---|
| pass | CompareBaseColor | base color factor/texture response | `Models/CompareBaseColor/glTF/CompareBaseColor.gltf` |
| pass | CompareMetallic | metallic range behavior | `Models/CompareMetallic/glTF/CompareMetallic.gltf` |
| pass | CompareRoughness | roughness range behavior | `Models/CompareRoughness/glTF/CompareRoughness.gltf` |
| pass | CompareNormal | normal map contribution | `Models/CompareNormal/glTF/CompareNormal.gltf` |
| pass | CompareAmbientOcclusion | occlusion strength/interaction | `Models/CompareAmbientOcclusion/glTF/CompareAmbientOcclusion.gltf` |
| pass | CompareAlphaCoverage | alpha coverage/cutout behavior | `Models/CompareAlphaCoverage/glTF/CompareAlphaCoverage.gltf` |
| pass | CompareAnisotropy | anisotropy baseline | `Models/CompareAnisotropy/glTF/CompareAnisotropy.gltf` |
| pass | CompareClearcoat | clearcoat and clearcoat roughness | `Models/CompareClearcoat/glTF/CompareClearcoat.gltf` |
| pass | CompareSheen | sheen intensity and tint | `Models/CompareSheen/glTF/CompareSheen.gltf` |
| pass | CompareSpecular | specular extension behavior | `Models/CompareSpecular/glTF/CompareSpecular.gltf` |
| pass | CompareTransmission | transmission baseline | `Models/CompareTransmission/glTF/CompareTransmission.gltf` |
| pass | CompareVolume | volume attenuation/thickness | `Models/CompareVolume/glTF/CompareVolume.gltf` |
| pass | CompareIor | index of refraction | `Models/CompareIor/glTF/CompareIor.gltf` |
| pass | CompareIridescence | iridescence response | `Models/CompareIridescence/glTF/CompareIridescence.gltf` |
| pass | CompareDispersion | dispersion behavior | `Models/CompareDispersion/glTF/CompareDispersion.gltf` |
| pass | CompareEmissiveStrength | emissive strength scaling | `Models/CompareEmissiveStrength/glTF/CompareEmissiveStrength.gltf` |
| pass | DiffuseTransmissionTeacup | diffuse transmission | `Models/DiffuseTransmissionTeacup/glTF/DiffuseTransmissionTeacup.gltf` |
| pass | UnlitTest | unlit workflow correctness | `Models/UnlitTest/glTF/UnlitTest.gltf` |
| pass | FlightHelmet | real-world integrated material stack | `Models/FlightHelmet/glTF/FlightHelmet.gltf` |
| pass | GlassHurricaneCandleHolder | transmission + volume in production-like asset | `Models/GlassHurricaneCandleHolder/glTF/GlassHurricaneCandleHolder.gltf` |

## Secondary Set (Deeper Parameter Sweep)

| Status | Model | Primary coverage | Asset path |
|---|---|---|---|
| pass | MetalRoughSpheresNoTextures | pure parameter-driven metal/rough response | `Models/MetalRoughSpheresNoTextures/glTF/MetalRoughSpheresNoTextures.gltf` |
| pass | MetalRoughSpheres | textured metal/rough response | `Models/MetalRoughSpheres/glTF/MetalRoughSpheres.gltf` |
| pass | AnisotropyStrengthTest | anisotropy strength progression | `Models/AnisotropyStrengthTest/glTF/AnisotropyStrengthTest.gltf` |
| pass | AnisotropyRotationTest | anisotropy direction/rotation | `Models/AnisotropyRotationTest/glTF/AnisotropyRotationTest.gltf` |
| pass | ClearCoatTest | clearcoat regression grid | `Models/ClearCoatTest/glTF/ClearCoatTest.gltf` |
| pass | SheenTestGrid | sheen regression grid | `Models/SheenTestGrid/glTF/SheenTestGrid.gltf` |
| pass | SpecularTest | specular regression grid | `Models/SpecularTest/glTF/SpecularTest.gltf` |
| pass | TransmissionRoughnessTest | transmission vs roughness interaction | `Models/TransmissionRoughnessTest/glTF/TransmissionRoughnessTest.gltf` |
| pass | TransmissionThinwallTestGrid | thin-wall transmission/ior coverage | `Models/TransmissionThinwallTestGrid/glTF/TransmissionThinwallTestGrid.gltf` |
| pass | IridescenceDielectricSpheres | dielectric iridescence cases | `Models/IridescenceDielectricSpheres/glTF/IridescenceDielectricSpheres.gltf` |
| pass | EmissiveStrengthTest | emissive test scene | `Models/EmissiveStrengthTest/glTF/EmissiveStrengthTest.gltf` |
| pass | DiffuseTransmissionPlant | diffuse transmission on organic asset | `Models/DiffuseTransmissionPlant/glTF/DiffuseTransmissionPlant.gltf` |

## Integration Spot-Checks

| Status | Model | Primary coverage | Asset path |
|---|---|---|---|
| pass | DamagedHelmet | high-quality PBR baseline scene | `Models/DamagedHelmet/glTF/DamagedHelmet.gltf` |
| pass | SciFiHelmet | layered materials and mask usage | `Models/SciFiHelmet/glTF/SciFiHelmet.gltf` |
| pass | StainedGlassLamp | layered transmission/ior/clearcoat | `Models/StainedGlassLamp/glTF/StainedGlassLamp.gltf` |
| pass | ToyCar | mixed materials in one asset | `Models/ToyCar/glTF/ToyCar.gltf` |
| pass | CarConcept | complex multi-material extension combo | `Models/CarConcept/glTF/CarConcept.gltf` |


