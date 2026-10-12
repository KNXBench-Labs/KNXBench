# 2026-10-12 — Claude — Group-address follow-ups: matrix object inventory

## Feedback check

Issue #4 (tree structure for group addresses): last comment is ours (2026-10-12
09:28 UTC) asking three short questions. No answer at the time of this session,
so the optional derived 2-/3-level tree and virtualization were not started.
Parked with preconditions in `docs/goals/group-address-four-views.md` (GA7, GA8).

## Decision: how unlinked objects reach the matrix

- Source of truth: the existing canonical per-device read `api.deviceDetail`
  (`GET /api/device/{id}`), which already returns every communication object of
  a device. No new endpoint, no client-side object model.
- Explicit per-device load, no bulk "load all": bounded requests, and the user
  decides which devices matter for the at-most-12 columns.
- Staleness token = fresh object per (installation snapshot, product language,
  project key). Replies apply only for their own generation and token; loaded
  devices are re-read after any tree update, mirroring how App refetches the
  selected device's detail in `handleTreeUpdate`.
- Pruning of chosen columns spares devices being re-read (including loaded
  entries that a just-arrived update made outdated before the re-read starts —
  found while designing; the pure test was tightened before implementing).
- Linked columns keep link-derived labels; inventory columns use the device node
  and object from the detail. Default column choice stays linked-only.

## Verification

- RED: 11 failing (missing module, no inventory columns, old copy). GREEN after
  implementation. Five guard mutations (stale-reply guard, device-ID check,
  linked-only default, pending-key pruning, project-switch reset) each caught.
- Browser: production preview, loopback-only namespace, 7/7. A narrow German
  frame showed state and button wrapping apart; fixed with an action group and
  row separators, pinned by a geometry assertion that fails on the old layout.
- Manual capture: real debug server, Sample house, two byte-identical runs.

## Not done / boundaries

No bulk load, no whole-project inventory claim, no virtualization, no derived
tree, no release/deployment/bus actions, no subagents.
