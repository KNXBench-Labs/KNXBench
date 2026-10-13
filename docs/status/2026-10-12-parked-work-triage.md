# Parked-work triage — 2026-10-12

Read-only triage of the six linked worktrees the owner asked to keep during the
2026-10-12 worktree prune. Nothing here merges, rebases, commits or deletes any of
them. Each recommendation is a proposal for the owner. It is not a status change:
per-ID status stays in the [ledger](LEDGER.md).

**Basis:** `origin/main` `5a33e95c` (fetched 2026-10-12). Self-review only.

## Method

For each worktree:

1. **Snapshot.** Committed branches were used as they are. For an uncommitted
   tree, `git add -A` was run into a *temporary* index and `commit-tree` built a
   dangling snapshot commit. The worktree's own index and files were not touched.
2. **File-by-file check against main.** Each changed path's package diff
   (merge-base → snapshot) was tested against main's tree with
   `git apply --cached --check`:
   - **on main**: the diff reverse-applies, so main already contains it;
   - **missing**: it applies forward cleanly, so main lacks it;
   - **conflict/partial**: neither applies, so main rewrote the area.
   For conflict/partial files, every added line was also searched for on main.
3. **Conflict map.** Built with `git merge-tree --write-tree --name-only
   --messages origin/main <snapshot>`, which writes nothing to any checkout.
4. **Acceptance.** Taken from the worktree's own handover entries and status
   documents. These are the owners' self-reports, not re-run here.

None of the six branches exists on the remote. Committed work uses the
`KNXBench <github@knxbench.com>` identity.

## Summary

| Worktree | State | Size vs merge-base | Per file vs main | `merge-tree` conflicts | Recommendation |
| --- | --- | --- | --- | --- | --- |
| `native-evidence-20261011` | 7 local commits, clean, 140 behind | 100 files, +5664/−94 | 87 missing, 13 conflict/partial, 0 on main | 10 content | **Adopt** as its own integration package |
| `ux-feedback-20261010` | 13 local commits, clean, 192 behind | 66 files, +2769/−102 | 54 missing, 12 conflict/partial | 9 content + 1 add/add | **Adopt AP1/AP2/AP3/AP5, drop AP4** (owner confirms AP4) |
| `functions-sites-20261009` | uncommitted on `e8f53a85`, 297 behind | 91 files, +5643/−92 | 61 missing, 30 conflict/partial | 23 content | **Ask user** |
| `import-lifecycle-20261011` | committed part on main; uncommitted residue | 6 files, +194/−3 | 5 missing, handover only conflicts | 0 (handover auto-merges) | **Ask user** (lean: archive and restart fresh) |
| `audit-findings-20261011` | HEAD on main; uncommitted residue | 13 files, +757/−27 | 7 on main, 6 superseded by main | 4 add/add + 1 content | **Drop** |
| `native-import-diagnostics-20261011` | HEAD on main; uncommitted residue | 14 files, +323/−4 | 10 on main, 4 present on main in rewritten form | 1 add/add + 3 content | **Drop** |

## Owner decisions (2026-10-12)

Recorded the same day; do not reopen without a new decision.

| Package | Decision | Carried out |
| --- | --- | --- |
| native-evidence | Adopt (unchanged recommendation) | Waits for the web lock |
| ux-feedback | Adopt AP1/2/3/5, **drop AP4** | Ported 2026-10-13 ([evidence](../evidence/ux-feedback-port-2026-10-13.json)): AP2/3/5 whole, AP1 without what main already had (archive, Dashboard links, row timestamp); ADR-0122; AP4 left out |
| functions-sites | **Keep as WIP** | Whole residue committed as `wip(functions)` `1154b72f` on local branch `feature/functions-sites-20261009` (91 files, each byte-checked against the backup; unreviewed, unpushed). Worktree removed |
| import-lifecycle | **Archive** | Preflight residue kept as a patch in the local prune backup. Worktree and branch removed |
| audit-findings, native-import-diagnostics | **Drop** | Worktrees and branches removed after backup |

