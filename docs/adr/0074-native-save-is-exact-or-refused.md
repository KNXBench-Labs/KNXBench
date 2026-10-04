# ADR 0074: A native save is exact or refused

Date: 2026-10-04
Status: Accepted
Session: goal-ui owner, Alpha follow-up UA7 (data integrity found during `MODEL-02`)

## Context

ADR-0071 found that `.knxdb` save silently collapsed a multiply placed device.
A probe of further model states showed the same class of defect elsewhere.
Every entity table is written with `INSERT … ON CONFLICT(id) DO UPDATE`,
`line.area_id` is `NOT NULL`, and `load_project` rebuilds `children` lists
from `parent_id` and `position`. The following states saved without error
and reopened different:

| State | What reopened |
| --- | --- |
| Two areas / group addresses / group ranges / building parts sharing an id | Only the last one |
| A line that no area lists (orphaned line) | Line gone |
| A child listed twice in its parent's `children` | Listed once |
| A child whose `parent` is set but which its parent does not list | Re-listed |

A device listed twice in one building part failed with a raw SQL `UNIQUE`
error, and an area listing another installation's line could reach an
`expect` in the save walk.

KNXBench's importers allocate fresh ids and build hierarchies consistently,
so none of these states is known to arise from a supported import today. They
remain reachable through bugs, future importers or hand-built projects, and
the store must not paper over them.

## Decision

- Before any write, `save_project` and `save_project_if_unchanged` run
  `knx_store::representable::check_representable`. It keeps the MODEL-02
  placement error (`StoreError::AmbiguousTopology`, which has repair
  commands) and reports everything else together as
  `StoreError::Unrepresentable(Vec<RepresentationIssue>)`. The findings are
  `DuplicateId { kind, id }`, `OrphanLine`, `UnknownLineReference`,
  `HierarchyMismatch { kind, id }` and `DeviceRepeatedInBuildingPart`.
- A refused save writes nothing; the previous file reopens unchanged.
- The rule is "exact or refused". The check does not repair or renumber,
  and the schema is not changed.

## Consequences

- A project in one of these states cannot be saved until it is fixed.
  Orphaned lines can be attached with `MoveLineToArea`. Hierarchy
  mismatches have no in-app repair at the time of this decision: the move
  commands refuse inconsistent parent/child lists as
  `…PlacementAmbiguous`. Duplicate ids have no repair command either. Both
  are documented gaps.
- `crates/knx-store/tests/lossless_save.rs` pins every case as refused with
  its named finding, and pins the representable variations (a device in
  building parts of two installations, a consistent nested hierarchy) as
  reopening exactly.
- The check walks the project once; no measurement suggested a performance
  concern, and none was optimised for.
