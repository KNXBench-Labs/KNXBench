# 2026-09-10 — Claude — Project documentation export (T13), Task 7: corpus test + doc close-out

Worktree: `.worktrees/t13-documentation-export`, branch `t13-documentation-export`.
Task 7 of the `2026-09-10-project-documentation-export` SDD plan — the final
task, test-and-documentation only. No changes to the existing HTTP route,
CLI subcommand, or web button (Tasks 1–6 already implemented those).

## What changed

### `crates/knx-app/tests/documentation_export.rs` (new)

Corpus-gated integration test. Imports the maintainer's real reference
project via `knx_etsproj::import_knxproj`, renders it with
`knx_report::render_html`, and asserts:

- every group address's `entry.address.format(style)` string appears in the
  rendered HTML;
- every device name, escaped via `knx_report::html::escape_text` the same
  way the renderer does, appears in the output;
- `<table`/`</table>` counts match, `<tr>`/`</tr>` counts match, and at
  least one table exists;
- the Summary section's nine `<tr><th>{label}</th><td>{value}</td></tr>`
  rows match counts computed independently from `Project` — not by calling
  `knx-report`'s own `compute_counts`, so the test cannot just be agreeing
  with the renderer's internal arithmetic;
- the document is self-contained: no `<script`, no `http://`, no `https://`;
- a second render with the same timestamp is byte-identical (determinism).

Skip-gated the same way `csv_roundtrip.rs` is: if
`OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj` doesn't
exist (gitignored, local-only, maintainer's real KNX installation), the test
prints `skip: OriginalData/ corpus not present (gitignored, local-only)` and
returns without failing. Confirmed via `grep` over the full
`cargo test --workspace` log that this message never fired during the gate
run — the corpus path genuinely executed.

Real corpus numbers, captured from the test's own `eprintln!` output, not
copied from any earlier plan/brief: **36 devices, 907 communication
objects, 514 group addresses**, rendering to **248799 bytes** of HTML with
**1** structural warning. All assertions passed against this real project —
none needed weakening.

Lives in `knx-app/tests/`, not `knx-report/tests/`, because
`xtask check-layering` walks dev-dependency edges too
(`xtask/src/layering.rs`), so a `knx-etsproj` dev-dependency on
`knx-report` would itself violate `knx-report`'s layering rule (`knx-core` +
`knx-projection` only). `knx-app` is the one crate allowed to see both
`knx-etsproj` and a pure crate like `knx-report`, exactly as
`csv_roundtrip.rs` already established for `knx-csv`.

`crates/knx-app/Cargo.toml` gained `knx-report.workspace = true` under
`[dev-dependencies]`, with a comment explaining why. No root `Cargo.toml`
change needed — `knx-report` and `chrono` were already declared as
workspace dependencies.

### Worktree-local, uncommitted symlink

`OriginalData -> /mnt/daten-i/Sourcecode/KNXBench/OriginalData`, created
because git worktrees do not carry over gitignored/untracked directories
from the main checkout. Verified via `git check-ignore` that the symlink
itself is not covered by the `.gitignore` pattern `OriginalData/` (trailing
slash only matches real directories) and shows as untracked
(`?? OriginalData`) in `git status`. Never staged — only the brief-mandated
explicit `git add crates/knx-app docs && git add -f .ai` commands were used,
never `git add -A` or a bare `git add .`.

### Documentation

- **`docs/IMPORT_EXPORT.md`** — new "§12. Project documentation export"
  section: crate/API summary (`render_html(&Project, &ReportOptions) ->
  HtmlReport`), self-contained-document properties, the eight ordered
  sections, "this is KNXBench's own document, not an ETS report" framing,
  the DPT-per-linked-object design rationale (vs. `knx-csv`'s
  `derive_dpt`), the document's self-stated limits, a note that PDF output
  is produced by the browser's own print dialog (no native PDF generation),
  and a Surfaces subsection naming the exact route
  (`POST /api/project/documentation-export {path}`), exact CLI invocation
  (`knx doc-export <store.knxdb> <out.html>`), and exact button name
  ("Export documentation…"). Own "Design record:" link at the end.

- **`docs/GAP_ANALYSIS_ETS.md`** — D4 row closed **for HTML only**, with the
  row's own text stating in-app printing and native PDF-without-a-browser
  remain explicitly open (not only in `KNOWN_LIMITATIONS.md`). T13 backlog
  bullet rewritten from open one-liner to a closed entry citing the crate,
  its dependency/layering rule, the `render_html` signature, all eight
  document sections, the honesty framing (cross-ref §38), the surfaces, the
  full test enumeration, and the real corpus numbers.

- **`docs/KNOWN_LIMITATIONS.md`** — seven new entries, §44–§50:
  44. no ETS report parity, none currently measurable (cross-ref §38)
  45. no native PDF output
  46. no manufacturer/product/program name resolution (cites `device.rs`)
  47. no parameter values or module-instance arguments (cross-ref §3, T18)
  48. single-language rendering only (cross-ref §37, T26)
  49. no in-application print preview
  50. no section selection (illustrated with the real ~249 KB HTML size for
      36 devices / 907 com objects / 514 group addresses)

