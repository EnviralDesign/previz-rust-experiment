# Long Horizon Implementation Log

This file captures medium/long-run implementation context so we do not lose intent across compaction.

## 2026-02-06

### Context
- User requested an autonomous longer-horizon run while away, with explicit context tracking.
- Active phase moved from M1-complete into M2 deepening.

### Completed slices before this log
- M2 material parameter command path and persistence:
  - scene-level material overrides added
  - command-based updates wired
  - runtime rebuild applies overrides
- Override identity hardening (first pass):
  - moved from `material_name` to `asset_path + material_slot`
  - exposed a collision with duplicate same-asset instances on reload
- Override identity hardening (second pass):
  - upgraded key to `object_id + material_slot`
  - added `SceneObject.id` and `SceneState::ensure_object_ids()` for legacy scene repair
  - runtime material bindings now track `object_id`

### Current in-progress slice
- Material UI scoping to selected object:
  - material list in panel now generated only from bindings that match selected object id
  - UI local material index is mapped to global runtime material index internally
  - goal: prevent accidental edits across unrelated assets and improve authoring clarity

### Additional progress made in this run
- App selection source-of-truth migrated to object-id:
  - `App.selection_id: Option<u64>` now stores active selection identity
  - index is now derived (`current_selection_index`) for UI/runtime lookup only
  - this reduces index drift risk as scene/object ordering evolves
- Material override identity bug fix:
  - confirmed collision case on reload for duplicate asset instances
  - override key now effectively resolves via `object_id + material_slot`
  - fallback compatibility retained for older scene files (`asset_path+slot`, then legacy `material_name`)

### New slice completed: texture/media override scaffolding
- Added scene schema for texture/media bindings:
  - `MaterialTextureBindingData` with `texture_param`, `source_kind`, `source_path`
  - scene stores bindings keyed by `object_id + material_slot + texture_param`
- Added command path:
  - `SetMaterialTextureBinding` validates non-empty source path
  - persists binding into scene model
  - returns explicit notice that runtime apply is pending
- Rebuild diagnostics:
  - if texture bindings exist, rebuild warning reports that runtime texture apply is not yet implemented
- Added tests:
  - scene serialization roundtrip for texture bindings
  - app command tests for validation + scene persistence

### Deliberate limitation (explicit)
- Runtime application of texture bindings is not yet wired because texture/sampler binding calls are not exposed in current Filament FFI surface.
- Next technical step is exposing texture parameter set/get plumbing in `build_support/bindings.cpp` and Rust wrappers.

### Follow-up completed in same run
- Exposed minimal texture-parameter FFI for KTX:
  - Added `filament_material_instance_set_texture_from_ktx(...)` C wrapper.
  - Added Rust binding and `Engine::bind_material_texture_from_ktx(...)`.
- Render runtime now retains bound textures:
  - `RenderContext` keeps `material_textures` alive and clears them on scene reset.
- Runtime apply path added:
  - `SetMaterialTextureBinding` now attempts immediate runtime apply when render is active.
  - Scene rebuild re-applies stored texture bindings.
- Current runtime format support:
  - `.ktx` only (image/video files are still persisted but reported as unsupported for runtime apply).

### Known constraints
- Texture path/slot authoring UI is not yet wired.
- Video texture/media pipeline remains planned, not implemented.
- Selection model is still index-based internally; object-id usage expanded for materials but not yet fully normalized app-wide.

### Next intended slices
1. Finish/verify scoped material UI behavior in real scene usage.
2. Move app selection source-of-truth from index-first to `object_id`-first.
3. Add texture slot override schema/commands using `object_id + slot` identity.
