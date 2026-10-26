# Production browser suites in the tag lanes (GATE-01)

Status: implemented on branch `perf/gate01-prod-lanes-20261026`; review, package
gate and publication pending.
Decision: [ADR-0154](../adr/0154-production-browser-suites-use-tag-lanes.md).
Task: [GATE-01](../open-work/GATE-01-test-gate-follow-ups.md), first bullet.

## Change

`tools/run_web_gate.py` runs each of the 16 production suites through
`run_production`: list the suite's inventory with one worker, split it by the
`@exclusive` tag, run the functional cases with the gate's `--workers` and the
exclusive cases on one worker. Stage names are `production-<suite>-inventory`,
`-functional` and `-exclusive`; an exclusive lane without cases is recorded as
empty. The receipt gains `production.<suite>` with total, functional and
exclusive counts. Case identity, zero retries and no skips are checked per lane
exactly as for the default suite. Playwright configs are unchanged.

## Lane review

Mechanical, per production config (details in ADR-0154): every selected spec
file is also in the default inventory; one static preview server per suite on
its own port, suites run sequentially; no global setup, environment switch or
shared server state; capture switches are `KNX_*` variables the gate removes;
no serial or `beforeAll` constructs. Only `reload-guard` sets a config `grep`,
and its inventory holds no exclusive case.

## Evidence

- Tests first: four new or changed cases in `tools/tests/test_web_gate.py`
  failed on the old runner (two plan assertions, the missing `run_production`,
  the call-site check) and pass on the change; the module passes 40/40 and all
  gate-tool modules (`test_*gate*.py`) 93/93.
- Measured run on the candidate: `run_web_gate.py --profile targeted --specs ''`
  with all 16 suites, receipt success, plan completed, inputs frozen, no failed
  stage. 557 production cases: 528 functional, 29 exclusive (`ux-feedback` 22,
  `table-columns` 4, `communication`, `devices`, `group-addresses` one each).
- Production stages (inventories included): 682.6 s against 850.3 s in the
  integrated gate of `ed156ef9b` (load at start 3.2 against 3.5).
  `table-columns` 254.3 → 152.0 s, `communication` 76.7 → 54.6 s,
  `device-download-support` 51.0 → 30.1 s, `devices` 96.8 → 82.6 s,
  `ux-feedback` 99.4 → 86.8 s. Single-file suites stay level;
  `group-addresses` measured 79.5 → 98.5 s with every case about 0.8 s slower,
  serial ones included, so that is host variance, not the split.

One run per side on one host; not a speed claim for other machines.

## Not covered

Case-level parallelism inside a file and concurrent suites remain open
(ADR-0154, alternatives). The earlier `file-menu-submenus` cold-start timeouts
at eight workers and the unproven historical SIGTERM timeouts are untouched.
