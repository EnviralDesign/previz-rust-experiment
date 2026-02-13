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

## Priority Set (Broad Coverage First)

| Status | Model | Primary coverage | Asset path |
|---|---|---|---|
| todo | CompareBaseColor | base color factor/texture response | `Models/CompareBaseColor/glTF/CompareBaseColor.gltf` |
| todo | CompareMetallic | metallic range behavior | `Models/CompareMetallic/glTF/CompareMetallic.gltf` |
| todo | CompareRoughness | roughness range behavior | `Models/CompareRoughness/glTF/CompareRoughness.gltf` |
| todo | CompareNormal | normal map contribution | `Models/CompareNormal/glTF/CompareNormal.gltf` |
| todo | CompareAmbientOcclusion | occlusion strength/interaction | `Models/CompareAmbientOcclusion/glTF/CompareAmbientOcclusion.gltf` |
| todo | CompareAlphaCoverage | alpha coverage/cutout behavior | `Models/CompareAlphaCoverage/glTF/CompareAlphaCoverage.gltf` |
| todo | CompareAnisotropy | anisotropy baseline | `Models/CompareAnisotropy/glTF/CompareAnisotropy.gltf` |
| todo | CompareClearcoat | clearcoat and clearcoat roughness | `Models/CompareClearcoat/glTF/CompareClearcoat.gltf` |
| todo | CompareSheen | sheen intensity and tint | `Models/CompareSheen/glTF/CompareSheen.gltf` |
| todo | CompareSpecular | specular extension behavior | `Models/CompareSpecular/glTF/CompareSpecular.gltf` |
| todo | CompareTransmission | transmission baseline | `Models/CompareTransmission/glTF/CompareTransmission.gltf` |
| todo | CompareVolume | volume attenuation/thickness | `Models/CompareVolume/glTF/CompareVolume.gltf` |
| todo | CompareIor | index of refraction | `Models/CompareIor/glTF/CompareIor.gltf` |
| todo | CompareIridescence | iridescence response | `Models/CompareIridescence/glTF/CompareIridescence.gltf` |
| todo | CompareDispersion | dispersion behavior | `Models/CompareDispersion/glTF/CompareDispersion.gltf` |
| todo | CompareEmissiveStrength | emissive strength scaling | `Models/CompareEmissiveStrength/glTF/CompareEmissiveStrength.gltf` |
| todo | DiffuseTransmissionTeacup | diffuse transmission | `Models/DiffuseTransmissionTeacup/glTF/DiffuseTransmissionTeacup.gltf` |
| todo | UnlitTest | unlit workflow correctness | `Models/UnlitTest/glTF/UnlitTest.gltf` |
| todo | FlightHelmet | real-world integrated material stack | `Models/FlightHelmet/glTF/FlightHelmet.gltf` |
| todo | GlassHurricaneCandleHolder | transmission + volume in production-like asset | `Models/GlassHurricaneCandleHolder/glTF/GlassHurricaneCandleHolder.gltf` |

## Secondary Set (Deeper Parameter Sweep)

| Status | Model | Primary coverage | Asset path |
|---|---|---|---|
| todo | MetalRoughSpheresNoTextures | pure parameter-driven metal/rough response | `Models/MetalRoughSpheresNoTextures/glTF/MetalRoughSpheresNoTextures.gltf` |
| todo | MetalRoughSpheres | textured metal/rough response | `Models/MetalRoughSpheres/glTF/MetalRoughSpheres.gltf` |
| todo | AnisotropyStrengthTest | anisotropy strength progression | `Models/AnisotropyStrengthTest/glTF/AnisotropyStrengthTest.gltf` |
| todo | AnisotropyRotationTest | anisotropy direction/rotation | `Models/AnisotropyRotationTest/glTF/AnisotropyRotationTest.gltf` |
| todo | ClearCoatTest | clearcoat regression grid | `Models/ClearCoatTest/glTF/ClearCoatTest.gltf` |
| todo | SheenTestGrid | sheen regression grid | `Models/SheenTestGrid/glTF/SheenTestGrid.gltf` |
| todo | SpecularTest | specular regression grid | `Models/SpecularTest/glTF/SpecularTest.gltf` |
| todo | TransmissionRoughnessTest | transmission vs roughness interaction | `Models/TransmissionRoughnessTest/glTF/TransmissionRoughnessTest.gltf` |
| todo | TransmissionThinwallTestGrid | thin-wall transmission/ior coverage | `Models/TransmissionThinwallTestGrid/glTF/TransmissionThinwallTestGrid.gltf` |
| todo | IridescenceDielectricSpheres | dielectric iridescence cases | `Models/IridescenceDielectricSpheres/glTF/IridescenceDielectricSpheres.gltf` |
| todo | EmissiveStrengthTest | emissive test scene | `Models/EmissiveStrengthTest/glTF/EmissiveStrengthTest.gltf` |
| todo | DiffuseTransmissionPlant | diffuse transmission on organic asset | `Models/DiffuseTransmissionPlant/glTF/DiffuseTransmissionPlant.gltf` |

## Integration Spot-Checks

| Status | Model | Primary coverage | Asset path |
|---|---|---|---|
| todo | DamagedHelmet | high-quality PBR baseline scene | `Models/DamagedHelmet/glTF/DamagedHelmet.gltf` |
| todo | SciFiHelmet | layered materials and mask usage | `Models/SciFiHelmet/glTF/SciFiHelmet.gltf` |
| todo | StainedGlassLamp | layered transmission/ior/clearcoat | `Models/StainedGlassLamp/glTF/StainedGlassLamp.gltf` |
| todo | ToyCar | mixed materials in one asset | `Models/ToyCar/glTF/ToyCar.gltf` |
| todo | CarConcept | complex multi-material extension combo | `Models/CarConcept/glTF/CarConcept.gltf` |

