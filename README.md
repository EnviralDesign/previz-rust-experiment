# Previz

Previz is a Windows-native Filament-based scene tool focused on robust scene authoring and portable playback bundles.

Current priorities:
- stable scene composition (assets, lights, environment, material tweaks)
- reliable save/load with runtime rehydration
- architecture that supports a simplified creative-facing runtime later

## Current State

Implemented now:
- custom Filament FFI bridge (C++ wrapper + Rust wrappers)
- glTF import via `gltfio`
- directional light and environment workflows (including HDR -> KTX generation)
- material parameter editing for loaded glTF material instances
- scene JSON serialization with runtime handle rebuild on load
- build pipeline split into maintainable support files in `build_support/`

## Vision

The project is moving toward two related products:
1. Authoring tool for technical setup and scene rigging.
2. Portable runtime bundle (zip + exe + assets + scene config) for creative operators.

Design principle:
- keep core runtime flexible and unopinionated
- keep creative-facing controls intentionally small

## Documentation

- `docs/V0_2_ARCHITECTURE.md`: active architecture and implementation plan for both of us.
- `SUMMARY_AND_NEXT_STEPS.md`: legacy integration notes retained for historical context.

## Prerequisites

- Windows 10/11 (x64)
- Rust 1.75+
- Visual Studio Build Tools (MSVC toolchain)

## Build and Run

```bash
cargo check
cargo run
```

First build may download Filament binaries and compile native dependencies.

## Project Layout

```text
build.rs                    Build orchestration for Filament + filagui + bindings
build_support/bindings.cpp  C++ C-ABI wrapper source used by build.rs
build_support/bindings.rs   Rust FFI declarations emitted to OUT_DIR
src/app/                    App loop, UI action handling, scene/runtime coordination
src/render/                 Render context and camera helpers
src/assets/                 Asset loading and lifetime management
src/scene/                  Serializable scene model and IO
src/filament.rs             Safe-ish Rust wrappers over raw FFI
```

## Notes

- Filament is C++ and non-reference-counted; object lifetime and drop ordering are critical.
- This project intentionally uses manual FFI boundaries instead of `bindgen` for stability with Filament's C++ API.
