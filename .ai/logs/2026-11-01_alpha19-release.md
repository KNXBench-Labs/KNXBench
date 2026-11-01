# Alpha 19 release acceptance log

Source cutoff `858b0a24d7028281397b65cd53548bd8e56e60f3`, coordination freeze
`264f57e616b998285432701301ebdf8a973d98df`, release branch `release/alpha19-20261101`
(version bump `58838c74f`, reader docs, fix `0695115ac`). Candidate identity comes from
the branch HEAD and revision-bound receipts; private receipts live under `<profile>`.

## Scope

- Combined delivered source since `v0.1.0-alpha.18`, version alignment and reader docs.
- Included packages: DIAG-03 recorder/bundle and its closeout (`7443e2ea6`, docs
  `858b0a24d`), OKX-01..07 distribution inspection including OKX-04 baggage/PNG/HTML,
  PRG-01 increments (run state, exact-0700 qualification, customer-adjustable refusal,
  byte-precise inactive selection, exemplar-bound restore), settings workspace,
  security dependencies (`zip` 9, `rustls` 0.23.46), product database 30->31.
- Project/store and core model 16 unchanged; product database 31 is data-only. Alpha 18
  refuses product 31, backup formats 3/4, `supportDiagnostics` history pages and
  System-2 checkpoint envelope 4; format-1/2 backups no longer restore. Disclosed.
- #30/#22 allocation mismatch unresolved; no hardware acceptance follows.

## Owner decisions for this release

- Start autonomously once DIAG-03 is delivered; publish autonomously after a fully green
  gate and clean dry runs on the same candidate.
- Pinned product corpus: a representative 28-file cross-section of the archived
  111-file selection instead of all of it. The legacy member-name test runs unchanged
  on it. The pinned 115-instance compatibility matrix cannot run on a subset and is
  replaced by a disclosed per-file isolated ingest differential, Alpha 18 CLI vs
  candidate CLI, on the same 28 files. No test was repinned.

## Candidate 1 (`b73c467a7`): rejected

- Release gate 29/30 green, inputs frozen. Red: the release-only device-diagnostics
  browser config, 8/8. After a simulated read the bundle carried the retained typed
  result instead of the token-reached raw recording, contradicting the diagnostic-bundle
  contract. Unreleased DIAG-03 source; no published artifact affected.
- Its AppImage and Docker dry runs were clean but do not carry over to candidate 2.
- Fix `0695115ac`: raw recording first for non-System-2, typed result only when the
  recording cannot be read (note "unavailable"), conflicts propagate, System-2 unchanged.
  Three route tests; on the pre-fix source the two raw-export tests fail, the System-2
  token test passes. Independent review approved; its test gap and minors are included.
- A separate full server-suite probe hung because its network namespace had no
  loopback; it is a harness error, not counted. The candidate-2 gate runs the suite.

## Candidate 2 (`0b827abee`): accepted and published

- Release gate 30/30 green, inputs frozen; package gate complete. Rust 7,978 passed /
  0 failed / 241 ignored; Vitest 4,875; fixture browser 969 + 69; production browser 625;
  extras 45 + 8 device-diagnostics; native 12 + 12; corpus 155/155 in 9 targets; product
  modules 15; legacy opt-in 1/1; ingest differential 28/28 identical.
- Dry runs 41324979516 (AppImage/MCP) and 41324981775 (Docker), publish jobs skipped.
- Tagged and published 1 November 2026; tag publishers 41330363960/41330363961
  succeeded. Public downloads, checksums, MCP version, AppImage host-library check and
  Broadway startup, Docker index/labels and an isolated amd64 smoke passed. The owner's
  container was unchanged. Receipt: `docs/evidence/release-alpha19-2026-11-01.json`.
