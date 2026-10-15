# Repository verification targets

Repository gates check a **runtime-selected workspace**, never the checkout in
which the executable was compiled. From the intended repository root:

```sh
cargo run -p xtask -- check-headers
cargo run -p xtask -- check-anchors
cargo run -p xtask -- check-ledger
cargo run -p xtask -- check-corpus-gates
cargo run -p xtask -- check-layering
```

Known traps around these gates (header width, disk space, staging the handover,
corpus links) are collected in [Developer pitfalls](DEVELOPER_PITFALLS.md).

From another working directory, select the candidate explicitly **before the
task name**:

```sh
/path/to/xtask --root /path/to/candidate check-headers
/path/to/xtask --root /path/to/candidate check-layering
```

`--root` may be relative to the working directory. The gate canonicalizes it,
requires that exact Cargo workspace root and checks the `xtask` and `knx-core`
workspace members at their expected manifest paths. There is no automatic
parent-directory search, build-tree fallback, fixture bypass or allow-empty
switch. A sub-crate, unrelated directory/workspace, missing/deleted target,
misplaced `--root` or extra scan-task argument fails. AppImage verification
uses the same root selection and retains its task-specific `--artifact-dir`
and `--tag` options. `freeze-fixture` is a separate storage helper, not a scan.

## Measured web tests

After `npm ci` in `apps/knx-web`, use:

```sh
npm run test:e2e       # build + complete default browser cases
npm run test:gate      # declared web/docs package gate
npm run test:benchmark # functional-only 1/2/4/4/2/1 scaling matrix
```

The [web gate contract](contracts/web-test-gate.md) defines the exact profile
coverage, Linux prerequisites, source-hash parallel admission and receipt limits.
`test:gate` is **web-only**, not a Rust/private-corpus/release acceptance. Its
browser default is at most four workers, capped by CPU affinity; timing/performance
and unreviewed/changed specs remain exclusive. Override with `-- --workers 1` for
serial execution or a constrained machine. Installed dependencies must match the
lock; the runner never installs them silently.

From the workspace root, an explicit immutable attempt path is also supported:

```sh
python3 tools/run_web_gate.py --profile web --output /path/to/new-attempt
```

`receipt.json` separates actual command exits, proof validity, queue wait and
input/build/version commitments. Existing attempt paths are refused. Do not edit
the candidate while a gate runs or retag old output as new execution. The three
existing execution leases remain in force; this version does not shorten them.
See the [audit](research/2026-10-14-test-suite-performance-audit.md) and
[measured matrix](evidence/web-test-optimization-2026-10-14.json). The approximately
2× functional-lane gain is host/scope-specific, not a promised whole-gate speedup.

## Native edit and reopen proofs

`tools/verify_name_editing.py` serves the actual built frontend with an isolated
real server. Its `--spec` selector covers `rename.native.ts`, `functions.native.ts`
and `name-recipe.native.ts`; the default selects only the first. A release gate
runs all three, in addition to `tools/verify_native_evidence.py`. Keep the usual
workspace/alpha/browser execution leases around these exclusive checks.

The reset API opens an empty project, and reload can restore an active project.
Open the demo copy through the rendered Commands button and the localized native
open command, scoped to the command dialog. The welcome-card button is not an
always-present file control; an unscoped combobox can instead match project
selects before a keyboard-opened palette is ready. These checks retain real
file-picker, edit, conflict, undo, save and reopen assertions, in EN/DE and wide/
narrow views. No mocked success or live bus operation is needed.

## Coverage contract

Each gate prints `gate target: <canonical root>`. Verify that this is **your
intended candidate**; another valid KNXBench checkout is not intrinsically
invalid. These guards do not know the caller's desired Git revision.

- Headers: each of `apps/`, `crates/`, `xtask/` must contribute non-generated
  Rust/TypeScript source. Missing/read-error/empty directories fail; generated
  bindings do not count as inspected sources. Output separates valid, absent,
  invalid and generated headers; the existing ratchet still applies.
