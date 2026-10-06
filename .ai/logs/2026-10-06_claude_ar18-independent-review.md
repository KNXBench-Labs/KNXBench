# 2026-10-06 — AR18 independent whole-product review (Claude, fresh session)

Brief: `docs/review/AR18_REVIEW_BRIEF.md`. Verdict file:
`docs/review/2026-10-06-alpha-independent-review.md`. Branch
`review/alpha-independent`, based on `f2b31538`.

## What was done

- Own worktree `../KNXBench.worktrees/alpha-review` (detached at `f2b31538`),
  own scratch and `CARGO_TARGET_DIR`, both leases around every workspace
  build/test, all `KNX*` variables unset, tests in a loopback-only namespace.
- Full gate list of the brief: all exit 0 (Rust 3,317/0/177, Vitest 2,071,
  Chromium 139, five xtask checks, cargo deny, check-appimage).
- 142 `OriginalData` corpus tests (own selection from the ignore reasons):
  142/0. Product DB used through a copy under a private `XDG_DATA_HOME`; the
  copy stayed byte-identical to the user's real database.
- Compare-harness revert mutant: 2 failures (503) back, restored.
- 19 adversarial `.knxproj` inputs built from the fictional sample; refusal
  paths of the store; migration rollback; CLI → server → Save As round trip.
- AppImage (copy, SHA-256 verified) offline under a private Xvfb; production
  web build in Chromium against the real server; privacy scan of the
  extracted AppImage.

## Result

`READY_WITH_CONDITIONS` — no CRITICAL; four IMPORTANT findings, each to be
fixed or explicitly accepted by the user before AR19:

- F1 Open/Import discard unsaved edits without asking (New refuses with 409).
- F2 duplicate / case-colliding archive members lost or substituted silently.
- F3 crafted 1–2 MB archives drive import to 2–3 GB RSS (no actual-size bound,
  no cumulative budget).
- F4 `knx import --store <existing>` replaces a project silently.

Nine MINOR findings (M1–M9), see the verdict file.

## Process notes

- One `cargo run -p xtask -- check-anchors` after writing the verdict ran
  without the leases and in the worktree's default `target/` (small xtask
  build, no workspace test). Later checks used the leases again.
- No product code changed. The only source edit was the temporary revert of
  the compare-harness fix, restored with `git checkout` and checked clean.
- The `OriginalData` and `project_dump.json` symlinks in the worktree were
  removed at the end; scratch artifacts stay in the reviewer's scratch
  directory until the user no longer needs them.
