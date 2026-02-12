# GPU Picking + Editor Overlay Refactor Spec

## 1. Purpose

This document specifies a refactor from the current hybrid editor interaction stack:
- 3D transform math in Rust
- 2D ImGui overlay drawing + picking in C++

to a scalable, renderer-native architecture with:
- Filament-rendered editor helpers (gizmo, light helpers, future tool handles)
- unified GPU picking pass for scene geometry and helpers
- UI-library-independent viewport interaction logic

The objective is correctness, scalability, and future-proofing.

## 2. Why Refactor

### 2.1 Current limitations

1. Gizmo rendering/picking is coupled to ImGui (`build_support/bindings.cpp`).
2. Visual and pick geometry are not guaranteed to match.
3. Rotation ring clipping has correctness issues due to mixed 2D/3D approximations.
4. Scene object picking is approximate (AABB/sphere), not pixel-accurate.
5. Future helper types (hundreds of lights, paint tools, helper widgets) would multiply bespoke logic.

### 2.2 Target capabilities

1. Pixel-accurate geometry picking in viewport.
2. Correct helper picking and visibility behavior at scale.
3. Transform gizmo is screen-size invariant but spatially 3D.
4. World-space now; local-space transform mode later without re-architecture.
5. UI toolkit (ImGui or replacement) can change without breaking viewport tools.

## 3. Scope

### In scope

1. Editor overlay subsystem in renderer.
2. Dedicated GPU pick pass.
3. Unified hit format (`object_id`, `sub_id`, hit distance, kind).
4. Migration of transform gizmo from ImGui overlay to renderer-native helpers.
5. Migration of scene selection from approximate CPU tests to GPU pick result.

### Out of scope (initial)

1. Undo/redo redesign.
2. Multi-select behavior redesign.
3. Video playback material systems.
4. Final visual styling of helper meshes.

## 4. High-Level Architecture

### 4.1 New subsystems

1. `editor_overlay` (Rust module)
- Owns helper descriptors and runtime helper drawables.
- Produces overlay draw data and pick draw data.

2. `pick_system` (Rust module)
- Owns pick render target(s), pass execution, and readback.
- Converts cursor location into authoritative pick hit.

3. `viewport_interaction` (Rust app logic)
- Mode-aware policy layer (select/translate/rotate/scale behavior).
- Consumes pick hits and dispatches commands.

### 4.2 Renderer views/passes

1. Main Scene Pass
- Existing scene rendering (PBR, environment, etc.).

2. Editor Overlay Pass
- Helper visuals only.
- Depth policy per helper type (depth-tested, always-on-top, mixed).

3. Pick Pass (offscreen)
- Flat-color or integer-ID rendering of pickables.
- Includes both scene geometry and helpers.

## 5. Data Model

## 5.1 Pick ID packing

Define a 32-bit pick key:
- `kind` (4 bits): scene mesh, gizmo axis, gizmo plane, ring, light helper, etc.
- `object_id` (20 bits): scene object identity.
- `sub_id` (8 bits): helper component/material slot/instance index.

If integer attachments unavailable, pack to RGBA8 and unpack on CPU.

### 5.2 Hit result

```rust
struct PickHit {
    key: u32,
    kind: PickKind,
    object_id: Option<u64>,
    sub_id: u16,
    depth: f32,
    screen_xy: [f32; 2],
}
```

### 5.3 Helper descriptors

```rust
enum HelperKind {
    TransformGizmo,
    DirectionalLight,
    SpotLight,
    PointLight,
    CameraWidget,
}

struct HelperInstance {
    helper_id: u32,
    kind: HelperKind,
    object_id: Option<u64>,
    transform: Mat4,
    scale_mode: ScaleMode, // ScreenInvariant | World
    depth_mode: DepthMode, // Tested | AlwaysOnTop | Mixed
    pickable_parts: Vec<HelperPart>,
}
```

## 6. Rendering Details

### 6.1 Screen-size invariance

Transform gizmo visual size should be constant in screen space.

Per frame:
1. Compute world scale factor from camera distance + vertical FOV + viewport height.
2. Apply uniform scale to gizmo root transform.
3. Maintain orientation mode:
- world mode: aligned to world axes
- local mode (later): aligned to selected object basis

### 6.2 Overlay depth policy

Support per-part options:
1. `DepthTested`: helper occluded by scene.
2. `AlwaysOnTop`: helper ignores depth.
3. `Mixed`: specific parts have specific policies.

Initial transform gizmo recommendation:
1. Axis/rings depth-tested.
2. Selected/active part can optionally draw xray accent pass.

## 7. Pick Pass Details

### 7.1 Render target

Minimum:
1. Color attachment for ID (`R32UI` preferred; fallback `RGBA8`).
2. Depth attachment.

