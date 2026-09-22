# T11 structural drag-and-drop design

Date: 2026-09-22

## Decision

T11 will implement two complete Project Explorer gestures: device to topology
line and device to building part. Both reuse the existing HTTP functions and
undoable domain commands. The existing Inspector selects remain their keyboard
equivalents.

Group address to communication object is deliberately excluded because the
existing command requires an explicit `Send` or `Receive` direction. A silent
drop default would change engineering semantics; a direction chooser is a
separate cross-panel interaction.

## Boundaries

- Only devices and targets in the first installation advertise drag/drop,
  matching the current Inspector and command ownership.
- Native drag payloads are untrusted and revalidated against the current tree.
- No optimistic tree mutation, new route, dependency, generic drag framework,
  or server-side validation duplicate is introduced.
- Outcomes use the existing translated toast/live-region path.
- Dragging itself causes no HTTP, KNX, multicast, discovery, gateway, or
  hardware activity. The drop sends only the existing project mutation.

## Evidence

The design follows `goal.md` §3.4 and
`.superpowers/sdd/goal/task-11-brief.md`. Existing ownership was traced through
`ProjectExplorer.tsx`, `Inspector.tsx`, `api.ts`, server domain move helpers,
and `Command::MoveDeviceToLine` / `Command::MoveDeviceToBuildingPart`.

Specification:
`docs/superpowers/specs/2026-09-22-structural-drag-drop-design.md`.

Self-review found no placeholder, contradictory scope, or unresolved public
choice. `git diff --check` and `xtask check-anchors` exited 0; anchor check
covered 389 links across 185 Markdown files. Product implementation has not
started and awaits the required written-spec approval.
