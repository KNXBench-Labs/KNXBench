# Alpha 15 release preparation

Owner request: release Alpha 15. The owner explicitly retained the rule of one
complete gate on the combined release candidate, then packaging and public
artifact smokes. Existing WP reviews and mutation suites are not replayed.

## Preparation observed

- Starting source: `21b9d46f`, fetched `origin/main`; Alpha 15 tag absent.
- Four application manifests, web package and both package-lock version fields
  advanced to `0.1.0-alpha.15`; Cargo.lock changes only the four application
  versions. No dependency update.
- Native/model schema remains 16; dedicated product database is v26 (Alpha 14
  used v24). Older product-database readers refuse v26. Original predecessor
  project-reader compatibility is not asserted.
- Isolated release worktree; the selected checkout's two unrelated research
  files remain untouched. Parked signal-race and manufacturer research branches
  are not silently adopted.
- Private pinned product-corpus copy prepared: 111 files, one later wrapper
  excluded, 115 expected package instances; copied bytes and original hashes
  checked. Matrix output is on a different filesystem from the corpus.
- Release wrapper delegates the full package scope to the maintained
  `tools/run_package_gate.py --full`, adding only release-only checks: explicit
  additional browser configs, native evidence and all three edit/reopen specs,
  opt-in product-corpus modules and matrix, plus website build and versions.
- The wrapper finalizer passed nine synthetic positive/rejection controls.
  This is verifier evidence, not application test acceptance.

## Review of the documentation pass

The scope note and reader routes were prepared by a worker and reviewed here
before the commit. Verified against source: the CLI flags
`knx device byte-order-probe --preview --system2-source`; `open_read_only`
copies an older product database into memory and migrates only the copy; none
of the cited feature commits is an ancestor of `v0.1.0-alpha.14`. Removed
"acceptance/publication pending" wording from eight reader-facing places,
because the tag freezes these files; the release page stays the authority for
downloads. Historical entries that use "pending" for other work are unchanged.

## Signal-test triage

A read-only triage found the parked signal-race fix proves a real early-SIGTERM
window, but its own gate still timed out, so it does not explain the flake and
stays parked. Not a release blocker. The isolated tooling preflight passed
317 tests (1 skipped) on the candidate inputs.

## Publication completed

Immutable tag `v0.1.0-alpha.15` points to `c3d5b35f`; main-first merge
`09a9d6ce` carries exactly the accepted product sources (handover differs).
Full gate 31/31 in 49.6 minutes, package 10 stages / web 68 stages, inputs
frozen. Public tag workflows AppImage `39303949954` and Docker `39303949950`
passed including publication. AppImage dry run `39300880522` supersedes failed
`39293684573` on the same source; Docker dry run `39293688659` passed. The
missing optional Rolldown binding's original cause is not established.

Public downloads/checksums/provider digests, MCP/server/OCI identities,
authenticated isolated save/reopen, native-10 → 16 migration (230 projected
names), extracted unchanged AppRun under Broadway, and all 37 served site
files passed. The external verifier initially rejected expected SIGTERM wait
143; the retained continuation proves signal semantics and completes the
remaining checks without replaying green product checks. Native-portal
warnings remain disclosed. Owner container ID/image/start, selected HEAD/index
and unrelated research-file hashes unchanged. No bus operation or deployment.

See `docs/releases/alpha15-acceptance.md` and
`docs/evidence/release-alpha15-2026-10-19.json`. The private evidence archive
contains 609 files plus manifest, all 610 members hash-verified, excluding bulk
builds/dependencies/downloads/corpus copies/credentials. Release helper guidance
now distinguishes expected intentional TERM from failed startup.

## Closure

Release freeze: released by claude-release-alpha15.
Alpha 16 follow-up: fail clearly after frontend dependency installation if a
required native binding is missing; no retroactive cause claim or tag move.
Candidate worktree, release branch (local/remote), task scratch, temporary
artifact copies and pulled version-image copies retired. The compact private
archive stays durable; every member and archive digest verified before deletion.
The selected dirty checkout and its unrelated research files remain in place.
