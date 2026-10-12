# 2026-10-12 — Claude — worktree prune and parked-work triage

Docs-only housekeeping on `origin/main` `5a33e95c`. No `apps/knx-web` edits, no
root-checkout changes, no lock taken or released.

## Prune

- Rule (user): remove a linked worktree only if its HEAD is an ancestor of
  `origin/main` **and** its dirty residue is empty or only `.ai/CURRENT_STATE.md`
  or tool scratch. Keep the six named packages, keep the web-lock worktree.
- Inventory of 30 linked worktrees, with the process cwd, HEAD ancestry, dirty
  paths and ignored work files of each.
- **Removed (21):**
  - `data-tooling-20261010`, `data-tooling-editing-20261011`,
    `data-tooling-rtf-20261010`, `data-tooling-stats-latest-20261011`;
  - `hardware-readiness-20261011`, `history-integration-20261009`;
  - `import-main-c0e72cdb5ee5`, `import-revalidation-c0e72cdb5ee5`;
  - `instruction-editor-20261011`, `instruction-integration-20261011`,
    `instruction-model-20261011`, `instruction-service-20261011`;
  - `login-feedback-limits-20261011`, `login-integration-20261011`;
  - `note-http-20261011`, `note-service-20261011`, `note-ui-20261012`;
  - `parameter-contract-followup-20261012`, `parameter-editors-20261011`;
  - `project-notes-20261011`, `rename-devices-ga-20261009`.
- Right before each removal, the script re-checked three things: HEAD unchanged,
  status byte-equal to the backup, no process cwd.
- **Backup first.** Per worktree it holds:
  - HEAD and branch;
  - `git diff HEAD --binary`, plus the porcelain status;
  - an untracked tar and an ignored-work tar. The ignored files are `.ai/logs`
    receipts and runners, including about 640 MB of codex gate artifacts each
    for the parameter and instruction deliveries, plus `.projectstats`;
  - a per-file sha256 and an overall `MANIFEST.sha256`.

  Copies sit in the profile scratch `triage-20261012/worktree-backup/` and,
  durably, in `KNXBench.backups/2026-10-12-worktree-prune/`. Both verify
  against the manifest and every tar lists. The backup stays local only.
- Local branches were not deleted (not requested).
- **Kept beyond the six:**
  - `web-params-20261012`: live, 15 processes, web-lock package 3;
  - `offline-naming-20261011`: an untracked device-naming design draft, absent
    on main;
  - `data-tooling-stats-20261011`: a modified `docs/ProjectStats.md`, different
    from main.
- `web-instructions-20261012` was already gone, delivered as package 2.
  `docs-refresh-20261008` and `$TMPDIR/devnav-ro` were already gone.

## Triage

`docs/status/2026-10-12-parked-work-triage.md` covers the six kept packages. The
method avoids touching their checkouts:

1. A temporary-index snapshot commit for each uncommitted tree.
2. A per-file `git apply --cached --check` (forward and reverse) against main's
   tree.
3. `git merge-tree --write-tree` for the conflict map.

Results:

| Package | Recommendation |
| --- | --- |
| native-evidence | adopt |
| ux-feedback | adopt AP1/2/3/5, drop AP4; ADR-0112 must be renumbered |
| functions-sites | ask (schema 12 vs main 14, attempt 4 failed) |
| import-lifecycle | ask (never-compiled preflight residue) |
| audit-findings | drop (superseded) |
| native-import-diagnostics | drop (fully on main) |

## Observations

- `docs/adr/README.md` on main has no rows for 0110–0112. Left to the owning
  tracks.
- About 5 GB freed on the project drive; the build targets are mostly shared.

## Owner decisions applied (2026-10-12 17:13)

The user's decisions were "functions: wip, lifecycle: archive, AP4: drop, drop
both, delete branches". "Drop both" means audit-findings and
native-import-diagnostics, as in the proposed template. `offline-naming` and
`data-tooling-stats` were not decided and were left alone.

- **Backups.** The four retired worktrees were backed up into the same local
  prune backup: HEAD and branch, diff, untracked and ignored tars, sha256 sums.
  The manifest now has 198 files and verifies.
- **functions-sites.**
  - Committed the whole residue as `wip(functions)` `1154b72f` with the KNXBench
    identity. All 91 files are byte-equal to the backup.
  - No home or corpus paths appear in the added lines.
  - The worktree was removed. The local branch is kept and not pushed.
- **Worktrees removed:** import-lifecycle, audit-findings,
  native-import-diagnostics.
  - The removal of native-import-diagnostics was interrupted by a 60 s tool
    timeout. Of the 787 files left behind, 777 were HEAD blobs and 10 were in
    the backup. Finished with `shutil.rmtree` and `git worktree prune`.
