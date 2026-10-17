# ADR 0135: The repository carries no local machine details

Date: 2026-10-17
Status: Accepted
Session: 7

## Context

KNXBench is a public repository. Handover notes, evidence receipts, history
records, test fixtures and commit messages had accumulated details of the
development machine: absolute mount points and checkout or worktree locations,
home and agent-profile directories, the host name and the exact kernel
release. A scan on 2026-10-17 found them in 56 tracked files and in seven
commit messages. None of it describes KNXBench; all of it describes one
machine, and it went stale whenever that machine changed (the checkout moved
disks the same day).

Affected passages sat inside records pinned byte for byte by the two
reorganization receipts
([documentation](../evidence/documentation-reorganization-2026-10-13.json),
[limitations](../evidence/limitations-reorganization-2026-10-13.json)), so the
scrub could not happen without touching those proofs.

## Decision

Tracked files, commit messages, tags and release notes contain no local
machine details. Placeholders replace them: `<sources>` (directory holding
checkouts and worktrees; `<old-sources>` for the location before the disk
move), `<profile>` and `<profile-scratch>` (agent profile data), `~` (a home
directory), `<host>` or "the dev machine". Test fixtures use obvious
placeholder homes such as `/home/alice`.

`tools/check_local_info.py` enforces the rule. It combines generic patterns
(local mount points, non-placeholder home directories, agent profile paths)
with the identifiers of whichever machine runs it, read at runtime (host name,
kernel release, `$HOME`, the checkout's parent directory, plus optional
`KNXBENCH_LOCAL_INFO_DENY` tokens), so the guard never has to spell out the
details it refuses. A line that deliberately shows such a detail as an
example (the guard's own patterns and fixtures) carries the marker
`local-info: allow`, which exempts that line only and stays visible in review.
`tools/check_documentation.py` calls it, which puts it in
CI and in the local web gate; `--message-file` and `--commits` check commit
messages.

Both receipts gain an explicit, per-record `redaction` (`reason`,
`previous_retained_sha256`). A redacted record is pinned to its new bytes,
keeps its offset and anchor checks, and names the replaced bytes by digest. In
the documentation receipt only its byte reconstruction (and therefore the
whole-source digest of its source) is skipped; the limitations receipt never
reconstructed sources, so there the pinned digest simply moves. Every
unredacted record is still verified byte for byte.

## Alternatives considered

* **Leave history records untouched.** Keeps the receipt's byte identity but
  keeps publishing the machine details the rule forbids.
* **Store the original text inside the receipt** so reconstruction still
  works. Re-publishes exactly what was removed.
* **Reconstruct originals from Git history** (`source_revision`). Fails on
  shallow CI clones and on any future history rewrite.
* **Hard-code the known machine's paths in the guard.** Would itself commit
  them, and misses the next machine.

## Consequences

* Agents write placeholders in handover notes and evidence from the start; the
  documentation gate fails otherwise.
* The guard cannot know every past machine. Old identifiers that no longer
  match the running machine are caught only by the generic patterns, by
  `KNXBENCH_LOCAL_INFO_DENY` or by the clone-local `.git/info/local-info-deny`
  (the place for things no machine reports, such as a real gateway address).
* Commit messages are checked only when someone runs the hook or `--commits`;
  CI does not block a push on them.
* The owner chose to rewrite the history the same day (KL-185): every file
  version, commit and tag message, with an audit that also covered the CPU
  model, distribution, an older kernel release, dash-joined path keys and the
  real gateway address. GitHub's read-only pull-request refs and the fork
  still serve the pre-rewrite commits; see
  [KL-185](../limitations/KL-185-git-history-local-machine-details.md).
* Local branches created before the rewrite belong to the old history. Their
  rewritten copies are kept as `refs/rewritten/*`; an old-history branch is
  moved with `git rebase --onto` (bases from the commit map) and never merged
  or pushed as it is.
