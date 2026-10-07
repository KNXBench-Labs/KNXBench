# 2026-09-30 — Channel Name/Number as first-class product data

Agent: codex. Goal lane: `goal.md` §12.3, KNOWN_LIMITATIONS §146 data half. No KNX/LAN/hardware traffic and no `apps/knx-web` source edits.

## Decision and implementation

ADR-0052 records nullable, verbatim `Channel/@Name` and `@Number` text. The import parser models both at the source boundary; the evaluator and `ComObjectChannel` project them separately from translated `@Text`. No label is invented. Product-DB v17→v18 reparses retained, hash-verified program blobs in savepoints, preserving scheme-evidence unknown rows except the now-typed unqualified `Channel/@Number` rows. It reconciles package counters and install reports; damaged/unclassifiable known program blobs get explicit failure evidence, and underflow aborts the migration. The UI field remains TypeScript-skipped until `goal-ui.md` U12 adopts it.

The ADR was initially drafted as 0051 before the concurrent commissioning lane published its independent ADR-0051 (Individual Address Write Enable). On rebase, this lane's ADR and all references were moved to **0052**; the commissioning ADR-0051 and its status entry were preserved. No commissioning behavior was changed.

## Verification evidence (aggregates only)

- Synthetic integration tests cover absent/empty/non-numeric attributes, prefixed lookalikes, evaluation/projection, actual v17 rewind, malformed blobs and reports, counter underflow, first-winner collisions, and preservation of other unknown evidence.
- Read-only corpus: fresh v18 ingest and upgrade of a separate genuine v17 database each processed 106 files, 0 failures. Streaming normalized row fingerprints agree across 3,067,570 dynamic nodes, 756 module arguments, 21,165 unknown rows, four report tables and 102 package counters (autoincrement evidence ids excluded).
- The opt-in 115-instance / 113-unique-package matrix compared with pre-v18 `origin/main`: public outcomes and all unrelated table/report totals agree. Only isolated/shared unknown totals (−281/−280), `ingest_unknown` (−280) and `package_install_unknown` (−95) moved. An independent read-only XML recount predicted 281 / 280 / 280 / 95; the 280 per-blob rows comprise 274 distinct blob/path keys plus six duplicated rows from reused blobs. The revised commitment passes.
- Branch gates before rebase: 134 workspace suites / 2,671 passing tests, 0 failures; warning-denied Clippy, rustfmt, header, anchor and layering checks passed. After rebase onto `origin/main` (including the independent commissioning ADR-0051), a fresh-target merged workspace run passed 136 suites / 2,689 tests with 0 failures, and warning-denied Clippy passed again. The final rebase corpus/anchor/header/layering checks are tracked in `.ai/CURRENT_STATE.md`.

Private raw corpus values, paths and per-product records were kept out of versioned documentation. §146 stays open for the UI presentation half; the alpha tag and user manual remain separate decisions/tasks.
