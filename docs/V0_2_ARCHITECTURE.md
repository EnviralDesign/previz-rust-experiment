# Previz V0.2 Architecture

This is the active architecture plan for the project.

Audience:
- us as implementers
- future collaborators who need to understand direction and constraints quickly

## Product Intent

Build a stable scene framework that lets technical users rig rich visual experiences while exposing a smaller set of controls for creatives.

Near-term:
- editor/runtime in one executable
- robust scene authoring and playback

Mid-term:
- split into Authoring runtime and Playback runtime bundle
- packaged handoff: executable + assets + scene config

## Non-Goals (for now)

- full DCC/editor feature parity
- general-purpose node-graph authoring UI
- deep animation tools
- broad plugin ecosystem

## Design Principles

- Stability first: lifetime-safe runtime and deterministic scene rebuild.
- Data-driven: scene JSON is the contract, not UI state.
- Small surface area: a few strong primitives over many one-off features.
- Portable delivery: paths, assets, and runtime assumptions should support zip-based handoff.

## High-Level Architecture

Current modules:
- `src/app`: event loop, UI interactions, high-level orchestration
- `src/render`: Filament render context and runtime rendering controls
- `src/assets`: glTF import, material instance tracking, provider lifetime management
- `src/scene`: serializable scene data model and persistence
- `src/filament`: wrapper layer around raw FFI
- `src/ffi`: generated low-level bindings

Target layering:
1. `SceneData` (pure serialized model)
2. `SceneRuntime` (resolved handles/resources for current process)
3. `Systems` (assets/material/media/lighting/state)
4. `UI/Commands` (authoring interactions mapped to runtime commands)

## Core Data Model Direction

Evolve current `SceneObjectKind` model toward explicit components:
- `Node`: name, transform, parent/children
- `Renderable`: mesh source, material assignment, visibility
- `Light`: directional/point/spot params
- `Environment`: IBL/skybox/intensity
- `MaterialInstance`: parameter map + texture/media slot bindings
- `MediaSource`: image/video/sequence abstractions
- `Behavior` (optional): simple declarative state/sequence controls

Important rule:
- runtime-only fields (engine entities, pointers, caches) remain non-serialized and must be rebuilt from `SceneData`.

## Runtime Lifecycle

### Load/Rehydrate
1. Parse scene JSON into `SceneData`.
2. Clear runtime scene/resources in safe order.
3. Recreate assets/lights/environment/material bindings.
4. Apply transforms/overrides.
5. Report partial failures without hard crash where possible.

### Save
1. Persist only data model fields.
2. Exclude runtime handles and transient caches.
3. Keep schema stable and versionable.

### Shutdown
Enforce drop ordering:
- material instances -> assets -> providers -> engine-dependent objects

## Scene Command Model (next)

Introduce explicit commands so UI stops mutating runtime state directly:
- `AddAsset`
- `AddLight`
- `SetEnvironment`
- `SetMaterialParam`
- `TransformNode`
- `LoadScene`
- `SaveScene`

Each command:
- validates inputs
- mutates `SceneData`
- updates `SceneRuntime`
- returns structured success/failure

## Asset and Material Strategy

Support both pipelines:
1. Self-contained glTF import (preferred quick path).
2. Kit-of-parts assembly:
   - mesh source
   - material template
   - texture/media slot binding

This supports technical rigging and simplified creative controls later.

## Media and State Direction

Near-term:
- image textures first-class

After stability:
- video textures as `MediaSource`
- lightweight state machine for experiences (timed/video/menu sequences)

State system constraints:
- deterministic and debuggable
- minimal feature surface
- declarative over custom scripting where possible

## Packaging Direction

Define two bundle profiles:
- `AuthoringBundle`: editable scene and diagnostics
- `PlaybackBundle`: limited controls, preconfigured paths, production-safe defaults

Bundle should include:
- executable
- scene json
- assets
- optional profile/config file

## Milestones

### M1: Stability and Boundaries
- finish separating scene data from runtime state
- formalize `SceneRuntime` type
- add load/reload stress test path
- strengthen error reporting for asset/material failures

### M2: Command-Driven Runtime
- add command API for scene mutations
- migrate UI handlers to command calls
- reduce `app` orchestration complexity

### M3: Kit-of-Parts Authoring
- support mesh + material + texture workflows beyond plain glTF
- add parameter/slot metadata for creative-safe controls

### M4: Media + Sequencing
- introduce media source abstraction
- add basic sequence/state controller

### M5: Portable Runtime Profile
- playback-oriented app mode/profile
- packaging/export path for handoff zip bundles

## Engineering Standards

- Always run `cargo check` after changes.
- Prefer explicit errors over panics in runtime/user-triggered paths.
- Add tests for serialization/runtime reconstruction behavior.
- Keep build/FFI artifacts maintainable (`build_support/` sources + small `build.rs` helpers).

## Open Questions

- scene schema versioning approach (`version` field + migrations)
- path strategy for portable bundles (relative root vs asset registry)
- how much behavior belongs in declarative state vs compiled hooks
- minimum creative-facing controls for first playback profile