- Anchors: `docs/` must contribute Markdown; root-level Markdown is checked too.
  An actual document with zero internal links is valid, not an empty scan.
- Corpus gates: both `apps/` and `crates/` must contribute Rust source. Output
  reports the number read, separately from silent-return violations. This is
  a textual source lint, **not** a private-corpus test run.
- Layering: Cargo metadata is resolved against the selected manifest, not CWD.
  All eight checked policy roots must be actual workspace members and graph
  nodes. Output reports resolved-package coverage; counts are measured, not
  pinned to today's source/package totals.
- AppImage: the selected target's existing artifact/version/manifest checks
  apply. No missing artifact or empty directory is a successful verification.

The scanners keep their documented exclusions and heuristics. Nonempty coverage
is not a completeness proof, immunity to concurrent tree changes, or assurance
that an old executable implements the current gate rules. Build changed gate
code in a fresh per-worktree `CARGO_TARGET_DIR`, confirm compilation, and keep
sources stable during verification. A stale pre-AR01 binary still has its bug;
Git pruning cannot rewrite its baked-in path. Do not silently bless it.

## Private-corpus test run

The textual lint above does not run a single corpus test, and CI cannot: the
corpus is private. The AR18 review (M7) found two corpus tests red for weeks.
One command now runs all of them:

```sh
unshare --user --map-root-user --net sh -c 'ip link set lo up && exec "$@"' sh \
  python3 tools/run_corpus_tests.py
```

- **What it runs:** `tools/run_corpus_tests.py` selects every test that is
  `#[ignore]`d for `OriginalData`. It groups them by Cargo target and runs
  each group with `--ignored`. `--list` prints only the selection.
- **What it prints:** test names and counts, never corpus content.
- **When it fails:**
  - exit `2`, with nothing run, if `OriginalData/` is missing;
  - exit `1` if any target fails, or if the passed count differs from the
    selection.
- **Environment:**
  - no `KNX_*` variables, so no hardware is addressed;
  - `XDG_DATA_HOME` points to a fresh temporary directory, so the
    developer's product database is neither read nor changed (M8);
  - network isolation is the `unshare` wrapper's job.

**When it must run:** on every release candidate (part of the
[final gates](https://github.com/KNXBench-Labs/KNXBench/blob/bca2d3336b96/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md)), and before merging a change to import,
storage, enrichment, the server's project routes or device download. The
2026-10-06 selection was 143 tests in 31 targets (`--list`).

## Release build provenance

Build every release candidate (AR17's AppImage included) with
`KNX_REQUIRE_CLEAN_TREE=1` in the environment. The build then fails unless
the checkout is clean and `HEAD` resolves, so the artifact's `--version`
names exactly the commit it was built from (ADR-0018 amendment,
KNOWN_LIMITATIONS §65). A plain development build stamps the last commit and
says nothing about uncommitted edits.

## Regression evidence

`xtask/tests/gate_scope.rs` exercises selected fixtures, unrelated/deleted
roots, sub-crate refusal, generated-only scans, argument mistakes and actual
runtime graph selection. Unit tests exercise missing/empty scans and intentional
nonempty fixture scopes. Fixture APIs do not authorize empty production scans.
AR01 reproduced a real old binary returning success over zero sources after
its owned build worktree was deleted; the new executable refuses that explicit
deleted target. Focused behavioral mutations are recorded in the AR01 log.
No KNX hardware, ETS corpus or UI/platform acceptance is implied by these gates.

## Storage command verification

The [storage command contract](contracts/STORAGE_COMMAND_CONTRACT.md) explicitly uses a
transactional whole-project-save fallback, not successful incremental no-op
arms. Its AR04 regression receipt distinguishes complete source routing from
individually executed behaviors, native reopen equality from external ETS
interoperability, and durable rollback from caller-owned memory/history recovery.
Private fixture tests must be selected by their actual offline scope; ordinary
workspace ignored counts are not proof that those fixtures executed.
