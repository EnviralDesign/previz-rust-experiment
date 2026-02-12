# 3D Transform Gizmos: Practical Math, Correctness, and Implementation

This document replaces and expands the original blog draft.

Goal: define a technically correct, implementation-ready model for 3D transform gizmos in an editor:

- Select
- Translate
- Rotate
- Scale

This is written for real-time editors using perspective cameras and mouse input.

---

## 1. Scope and Design Targets

### 1.1 User-facing behavior

- Gizmo is anchored to selected object pivot.
- Gizmo can operate in:
  - World space
  - Local space
  - (Optional later) camera/view space
- Modes:
  - Select (`Q`)
  - Translate (`W`)
  - Rotate (`E`)
  - Scale (`R`)
- Handles:
  - Axis handles (`X`, `Y`, `Z`)
  - Plane handles (XY, XZ, YZ) for translate/scale (recommended)
  - Center/uniform handle where applicable

### 1.2 Technical targets

- Camera orientation should not break drag semantics.
- Drag should be stable at shallow angles and large distances.
- Drag should be deterministic: same start state + same cursor path => same result.
- Numerical robustness:
  - Handle near-parallel rays/planes.
  - Avoid NaNs/infinite values.
  - Avoid sudden jumps on drag start.

---

## 2. Core Data Model

Use an explicit drag state captured on mouse-down.

```text
GizmoState
  pivot_world: vec3
  basis_world: mat3      // columns are X/Y/Z axis directions in world
  mode: Select | Translate | Rotate | Scale
  space: World | Local
  active_handle: None | AxisX | AxisY | AxisZ | PlaneXY | PlaneXZ | PlaneYZ | Uniform
  drag_start_mouse_px: vec2
  drag_start_transform: Transform
  drag_reference:
    // mode-specific cached values, see sections below
```

`basis_world`:

- World mode: identity basis (`X=[1,0,0]`, etc.).
- Local mode: object orientation basis in world space.

Do not recompute this basis during drag for a single operation (unless intentionally designing dynamic basis behavior).

---

## 3. Camera Ray Construction

Given mouse pixel `(mx, my)` and camera:

1. Convert pixel to NDC:
   - `x_ndc = 2*mx/width - 1`
   - `y_ndc = 1 - 2*my/height`
2. Unproject to world-space ray.

Two valid methods:

- Inverse view-projection matrix unprojection.
- Camera basis + FOV/aspect construction.

Output:

- `ray_origin` (camera position for perspective)
- `ray_dir` normalized

Always normalize `ray_dir`.

---

## 4. Handle Picking (What Did We Click?)

### 4.1 Recommended hybrid strategy

- Primary: screen-space distance to projected primitives.
- Secondary: world-space ray intersection fallback.

Why:

- Screen-space picking is intuitive and stable with thick lines.
- World fallback helps when projections become tiny/degenerate.

### 4.2 Axis picking in screen space

Project:

- `P0 = pivot_world`
- `P1 = pivot_world + axis_dir * handle_len_world`

to screen:

- `p0 = project(P0)`
- `p1 = project(P1)`

Compute point-to-segment distance from mouse to segment `(p0,p1)`.

Pick axis with minimum distance under threshold (`~6-12 px`).

### 4.3 Plane/rotation ring picking

- Translate/scale plane handles: pick projected quad/triangle region.
- Rotation rings: either projected circle arc test or world-plane ray hit with radial band test.

---

## 5. Translation Math (Axis and Plane)

This is where many implementations go wrong if they use raw mouse deltas.

## 5.1 Axis translation (correct model)

Define:

- Axis line: `L_axis(t) = O + t * A`
  - `O = pivot_world`
  - `A = active axis unit vector in world`
- Mouse ray: `R(s) = C + s * D`

Each frame, compute closest points between two lines (ray treated as line first).

Solve for `t` using standard closest-lines formula:

```text
w0 = C - O
a = dot(D,D)
b = dot(D,A)
c = dot(A,A)
d = dot(D,w0)
e = dot(A,w0)
denom = a*c - b*b
t = (a*e - b*d) / denom
```

If `denom` is near zero (near parallel), use fallback:

- Keep last valid `t`.
- Or use a support plane method (below).

On drag start cache `t0`, then per-frame delta:

- `delta = t - t0`
- `new_position = start_position + A * delta`

This is camera-robust and world-consistent.

## 5.2 Plane translation (TODO in old draft; completed here)

For active plane with normal `N` through pivot `O`:

- Intersect mouse ray with plane:
  - `den = dot(D, N)`
  - if `abs(den) < eps`: parallel -> fallback
  - `s = dot(O - C, N) / den`
  - `hit = C + s*D`

On drag start cache `hit0`.
Per frame:

- `delta_world = hit - hit0`
- Constrain to plane basis if needed.
- `new_position = start_position + delta_world`

This is usually the most ergonomic translation mode.

## 5.3 Support plane fallback for axis translation

If axis and ray become near-parallel, line-line becomes unstable.

Use a support plane containing axis:

