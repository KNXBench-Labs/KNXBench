# GATE-01 — Test-gate speed and stability follow-ups

**Status:** Open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

Small and optional; none blocks a delivery.

## Open

- Production browser suites now run in the tag lanes
  ([ADR-0154](../adr/0154-production-browser-suites-use-tag-lanes.md),
  [record](../status/2026-10-26-production-browser-lanes.md)). Their functional
  lane runs case-parallel since `3af5ea9e4` (production 681.7 → 455.7 s;
  [ADR-0155](../adr/0155-production-functional-case-parallelism.md),
  [record](../research/2026-10-27-production-case-parallelism.md)). Concurrent
  suites were assessed on 27 October 2026 and are not admitted as a small
  change: aggregate worker scheduling, global exclusive isolation and receipt
  writing need a separate design; no overlap speedup was measured.
  Default-lane case parallelism was assessed on the same date and is not admitted:
  retained four-worker scheduling has only about 28 s of fixed-duration model
  headroom, not a measured gain
  ([assessment](../research/2026-10-27-default-browser-parallelism-assessment.md)).
  Reopen on a material new tail or an explicit priority for the larger scheduler.
- Three `file-menu-submenus` cases timed out at eight workers while a cold
  Vite development server started (they pass at the gate's four workers).
- The cause of the earlier observed SIGTERM-test timeouts remains unproven;
  no field timeout is attributed to the separately repaired spawn/signal window.

The spawn/signal correction was delivered as `ed156ef9b`; its
[acceptance record](../status/2026-10-26-browser-tmp-spawn-signals.md) owns the
bounded result. It is no longer pending work; the two performance bullets and
unresolved historical timeout causes above remain open.

The two SIGTERM pidfd-identity fixtures in `tools/tests/test_browser_tmp_launch.py`
used timed lifetimes (4 s fixture or child, 8 s sentinel or wrapper). One failed
in a package gate on 27 October 2026 ("identities changed before pidfd
retention") and reproduced once in 12 runs under CPU oversubscription; a 5 s
stall injected before identity pinning fails both on the old fixtures and passes
on the new ones. Fixture lifetimes are now event-bound (parent death, or a
proceed file; 120 s only bounds a leak), and failure paths reap the direct
children. The cleanup test previously reached its timeout fallback only because
its fixture outlived a 3 s wait; it now forces that fallback with a simulated
first timeout and asserts the fallback ran. Reinstating the earlier Popen-signal
fallback in memory still fails it. This closes that flake only; it says nothing
about the older SIGTERM timeouts above.

Delivered as main-first merge `0791ddb77` on `909412381` (feature `081ccb1a5`,
review correction `953b2c9b8`). Independent review first blocked on a lost
cleanup-fallback oracle; the corrected test forces its first timeout and the
confirmation review approved it. The integrated change-scoped gate passed
10/10 stages with frozen inputs and no touched or unchecked user data:
461 registered tool tests, 455 passed and 6 optional skips. No additional Rust,
browser, bindings or corpus execution was selected for this test-only change.
Local, tracking and live `main` matched after publication. Original failed
attempts, stall probes, fallback mutation and accepted receipts remain in
`<profile>/evidence/gate01-sigterm-flake-20261027/`.

## Spawn-record reader correction — 27 October 2026

A separate integrated package gate exposed a SIGHUP fixture `IndexError` while
reading an unfinished `child ` stdout record. The test-only reader correction
polls before reading and accepts newline-terminated records only. Final owning
module: 14 methods, zero skips; five deterministic synthetic framing controls
are calibrated against the original reader. These are not application tests or
an explanation of the older SIGTERM/cold-Vite timeouts. Published main-first as
`bc1b47a9a` after independent review APPROVE and branch/integrated gates10 green;
see the [bounded record](../status/2026-10-27-spawn-record-framing.md).

## Known boundary

Killing the stage supervisor with SIGKILL can still orphan children; see the
[web test gate contract](../contracts/web-test-gate.md).

## References

[ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md) ·
[test-suite speed dossier](../research/2026-10-19-test-suite-speed.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#test-gates-change-scoped-by-default-19-october-2026)
