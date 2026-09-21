# T09 — scan reconciliation

## Result

Completed line-scan sessions can be compared with the currently open project
and explicitly selected findings can be applied as one undoable command batch.
The scan itself remains read-only and reconciliation emits no KNX frame.

## Behaviour and invariants

- The shared comparison reports `unexpected`, `missing`, and
  `excluded_in_project` separately.
- Scanner-self and excluded addresses are unexamined evidence, never actionable
  missing-device findings.
- The UI starts with an empty selection and clears it whenever the project tree
  changes through reconciliation, undo/redo, or another mutation.
- Unexpected addresses create minimal product/program-less devices. A matching
  topology line is resolved across every installation; otherwise the first
  installation's unassigned list is used.
- Missing addresses must resolve to exactly one current device. Deletion is
  refused while building placement, parameter data, module instances, or group
  links still depend on the device.
- One batch and one undo cover all selected changes. Allocator state and exact
  topology ordering are restored, so apply followed by undo is structurally
  identical. Empty selection is a true no-op with no undo entry.

## Review fixes

The first independent review found four Important issues: scanner-self evidence
was actionable, deletion could orphan dependent data, line lookup stopped at
the first installation, and the UI comparison could go stale after project
changes. Each received a regression test and a focused fix. The follow-up
review reported 0 Critical and 0 Important findings. A separate HTTP assertion
proves reconciliation sends no additional fake-tunnel frame.

## Verification

All commands exited 0 after the final changes:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --no-fail-fast`
- `cargo run -p xtask -- check-layering`
- `cargo run -p xtask -- check-headers` (186 with headers, 162 without; ceiling 162)
- `cargo run -p xtask -- check-anchors` (375 links in 175 Markdown files)
- `cargo deny check`
- `npx tsc --noEmit`
- `npx vitest run` (821 tests in 59 files)

The Rust build used a lean target under `/tmp` because the data volume had only
about 4 GiB free; two failed linker attempts were storage-only and disappeared
after deleting this worktree's reproducible `target/` cache. No real gateway,
private LAN endpoint, or KNX device was contacted.