- **Branches.** Deleted 24 local branches, one per retired worktree (21 + 3).
  Each was checked as an ancestor of `origin/main` and deleted with
  `update-ref -d <ref> <sha>`. The list is in `deleted-branches.txt` in the
  backup, and every SHA is still reachable from main.
- **Left alone:**
  - the remote branch `test/import-integrity-revalidation-20261010`;
  - the local branches without a worktree:
    - `design/group-address-studies-aca414` and `docs/hygiene-audit-20261011`,
      one unmerged commit each;
    - `feature/history-20261009`, already on main;
  - the non-worktree directories `co-publication-20261009`,
    `co-stats-20261009` and `xdg` under `KNXBench.worktrees/`.

## Leftovers sorted (2026-10-12 18:11)

- **Bundle first.** Wrote `leftovers.bundle` with four refs to
  `KNXBench.backups/2026-10-12-branch-leftovers/`. It verifies, and its
  `list-heads` equal `refs.txt`. The directory also holds a copy of `xdg/`, a
  README and a manifest.
- **Branches deleted:**
  - `feature/history-20261009` with `branch -d`;
  - `design/group-address-studies-aca414` (contained in main) and
    `docs/hygiene-audit-20261011` (unique, bundled), both with
    `update-ref -d <sha>`;
  - the remote `test/import-integrity-revalidation-20261010`, with
    `--force-with-lease`, after confirming it is an ancestor of main and has no
    open PR.
- **Folders removed:** `co-publication-20261009`, `co-stats-20261009` and `xdg`.
- **Not replayed.** The hygiene audit's corrections were not cherry-picked: they
  conflict in 4 of 6 files and encode outdated facts. The five still-stale
  claims are listed in the triage note instead.
- **Not in the request, left in place:** remote `fix/docker-import-diagnostics-20261009`
  and `fix/import-status-20261011`, both ancestors of main.

## Log restored (2026-10-12)

Commit `e61158c6` published this log as a single newline. The end-of-file
cleanup used `open(p,'w').write(open(p).read()...)`, which truncates the file
before reading it, and the blob readback compared against the already-truncated
local copy. Restored here from `16c5eb84` plus the leftovers section as written.
## Stale claims fixed, last remote branches gone (2026-10-12 19:13)

Each claim was re-checked against the source before rewording:

1. **`KNOWN_LIMITATIONS` §private schema-23 review.** The claim is now marked
   historical and pre-repair. The production importer runs on that project in
   `private_schema23_reconciliation.rs` via `import_ets_project_bytes`; passing
   is cited from the closure receipt, not re-run here.
2. **Selective import.** `SELECTIVE_MODEL_VERSION` = 14 = `CURRENT_SCHEMA_VERSION`.
   - v13 (`7db41a5a`) remaps instruction overlays.
   - v14 (`e974624e`) keeps destination notes and refuses source notes.
3. **PDB-9 display-only attributes.** The opt-in Checked editor (web package 3)
   covers Time (decimal count) and integer Slider fields. Colour/Picture are a
   fallback view only.
4. **`DATA_MODEL`.** The instruction edit API and editor are published and
   mounted (web package 2).
5. **`OPEN_WORK` and `ROADMAP` audit date.** Now worded as the "baseline audit"
   with dated sections since.

The remote branches `fix/docker-import-diagnostics-20261009` (`af1055e9`) and
`fix/import-status-20261011` (`30f054b3`) were both ancestors of main with no
open PR. They were deleted with an expected-SHA lease. The remote now has
`main` only.

## Last two worktrees removed (2026-10-12 19:19)

The user's decision: "Wenn sie nicht mehr benötigt werden dann weg" (if they
are no longer needed, remove them). Both were checked and neither is needed:

| Item | Finding |
| --- | --- |
| offline-naming | HEAD `f7a5d8dc` is an ancestor of main. Residue: handover copy plus the untracked `docs/contracts/offline-naming-recipe.md` device-naming draft, not on main or linked |
| data-tooling-stats | Detached `cdff5b56`, an ancestor. Residue: an 11 October ProjectStats refresh, newer than main's 10 October one but stale, plus the `.projectstats` cache (the root has its own) |

- Backed up both to `KNXBench.backups/2026-10-12-worktree-prune/`. The manifest
  now has 214 files and verifies.
- Removed both worktrees after re-checking HEAD, status and process cwd.
- Deleted `feature/offline-naming-20261011` with an ancestor check and
  `update-ref -d` with the expected SHA. `branch -d` refused because it compares
  against the root checkout's stale HEAD.
