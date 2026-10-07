# Remote-branch consolidation (2026-10-07)

## Authorization and scope

The user requested deleting remote branches and merging useful work first.
Keep the default `main`; this is not permission to delete the repository,
change ownership/visibility, publish a release, deploy or contact KNX hardware.
Original main: `e073f0a9d1a8facbbbbe1784345e7ada943363a8`. Repository verified private.

## Inventory and preservation

Live inventory: **35 remote branches**, including main; **25 tips** already
reachable from main. All 35 exact refs/hashes and then-local branches/tags were
saved in a verified bundle before any remote deletion:

`/mnt/daten-i/Sourcecode/KNXBench.backups/2026-10-07-remote-prune-165234/all-branches-before-deletion.bundle`

The sibling archive contains the full remote inventory, branch decisions,
source fingerprints, ordinary-gate logs and the new development AppImage.
This is durable storage, not an expiring agent scratch directory. Local retired
branch and original stash refs remain; deletion does not discard their content.

## Merge decisions

- **Merge `fix/kl-158`** (`318955d189fb`): complete post-alpha AppImage display
  policy/build hook with tests, primary-source/pinned-plugin contract and
  historical native receipt. A real new merged AppImage was built.
- Four conflicts were documentation/handover only. Independent additions from
  both sides were retained; current DPT, inference and Flow features were not
  reverted. Source/configuration files merged without conflict.
- Older commissioning branches are not useful new functionality: current
  service-control/download adapters and several tests match their snapshots;
  current path admission additionally protects missing/alias destinations and
  current download records retain `record_never_connected`. Replaying older
  variants is not a worthwhile consolidation. One old prepublication evidence
  JSON is retained in the bundle rather than treated as current acceptance.
- Old U21 Flow WIP predates the delivered readability/window/navigation follow-up;
  do not replace newer main sources with it. Its intermediate measurements stay
  in the bundle. Native-probe WIP remains explicitly unreviewed, not a tested
  product feature. Stashes remain complete graphs with staged/untracked parents.

## Executed verification

- Ordinary Rust workspace: **3430 passed / 0 failed / 178 ignored**, 194 result
  blocks, offline in a mechanically checked loopback-only namespace.
- Workspace all-target Clippy `-D warnings` and fmt: exit **0**.
- Launcher Python tests: **19 passed**; shell syntax check passed.
- Web: **2162 passed**, frontend TypeScript/Vite production build passed.
  Retained stderr includes test-harness warnings, not silently discarded;
  no baseline warning-count comparison is claimed. The process exit is 0.
  Aggregate parser removes ANSI formatting instead of mistaking
  colored output for a missing summary.
- Real Tauri CLI **2.11.4** AppImage build completed; package validator passed.
  SHA-256 `c1cc776963f09117344b66b61832c281a9e7da1584febe0ae9b124938de8d910`. Actual
  `apprun-hooks/linuxdeploy-plugin-gtk.sh` contains the owned policy. Initial
  inspection mistakenly excluded that runtime hook because it shares the
  deploy tool's basename; direct inspection corrected the harness, not source.
- Source/configuration fingerprints remain equal to the accepted candidate.
  Final five explicit-root repository gates and staged whitespace checks are
  repeated after documentation edits.

Self-review only. No new native startup/pixel/e2e/AT/GPU/corpus acceptance:
Xvfb/Weston and the previous extracted display tools are unavailable. The
historical native receipt is not relabelled as a fresh current-image run.
The development AppImage was built from a dirty candidate at the earlier base;
it is not a clean-tree tagged release or a replacement of the alpha.4 asset.

## Deletion protocol

After merge publication/readback, compare all non-main live refs against the
saved inventory again. Use an atomic deletion push with an exact expected-SHA
lease for each of the **34** non-main refs; any concurrent ref change refuses
rather than deleting unseen work. Read back the entire remote branch list,
require only main, prune tracking refs, and keep release tags unchanged.
Final deletion/main/root/worktree state is recorded by a closing receipt after
the operation; this pre-deletion commit does not certify future effects.

## Closing deletion receipt

At 2026-10-07 17:57 CEST, the leased atomic deletion completed and a fresh full
remote-head read returned **only `main`**. Exactly **34 non-main refs** were
deleted; every deleted ref/hash is retained in the durable inventory/bundle.
KL-158 feature merge: `ae30674806b8211d7882e308d6a05f4b7a07c533`.
Published consolidated head before this metadata receipt: `202dca90c19dece6eb14384643c6dbbf82363372`.

A concurrent upstream statistics commit `91ee61f514125207642234a7e3c9ae09fac11dba` changed only
`docs/ProjectStats.md`; it was inspected and merged, not overwritten. Source
fingerprints stayed identical, so ordinary Rust/Web/build evidence carried over
and all five repository gates were repeated. The initial expected-main guard
refused before publication/deletion when that update appeared.

Local retired branches/stashes remain available; this task deletes remote
branches, not local preservation refs. Release tags, the existing downloadable
alpha asset, private visibility and ignored root data are unchanged. Final
metadata publication/readback and owned-worktree cleanup are verified after
this commit; the final main hash is read from Git rather than embedded here.
