# Version identity and dirty builds

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-065)

## Development stamps versus guarded releases (20 October 2026 reconciliation)

The final AR13 update in the retained record already introduced a separate
release mode. Development stamping still has no dirty-tree suffix. Setting
`KNX_REQUIRE_CLEAN_TREE=1` makes the shared build helper re-run and refuse
when Git cannot confirm the workspace/HEAD, reports modified or non-ignored
untracked paths, or an explicit `KNX_BUILD_SHA` disagrees with HEAD. Ignored
files are not a complete build-input attestation; the version string alone
does not prove the guarded mode was enabled.

Evidence: [build-stamp implementation and tests](../../crates/knx-build-stamp/src/lib.rs),
[version contract](../adr/0018-program-versions-and-file-headers.md). No new
build or published-artifact verification is claimed by this documentation fix.

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 65. `--version` names a commit, never a working tree

`knx --version` and `knx-server --version` print
`<name> 0.1.0-alpha.1+g<short-sha>` ([ADR-0018 §2](../adr/0018-program-versions-and-file-headers.md)).
The sha is the `HEAD` commit at the time cargo last ran that binary's
`build.rs`. It says nothing about whether the tree was clean: uncommitted
edits, staged or not, are invisible, and there is no `.dirty` marker,
because cargo re-runs a build script for files it has been told to
watch and has no notion of "anything changed anywhere". A build from a
modified tree therefore reports the last commit's sha with a straight
face. Cost: someone bisecting from a `--version` string is looking at
that commit *plus whatever was uncommitted at build time*. Lifted if: a
build ever runs `git status --porcelain` and accepts that the marker can
then be stale in the other direction (a `dirty` stamp that outlives the
edits, until the script next happens to re-run) — a trade this project
has not taken.

Two ways the sha could have been *wrong* rather than merely incomplete
were found in review (2026-09-12) and are handled. After `git pack-refs`
— routine under `git gc --auto` — the loose branch file disappears, and a
watch on it alone went stale: every later commit was invisible to
`--version` until `HEAD` itself moved. The `HEAD` reflog is watched too
now; it is appended on every commit, checkout and reset, packed or not.
And a source tree unpacked inside an unrelated repository was stamped
with *that* repository's commit; `git rev-parse --show-toplevel` must now
equal the workspace root, canonicalized, or nothing is emitted. Without
git at all, or with `.git` excluded (the Docker build), the metadata is
simply absent — `knx 0.1.0-alpha.1` — unless `KNX_BUILD_SHA` is passed
in. Absence is the intended failure direction; a false number is the one
this section, and the ADR, exist to rule out.

**Update, 2026-10-04 (AR13) — lifted for release builds.** Both build
scripts now call `crates/knx-build-stamp`. Development builds stamp exactly
as described above. With `KNX_REQUIRE_CLEAN_TREE=1` the build script re-runs
on every build (it watches a path that never exists) and fails unless git
confirms this workspace, resolves `HEAD`, reports no modified or untracked
non-ignored path and any explicit `KNX_BUILD_SHA` names that commit. Measured
on a real checkout: clean → `+g345d0bc5`; one edit afterwards → the next
build refuses and names the path; without the always-re-run watch the same
edit was silently stamped with the clean commit. So a release artifact built
this way names only a commit it was built from; a development build still
names a commit, never a tree.
<!-- END retained catalogue record -->