The local branches of all 24 retired worktrees were deleted afterwards. Each SHA
was checked as an ancestor of `origin/main` and deleted with an expected-SHA guard.

## native-evidence-20261011

**Delivers.** Seven locally committed slices on the import-lifecycle commit
`4e0e874d`:

- `0cfd6821`: durable historical native import reports,
  ADR-0113
  (`docs/adr/0113-durable-native-import-evidence.md` on the unpushed branch);
- `1eef3c75`: CLI original-report capture;
- `00143966`: retained source-report catalogue;
- `71500757`: inert explicit download of retained payloads;
- `f0bfc775`: native-export privacy consent in the File menu;
- `3376dd21`: read-only modeled project metadata;
- `b76ee907`: read-only recorded device state in the Properties Inspector.

Together these address two open rows of the
[import follow-up matrix](2026-10-11-import-application-followups.md#remaining-functional-work--not-completion-claims):
"Full historical import diagnostics" and the read-only half of "Stored
BusAccess/IP configuration". The diff does not touch `knx-store`, `knx-core`
or `knx-model`, so there is no native schema bump.

**Already on main.** None of the 100 paths. The four new contracts
(`modeled-project-metadata`, `native-export-privacy`,
`project-payload-catalogue`, `recorded-device-state`),
`crates/knx-app/src/import_evidence/source_reports.rs` and ADR-0113 are absent
from main. ADR number 0113 is still free on main, so it does not collide.

**Conflict map (10, all content).**
- Web: `apps/knx-web/src/Dashboard.tsx`, `DiagnosticsCompanion.test.tsx`,
  `messages/de.ts`, `messages/en.ts`.
- Rust: `crates/knx-app/src/lib.rs`.
- Docs: `docs/ARCHITECTURE.md`, `IMPLEMENTATION_STATUS.md`, `IMPORT_EXPORT.md`,
  `KNOWN_LIMITATIONS.md`, `status/2026-10-11-import-application-followups.md`.

`App.tsx`, `Dashboard.test.tsx` and the handover do not apply cleanly but
auto-merge.

**Acceptance.** Every slice has its own frozen local chain; the final one has
1944 frozen inputs:

| Check | Result |
| --- | --- |
| Rust | 1315/0, 100 ignored |
| Web | 2737/0 in 183 files |
| Built browser cases | 12/0 |
| Real-native cases | 12/0 |
| Quality stages | 16, all green |

Self-review only. The branch's own handover says the
preflight-owner reconciliation was read-only and that foreign code was not
accepted. There is no fresh-main integration or publication.

**Recommendation: adopt.** The work is complete, accepted and fills recorded
gaps. Integrate it as one package:
1. Merge fresh main.
2. Resolve the 10 conflicts additively; main's newer Dashboard (project-status
   cards, import findings) wins on layout.
3. Run one combined full gate.

It touches `apps/knx-web`, so it needs the web lock. Add the missing ADR index
row for 0113 when it lands.

## ux-feedback-20261010

**Delivers.** Session 1 UX feedback, AP1–AP5
(local acceptance in `docs/status/2026-10-11-ux-feedback-local-acceptance.md` on the unpushed branch):

- **AP1**: error/warning toasts link to diagnostics and a 100-entry error archive;
- **AP2**: Flow camera transitions, plus PNG and 1–30 s GIF image export with
  consent;
- **AP3/KL-43**: per-category motion preferences (Toasts, Achievements, Flow)
  that can only reduce motion;
- **AP4**: project-inventory cards on the overview;
- **AP5**: four additive achievement milestones and three optional error
  wrappers.

**Already on main.** None of the 66 paths as-is:

- Main has no Flow image export and no per-category motion setting.
- Achievements exist on main, but without the four new milestones.
- AP4 overlaps main's newer **Project status** dashboard (three labelled cards,
  delivered by the dialogs package on 2026-10-12). That part is effectively
  superseded.

