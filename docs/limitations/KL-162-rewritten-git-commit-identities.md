# Rewritten Git commit identities

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-162)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 162. Commit hashes cited before 2026-10-07 refer to the rewritten history

**Limitation.** On 2026-10-07 the whole Git history was rewritten to remove a
personal identity from every commit, tag, message and file version (user
decision). Every commit therefore has a new hash. Status logs, dossiers,
ledger rows, evidence files and handovers written before that day cite the
old hashes, which no longer exist in the repository. Commit messages were
rewritten together with the history, so short hashes inside them already
point at the new commits.

**Impact.** To follow an old hash, look it up in
[`docs/history/COMMIT_MAP_2026-10-07.txt`](../history/COMMIT_MAP_2026-10-07.txt)
(old hash → new hash, all 2,261 commits). Abbreviated hashes match on their
prefix. Artifact digests, test counts and file contents cited in those
documents are unaffected. The tag `v0.1.0-alpha.4` and its pre-release were
withdrawn, and the archived release records describe a build that no longer
exists for download.

**Second, partial rewrite (2026-10-08).** Commits made on 2026-10-08 between
10:28 and 13:35 CEST had reintroduced a personal e-mail address (four story
commits) and a home-directory path (two LCARS commits and one handover
version). On the user's decision the history from `c0237937` (old hash) on was
rewritten again; 27 commits got new hashes, the final tree stayed
byte-identical, and the tag `v0.1.0-alpha.5` and everything before 10:28 kept
their hashes. Map: [`docs/history/COMMIT_MAP_2026-10-08.txt`](../history/COMMIT_MAP_2026-10-08.txt).
Hashes of those 27 commits cited in maintained documents and handovers were
updated in place; links to removed documents now point at `138403ed6084`, the
same tree as the old `6a1ba6ae5d54`. A branch created from `main` between
10:28 and 13:35 must be moved onto the new history
(`git rebase --onto <new base> <old base> <branch>`, bases from the map) and
must never be merged back as it is.

**Third rewrite: public launch (2026-10-08).** Before the repository went
public, an audit of every reachable object found two things in the history
that were deleted from the tree long ago but must not be published: the
directory `az-and-sensor-data/` (added 2026-09-02, removed 2026-09-06:
third-party ETS-plugin AutoSave data of an alarm panel and a sensor plugin,
28 manufacturer firmware files, plugin help and compiler files) and
`KV v2.5 - demo.knxproj` (2026-09-06 to 2026-09-10, a third-party demo
project). On the user's decision both paths were removed from the whole
history with `git filter-repo --invert-paths`. 2,279 of 2,311 commits got
new hashes; the 30 commits before 2026-09-02 kept theirs. Two commits
became empty and were dropped: the removal commit `b79a1c40` and
`a25bbad6`, which only added the demo project (both map to forty zeros).
Every commit tree that did not contain one of the two paths is
byte-identical to its predecessor, including the final tree of `main`.
The tag `v0.1.0-alpha.5` was rewritten with its commit (`aca70fd7` →
`a0ff1f55`). The secret, identity and private-data scan found no other hit.
Map: [`docs/history/COMMIT_MAP_2026-10-08-public.txt`](../history/COMMIT_MAP_2026-10-08-public.txt).
Hashes are **not** updated in place this time: to follow any hash cited
before this rewrite, chain the maps (2026-10-07, then 2026-10-08 for the 27
commits it covers, then this one). The repository was recreated under the
same name, so no rewritten-away commit is reachable on GitHub. Every local
branch, stash and worktree created before this rewrite belongs to the old
history: move it with `git rebase --onto` (bases from the map) and never
push or merge it back as it is, because that would republish the removed
files. The story companion's commit links still cite pre-rewrite hashes and
need a remap by its owner before the next edition.
Pinned GitHub links to removed documents were moved with it: `138403ed6084`
is now `bca2d3336b96` and `aa0ff14ff536` is now `a584007fc05a` (242 links in
32 maintained files; frozen story editions and handovers keep the old text).

**Lifted when.** Not applicable; the mapping file is the permanent bridge.
<!-- END retained catalogue record -->

## Fourth rewrite: local machine details (17 October 2026)

2,963 of 3,216 published commits got a new hash again when local machine
details were removed from the whole history ([KL-185](KL-185-git-history-local-machine-details.md),
[ADR-0135](../adr/0135-repository-carries-no-local-machine-details.md)). To follow a
hash cited before this rewrite, chain the earlier maps with
[`COMMIT_MAP_2026-10-17.txt`](../history/COMMIT_MAP_2026-10-17.txt). Commit
messages were rewritten with the history, so short hashes inside them already
point at the new commits; pinned GitHub links were moved, hashes in prose were
not.
