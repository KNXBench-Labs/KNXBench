# WP-18 — Device comparison shows what differs (19 October 2026)

Branch `feat/wp18-comparison-diff-20261019`, off `origin/main` `66dd858b`.

## Findings before coding

- `/api/device-compare` (`apps/knx-server/src/device_compare_routes.rs`)
  already returns byte-level `changes`: run address, segment, device bytes and
  project (planned) bytes, from `download_changes` per plan write step.
- Raw `decoded` rows come from `knx-productdb::image_comparison::decode`: one
  row per read region; device side is always the read hex; project side is
  the hex of the planned bytes or `null` when any byte in the region has no
  planned value. The project side is built from an address map of all plan
  writes, `changes` per step, so the two sources are computed separately.
- `ComparisonTable` is shared with `DeviceInspectionPanel` (Device checks).

## Decisions

- Pure rules in `apps/knx-web/src/comparisonDiff.ts`; the table stays thin.
- Byte marks: equal-length comparison of the displayed hex tokens; a byte in
  `changes` is always *different* (the union can only add marks). Missing or
  malformed counterpart → *unknown*.
- Proven equal (hideable): known decoded kind, both sides known, equal, no
  reason; raw range fully read on both sides, equal length > 0, all equal and
  no `changes` byte. Everything else stays visible.
- Toggle: a checkbox "Show equal rows (N)", per mounted table, not persisted.
- `DeviceInspectionPanel` (controller decision after the pause): its result is
  the same `DeviceCompareResponse`, which already carries byte-level
  `changes`, so it now passes them (one-line change). Unit test
  `DeviceInspectionPanel.test.tsx` › "opens with differences only and never
  hides a differing or unread byte, using the server's byte changes" was RED
  before the change (row hidden) and GREEN after.
- Existing e2e fixture's change run address corrected from 4003h to 4004h to
  match its own raw row (byte index 1 of the run at 4003h).

## Verification

Before the pause: RED logs, focused Vitest, focused Playwright (production
preview, intercepted API, private network namespace: devices 45 passed,
table-columns 139 passed), 6 display-rule mutants killed, documentation,
anchor, header and layering checks.

After the rebase (resume): all 22 `tsconfig*.json` typechecks passed; the
Device checks wiring mutant (drop `changes`) killed by the new panel test.
Package gate attempt 1 (change-scoped, base `origin/main`): every stage passed
except `web-gate`, whose Vitest stage had 4038 of 4039 passing; the one
failure was the `LogPanel` 16000-entry test timing out at 5 s under host load
(outside this package; passes alone). The controller is landing a separate
test-hygiene fix on main; the gate is to be rerun after a rebase onto it.
