# Alpha 8 release preparation

## Initial candidate

Owner requested Alpha 8 including Docker on 2026-10-10. The shared root remains
stale/dirty and is not synchronized. Own candidate `7b6389f5` was based on
published `7d39628f`, with aligned application versions, unchanged external
dependencies and handwritten notes. Self-review only; no live container swap.

Its 26 frozen whole-source stages passed: Rust3909/0/190 ignored,
frontend2611/0/164 files, Chromium244/0/0 skipped, conventional corpus143/143
in31 targets, explicit original/native/instance/RefId/reconciliation checks,
builds/types/strict Clippy/format/five repository gates/docs/54 Python tests.
AppImage dryrun38079894738 and native Docker dryrun38079896624 passed.

Additional download/AP1 tests2/0 and reference containers3/0 passed. The first
matrix run refused its same-filesystem output before observation; its unchanged
separate-filesystem retry passed1/0. No safety guard or corpus pin was modified.
Raw private evidence stays ignored/restricted; no individual input identities,
values, paths or digests are published here.

## Reconciliation

The final publication guard detected `fd6b1cff` before any main/tag/release write.
That upstream package adds bounded opaque-payload report enrichment, independent
same-member diagnostics and explicit unverified role candidates (ADR0109).
The combined candidate includes it; the complete authoritative upstream journal
and the initial owned checkpoint are preserved. Initial acceptance remains
historical, not relabelled as acceptance of changed application inputs.

At reconciliation, combined-source acceptance, exact-source packaging dry runs,
immutable tag/prerelease, external checks and documentation/cleanup were pending.
That paragraph describes the historical checkpoint, not current release state.

## Verified publication — 2026-10-10 23:28 CEST

Final sourcefdddad8f passed26 main+6 supplementary stages: Rust3912/0/190,
Web2611/0/164 files, Chromium244/0/0 skipped, corpus146/0 and explicit
original/native/instance/RefId/source/reference checks. Both dryruns and both tag
workflows passed. Immutable annotatedv0.1.0-alpha.8, exact source-main/body/tag
readbacks and all three downloaded asset sizes/digests/checksums are verified.
MCP executes matching build; AppImage extracts; anonymous multiarch Docker
version/latest and full OCI labels match. Published HTTPS/auth/save/reopen and
isolated orderly-stop0 pass; existing user container remains unchanged/running.
Public receipt: docs/evidence/release-alpha8-2026-10-10.json.

At that checkpoint, documentation-only gates/main publication and cleanup remained.
No new interactive desktop/Orca/Pi/ETS/bus or memory certification or live deployment.
Native/model12 backups remain mandatory. Self-review only; private proof stays
restricted/ignored. No subagent/quota checks. The tag must not move.

## Delivery closure — 2026-10-10 23:36 CEST

Documentation publicatione7fc8def has exact local/tracking/live readback, unchanged
runtime and exact public receipt bytes. All five fresh-target repository gates,
233 Markdown/3006 local targets/33 chapters/0 errors, privacy and whitespace pass.
Root HEAD/index/protected foreign files and inherited journal are unchanged.
Owned source checkout/local+remote branch, a8/a82 build/download/runtime scratch
and verification image are removed. Private receipts/logs/scripts and matrix
outputs are archived and verified; user container identity/image/start time
remain unchanged and running. Only this final docs checkout/branch remains for
its own closing commit, then immediate verified retirement. No functional work
remains. Canonical final cleanup/root checkpoint records that retirement.
