# Light + Shadow Validation Plan

This is the working plan for implementing and validating full light-type support plus editor/harness coverage.

Status legend:
- `todo` not started
- `in_progress` active work
- `done` completed and verified
- `blocked` needs user/product decision

## Goals

- Formalize light objects so we can author every Filament-supported light type from the editor.
- Keep the existing directional-light path, but rename and model it explicitly.
- Add per-light shadow controls with safe defaults.
- Add pickable visual helper geometry for each light type, rendered in the same overlay/picking style family as the transform gizmo.
- Extend harness to run a repeatable light test suite and capture the **full application window** (viewport + UI).
- Validate lighting and shadows from multiple angles, including transparent-object behavior.

## Constraints and Non-Goals

- No giant one-off batch script. Use a tracked plan + small reusable harness workflow.
- Keep defaults stable first; advanced global shadow tuning can stay centralized in render config for now.
- Do not change transform gizmo interaction model; light helpers should integrate with existing selection/pick flow.

## Implementation Phases

| Status | Phase | Outcome |
|---|---|---|
| done | 0. Baseline + API audit | Locked supported list to `Directional`, `Sun`, `Point`, `Spot`, `FocusedSpot`; default parameter ranges and shadow subset defined in `LightData`. |
| done | 1. Scene model refactor | Added `SceneObjectKind::Light(LightData)` and load-time migration from legacy `DirectionalLight`. |
| done | 2. FFI + safe wrappers | Added generic light create/update FFI with per-type params and shadow-option subset. |
| done | 3. Runtime wiring | Replaced directional-only paths with typed light runtime updates for add/update/delete/rebuild. |
| done | 4. Editor UX | Added explicit create actions for all light types and type-aware inspector controls. |
| done | 5. Light helper overlay | Added pickable helper geometry per light type with overlay/pick integration and selection feedback. |
| done | 6. Harness light suite | Added reusable harness sweep with light/camera variants, full-window capture, and per-case reports. |
| done | 7. Validation + fixes | Completed sweep and fixed issues until all cases passed. |

## Phase Details

### Phase 0: Baseline + API audit

- Validate Filament light types available in our build and lock the supported list in this doc.
- Capture expected shadow behavior by type (including transparent materials) from Filament docs for implementation targets.
- Record default values we will ship for:
  - intensity units/ranges
  - shadow enabled/casting
  - spot cone defaults
  - falloff/range defaults

### Phase 1: Scene model refactor

Planned code areas:
- `src/scene/mod.rs`
- `src/scene/serialization.rs`
- `src/app/mod.rs` (command enums and errors)

Planned model shape:
- `SceneObjectKind::Light(LightData)`
- `LightData { light_type, common, params_by_type, shadow }`
- Keep backward compatibility for existing directional-light scenes (migration on load).

### Phase 2: FFI + safe wrappers

Planned code areas:
- `build_support/bindings.cpp`
- generated bindings (`src/ffi/mod.rs` include target)
- `src/filament.rs`

Planned capabilities:
- create/update per light type
- set color/intensity
- set direction/position by type
- set spot inner/outer cone
- set falloff/range where supported
- set cast shadows + exposed shadow options subset

### Phase 3: Runtime wiring

Planned code areas:
- `src/render/mod.rs`
- `src/app/mod.rs`

Changes:
- Move from single active-light handle to runtime-per-light-object updates.
- Ensure scene rebuild and object delete paths cleanly remove light entities.
- Ensure transform updates for non-directional lights (position) and directional/sun (direction/orientation).

### Phase 4: Editor UX

Planned code areas:
- `build_support/bindings.cpp` (ImGui panel)
- `src/app/mod.rs`
- `src/ui/mod.rs`

Changes:
- Main menu entries:
  - `Add Directional Light`
  - `Add Sun Light`
  - `Add Point Light`
  - `Add Spot Light`
  - `Add Focused Spot Light`
- Inspector shows controls by selected light type.
- Outliner names become explicit and stable (e.g., `Point Light 1`).

