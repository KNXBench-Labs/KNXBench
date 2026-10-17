# Git history still carries local machine details

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-185)

<!-- BEGIN retained catalogue record -->

## Current tree clean, history not rewritten (17 October 2026)

Since [ADR-0135](../adr/0135-repository-carries-no-local-machine-details.md)
the tracked files carry no local machine details, and
`tools/check_local_info.py` keeps it that way. Earlier commits are unchanged:
their file contents and seven commit messages (one of them the commit tagged
`v0.1.0-alpha.5`) still name local paths, the host name or the kernel release.
Anyone who clones the repository can read them with `git log -p`.
Removing them needs a history rewrite and a force-push, which changes every
later commit hash and moves release tags; that is an owner decision and has
not been made. The two reorganization receipts name the replaced bytes of
eight redacted records by digest only.
<!-- END retained catalogue record -->

## History rewritten (17 October 2026)

Owner decision: rewrite. `git filter-repo` replaced the details in every file
version and in every commit and tag message of `main`, all nine release tags
and every local branch; the rule categories, counts and old/new tips are in the
[rewrite receipt](../evidence/history-rewrite-2026-10-17.json). The audit
behind it found more categories than the first scrub: the CPU model, the
distribution and desktop name, an older kernel release, dash-joined path keys
and the address of the real KNX gateway; the tree was cleaned of them in the
same rewrite. 2,963 of the 3,216 published commits got new hashes (the 253
earliest predate every hit and kept theirs), none was dropped, and authors,
dates and parent structure are unchanged. Every changed file on
every branch and tag tip equals the same rules applied independently; no
binary object was touched; a scan of all 37,503 objects found no residue.
Old hashes: [`COMMIT_MAP_2026-10-17.txt`](../history/COMMIT_MAP_2026-10-17.txt).
241 pinned GitHub links now point at the rewritten commits (19 of the linked
old file versions contained local details).

**What stays outside this repository's reach.** GitHub keeps the read-only
pull-request refs `refs/pull/3/head` and `refs/pull/24/head`; through them the
pre-rewrite commits (2,399 and 3,140 of them) remain fetchable by hash. The
fork of the repository holds its owner's copy. Removing either needs GitHub
Support or the fork owner; recreating the repository would delete its issues
and pull requests and was not chosen. Release assets and Docker images are
unchanged (no hits); their embedded revisions name old hashes.
