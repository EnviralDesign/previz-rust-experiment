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

### Known constraints
- Texture path/slot authoring UI is not yet wired.
- Video texture/media pipeline remains planned, not implemented.
- Selection model is still index-based internally; object-id usage expanded for materials but not yet fully normalized app-wide.

### Next intended slices
1. Finish/verify scoped material UI behavior in real scene usage.
2. Move app selection source-of-truth from index-first to `object_id`-first.
3. Add texture slot override schema/commands using `object_id + slot` identity.
