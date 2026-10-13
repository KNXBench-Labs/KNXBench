# 2026-10-13 — Claude — tester download reports (verification flow, part A)

Branch `feature/tester-download-reports` (worktree `kl170-issue10`).

## Owner decisions (2026-10-13)

- Verification flow in the device download tab, simple explanation and
  step-by-step guide; reports by issue or to issue@knxbench.com.
- issue@knxbench.com works; for tester reports only (contribute@ unchanged).
- Reports include parameter values; no device name or address.

## Facts checked

- GitHub issue forms: text fields are prefilled from query parameters named by
  field id; URLs over the server limit answer 414 (github/docs
  `creating-an-issue.md`, section "Creating an issue from a URL query").
- `application_program.source_sha256` names the ingested program file, a
  sharper identity than a package hash.
- The server session moves `PreparedDownload` into its task, so the report is
  prepared at admission and kept with the session.

## Gate

Mutants 5/5 killed (report while running, lost stopped step, space kept in
encoding, report without write, values in the summary). First full test run
failed two wall-clock budget tests (`dynamic_tree.rs`, 20 s) at load average
36 from parallel sessions; `cargo test` then skipped the remaining binaries.
Rerun with `--no-fail-fast` at load ~5: 4257/0/197. Same input fingerprint
for gate and rerun. Corpus 144/144.
