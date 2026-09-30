# U11 / ISSUE-09 — exact group-link order on undo (2026-09-30)

## Review finding

`UnlinkComObject` previously returned `LinkComObject` as its inverse. Linking appends: a failed atomic paired unlink with another link interleaved changed the existing link order during rollback, despite reporting failure. The source of the finding is `crates/knx-core/src/command.rs` (Unlink/Batch), not an undocumented ETS ordering rule. The named test `failed_unlink_both_and_undo_preserve_original_group_link_order` failed before the fix (exit 101; project inequality). No bus connection or device write was used.

## Correction and focused evidence

The internal `RestoreGroupLink` inverse retains the removed `GroupLink` and its index. It validates the index before insertion, restores imported duplicate/dangling links verbatim, and returns `UnlinkComObject` for redo. `knx-store/src/command_sync.rs` handles the new variant consistently with the existing no-op incremental group-link sync; server project saves use their existing whole-project path. Added regressions cover failed paired unlink, successful paired unlink undo/redo with an interleaved link, imported duplicate links, and out-of-bounds restore without mutation. A deliberate append-only mutation made the order regression RED (exit 101 without compilation error); restoring indexed insertion made it GREEN.

Final branch evidence after the correction: `knx-core --lib` 614 passed, `knx-store --lib` 80 passed, `http_edit_routes` 19 passed; the 78-file web suite passed 1,213 with TypeScript/build and six local Chromium layout cases. The corpus-backed branch gate passed `cargo fmt`, strict workspace Clippy, 125 Rust suites / 2,588 passed / 0 failed / 148 ignored / 0 `SKIP:`, and all four `xtask` checks (layering, headers 311/161 at ceiling, anchors 397/215 none dead, corpus gates). `git diff --check` and the added-line security scan had zero findings. Secrets and credentials were not copied into this log.

## Remaining delivery boundary

The in-session full-diff review and current branch gate are complete; the isolated feature still must be committed and reconciled with remote `main`, which advanced in parallel and overlaps the server domain and handover/research/limitations/status docs. Keep both sides of any conflict, repeat full gates on the actual integration tree, push/read back, release the web lock, and clean only task-owned artifacts. Then follow `goal-ui.md`: U12 (ISSUE-05, then ISSUE-08 UI half after prerequisites) and U13 (user-decided whole-track review/handover).
