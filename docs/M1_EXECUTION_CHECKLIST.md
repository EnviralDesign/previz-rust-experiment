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

- [ ] Introduce explicit `SceneRuntime` struct to hold runtime-only state:
  - entity mappings
  - loaded asset handles
  - selected runtime references
- [ ] Remove remaining implicit runtime state from `SceneState` where possible.
- [ ] Ensure load path is `SceneData -> rebuild runtime` only.
- [ ] Ensure clear/rebuild order is explicit and tested.

## 2. Command Skeleton

- [ ] Add command enum and execution entry point:
  - `LoadScene`
  - `SaveScene`
  - `AddAsset`
  - `AddLight`
  - `SetEnvironment`
  - `TransformNode`
- [ ] Migrate one existing operation at a time from direct mutation to commands.
- [ ] Standardize command result shape (success/warning/error payload).

## 3. Error and Diagnostics

- [ ] Replace remaining panic-prone user-triggered paths with structured errors.
- [ ] Add concise UI-visible status for command failures and partial rebuild failures.
- [ ] Add logging around load/rebuild phases with object counts and per-object failure context.

## 4. Camera/Navigation Foundation

- [ ] Define camera interaction contract (orbit/pan/dolly semantics).
- [ ] Implement/normalize focus-selected (`F`) flow:
  - if selection exists, frame selected bounds
  - if no selection, no-op or frame whole scene (choose and document)
- [ ] Keep camera behavior independent from future picking/gizmo systems.

## 5. Selection Foundation (without full picking yet)

- [ ] Define single source of truth for current selection id/index.
- [ ] Ensure outliner selection updates runtime selection state deterministically.
- [ ] Add selected-object viewport highlight hook (stub is acceptable for M1 if wired cleanly).

## 6. Testing and Validation

- [ ] Add a load/reload stress loop test harness path (non-GPU logic where possible).
- [ ] Keep serialization tests green and expand as schema evolves.
- [ ] Add smoke tests for runtime rebuild error aggregation behavior.

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
