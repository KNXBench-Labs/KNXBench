# ADR 0071: Ambiguous topology is repaired explicitly, never collapsed on save

Date: 2026-10-04
Status: Accepted (core, store, server); web UI follows under the Web lock
Session: goal-ui owner, Alpha package UA5 (`MODEL-02`)

## Context

The domain model can hold a device in several topology slots (two lines,
twice on one line, a line and the unassigned list, twice unassigned, or slots
in two installations) and a line listed by several area entries. Editing such
entities was already refused — KNXBench does not guess which placement is
real — but there was no way out except outside the application.

While writing the repair tests a data-integrity defect surfaced: the native
`.knxdb` schema stores **one** placement per device (`device.line_id`) and
**one** area per line, and `save_project` wrote each occurrence with an
upsert. An ambiguous project therefore saved "successfully" and reopened with
whichever placement was written last; the others vanished silently. A line
listed by an area of another installation could even hit an `expect` in the
save walk.

## Decision

- **Store:** `save_project` (and `save_project_if_unchanged`) refuses a
  project with a device placed more than once or a line listed by more than
  one area entry, before any write, with
  `StoreError::AmbiguousTopology { devices, lines }`. The file on disk stays
  unchanged. This replaces silent last-write-wins.
- **Core:** two explicit, undoable repair commands:
  - `RepairDevicePlacement { device, keep: DevicePlacementSlot }` keeps the
    first occurrence in the chosen slot (`Line(id)` or
    `Unassigned(installation)`) and removes every other occurrence in every
    installation.
  - `RepairLineOwner { line, keep: AreaId }` keeps the first reference in the
    chosen area and removes all other area references to the line.
  - Both refuse when nothing is ambiguous (`RepairNotNeeded` — a repair is not
    a move) and when the kept slot is not a current placement
    (`RepairKeepNotPresent` — a repair only removes). The kept area must be in
    the line's installation (`CrossInstallation`, ADR-0070).
  - Addresses, links, parameters and building placement are not touched; an
    address that no longer matches the kept line stays visible for its own
    explicit repair.
  - The undo-only inverses `RestoreDevicePlacements` / `RestoreLineOwners`
    record every removal with its index and reinsert in reverse order, so undo
    restores the exact imported lists.
- **Server:** `POST /api/repair/device-placement`
  (`deviceId` + exactly one of `keepLineId` / `keepUnassignedInstallationId`)
  and `POST /api/repair/line-owner` (`lineId`, `keepAreaId`).

## Consequences

- An ambiguous project can be kept in memory, repaired and then saved; it can
  no longer be saved lossy. A `knx-cli` import that produces such a project now
  fails at save with the explicit error instead of writing a collapsed file.
- Duplicate entity **ids** (the same id used for two different entities) are
  not repaired: renumbering needs reference rewriting and stays a known gap.
  Ambiguous building-part or group-range placement has no repair command yet.
- The web UI must still offer the choice of the kept placement.
