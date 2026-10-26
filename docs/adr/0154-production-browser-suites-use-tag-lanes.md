# ADR 0154: Production browser suites use the tag lanes

Date: 2026-10-26
Status: Accepted
Session: GATE-01 production-lane follow-up
Amends: [ADR-0144](0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md),
point 2, last sentence ("Production suites keep the one worker their configs pin").

## Context

ADR-0144 split the default browser suite into a functional lane (four workers)
and an `@exclusive` lane (one worker), but left the 16 production suites on one
worker each. A four-worker trial then overrode the configs without a lane review
and measured little gain (473 s against 505.6 s for 15 suites, under foreign
load), so it was withdrawn. Since then the production suites grew to about
850 s per web gate (`table-columns` alone 254 s in the integrated gate of
`ed156ef9b`); [GATE-01](../open-work/GATE-01-test-gate-follow-ups.md) asked for
a lane review per suite.

The review of 26 October 2026 checked the mechanical criteria that replaced the
hash admission in ADR-0144 for every production config and its spec files:

- All 35 spec files that the 16 production configs select are also in the
  default inventory, so they already run file-parallel on four workers and are
  covered by the `tools/browser_lanes.py` tag guard.
- Each config starts one static `vite preview` server that all of its workers
  share; the gate runs the suites one after another, each stage in its own
  network namespace. Two suites (`table-columns`, `ux-feedback`) build into
  their own `target/` directory first.
- No production config sets a `globalSetup`, environment switch or shared server
  state. The specs write only to Playwright's output path or to capture
  locations behind opt-in variables (`KNX_*`, `UX_FEEDBACK_SHOTS`,
  `ARCHIVE_TOAST_SCREENSHOTS`, `IMPORT_STATUS_SCREENSHOTS`, `UPDATE_SCREENSHOTS`,
  `FLOW_EXPORT_OUTPUT`, `FLOW_CAMERA_OUT`, `FLOW_CAMERA_NEGATIVE`), all of which
  the gate environment removes. No spec uses `describe.serial`,
  `describe.configure` or `beforeAll`.
- 29 of 557 production cases carry `@exclusive` (`ux-feedback` 22,
  `table-columns` 4, `communication`, `devices` and `group-addresses` one each).

## Decision

Each production suite runs like the default suite: compile its inventory, split it
by tag, run the functional cases with the gate's worker count and the exclusive
cases on one worker. The stage names are `production-<suite>-inventory`,
`-functional` and `-exclusive`; an exclusive lane without cases is recorded as
empty, never asked to run nothing. Case identity equality per lane, zero
retries and no skips apply unchanged. Parallelism stays per file, as in the
default suite (`fullyParallel: false`); suites still run one after another.

The configs keep `workers: 1` for standalone runs; the gate passes `--workers`.
A config-level `grep` (only `reload-guard` has one) stays in force: Playwright
applies the lane's `--grep`/`--grep-invert` in addition to it, and any mismatch
would fail the case-identity check rather than pass silently. When a suite's
spec files hold tagged `@exclusive` scopes but its inventory compiles no
exclusive case, the gate stops, as it does for the default suite.

## Alternatives considered

- **Case-level parallelism (`--fully-parallel`).** Estimated at about 380 s
  instead of about 610 s from per-case durations, but it would make every test
  within one file independent of its neighbours, which no review has checked.
  Left open.
- **Several suites at once.** Each stage has its own network namespace, so the
  shared port numbers (4173, 4831–4833) would not collide, but the leases,
  build directories and CPU budget would need their own review. Left open.
- **Keep one worker.** Leaves about 170 s per web gate on the table for no
  identified risk.

## Consequences

One measured run of all 16 suites on the candidate (targeted profile, no
default specs) took 682.6 s of production stages against 850.3 s in the last
complete integrated gate, with inputs frozen and every lane's case identities
equal to its inventory ([status record](../status/2026-10-26-production-browser-lanes.md)).
The old figure ran after the whole default suite, the new one without it, so
cache and load conditions differ.
Single-file suites (`catalogue`, `group-addresses`, ...) gain nothing and pay
one extra preview-server start when they have exclusive cases. Numbers come
from one host; they are not a speed promise for other machines.

Stage names change from `production-<suite>` to `production-<suite>-functional`
plus `-exclusive`. Tools that compare stage names across receipts need to know
the split; the package gate wraps the whole web gate as one stage and is not
affected.