### Phase 5: Light helper overlay

Planned code areas:
- `src/render/light_helpers.rs`
- `src/render/pick.rs`
- `src/app/mod.rs`

Design targets:
- Render in overlay pass with pick IDs (same architecture family as transform gizmo).
- Keep visual size readable across zoom levels.
- Helpers per type:
  - directional/sun: arrow + direction cue
  - point: sphere/cross marker + optional radius ring
  - spot/focused spot: cone/frustum cue
- Helpers are selectable and move/rotate with existing transform tools.

### Phase 6: Harness light suite

Planned artifacts:
- `harness/LIGHT_VALIDATION_PLAN.md` (this file, updated as we run)
- `harness/out/light_sweep_round1/...`
- `harness/out/light_sweep_round1/summary.csv`

Harness additions:
- deterministic test-scene builder mode (not relying on manual UI clicks)
- full-window screenshot capture profile (UI always included for this suite)
- scenario manifest (small, structured list of cases; no giant script)
- angle sweep support (camera and/or light transforms)
- per-case report fields for:
  - light type
  - shadow enabled/options
  - transparent test object present
  - screenshot path
  - pass/fail + notes

### Phase 7: Validation + fixes

Test scene baseline:
- floor plane + back wall
- opaque cube + sphere
- transparent sphere/glass object
- optional emissive reference object
- environment: `AdamsPlace`

Coverage matrix (minimum):
- each light type present alone
- each light type with shadows on/off
- directional/sun rotation sweep
- point translation and falloff sanity
- spot/focused spot cone angle sanity
- mixed-lights scene (at least one directional + one local)
- transparent-object interaction snapshot
- UI stack check (helpers + outliner + inspector all visible and coherent)

## Working Checklist

| Status | Item |
|---|---|
| done | Lock supported light-type list from local Filament headers. |
| done | Define `LightData` schema + backward-compat migration strategy. |
| done | Add FFI creation/update APIs for all light types. |
| done | Replace single-light runtime assumptions in render/app layers. |
| done | Add editor create actions + per-type inspector panels. |
| done | Add overlay helper geometry + pick integration for light helpers. |
| done | Add deterministic harness light-lab scene construction. |
| done | Add multi-angle capture cases and summary reporting. |
| done | Run sweep, triage issues, and fix until pass. |

## Open Decisions

| Status | Decision | Current default |
|---|---|---|
| done | How much shadow-option surface area to expose per light in v1? | Exposed practical subset per light; advanced/global tuning remains centralized. |
| done | Should `Sun` be user-facing separately from `Directional` in UI? | Yes; separate menu entries and explicit outliner names are implemented. |
| done | Light-helper visual style final polish level for this pass? | Functional + clear helper set shipped; deeper visual polish deferred. |

## Validation Summary

- Sweep run output: `harness/out/light_sweep_round1/summary.csv`
- Case count: `14`
- Pass rate: `14/14`
- Coverage included:
  - all supported light types
  - shadow on/off cases
  - multiple directional/sun/spot/focused-spot angles
  - point-light translation/falloff sanity
  - transparent-object import in the suite
  - full-window screenshots including UI, hierarchy, and inspector

## Post-Feedback Fixes

- `done` Preserve camera state on object delete/rebuild (no forced re-home after rebuild).
- `done` Fix spotlight helper origin so gizmo origin aligns with the light source (cone tip at origin).
- `done` Eliminate unsafe `Vec<bool>` FFI usage for material binding UI arrays (replaced with byte-backed buffers).
- `done` Prevent scene picking/transform from firing while ImGui captures mouse input.
- `done` Reset stale pick/gizmo state when switching transform modes.
- `done` Clarify Filament light behavior in inspector:
  - point lights do not cast shadows
  - directional/sun intensity is `lux`
  - point/spot/focused spot intensity is `lumen`
  - `Spot` keeps cone/intensity decoupled, `Focused Spot` couples them physically
