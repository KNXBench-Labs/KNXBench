# 2026-10-04 — Claude (goal-ui owner) — UA5: explicit topology repair (MODEL-02)

## Scope
Core, store and server half of MODEL-02 (`goal-ui.md` §3b UA5). ADR-0071.

## Changes
- `knx-core/src/command.rs`: `DevicePlacementSlot`, `RemovedDevicePlacement`,
  `RemovedLineReference`; commands `RepairDevicePlacement`,
  `RestoreDevicePlacements` (undo-only), `RepairLineOwner`,
  `RestoreLineOwners` (undo-only); errors `RepairNotNeeded`,
  `RepairKeepNotPresent`. Helpers `repair_device_placement`,
  `repair_line_owner`, their restores, and `remove_occurrences` (spares the
  first occurrence in the kept container, records removal indices).
- `knx-store`: `check_unambiguous_topology` runs before every save
  transaction; new `StoreError::AmbiguousTopology { devices, lines }`.
- Server: `POST /api/repair/device-placement`, `POST /api/repair/line-owner`
  (`deny_unknown_fields`; exactly one keep field for devices).

## Discovery (data integrity)
The first store test expected an ambiguous project to save and reopen
unchanged. It reopened with the device only in `unassigned`: the schema holds
one placement per device and one area per line, and the save upserts each
occurrence, so the last one won silently. A line listed by an area of another
installation would reach an `expect` in the save walk. Save now refuses both
before writing; the file on disk stays as it was. A `knx-cli` import that
produces such a project now fails at save with the explicit error. ETS import
validation already refuses duplicate `Id`s, so the case is rare in practice.

## Evidence
- RED: `crates/knx-core/tests/topology_repair.rs` did not compile (no repair
  API). GREEN 8/8. Two first-draft test mistakes fixed (double apply after
  `round_trip`; `RenameLine` does not check ownership → `MoveLineToArea`).
- HTTP 2/2 (`apps/knx-server/tests/topology_repair_routes.rs`), store 2 new
  tests in `crates/knx-store/tests/command_persistence.rs`.
- Mutants 10/10 caught (after sandwiching device 3 between duplicates so
  "append on restore" and "restore in forward order" are observable).
- Gates: fmt, clippy -D warnings, workspace tests (160 blocks, 3,079 passed, 0 failed, 176 ignored), layering, headers, anchors, corpus gates, diff-check — all green.

## Not done
Web UI for choosing the kept placement; duplicate-id renumbering;
building-part/group-range placement repair.
