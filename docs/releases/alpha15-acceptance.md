# Alpha 15 release acceptance — 19 October 2026

Immutable source: `c3d5b35f0f482dc64bebd07746a834e5b676215b`, tagged
`v0.1.0-alpha.15` from `release/alpha15-20261019`. Source cutoff before version
and documentation preparation: `21b9d46f`. The owner requested publication;
the full gate and packaging checks were its acceptance conditions. This
result-only record does not change the accepted product or move its tag.

## Evidence and review boundary

One candidate, one complete local release gate: **31/31 ordered stages green**
in **49.6 minutes**. The maintained package gate ran with `--full` (10 stages),
including the **68-stage web gate**. HEAD, consumed-input fingerprints and the
clean product tree stayed frozen. The real user-data guard recorded no touched
or unchecked user directories. Native edits, additional browser configurations
and the opt-in product corpus were explicit release extras.

Scope-note review was in-session self-review, not a new independent approval;
individual work-package reviews and mutation suites were not replayed. The
wrapper's finalizer separately passed nine synthetic success/rejection controls.
These controls are verifier evidence, not application test counts.

## Measured results

| Check | Result |
|---|---|
| Versions, diff-check, fmt, strict Clippy, repository checks | Passed |
| Component tests (Vitest) | 4,134 passed |
| Fixture Chromium | 859 passed: 794 parallel + 65 exclusive |
| Production browser configurations | 506 passed across 16 configurations |
| Additional browser configurations | 42 passed across 8 configurations: 37 ordinary cases + 5 diagnostic-simulator cases |
| Rust workspace | 5,053 passed; 0 failed; 215 ordinarily ignored; 54 test binaries |
| Generated bindings | 59 tests in 2 binaries; no drift |
| Conventional private corpus | 155/155 in 9 targets |
| Opt-in product corpus | dynamic tree 6, legacy names 1, parameter views 1, program code 5, standalone 3 |
| Pinned product matrix | 11 passed, covering 115 instances |
| Native evidence / rename / Functions / naming recipe | 12 / 4 / 4 / 4 passed |
| Website / Story tests | 29 / 76 passed |
| Approved release-mode website build | Passed, 39 inventoried files |

Ordinarily ignored tests are not counted as passing; the explicit corpus scopes
above have their own acceptance. The matrix uses byte-checked copies of its
pinned private scopes and excludes the later wrapper outside that composition.
No claim is made that every ignored test ran.

## Retained non-green attempts

- The early KL-catalogue invocation used an inappropriate isolated Python
  import mode and ran no tests. Its original harness-error log is retained;
  the proper discovery command passed 14/14. No product change was needed.
- The first AppImage dry run (`39293684573`) failed because its installed
  dependencies lacked the optional native Rolldown binding (62 packages rather
  than the successful run's 63). Why npm omitted it is not established.
  The failed log remains a failure, not a green gate stage. The second run
  (`39300880522`) passed on the exact same source SHA.
- A pre-publication receipt validator rejected before mutation because it
  incorrectly assumed every receipt had `planCompleted`. It was corrected to
  validate each actual receipt contract; source acceptance was not replayed.
- External verification initially stopped after successful artifact/OCI checks:
  its assertion incorrectly required Broadway's teardown status to be zero
  after deliberately sending SIGTERM. The actual Bash `wait` status was 143.
  A deterministic signal control and the recorded startup/teardown evidence
  establish the expected result. A separately recorded continuation accepted
  that evidence and completed the remaining checks without rerunning green
  product checks. Both receipts and the original error are retained.

The parked early-SIGTERM tool-test fix was not included: it concerns a different
race and was not demonstrated to explain the reported flake. The frozen
candidate's complete gate itself passed.

## Packaging and publication

Dry runs on the exact SHA: AppImage `39300880522` and Docker `39293688659`, both
successful; all publication jobs skipped. Docker built `linux/amd64` and
`linux/arm64`. The tag did not exist during these dry runs.

Tag-push workflows AppImage `39303949954` and Docker `39303949950` succeeded,
including their actual publication jobs, on the immutable candidate. The
GitHub pre-release body was read back and its links point to validated tag
paths. Public artifacts were downloaded afresh and their SHA256SUMS, sizes and
provider digests verified. MCP/server build suffixes and the OCI full revision
match the tag. The Docker version and `latest` index digests match remotely.

The [publication receipt](../evidence/release-alpha15-2026-10-19.json) records
artifact hashes, workflow identities and exact verification scope:

- Isolated authenticated Docker create/save/switch/reopen with no host listener.
- Byte-exact public schema-10 demo opened, explicitly saved as schema 16 and
  reopened; all 230 projected names preserved, original fixture unchanged.
  The history endpoint answered before and after; individual undo/redo
  entries were not compared.
- Published AppImage's unchanged extracted AppRun started under Broadway in
  loopback-only isolation: embedded API reports Alpha 15, frontend HTTP 200,
  no EGL_BAD/abort lines, no forbidden bundled `libwayland-client`. Expected
  restricted-namespace desktop-portal warnings remain in the private log.
  This does not prove FUSE launch, Mesa EGL, native visual or screen-reader
  behaviour.
- All 37 served website files match the accepted build's bytes, including
  media and approved Story output. The two excluded files (`CNAME`,
  `.nojekyll`) are hosting controls. No website deployment was performed.
- Owner container ID, immutable image ID and StartedAt remained identical;
  selected checkout HEAD/index and unrelated research-file hashes unchanged.

## By the numbers and remaining boundaries

One candidate; one complete local gate; two AppImage dry runs (one retained
failure); one Docker dry run; zero accidental publications. There is no silent
upgrade of the owner's running installation. The compact private archive has
609 evidence files plus its manifest; all 610 members were read back and
hash-verified before bulk retirement. Its digest is in the publication receipt.

Native schema stays **16**; product database moves **24 → 26**. Back up before
upgrading; earlier product-database readers refuse v26. Opening new project
files in a previous release was not verified. No live KNX/device operation,
new ETS compatibility, Secure behaviour or expanded programming admission is
claimed. A post-install native-binding check is an Alpha 16 follow-up, not a
retroactive claim that the first packaging failure's cause was established.
