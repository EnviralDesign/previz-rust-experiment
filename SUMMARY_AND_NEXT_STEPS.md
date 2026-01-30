# Filament Integration Summary

## Project Objective
Upgrade the `previz-rust-experiment` project to use **Filament v1.69.0**.

## Journey So Far

### 1. Dependency Analysis & Strategy Shift
*   **Initial Status:** The project relied on an outdated `filament-bindings` crate.
*   **Challenge:** The existing bindings were incompatible with v1.69.0 and maintaining them was not feasible due to the complexity of Filament's C++ API.
*   **Decision:** We opted to drop the external crate and implement **custom FFI bindings** directly within the project.

### 2. The Bindgen Blockade
*   **Attempt:** We initially tried using `bindgen` to automatically generate Rust bindings from Filament's C++ headers.
*   **Failure:** `bindgen` (and tools like `cxx`) choked on Filament's extensive use of modern C++ templates, C++20 features, and STL types.
*   **Pivot:** We switched to a **manual "C-Wrapper" approach**. We write a small C++ file (`bindings.cpp`) that exposes a simplified C-ABI interface, and a corresponding Rust build script that compiles it and generates the Rust FFI signatures (`bindings.rs`).

### 3. Build & Linker Hell
*   **Compilation:** Filament requires **C++20** (`/std:c++20`). We updated our `cc::Build` configuration to match.
*   **Library Mismatches:** We faced several linker errors:
    *   Missing `vkshaders.lib` (renamed/removed in recent Filament versions; replaced with `shaders.lib`).
    *   Missing distinct libraries like `filamat`, `ibl`, which were causing unresolved external errors.
*   **Runtime Linkage (CRT):** Filament's official Windows binaries are built with the **Dynamic CRT (`/MD`)**. Our build initially defaulted to static or mixed CRT settings, causing conflicts. We explicitly forced `/MD` in our build script to match.

### 4. The ABI Crash Loop (The "Recurring Issue")
This has been the most persistent hurdle.
*   **Symptom:** `STATUS_ACCESS_VIOLATION` (0xc0000005) immediately upon calling certain Filament functions.
*   **Root Cause:** **C++ ABI Incompatibility across FFI**.
    *   We were passing the `Entity` class by value between Rust and C++. Even though `Entity` is just a wrapper around a `uint32_t`, passing a C++ class by value from Rust is unsafe because the ABIs (how data is passed in registers vs stack) differ.
*   **Fix:** We rewrote the wrapper functions to pass strict primitives (`int32_t`). We utilized Filament's `Entity::smuggle(entity)` and `Entity::import(id)` to safely convert between the handle and the ID at the boundary.

## Current Status
We now initialize the Engine, Renderer, Scene, View, Camera, and render a triangle
successfully with Filament v1.69.0.

Recent fixes:
* **Enum ABI alignment:** Filament enums are `uint8_t` in v1.69.0. We updated the
  Rust enums/FFI signatures to `u8` to avoid invalid values at the C++ boundary.
* **Material version mismatch:** `bakedColor.filamat` was compiled with an older
  Filament version. We now compile `assets/bakedColor.mat` at build time using
  the v1.69.0 `matc` tool so the material version matches the engine.
* **Shutdown ordering:** We ensure `Engine` is dropped last to avoid use-after-free
  at exit.

## Next Steps
* Remove any remaining warnings we decide to keep (optional).
* Add basic cleanup helpers (e.g., explicit destroy for owned material instances).
* Start loading assets (gltfio/filameshio) and lighting (sun + IBL).
