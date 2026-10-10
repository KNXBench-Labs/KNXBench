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

Pending: combined-source acceptance and exact-source packaging dry runs, explicit
immutable tag/prerelease, independent downloaded-asset/anonymous Docker checks,
final documentation and owned cleanup. Later parallel work is outside this
frozen release snapshot. No bus action, subagent or quota checks.
