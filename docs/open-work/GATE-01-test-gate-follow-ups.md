# GATE-01 — Test-gate speed and stability follow-ups

**Status:** Open. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

Small and optional; none blocks a delivery.

## Open

- Production browser suites run one after another with one worker each
  (771.7 s in the last complete gate; `table-columns` alone 231.3 s). A lane
  review per suite could let safe suites run in parallel again.
- Three `file-menu-submenus` cases timed out at eight workers while a cold
  Vite development server started (they pass at the gate's four workers).
- The spawn/signal window of `browser_tmp.run_command` (forwarders installed
  after the spawn) has a fix and regression tests pending full acceptance, see
  [the status note](../status/2026-10-26-browser-tmp-spawn-signals.md). No
  field timeout is attributed to it; the cause of the observed
  `file-menu-submenus` and earlier SIGTERM-test timeouts remains open.

## Known boundary

Killing the stage supervisor with SIGKILL can still orphan children; see the
[web test gate contract](../contracts/web-test-gate.md).

## References

[ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md) ·
[test-suite speed dossier](../research/2026-10-19-test-suite-speed.md) ·
[history](../history/OPEN_WORK_2026-10-25.md#test-gates-change-scoped-by-default-19-october-2026)