**ADR collision.** The branch adds `docs/adr/0112-category-motion-preferences.md`.
Main's 0112 is already `native-instruction-overlays`, and 0113 is reserved by
native-evidence. On adoption, renumber it to the next free number from a fresh
`git ls-tree origin/main docs/adr/` and rewrite every reference.

**Conflict map (10).**
- add/add: `apps/knx-web/src/Dashboard.test.tsx`.
- content: `apps/knx-web/e2e/archive-size.e2e.ts`, `App.tsx`, `Dashboard.tsx`,
  `LogPanel.tsx`, `LogPanel.test.tsx`, `SettingsPanel.tsx`, `Toast.tsx`,
  `toast.ts`, `useToasts.test.tsx`.

`DiagnosticsCompanion.test.tsx` and the handover auto-merge.

**Acceptance.** Admission 3 on `77b9b66f` ran 11 phases, all green:

| Check | Result |
| --- | --- |
| Web Vitest | 2674/0 in 173 files |
| Browser cases | 52 |
| Typecheck and builds | green |

Self-review only. The branch notes that coordinated Session 2 integration,
serial status/ledger reconciliation and combined gates are still open.

**Recommendation: adopt AP1, AP2, AP3 and AP5 and drop AP4.** The owner should
confirm the AP4 drop. Adoption is a web-lock package:
1. Rebase onto fresh main.
2. Re-resolve the Dashboard/LogPanel/Toast areas against main's dialogs and
   project-status work.
3. Renumber the ADR and gate the whole web suite again.

The old acceptance does not carry over to the rebased tree.

## functions-sites-20261009

**Delivers.** An uncommitted implementation of KNX Functions (`MODEL-07`) on
the history base `e8f53a85`. It includes:

- ADR-0099 (`docs/adr/0099-project-functions.md`; 0099 is still free on main);
- a native schema migration with `migration_recovery.rs` and
  `tests/function_schema.rs`;
- parser, diff and report support;
- web UI.

Native Sites (`MODEL-05`) was planned as a later step and not started.

**Already on main.** None of the 91 paths. Both ledger rows are still `LATER`.

**Blocking facts.**
- The residue sets `CURRENT_SCHEMA_VERSION = 12` (base 11). Main is now at
  **14**, with other content for 12–14. The migration must be renumbered and
  rebuilt on top of 14, together with its history-upgrade path. It cannot be
  merged as it stands.
- The tree is 297 commits behind main.
- The last acceptance (attempt 4) failed. Its first 20 stages passed, including
  full Rust, Web 2549/0 in 158 files, intercepted browser and the registered
  private corpus. The production workflow then failed because the comparison
  panel was present but hidden.

**Conflict map (23, all content).**
- Rust: `apps/knx-mcp/tests/tools.rs`, `apps/knx-server/src/lib.rs`,
  `apps/knx-server/tests/http_project_history.rs`,
  `crates/knx-core/src/{command,project}.rs`, `crates/knx-diff/src/diff.rs`,
  `crates/knx-store/src/{migration,project}.rs`,
  `crates/knx-store/src/project_history/snapshot.rs`,
  `crates/knx-store/tests/project_history.rs`.
- Web: `DiagnosticsCompanion.test.tsx`, `Inspector.tsx`,
  `bindings/ProjectTree.ts`, `messages/{de,en}.ts`.
- Docs: `ARCHITECTURE`, `COMPATIBILITY`, `DATA_MODEL`,
  `IMPLEMENTATION_STATUS`, `IMPORT_EXPORT`, `KNOWN_LIMITATIONS`, `ROADMAP`,
  `manual/implementation-status`.

**Recommendation: ask the user.** It is the only Functions implementation, but
it is uncommitted, unaccepted and built on a schema number main has since used
for other content. The options are:

- **(a) Preserve and park.** Commit it as an unreviewed `wip(...)` commit on its
  own branch, or keep the archived patch, then remove the worktree. A future
  MODEL-07 goal restarts from ADR-0099 on schema 15.
- **(b) Resume now** as a scoped package that rebases the migration onto 14.
- **(c) Drop it.**

