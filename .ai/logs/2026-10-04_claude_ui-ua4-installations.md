# 2026-10-04 — Claude (goal-ui owner) — UA4: every installation editable (MODEL-01)

## Scope
Core and server half of MODEL-01 (`goal-ui.md` §3b UA4). ADR-0070.

## Changes
- `knx-core/src/command.rs`: ~40 `installations.first()/first_mut()` sites
  replaced by owner resolution (`owning_installation`, `require_unique_*`
  now return the owner index, new `require_unique_group_address`,
  `device_installation`, `parameter_installation`, `create_installation`,
  `same_installation`, `target_installation`). Delete inverses carry the
  installation (`Restore{GroupAddress,Area,Line,BuildingPart,GroupRange}.installation`).
  Root creates gained `installation: Option<InstallationId>` (56 constructors
  across the workspace updated mechanically; `None` = previous default).
  New `RenameInstallation`, `RestoreDeviceBuildingPlacement`, errors
  `CrossInstallation`, `GroupAddressPlacementAmbiguous`,
  `ParameterPlacementAmbiguous`.
- Server: `PATCH /api/installations/{id}`, optional `installationId` on the
  four root create routes (`*_in_impl` domain variants; old functions delegate).

## Evidence
- RED: `crates/knx-core/tests/multi_installation.rs` with the then-existing
  API: 5 of 7 tests failed with `AreaNotFound`/`LineNotFound` or the parameter
  landing in installation 0. GREEN 10/10 after (two tests need the new API,
  one added during self-review).
- Self-review found an undo hazard: the inverse of a building move was the
  checked forward move, so undoing a move away from an imported
  cross-installation placement would be refused (and lost from the stack).
  Fixed with `RestoreDeviceBuildingPlacement`; reverting the inverse makes
  `an_imported_cross_installation_building_placement_stays_undoable` fail.
- One older core test pinned the first-installation parameter semantics; it
  now pins the in-place edit and still the `IdInUse` guard for a new row.
- Mutants 8/8 caught (ambiguity, cross check, explicit-target mismatch,
  restore installation, parameter owner, rename inverse, link device check,
  server area target).
- Gates: fmt, clippy -D warnings, workspace tests (158 blocks, 3,067 passed, 0 failed, 176 ignored), layering, headers, anchors, corpus gates, diff-check — all green.

## Not done
- Web: installation rename control, installation choice for root creates,
  later-installation targets in dropdowns/drag-and-drop. CSV import still
  creates in the first installation.
