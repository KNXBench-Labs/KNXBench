# 2026-10-19 — Claude — Fit assertion measured the loading catalogue

## Symptom

`independent fit assertion rejects an inflated rendered floor and accepts
restoration` (`apps/knx-web/e2e/table-column-widths.e2e.ts`) failed in two
package gates (gate isolation full gate 4 on `a86cb1f9`; product-DB change-
scoped gate 2 on `8a0e9c18`, which already had the font barrier), each time
alone among ~785 browser cases, each time `[.., 77, 84, 77, ..]` expected vs
`[.., 76, 84, 71, ..]` received (Manufacturer, Order number).

## Cause

`openList()` (shared fixture) returns when `.devices-workspace` is visible. The
catalogue request (`/api/devices`) is still in flight; product cells show
*Not resolved* until it answers. Under gate load the first `intrinsicFloors`
call fell into that window, the second did not.

## Evidence

- RED probe: route `/api/devices` held until after the first measurement, old
  test body: fails with exactly 77 -> 76 and 77 -> 71 (deterministic).
- Fix (spec-local, shared helper untouched because other specs test the
  loading state): `openResolvedList()` = `openList()` + `waitForCatalogue()`
  (a resolved product cell visible, loading status gone); used by the six
  Devices-measuring tests.
- Regression: catalogue held, loading status visible, `waitForCatalogue(…,
  1000)` must reject, then release and wait. First draft (release, then wait,
  then measure) was NOT discriminating: an empty wait still passed because the
  answer arrived before the measurement. Retained version: empty-wait mutant
  fails at `rejects.toThrow()`; spec restored byte-exact.
- Whole spec 50/50 (offline, one worker, no retries); table-columns fixture
  typecheck; browser lanes 0 untagged.

## Not changed

Application code, width budgets, tolerances, retries, the shared fixture.

## Review round 1 (Codex, accept_with_fixes)

F1 MINOR: the two reload paths (zoom cases, stored scale) reopened Devices and
measured without the catalogue wait. Fixed: `waitForCatalogue(page, "en")`
after each reopen; contract and status text name the reload paths. The gate
started on the previous head was stopped and voided.