Lean: (a).

## import-lifecycle-20261011

**Delivers.** The committed part, `4e0e874d` (large native history images,
native diagnostic availability), is already on main. It went through the
tested merge `481f2cd9` and the publication receipt `e27aca52`.

The uncommitted residue is the unfinished **initial native-write admission
preflight**:

| File | Change |
| --- | --- |
| `apps/knx-server/src/domain.rs` | +25 |
| `crates/knx-app/src/project_history.rs` | +20/−3 |
| `crates/knx-projection/src/lib.rs` | +15 |
| `crates/knx-app/tests/native_write_preflight.rs` | new |
| `apps/knx-server/tests/http_native_write_preflight.rs` | new |

It also carries a handover entry.

**Already on main.** None of the preflight. `InitialNativeAdmission` and the
`native_write_preflight` tests exist neither on main nor in native-evidence. The
follow-up matrix still lists "Early import write-capability preflight" as open.

**Conflict map.** Only the handover. The five code files apply cleanly to main.

**Acceptance.** None. The worktree's last entry (2026-10-11 13:34) records a
concurrent-writer collision that produced duplicate definitions. The last HTTP
attempt was refused at compilation. The definitions are unique again now, but
nothing has been compiled or gated since.

**Recommendation: ask the user.** Lean: keep the archived patch as a draft,
remove the worktree, and implement the preflight as a fresh package from
current main under its own acceptance.

## audit-findings-20261011

**Delivers.** Its HEAD `420d0242` (download-layout publication receipt) is on
main. The uncommitted residue is an **earlier candidate** of the
large-history-image work: ADR-0111, `contracts/large-history-images.md`,
`project_history/image_codec.rs`, `tests/http_large_project_history.rs`, plus
store, server and doc edits.

**Already on main.** All 13 files exist on main:

- 7 reverse-apply, so they are identical in effect.
- The other 6 are later revisions on main (since `4e0e874d`). Main adds the
  512-MiB cumulative decoded budget, prefix-only size accounting, schema
  12/13/current snapshot acceptance and the extended ADR and contract text.
- The residue's remaining differences are older variants of lines that main has
  rewritten. No file or feature exists only in the residue.

**Conflict map (5).**
- add/add: the HTTP test, `image_codec.rs`, ADR-0111 and the contract.
- content: `crates/knx-store/src/project_history.rs`.

**Acceptance.** Superseded. Main's version carries the accepted and published
gates (`481f2cd9`).

**Recommendation: drop.** Remove the worktree. A backup of its residue already
exists (see the prune record in the handover).

## native-import-diagnostics-20261011

**Delivers.** Its HEAD `8b4d249d` is on main. The import-status package was
published as `3d6c9d1f`. The uncommitted residue is the native
import-diagnostics availability notice: `ImportDiagnosticsNotice`, the
projection flag, an HTTP test, an e2e spec and the contract
`native-import-evidence.md`.

**Already on main.** All 14 files exist on main:

- 10 reverse-apply.
- In the 4 conflict/partial files (`Dashboard.tsx`, `Dashboard.test.tsx`,
  `messages/{de,en}.ts`), every added line is present on main except one: the
  JSX mount. Main has it at `Dashboard.tsx:102` as
  `<ImportDiagnosticsNotice available={tree.import_report_available} />`, and
  also mounts the notice in `App.tsx` and `ProjectExplorer.tsx`.

**Conflict map (4).** add/add: `Dashboard.test.tsx`. Content: `Dashboard.tsx`
and `messages/{de,en}.ts`. In all four, main rewrote the area.

**Acceptance.** Superseded by the published import-status and import-lifecycle
gates.

**Recommendation: drop.** Remove the worktree; its residue is backed up.

## Not among the six

These worktrees were kept because their residue goes beyond the handover:

- **`offline-naming-20261011`**: an untracked design draft,
  `docs/contracts/offline-naming-recipe.md`, for numbered **device** naming
  (FUTURE-02). It is unaccepted. Main implements only the literal
  **group-address** naming recipe (`contracts/offline-name-recipe.md`).
