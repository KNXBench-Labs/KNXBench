# Identity rewrite and repository recreation (Claude, 2026-10-07)

User decision (two clarify rounds): no personal identity anywhere in the
repository, its history, artifacts, agent instructions or memory; replace it
with `KNXBench`. All commits, the tag and the stashes become
`KNXBench <github@knxbench.com>`; the full name goes too. The repository is
deleted and recreated under `KNXBench-Labs` (owner login via the Hermes vault,
gh logged in as the owner by the user), alpha.4 is withdrawn and a new
AppImage is published as `v0.1.0-alpha.5`; app identifier
`com.knxbench.knxbench-labs` (user's choice); side branches and stashes are
rewritten too; local backups and Hermes evidence stay unchanged and private.

## Backup before anything destructive

`/mnt/daten-i/Sourcecode/KNXBench.backups/2026-10-07-pre-identity-rewrite/`:
`all-refs.bundle` (`git bundle --all`, verified, includes the three stash
commits as `refs/stash-keep/*`), `refs.txt`, `stash-list.txt`, and the
metadata of an earlier `filter-repo` run that was found in `.git/filter-repo`
(moved aside so the new run is a full run, not a "continuation").
Local and private; it contains the old identity and must never be published.

## Rewrite

Tool: `git-filter-repo` 2.47 (installed with `uv tool install`).
`--replace-text` and `--replace-message` with one ordered rule file, plus
commit and tag callbacks that set every author, committer and tagger.
Rules, in order (described, not quoted, so this log carries no pattern):

1. any address at the personal mail provider's domain → `github@knxbench.com`
   (catches a co-author trailer variant that the first dry run missed);
2. the old reverse-domain app identifier (both historical forms) →
   `com.knxbench.knxbench-labs`;
3. the personal account name, any case → `KNXBench-Labs`;
4. first name + surname, any whitespace including a line break → `KNXBench`;
5. the home-directory path of the local user → `/home/knxbench`;
6. the first name as a whole word (three casings) → `KNXBench`/`knxbench`.

Deliberately no rule for the surname alone: a KNX manufacturer shares it.

Dry runs on a `--mirror` clone in scratch first. The first one found one
leftover (a co-author trailer with a variant address), the rule was widened; the second
one scanned clean. The real run produced the identical `main`
(`8ab30733…`), proving determinism. Object scan (all 23,038 reachable
objects, text and binary separately): 0 hits, and no binary object ever
contained a hit, so no SQLite/PNG/AppImage bytes were touched.

Stashes: `filter-repo` rewrote `refs/stash` and its reflog; all three entries
were checked against the rewritten keep-refs (identical) before those were
deleted. `origin` was re-added (filter-repo removes it on purpose).

## Follow-ups in the same package

- App identifier → ADR-0087, KNOWN_LIMITATIONS §161; local data folder
  copied (`cp -a`, `diff -r` identical, original untouched).
- KNOWN_LIMITATIONS §162 + `docs/history/COMMIT_MAP_2026-10-07.txt`.
- Versions 0.1.0-alpha.5 (CLI, desktop, web, lockfiles).
- Crawler link removed (private repository).
- Hermes SOUL.md, projectstats skill, repository-delivery template and two
  Claude memory files rewritten to the KNXBench identity; repo-local git
  identity set (global git config left alone).

## Pitfall hit

The first workspace gate showed 10 `knx-store` migration failures
(`NotFound`). Cause: the shared `target/` held a `knx_store` test binary built
at 17:07 in the since-deleted worktree `remote-prune-merge-20261007`, with
that worktree's fixture paths baked in through `CARGO_MANIFEST_DIR`; Cargo
still considered it fresh. `-p knx-store` alone passed 25/25. Consequence: the
acceptance gate and the release AppImage are produced in a fresh
`CARGO_TARGET_DIR` from the committed candidate.

## Not done

No public visibility change. Global git config, other Hermes profiles and the
local backups/evidence keep their content.
