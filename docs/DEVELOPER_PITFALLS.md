# Developer pitfalls

Repository facts that have tripped up contributors and coding agents more than
once. Each entry was checked against the source on `origin/main` `ae9c44f4`
(2026-10-12); if the code moves, fix the entry instead of working around it.
How to run the gates themselves is in [Repository verification targets](VERIFICATION.md);
this page only lists the traps around them. Think of it as the list of rakes
already found lying in the grass.

## Gates

- **CI runs exactly these checks** (`.github/workflows/`): `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `npm ci`, `npm test` and `npm run build` in `apps/knx-web`, the ts-rs bindings
  diff, and the `xtask` checks `check-layering`, `check-headers`, `check-anchors`,
  `check-ledger` and `check-corpus-gates`. A local gate that skips one of them is
  a partial gate; say which part ran.
- **`xtask` checks the workspace you point it at.** Since `dea70329`
  (2026-10-01) the scan tasks resolve their root at runtime (`--root PATH`, or
  the current workspace) instead of the checkout they were compiled in. Older
  binaries could report success for a deleted worktree; judge an unexpected
  green by the size of its output, not only the exit code.
- **Header rule (ADR-0018, `xtask/src/headers.rs`):** line 1 is one sentence, a
  `//!` doc in Rust or a single-line `/** … */` in TypeScript, at most
  `MAX_WIDTH` = 100 characters. Wrapping it into a multi-line block turns it
  into a missing header. Files without a header are counted against
  `ABSENT_CEILING`; fix the header, never raise the ceiling.
- **A full disk looks like a build failure.** A shared `target/` directory can
  grow past 150 GB. `No space left on device` or a linker `Bus error` before any
  test result is an environment problem; check `df -h` before a full gate.
- **Read gate verdicts from the gate, not from a pipeline.** In
  `cmd | tail -1; echo $?` the status belongs to `tail`. Redirect to a log and
  read the gate's own exit line.

## Handover and Git

- **`.ai/` is ignored, but `.ai/CURRENT_STATE.md` is tracked.** A plain
  `git add .ai/CURRENT_STATE.md` prints the ignored-path hint and exits 1, so a
  `git add … && git commit` chain stops there. Stage the handover with
  `git add -u -- .ai/CURRENT_STATE.md`; a new log file needs `git add -f`.
- **Numbered documents collide in parallel work.** `KNOWN_LIMITATIONS §N`, ADR
  numbers and research chapters are assigned per branch. Pick the next number
  after `git fetch`, from `git ls-tree --name-only origin/main docs/adr/` (or
  the fetched file), not from a local checkout. In a merge collision the
  integration branch keeps its numbers.
- **Status of record lives in one place.** Per-ID status is
  [`docs/status/LEDGER.md`](status/LEDGER.md), checked by
  `cargo run -p xtask -- check-ledger`. Other documents describe; they do not
  open or close work.

## Private corpus

- **`OriginalData/` is ignored in both spellings** (`.gitignore`), because a
  worktree borrows the corpus through a symlink, which `OriginalData/` alone
  would not match. A fresh worktree has no corpus, so corpus-backed tests take
  their skip path there; link the corpus before trusting a green run.
- **`walk_corpus_files` skips symlinks** (`crates/knx-testsupport/src/lib.rs`)
  and panics on unreadable directories. A symlink farm therefore measures as an
  empty corpus; copy files into a scratch directory and point
  `KNXBENCH_PRODUCT_CORPUS` at it instead.
- **Absent private data must never produce a green test.** `#[ignore]` with a
  reason is honest; a test that silently returns early is not. `check-corpus-gates`
  looks for such silent gates.

## Schema and storage

- **Use the version constants, not literals.** `CURRENT_SCHEMA_VERSION` exists
  in `knx-core` (project model) and in `knx-store` (store migrations). A test
  asserting a literal version breaks on the next migration.
- **Product-DB rewinds share one helper.** Older-version fixtures go through
  `crates/knx-productdb/tests/v20_rewind/mod.rs`; a new migration adds its drops
  there as well as to any rewind that bypasses it.
- **Positional `INSERT INTO <table> VALUES (…)` in tests encodes the old column
  count.** Convert such statements to named columns when a migration adds
  columns.

## Web app

- **Two tests pin the diagnostics companion's dependencies.**
  `apps/knx-web/src/DiagnosticsCompanion.test.tsx` pins its transitive imports,
  and `apps/knx-web/src/diagnosticShell.test.ts` scans literal hyphenated ids as
  class names. A new module on a shared import path can turn them red, so run
  the whole Vitest suite once, not only the package's own tests.
- **Bus-workspace browser fixtures must answer
  `GET /api/bus/monitor/telegrams`.** The bus monitor panel polls it
  (`pollBusTelegrams` in `apps/knx-web/src/api.ts`; server route in
  `apps/knx-server/src/bus_routes.rs`).
- **Motion settings live in `data-motion-level`** (set by
  `apps/knx-web/src/motion.ts`), not `data-motion`; assert the attribute the
  app actually sets.
