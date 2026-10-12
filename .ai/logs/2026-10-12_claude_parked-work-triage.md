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
