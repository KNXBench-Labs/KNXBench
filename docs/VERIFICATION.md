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

## Package gate: targeted by default, complete for releases

One maintained runner replaces the private package-gate scripts
([ADR-0144](adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md)):

```sh
python3 tools/run_package_gate.py --base origin/main --dry-run  # show the plan
python3 tools/run_package_gate.py --base origin/main            # routine package
python3 tools/run_package_gate.py --full                        # Alpha, Beta, release
```

The routine run always does the cheap static checks (format, documentation,
browser lane guard, the five xtask checks, Python tool tests, diff check) and
then only what the changed paths can affect: the Rust members that changed and,
for library changes, their reverse dependents; crates whose sources name a
changed file; browser specs whose import/navigation closure reaches a changed
web file; production suites for production-bundle inputs; bindings when
`knx-projection` is affected; corpus tests of the affected packages; website
and story tests for their directories. A path without a rule selects the
complete gate. `--full` runs the complete web profile (which itself holds the
documentation, tool-test, lane-guard, xtask and diff checks, so the runner
lists it as one stage), the whole Rust workspace,
the bindings check, every corpus target, website and story tests.

The receipt (`target/package-gates/<id>/receipt.json`) holds the plan with the
paths that selected each stage, every stage's exit, wall time and counts, and
the source fingerprint at start and end. Run it on a committed candidate in a
worktree nobody edits meanwhile. It takes the usual leases (browser stages via
the web gate, Rust and corpus stages under `knx-workspace-gates.lock`) and
links the private corpus from the main checkout only for the corpus stage.

## Rust workspace gate

The web bundle comes first, also in a fresh worktree:

```sh
(cd apps/knx-web && npm ci && npm run build)
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

`knx-desktop`'s Tauri build script bundles `apps/knx-web/dist` as the
release frontend, so Clippy and the tests stop at that build script while the
bundle is missing; it then names this command. Never drop the crate with
`--exclude knx-desktop`: the gate would no longer show that the desktop shell
still builds. CI keeps the same order, and `tools/tests/test_rust_gate_order.py`
holds both to it. The web gate's own `build` stage leaves the bundle in place,
so a package that runs `tools/run_web_gate.py` first can go straight on to
Clippy.

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
browser default is at most four workers, capped by CPU affinity; cases tagged
`@exclusive` (real-time and animation measurements, enforced by
`tools/browser_lanes.py`) run alone. Override with `-- --workers 1` for
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

## Generated bindings and release-media preflight

A bindings check must compare the complete, nonempty producer/consumer path sets
and preserve quoted literal content. Formatting differences outside tokens are
not model drift; whitespace inside a quoted literal remains significant.

The locked TypeScript 7.0.2 package exposes its scanner through
`typescript/unstable/ast`; it does not have the legacy absolute-directory
CommonJS entry used by an older private release comparator. Resolve the declared
export from the web package and verify its installed signature before using it.
Comparator controls must reject quoted/escaped-space drift, missing/extra/empty
file sets and invalid literals, while admitting formatting-only differences.
A comparator setup failure is not evidence of generated-type drift, nor a pass.
Keep its original refusal and inspect unchanged input manifests before reusing
successful predecessor stages; unstarted steps still need actual execution.

Every `playwright*.config.ts`, including a config inheriting the manual capture
config, explicitly imports `./browserTmp`. The repository tooling test checks
that entry-point contract. Check tooling and scanner availability before long
release suites, and execute the real release-media capture after the server
build. A config-list preflight also needs `KNX_SERVER_BIN` and a task-owned
`KNX_RELEASE_MEDIA_DIR`; it neither starts the server nor proves a capture.

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
  each group with `--ignored`; tests in a bundled `tests/it` binary are named
  by module path and run with `--exact`. `--list` prints only the selection,
  `--package NAME` (repeatable) restricts it to the named packages, which the
  change-scoped package gate uses.
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
[final gates](https://github.com/KNXBench-Labs/KNXBench/blob/6222e17754f3/docs/archive/alpha-0.1/ALPHA_FINAL_GATES.md)), and before merging a change to import,
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
