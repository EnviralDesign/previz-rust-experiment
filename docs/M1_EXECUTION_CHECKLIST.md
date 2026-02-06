# M1 Execution Checklist

This is the concrete implementation checklist for **M1: Stability and Boundaries**.

Goal:
- make scene/runtime behavior predictable and resilient before adding heavy authoring UX.

## Exit Criteria

- scene load/save/reload is deterministic and does not leak or crash
- runtime rebuild reports partial failures without process abort
- `SceneData` and runtime handles are clearly separated in code
- camera foundation behavior is consistent (including focus-selected flow)
- `cargo check` and scene serialization tests pass on every change set

## Workstreams

## 1. Runtime Boundary Hardening

- [x] Introduce explicit `SceneRuntime` struct to hold runtime-only state:
  - entity mappings
  - loaded asset handles
  - selected runtime references
- [x] Remove remaining implicit runtime state from `SceneState` where possible.
- [x] Ensure load path is `SceneData -> rebuild runtime` only.
- [x] Ensure clear/rebuild order is explicit and tested.

## 2. Command Skeleton

- [x] Add command enum and execution entry point:
  - `LoadScene`
  - `SaveScene`
  - `AddAsset`
  - `AddLight`
  - `SetEnvironment`
  - `TransformNode`
- [x] Migrate one existing operation at a time from direct mutation to commands. (scene edits now routed through command execution, including transform/light/environment updates)
- [x] Standardize command result shape (success/warning/error payload).

## 3. Error and Diagnostics

- [ ] Replace remaining panic-prone user-triggered paths with structured errors. (partial)
- [x] Add concise UI-visible status for command failures and partial rebuild failures.
- [x] Add logging around load/rebuild phases with object counts and per-object failure context.

## 4. Camera/Navigation Foundation

- [x] Define camera interaction contract (orbit/pan/dolly semantics).
- [x] Implement/normalize focus-selected (`F`) flow:
  - if selection exists, frame selected bounds
  - if no selection, no-op or frame whole scene (choose and document)
- [x] Keep camera behavior independent from future picking/gizmo systems.

## 5. Selection Foundation (without full picking yet)

- [x] Define single source of truth for current selection id/index.
- [x] Ensure outliner selection updates runtime selection state deterministically.
- [x] Add selected-object viewport highlight hook (stub is acceptable for M1 if wired cleanly).

## 6. Testing and Validation

- [x] Add a load/reload stress loop test harness path (non-GPU logic where possible).
- [x] Keep serialization tests green and expand as schema evolves.
- [x] Add smoke tests for runtime rebuild error aggregation behavior.

## Implementation Order (Recommended)

1. `SceneRuntime` type introduction
2. command skeleton with `LoadScene` and `SaveScene`
3. runtime rebuild integration through command path
4. camera contract + focus-selected flow
5. selection state unification
6. diagnostics and test expansion

## Deferred to M2/M3

- click-to-select picking
- non-visual object proxies/icons
- transform gizmos (translate/rotate/scale)

These are intentionally deferred to preserve foundational quality.
