# Repository closeout before the GitHub move (2026-10-07)

## User authorization and scope

The user requested committing/pushing all retained local work and cleaning the
inactive worktrees. This authorizes preservation and checkout cleanup, not
merging old work into main, changing repository visibility/ownership, deploying,
contacting KNX hardware or replacing the released alpha tag/assets.

The repository was verified private (`KNXBench-Labs/KNXBench`). Deliberately
ignored private data, local settings, original archives and runtime databases in
the root were neither staged nor uploaded.

## Preserved source and Git state

| Retained worktree | Published branch | Source snapshot | Scope |
| --- | --- | --- | --- |
| KL-158 | `fix/kl-158` | `318955d189fb52ba08a35d0781f9519e069592bb` | 16 paths including its implementation receipt; source preservation, not integration |
| U21 predecessor | `u21-fix` | `e28decb3a0892a2e97396b42c28aeec6a27da292` | 9 paths; explicitly marked historical WIP |

Both commits use `github@knxbench.com` as author and committer, without a
Co-Authored-By trailer. All **28 pre-existing local branch tips** were compared
with live remote refs; **23 refs** were created/updated in one atomic push.
Every requested remote ref was then read back at its exact intended hash.
Existing release tags were unchanged. No old branch was force-pushed or
silently rebased into current main.

Eight distinct previously local-only commits (59 distinct outgoing blobs)
and the working snapshots received a bounded sensitive-pattern check for
private keys, GitHub tokens and AWS access keys, with no findings. This is
not a full privacy, license or public-release audit.

The DPT precheck and its original local note are preserved as historical
records. The precheck now explicitly points to the subsequent document-wide
audit rather than presenting its earlier implementation observations as current.

## Real verification in this closeout

- KL-158 launcher unittest suite: **19 passed**.
- U21 animator/dynamics Vitest suites: **27 passed in 2 files**.
- U21 flow-study TypeScript check: exit **0**.
- The current `xtask` was rebuilt; explicit-root layering, headers, anchors,
  ledger and corpus-source lint passed against main and the KL-158 snapshot.
- Both staged source snapshots passed whitespace checks; committed paths and
  bytes were checked against their retained working inputs before removal.
- The final docs-only publication repeats the five explicit-root gates on its
  actual checkout and staged whitespace checks.

This is **self-review and focused verification**, not an independent review,
full Rust/UI/corpus regating, native acceptance, a new AppImage build or a
hardware test. The historical KL-158 artifact receipt is retained as historical
evidence; it is not relabelled as an artifact rebuilt by this task.

## Durable backup and cleanup

Backup: `/mnt/daten-i/Sourcecode/KNXBench.backups/2026-10-07-pre-transfer-161629/`.

- Tracked/index patches, file manifests and byte-verified archives from all
  three original checkouts; source files and relevant ignored evidence notes
  retained, without copying root private data into Git.
- The development AppImage was copied and checksum-verified before its cache
  was removed.
- `all-local-heads.bundle` contains all then-local branches/tags and passed
  `git bundle verify`.
- No process cwd remained in either retired worktree; both were clean and
  their exact branch tips were verified remotely before normal
  `git worktree remove` (no force) removed them.
- Original `kl-158` and `u21-fix` worktree paths are absent; registry pruned.
  The root checkout and all local branches remain. Remote branches are retained
  intentionally, because they are the requested preservation copies.

Final receipt publication uses an isolated docs checkout. Before synchronizing
the root, all owned root document copies are compared against the published
commit; only the proven duplicate edits are restored/removed for a fast-forward.
The temporary docs checkout is then retired. Final live-ref equality, clean root,
only-root worktree registry and zero locally unpushed branch commits are checked
after publication; the final commit hash is reported from Git, not embedded
self-referentially in this receipt.

## Late stash preservation

The final check also found three older commissioning stashes. Their working,
index and (where present) untracked-parent trees were inspected for sensitive
file types and key/token patterns; no findings. They are preserved as their
original merge commits, including every parent, rather than flattened and
losing the captured untracked files:

| Original stash | Published preservation branch | Original object |
| --- | --- | --- |
| `stash@{0}` | `archive/2026-10-07/stash-0-cb18aaee` | `cb18aaeedf2f37e87f790018999ff17fe9864f81` |
| `stash@{1}` | `archive/2026-10-07/stash-1-66e82007` | `66e82007d4b498d547b7269049fc178ff325b17d` |
| `stash@{2}` | `archive/2026-10-07/stash-2-9d14c93c` | `9d14c93ced6fb6018f3dc8aeebf520cd1caec11d` |

All three remote refs were read back at those exact hashes. The original local
stash list remains unchanged: no pop, apply or clear into the current checkout.
A verified `all-local-heads-with-stashes.bundle` and a stash inventory are in the
durable backup. This increases the retained local branch count from 28 to 31.
The archived commissioning WIP has **not** been integrated or feature-regated;
these branches preserve its original source and history only. To recover a
stash elsewhere, use its preservation ref as a stash object, so Git also knows
about any captured index/untracked-parent state.

## Remaining boundaries

`fix/kl-158`, `u21-fix` and the older parked branches are preserved, not declared
current-main features. Integration of any such branch needs its own deliberate
review and gates. The original alpha tag/assets and private visibility remain
unchanged; the proposed account rename/repository transfer is separate work.