- **`docs/IMPLEMENTATION_STATUS.md`** — new dated append-only entry, "T13,
  project documentation export, HTML only (2026-09-10)", covering the
  crate/deps/layering, the `render_html` API and its determinism guarantee,
  `HtmlReport::warnings` semantics, the `model.rs`/`render.rs` split, the
  eight sections, the deliberate two-source device-detail section
  (`build_device_detail` plus direct `project.devices.get(id)` reads for
  `commissioning`/`product_ref`/`program_ref`/`binary_data` — flagged
  explicitly as a pattern worth a reviewer's attention if either source
  changes shape), the honesty framing, surfaces, and the same re-derived
  test/corpus numbers. States explicitly that `docs/ROADMAP.md` was checked
  and correctly left untouched for T13/D4 (it names neither), and notes the
  separate, unrelated ROADMAP.md memo addition below.

- **`docs/ROADMAP.md`** — **separate, unrelated addition**, from an explicit
  mid-task live user request, not part of T13/D4: a short memo appended to
  the existing "Cross-cutting — Motion and animation" section recording two
  candidate visual style directions for whenever T27's motion work actually
  gets designed — (1) Apple-like: sleek, subtle, clean, restrained; (2)
  "techy glitch / cyberpunk OS", but still clean and sleek rather than
  noisy. Explicitly not decided, not designed, no task opened. Kept in its
  own commit, separate from the T13/D4 documentation work, per "focused
  commits, don't mix unrelated changes."

## Gate results (all re-run from this worktree, all green)

- `cargo fmt --all --check` — clean (after one `cargo fmt --all` pass to fix
  three minor formatting diffs in the newly-written test file).
- `cargo clippy --workspace --all-targets -- -D warnings` — clean.
- `cargo test --workspace` — **718 passed, 0 failed** (summed across every
  `test result: ok. N passed` line in the full log; zero `FAILED` matches).
  Corpus test confirmed to have actually executed against the real project,
  not skipped (its skip message never appears in the log).
- `cargo run -p xtask -- check-layering` — ok, including the new
  `knx-app` → `knx-report` dev-dependency edge.
- `cargo deny check` — advisories ok, bans ok, licenses ok, sources ok (only
  informational "advisory was not encountered" warnings for RUSTSEC IDs
  that don't apply to any resolved crate — pre-existing, unrelated to this
  change).
- From `apps/knx-web`: `npx tsc --noEmit` — clean. `npm test -- --run` —
  **145 passed** (13 test files). `npm run build` — succeeded;
  `dist/.gitkeep` (deleted by the build) restored via
  `git checkout -- dist/.gitkeep`.

## Re-derivation methodology

Every count written into the documentation was re-derived at the moment of
writing, not copied from the task brief or any earlier plan:

- `knx-report` test count (43): `cargo test -p knx-report -- --list`.
- Server documentation-export test count (4):
  `grep -c "#\[test\]\|#\[tokio::test\]" apps/knx-server/tests/http_documentation_export.rs`.
- CLI documentation-export test count (4):
  `grep -c "#\[test\]" apps/knx-cli/tests/cli_documentation_export.rs`.
- Frontend button test count (6):
  `grep -c "it(\|test(" apps/knx-web/src/DocumentationExportButton.test.tsx`.
- Real corpus numbers (36 devices / 907 com objects / 514 group addresses /
  248799 bytes / 1 warning): the corpus test's own `eprintln!` output,
  captured from a real (non-skip) run.
- Full workspace totals (718 passed, 0 failed): summed and grepped from a
  complete `cargo test --workspace` log.

## Concerns / notes for the reviewer

1. **Scope deviation**: the `docs/ROADMAP.md` motion-style memo is unrelated
   to T13/D4 and was added only because of an explicit mid-task live user
   request. It's kept as its own separate commit specifically so it can be
   reviewed/accepted/rejected independently of the T13/D4 documentation
   closure.
2. **`OriginalData` symlink**: worktree-local, uncommitted, verified never
   staged. Anyone continuing this branch elsewhere will need to recreate it
   for the corpus test to exercise the real path (it degrades gracefully to
   skip otherwise).
3. Two `Edit` tool "string not found" failures occurred while editing
   `KNOWN_LIMITATIONS.md` and `IMPLEMENTATION_STATUS.md`. The first was a
   transient tool glitch (identical retry succeeded). The second was a real
   transcription error on my part (dropped the words "on every path" when
   copying anchor text from an earlier tool output) — caught and fixed by
   re-reading the exact source text before retrying. No content was lost or
   corrupted; flagging only as a process note in case anyone else hits the
   same pattern.
4. The device-detail section's two-source read pattern
   (`build_device_detail` plus direct `Devices::get` field reads) is
   pre-existing from earlier tasks in this SDD plan, not introduced by
   Task 7 — documented here only because Task 7 is where it got written up
   for the first time.
