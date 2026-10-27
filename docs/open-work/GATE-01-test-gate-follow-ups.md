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
  suites and case-level parallelism in the default lane remain unreviewed.
- Three `file-menu-submenus` cases timed out at eight workers while a cold
  Vite development server started (they pass at the gate's four workers).
- The cause of the earlier observed SIGTERM-test timeouts remains unproven;
  no field timeout is attributed to the separately repaired spawn/signal window.

The spawn/signal correction was delivered as `ed156ef9b`; its
[acceptance record](../status/2026-10-26-browser-tmp-spawn-signals.md) owns the
bounded result. It is no longer pending work; the two performance bullets and
unresolved historical timeout causes above remain open.

The two SIGTERM pidfd-identity fixtures in `tools/tests/test_browser_tmp_launch.py`
used timed lifetimes (4 s fixture, 8 s sentinel). One failed in a package gate
on 27 October 2026 ("identities changed before pidfd retention") and reproduced
once in 12 runs under CPU oversubscription; a 5 s stall injected before identity
pinning fails both on the old fixtures and passes on the new ones. The fixtures
are now event-bound (exit on parent death or on the cleanup release file; 120 s
only bounds a leak), and failure paths reap the direct children. This closes that
flake only; it says nothing about the older SIGTERM timeouts above.

## Known boundary

Killing the stage supervisor with SIGKILL can still orphan children; see the
[web test gate contract](../contracts/web-test-gate.md).

## References

[ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md) ·
[test-suite speed dossier](../research/2026-10-19-test-suite-speed.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#test-gates-change-scoped-by-default-19-october-2026)