1. `view = normalize(camera_pos - O)`
2. `N = normalize(cross(A, view))`
3. If `|N|` too small, choose alternate `view` or world up fallback.
4. Intersect ray with plane `(O, N)`.
5. Project hit delta onto axis `A`.

---

## 6. Rotation Math (Axis-Constrained)

Common robust method:

1. Rotation axis `A` through pivot `O`.
2. Define rotation plane with normal `A`.
3. Intersect current and start rays with that plane:
   - `p0`, `p1`.
4. Build vectors from pivot:
   - `v0 = normalize(p0 - O)`
   - `v1 = normalize(p1 - O)`
5. Signed angle around `A`:
   - `angle = atan2(dot(A, cross(v0, v1)), dot(v0, v1))`
6. Apply delta angle to start orientation.

Use quaternions internally to avoid Euler instability.

If exporting Euler values, convert at UI boundary only.

Fallback if ray-plane is unstable:

- Use projected-axis screen tangent approximation (last resort).

---

## 7. Scale Math (Axis and Plane/Uniform)

## 7.1 Axis scale

Use the same geometric measurement style as translation:

- Compute parameter on active axis, `t`.
- Ratio against start:
  - `s = t / t0` (careful near zero)
- Better: additive delta mapped via exponential:
  - `s = exp(k * (t - t0))`

Then:

- `new_scale_axis = start_scale_axis * s`

Clamp to minimum positive value (`>= 1e-4`) unless negative scaling is explicitly supported.

## 7.2 Plane scale

- Ray-plane hit in plane space.
- Measure 2D distance/vector from pivot in plane basis.
- Scale XY/XZ/YZ components from ratio.

## 7.3 Uniform scale

- Use distance from pivot in a stable drag plane.
- Ratio current/start distance.

---

## 8. World vs Local Space

### 8.1 Translation

- World space: axis directions fixed to global basis.
- Local space: axis directions from object orientation at drag start.

### 8.2 Rotation

- World mode: rotate around world axis through pivot.
- Local mode: rotate around local axis transformed to world at drag start.

### 8.3 Scale

- Local mode is usually expected for non-uniform scale.
- For rotated objects with parent transforms, define whether scale is object-local pre-rotation or world-axis aligned, and keep it consistent.

---

## 9. Selection and Deselect Rules

Recommended:

- In Select mode:
  - Click object -> select.
  - Click empty background -> deselect.
- In Transform modes:
  - Click gizmo handle -> start transform.
  - Click empty background -> keep selection (avoid accidental loss during manipulation).

This matches practical DCC/editor behavior and reduces frustration.

---

## 10. Multi-Select Compatibility Plan

Do not block future multi-select with single-select assumptions.

Model now:

- `selected_ids: Vec<ObjectId>`
- `active_id: Option<ObjectId>`

Single-select behavior can still be enforced by keeping `selected_ids.len() <= 1`.

When multi-select lands:

- Gizmo pivot options:
  - active object pivot
  - median point
  - bounding box center
- Transform applies to all selected items.
- Inspector:
  - show common fields
  - mixed values as indeterminate
  - edits fan out only to objects supporting that field

---

## 11. Numerical and UX Stability Checklist

- Freeze drag basis and start transform at mouse-down.
- Hysteresis on handle pick (don’t switch handles mid-drag).
- Clamp and validate all computed values.
- Keep last valid solution if current frame degenerates.
- Distinguish click vs drag with movement threshold.
- Optional snapping:
  - translate step
  - rotate degrees
  - scale ratio increments

---

## 12. Minimal Pseudocode (Axis Translate)

```text
onMouseDown(handle=AxisX):
  state.mode = Translate
  state.axis = basis_world.x
  state.start_transform = object.transform
  state.t0 = closestParam(mouseRay(), line(pivot, axis))

onMouseDrag():
  t = closestParam(mouseRay(), line(pivot, axis))
  if valid(t):
    delta = t - state.t0
    object.position = state.start_transform.position + axis * delta
```

---

## 13. Testing Matrix

Test all modes across:

- Camera near/far distances
- Camera facing almost parallel to active axis
- Very small and very large object scales
- Rotated objects (local mode)
- Parented transforms (if hierarchy exists)
- Mixed frame rates / large dt spikes

Manual correctness checks:

- Orbit camera while dragging axis: movement should remain axis-constrained.
- Drag direction should be continuous with no sign flips near center.
- No sudden jump at drag start.

---

## 14. Implementation Phasing (Recommended)

1. Axis translate with line-line math + fallback plane.
2. Plane translate.
3. Axis rotation via ray-plane + signed angle.
4. Axis scale with geometric ratio mapping.
5. Local/world toggle.
6. Snapping, undo chunking, multi-select transform fan-out.

---

## 15. Why This Model Works

- It maps 2D cursor input to 3D constraints using explicit geometry.
- It avoids camera-orientation hacks based on raw screen deltas.
- It provides stable, predictable manipulations that match DCC expectations.

If behavior feels wrong, inspect:

- handle picking,
- drag reference state capture,
- degeneracy fallback paths,
- and transform composition order.