Optional:
1. Secondary attachment for linear depth.
2. Optional normal for advanced snapping.

### 7.2 Draw order in pick pass

1. Clear ID to `0` (background/no hit).
2. Draw scene geometry with object IDs.
3. Draw helper geometry with helper IDs and sub-IDs.
4. Read back 1x1 pixel at cursor.

### 7.3 Readback mode

Phase 1:
- synchronous 1x1 readback on click.
- optional hover read every N frames.

Phase 2:
- async double-buffered readback with 1-frame latency.

## 8. Interaction Policy

### 8.1 Selection precedence

1. If transform drag active: lock to active helper part.
2. Else if hit helper part: helper interaction.
3. Else if hit scene object: object selection.
4. Else background: deselect in Select mode.

### 8.2 Tool behavior

1. Select mode: scene selection only (unless helper widgets explicitly enabled).
2. Transform modes:
- click helper part -> begin drag constraint
- drag uses existing robust math in Rust
- helper picking only used for target acquisition and hover state

## 9. Migration Plan

### Phase 0: Prep

1. Introduce `PickKey` and decoding utilities.
2. Introduce `PickHit` and app-level selection policy abstraction.
3. Keep existing behavior unchanged.

### Phase 1: Scene GPU picking

1. Build pick pass for scene geometry only.
2. Replace current approximate object picking.
3. Validate pixel-accurate select/deselect behavior.

### Phase 2: Helper overlay foundation

1. Add `editor_overlay` registry and helper descriptors.
2. Render static helper prototype (non-interactive).
3. Ensure screen-invariant scaling works.

### Phase 3: Gizmo helper geometry + helper pick IDs

1. Implement transform gizmo mesh/line geometry in overlay.
2. Emit sub-part IDs for axis/plane/ring/uniform/arcball.
3. Route hover + click from pick hit to existing drag math.

### Phase 4: Remove ImGui gizmo path

1. Delete gizmo draw/pick block from `build_support/bindings.cpp`.
2. Keep ImGui only for panel UI.

### Phase 5: Additional helpers

1. Directional helper, point helper, spot cone.
2. Optional camera axis widget.

## 10. File/Module Changes

## 10.1 New modules (proposed)

1. `src/render/pick.rs`
2. `src/render/editor_overlay.rs`
3. `src/editor/interaction.rs`

### 10.2 Existing modules to modify

1. `src/render/mod.rs`
- initialize/manage overlay and pick pass resources

2. `src/app/mod.rs`
- consume `PickHit` instead of approximate CPU scene picking
- helper precedence and drag lock policy

3. `src/filament.rs`
- expose needed render target/pick API hooks

4. `build_support/bindings.cpp`
- remove viewport gizmo rendering/picking logic after migration

## 11. Performance Notes

1. 1x1 pick reads are cheap but can stall if done every frame synchronously.
2. Use click-only sync first, then async hover if needed.
3. Helper geometry is tiny; scene pick pass is dominant cost.
4. Add settings:
- `editor.helpers.enabled`
- `editor.pick.hover_enabled`
- `editor.pick.async_enabled`

## 12. Risks and Mitigations

1. Risk: Pick pass API friction in Filament wrapper.
- Mitigation: phase scene pick first with minimal wrapper surface.

2. Risk: ID precision/format portability.
- Mitigation: support both integer target and RGBA packing.

3. Risk: visual mismatch between overlay and pick geometry.
- Mitigation: generate both from shared helper descriptor data.

4. Risk: drag regressions during migration.
- Mitigation: preserve current Rust drag constraint math; replace only acquisition/render path first.

## 13. Acceptance Criteria

1. Clicking visible mesh pixels selects that object reliably.
2. Clicking background reliably deselects in Select mode.
3. Transform gizmo remains constant screen size while moving in 3D.
4. Helper hover/click is stable under orbit/pan/zoom.
5. Rotation ring clipping/visibility behaves consistently (no front-side false clipping).
6. Removing ImGui overlay gizmo code does not break transform tooling.

## 14. Implementation Order Recommendation

1. Scene GPU pick pass first (high impact, low visual risk).
2. Overlay gizmo rendering second.
3. Helper sub-part picking third.
4. ImGui gizmo removal fourth.
5. Light helpers and extra tools afterward.

## 15. Open Decisions

1. ID format default: `R32UI` vs `RGBA8` packed.
2. Depth policy defaults per helper class.
3. Hover pick cadence and async policy.
4. Whether camera axis widget shares overlay pass or separate mini-viewport.

## 16. Immediate Next Slice

1. Implement `Phase 1` only:
- Add pick pass for scene geometry.
- Replace `pick_scene_object` approximate logic in `src/app/mod.rs`.
- Keep current gizmo path untouched for this slice.

This gives immediate UX wins for object selection while keeping migration risk controlled.
