# PRG-01-N9 — post-download check consumer consistency (21 November 2026)

Slice: audit CLI, server (history/reconcile), web and report readers of the
post-download check record for drift. Simulator/fixture evidence only; no bus,
hardware or ETS access; hardware-untested.

## Findings

- Web admission, activity history and the check type agree: the shared case table
  (`postDownloadCheck.cases.json`) is run by Rust and the web reader.
  A rule-by-rule comparison of `MatchedWire` and `admitRunState` found no drift.
- Drift 1 (fixed): the report summary dropped the run-state reason that the CLI
  lines and web view state ([ADR-0186](../adr/0186-report-summary-keeps-the-run-state-reason.md)).
- Drift 2 (pinned, not changed): a report file's `postDownloadCheck: null` reads
  as absent, history refuses it. It never becomes a result and reconcile refuses it.

## Tests

`crates/knx-app/src/download_report_consumer_tests.rs`: report file against the
whole shared table, null/absent characterisation, summary-vs-CLI agreement on
every fixture. Gate output is recorded in the PR.

## Still open

Broader PRG-01 items are unchanged (see the detail file). Reconcile HTTP and the
CLI `--report` writer were not re-tested beyond existing suites.
