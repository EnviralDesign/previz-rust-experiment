# Previz - Filament Renderer Proof of Concept

A minimal Rust application demonstrating Google Filament renderer integration on Windows.

## Overview

This is a "hello world" proof of concept that:
- Creates a native Windows window using `winit`
- Initializes the Filament rendering engine with OpenGL backend
- Renders a simple colored triangle with vertex colors

## Prerequisites

- Rust 1.75+ (stable)
- Windows 10/11 (x64)
- Visual Studio Build Tools (for linking)

## Building

```bash
cargo build
```

The first build will take a while as it downloads the prebuilt Filament binaries (~700MB).

## Running

```bash
cargo run
```

You should see a window with a colorful triangle (red, green, blue vertices) on a dark blue background.

Press **ESC** or close the window to exit.

## Project Structure

```
previz-rust-experiment/
├── Cargo.toml           # Dependencies and project config
├── src/
│   └── main.rs          # Main application code
└── assets/
    └── bakedColor.mat    # Material source compiled at build time
```

## Technical Notes

### Why custom bindings?
We use a small C++ wrapper and manually authored Rust FFI for Filament v1.69.0.
Filament headers are too complex for `bindgen` due to extensive templates/C++20 usage.

### API Safety
All Filament operations are marked `unsafe` because:
- Filament C++ objects don't use reference counting
- Resources must be manually released
- Memory management is explicit

### Material Files
Filament uses pre-compiled materials (`.filamat` files). We keep the source
material in `assets/bakedColor.mat` and compile it during the build using
Filament's `matc` tool from the downloaded release.

## Next Steps

This POC sets the foundation for:
1. Loading 3D models (using Filament's `filameshio` or `gltfio`)
2. Adding lighting (sun lights, IBL)
3. Creating custom materials
4. Building a full application/UI framework

## Resources

- [Google Filament GitHub](https://github.com/google/filament)
- [Filament Documentation](https://google.github.io/filament/)
- [Filament Materials Guide](https://google.github.io/filament/Materials.html)

## License

This POC is open source. Filament is licensed under Apache 2.0.