- **`data-tooling-stats-20261011`**: a detached HEAD with an unpublished
  `docs/ProjectStats.md` refresh (2131 vs 2077 measured commits). It is stale
  by now, because the stats get regenerated on demand.

**Owner decision (2026-10-12):** remove them if they are no longer needed.
Neither was: both HEADs are ancestors of main, and no process used them. Both
were backed up to the local prune backup and removed, and the branch
`feature/offline-naming-20261011` was deleted.

- **Device-naming draft.** It is not on main and nothing links to it. It survives
  only as `offline-naming-20261011/untracked.tar` in
  `KNXBench.backups/2026-10-12-worktree-prune/`. A future FUTURE-02
  device-naming goal can start from it; it is a design candidate, not an
  accepted contract.
- **Stats refresh.** It was dropped, not published. Main's `docs/ProjectStats.md`
  (10 October) is regenerated on request, and a fresh run supersedes the
  11 October numbers.

`web-params-20261012` is the active web-lock holder (package 3) and was not
inspected.

## Leftover branches and folders (sorted 2026-10-12)

The user asked for these to be sorted as well. All four refs are kept in a
verified local bundle outside the repository before deletion.

| Item | Finding | Action |
| --- | --- | --- |
| `feature/history-20261009` (local) | Ancestor of main | Deleted (`branch -d`) |
| `design/group-address-studies-aca414` (local) | Its 7 tool files are identical on main; main's `docs/design-studies/group-addresses.md` is the branch's file plus a later owner-decision section | Deleted (fully contained) |
| `docs/hygiene-audit-20261011` (local) | Unpublished docs audit `2cda0d37` against main `0bed9e2a`; unique, but its corrections encode facts that have since moved again (model v13, "shell mount pending") | Bundled and deleted; stale claims recorded below instead of replaying old text |
| `test/import-integrity-revalidation-20261010` (remote) | Ancestor of main; no open PR | Deleted with an expected-SHA lease |
| `KNXBench.worktrees/co-publication-20261009`, `co-stats-20261009` | Empty directories | Removed |
| `KNXBench.worktrees/xdg` | A single `mise` migration marker from 2026-10-06, no process using it | Copied to the backup, removed |

Two further remote branches, `fix/docker-import-diagnostics-20261009` and
`fix/import-status-20261011`, are also ancestors of main. They were not in the
request and are left in place.

**Stale claims on main.** The hygiene audit corrected these on its own branch,
but the corrections never reached main. All five were re-checked against current
source and reworded the same day (not copied from the audit). Line numbers below
refer to `9152877c`:

| File:line on `9152877c` | Claim | Why it is stale |
| --- | --- | --- |
| `KNOWN_LIMITATIONS.md:186` (private schema-23 review) | "The current importer does not yet import that project end to end" | Repaired by the [import-integrity integration](2026-10-10-import-integrity-integration.md) |
| `KNOWN_LIMITATIONS.md:199` (selective import) | "Only normalized model v12 is admitted" | Native schema has since moved to 14; the admitted model version must be re-read from source |
| `KNOWN_LIMITATIONS.md:906` (PDB-9) | "the editor does not use them yet (e.g. no slider, no hh:mm:ss duration …)" | Time/slider controls exist ([host contract](../contracts/parameter-editor-host.md)); web package 3 mounts them |
| `DATA_MODEL.md:44` | "Application edit API and isolated editor acceptance remain separate and pending" | Instruction edit API and editor are published and mounted (web package 2) |
| `OPEN_WORK.md:90`, `ROADMAP.md:40` | Audit basis "2026-10-08" | Page has been updated continuously since; the date reads as current |

## Observations

- The ADR index (`docs/adr/README.md`) on main has no rows for 0110–0112. Those
  files exist; the tracks that own them should add the rows. Not changed here.
- Both committed packages and every adoption touch `apps/knx-web`. Integrate
  them one at a time under the web lock, after the current holder releases it.
