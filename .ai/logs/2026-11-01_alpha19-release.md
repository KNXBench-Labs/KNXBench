# Alpha 19 release acceptance plan

Observed 2026-11-01 13:31 CET. Source cutoff `858b0a24d7028281397b65cd53548bd8e56e60f3`,
coordination freeze `264f57e616b998285432701301ebdf8a973d98df`, release branch
`release/alpha19-20261101` (version bump `58838c74f` plus the reader-docs pass on top).
Candidate identity comes from the branch HEAD and revision-bound receipts.

## Scope

- Combined delivered source since `v0.1.0-alpha.18`, version alignment and reader docs.
- Included packages: DIAG-03 recorder/bundle and its closeout (`7443e2ea6`, docs
  `858b0a24d`), OKX-01..07 distribution inspection including OKX-04 baggage/PNG/HTML,
  PRG-01 increments (run state, exact-0700 qualification, customer-adjustable refusal,
  byte-precise inactive selection, exemplar-bound restore), settings workspace,
  security dependencies (`zip` 9, `rustls` 0.23.46), product database 30->31.
- Project/store and core model16 unchanged; product database31 is data-only. Alpha 18
  refuses product31, backup formats 3/4, `supportDiagnostics` history pages and
  System-2 checkpoint envelope 4; format-1/2 backups no longer restore. Disclosed.
- #30/#22 allocation mismatch unresolved; no hardware acceptance follows.
- No release tag, GitHub release or registry push is authorized before the owner's GO.

## Not yet run (no results exist)

- Complete maintained `tools/run_package_gate.py --full` and the release extras
  (browser configs, native evidence/naming specs, whole-corpus product modules).
- Same-candidate AppImage/MCP and amd64+arm64 Docker workflow dry runs; publish jobs
  must stay skipped.
- Pinned product-corpus opt-ins need a per-release private restoration permission.
- `docs/releases/alpha19.md` keeps two `RELEASE-GATE-PENDING` placeholders for the
  release owner to fill from actual receipts.

## Evidence and boundaries

Private evidence belongs under `<profile>/evidence/`; checkouts under `<sources>`.
Final results are recorded only after actual execution. No fabricated acceptance or
silent skips. Dirty root and user installation data stay protected.
