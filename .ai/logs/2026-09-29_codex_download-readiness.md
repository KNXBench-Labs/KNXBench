# Download readiness: offline coverage, pre-write backup and scoped evidence

Date: 2026-09-29
Agent: codex (Hermes)
Scope: product-data planning, CLI/HTTP/UI support display and pre-write snapshots; **no real bus action**.

## Decisions

- An offline plan is not proof of a live device. `verified` requires a cited hardware run for that application and download scope; a concrete plan without that evidence is `untested`, and no plan is `unsupported` with a named reason.
- The executor reads the load states and every memory region the plan will overwrite before its first device-mutating step. If it cannot read and persist a backup (new owner-only JSON file with file + directory sync and read-back), it aborts without that mutation. The file is a **region backup, not a full device dump**; it can contain private device configuration and must stay private.
- Restore is a separately confirmed download, with an additional untested acknowledgement when applicable. It rederives the procedure and refuses altered step descriptions or memory-region shape. No automatic rollback or live restore guarantee is claimed. See `docs/adr/0049-download-readiness-is-per-plan-and-backups-are-pre-write.md`.

## Corpus measurement and regression

- 103 local `.knxprod` packages, 246 programs. 55 plannable default/no-link images (1 cited hardware scope, 54 untested); 191 refused. Inside `MV-0701`/`MV-0705`: 55 of 181 planned, 126 refused.
- Refusals across all programs: not-memory-mapped 24; procedure-style 41; unmodelled-step 1; parameter-evaluation 59; parameter-value 33; image-structure 33. An earlier stale diagnostic had different category counts although the support totals matched; the private corpus test was run to completion, exposed the mismatch, the documentation and pin were corrected, and the complete ignored test passed (1/1, 922.16 s). Those figures are local-corpus-only, not KNX-wide or live-device support.
- Review found a restore CLI path that initially lacked the extra acknowledgement and closed it before delivery. Also changed a failure message that overpromised restoration. `No access key may be guessed`; backup and status messages never print a key. No access keys were committed.

## Branch verification and open boundary

- Branch Rust workspace: 125 suites, 2551 passed, 0 failed, 147 ignored. CLI focused tests, Rust fmt, Clippy, and xtask layering/headers/anchors/corpus gates passed. Full web tests/build and cargo-deny check passed. The explicitly ignored private-corpus regression passed separately.
- Backup/restore are simulator-verified only. Hardware remains parked by the user's “dann parken”, including K13/K14 and RF. Native desktop/WebKitGTK is not proven by web tests. Unknown and unsupported product procedures fail closed.
- Before integration: compare `origin/main` (the UI handover may move it), preserve the root's unrelated untracked `docs/paperclip-shutdown/`, re-run merged-result gates, then update `.ai/CURRENT_STATE.md` and release the web lock. Keep `stats.md` as its explicitly dated generated snapshot unless its own statistics generator is available; do not invent new AI-usage counts.
