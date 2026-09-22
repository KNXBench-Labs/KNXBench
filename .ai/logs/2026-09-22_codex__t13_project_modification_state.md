# T13 Task 1 — honest project modification state

Date: 2026-09-22

## Delivered

- `Project::same_user_content_as` compares the complete aggregate after
  normalizing only synthetic `IdAllocators`; `Project` and `StringTable` are
  cloneable for this snapshot comparison.
- `AppState.clean_project` is a transient clean snapshot. New, native open,
  ETS import, Save, and Save As establish it only on success; failed saves do
  not move it. Store schema version 9 is unchanged.
- Replacement publication extends T12's project → command stack → import
  counts → store path lock order with the clean snapshot as the final lock.
  Current-tree publication and all command/undo/redo response paths use the
  same baseline comparison.
- `ProjectTree.is_modified` is independent of undo/redo availability. The
  desktop Quit guard consumes it; toolbar history controls remain unchanged.
- Stable limitation headings §§81/103 remain intact and their bodies now
  document the resolved behavior and disagreement proofs.

## TDD evidence

- RED: `knx-core` failed because `Project` had no `clone` or
  `same_user_content_as`; `knx-projection` failed because `ProjectTree` had no
  `is_modified`.
- RED: direct mutation with an empty command stack returned
  `is_modified == false`; the HTTP edited-project response likewise returned
  false. Both intended App quit disagreement tests failed in opposite ways.
- GREEN: core 1/1, projection 37/37, server modified-state 2/2, HTTP project
  routes 13/13, atomic replacement regression 1/1, App 75/75, TypeScript,
  rustfmt, focused warning-denied Clippy, and `git diff --check` all pass.

All Cargo work used `/var/tmp/knxbench-t13-target` with incremental and debug
info disabled and two build jobs. No KNX/LAN/multicast/gateway/hardware
traffic occurred. No private-LAN literal, prohibited fixture,
`docs/LIMITATION_TRIAGE.md`, group-address notation, theme, or reduced-motion
change was introduced.
