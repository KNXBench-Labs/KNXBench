# T12 UI residue batch A — final reconciliation

Date: 2026-09-22
Branch: `t12-ui-residue`
Base: `5ac0732` (`origin/main` at batch start)

## Delivered scope

T12 closes six numbered UI limitations while preserving their original headings
and therefore their durable Markdown anchors:

- §19: search selections reveal collapsed Project Explorer ancestry and scroll
  one canonical row.
- §24: the plain-web `FsPicker` accepts native file drops and serial sequential
  multi-file uploads without changing its single-path result contract.
- §§23/30: `/api/project/download` streams the current project in bounded body
  chunks, and the plain-web File menu exposes the browser download separately
  from mounted-volume Save As.
- §96: an exact-token owned successful operation can recover the current server
  tree after its POST response is lost. An `open` snapshot also restores stored-
  path state; direct frontend coverage proves the next plain Save does not ask
  for a Save As path.
- §118: direct and recovered loads publish one localized basename through the
  existing non-error polite status-toast region. Covered failed loads assert
  that they never enter that success region.

Report preview and section selection (§§49/50) remain deferred. The required
T14 crate/API report was absent at Task 7 preflight, so T12 did not guess a
frontend contract or edit report-preview behavior.

## Commit evidence

- `3c706f3`, `5a2a7a0`, `35bf4af`: search reveal and its review corrections.
- `9366905`, `03ee7cb`, `adea794`: dropped/multi-file uploads and race fixes.
- `96993df`: bounded download streaming and response-body lifetime.
- `4d9a0a3`: plain-web browser download command.
- `ff1fe94`, `00cdffd`: current-project recovery and atomic replacement state.
- `8bb03eb`: localized successful-load status announcement.
- Task 7 reconciliation is the commit containing this log, with subject
  `docs(ui): six loose ends file paperwork`.

The accidentally tracked Task 3 scratch report was removed. Its durable evidence
is consolidated here; `.ai/CURRENT_STATE.md` no longer links to that scratch file.
`docs/LIMITATION_TRIAGE.md` was not edited.

## Verification

All Cargo commands used the local low-debug environment:

```text
CARGO_TARGET_DIR=/var/tmp/knxbench-t12-target
CARGO_INCREMENTAL=0
CARGO_PROFILE_DEV_DEBUG=0
CARGO_PROFILE_TEST_DEBUG=0
CARGO_BUILD_JOBS=2
```

| Command | Result |
| --- | --- |
| `npx tsc --noEmit` | exit 0 |
| `npx vitest run` | exit 0; 900 passed across 63 files |
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | exit 0 |
| `cargo test --workspace --no-fail-fast` | exit 0; 1,953 passed, 0 failed, 5 ignored across 92 result blocks |
| `cargo run -p xtask -- check-layering` | exit 0 |
| `cargo run -p xtask -- check-headers` | exit 0; 194 well-formed, 162 absent at the documented ceiling, 30 generated skipped |
| `cargo run -p xtask -- check-anchors` | exit 0; 389 links across 182 Markdown files, none dead |
| `cargo deny check` | exit 0; advisories, bans, licenses, and sources all OK |

The first Task 7 anchor run correctly rejected five links after resolved sections
§§23/24/30 had acquired replacement headings. Restoring the original stable
headings fixed the durable anchors without weakening their resolved bodies; the
fresh final anchor run above passed.

## Scope and environment audit

No KNX, multicast, LAN, gateway, or hardware command or traffic occurred. No
private-LAN literal, forbidden T12 individual-address fixture, group-address
selector, or dotted group-address notation was added. The batch preserves the
existing theme-token and reduced-motion boundaries; Task 7 changes no styling.
All newly exercised user-facing success text continues to come from the existing
English/German `loadProgress.succeeded` catalogues.

At Task 7 preflight `/mnt/daten-i` had 168 GB free. The project-local
`.worktrees` directory contained only the active `t12-ui-residue` worktree after
the user-requested removal of stale clean merged project-local worktrees. That
cleanup also removed anything cached inside those removed worktrees. It did not
cover, and this log does not claim cleanup of, the separately named
`/mnt/daten-i/Sourcecode/KNXBench.worktrees` directory.

## Next goal task

Continue with goal Task 13. Task 14 remains the prerequisite for the deferred
§§49/50 UI half because its crate/API report must define the contract first.
