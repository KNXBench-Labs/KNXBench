# P4b relative source retention and absolute-image integrity

Date: 2026-10-16
Agent: codex
Base: e9b8cb2e2c666fb5572cadb1f944b73f22789af3

## Observed baseline

The retained network-isolated Release experiment completed successfully against
three selected real applications in one package. It measured existing production
import/evaluation/code/image behaviour: 186 active parameter and12object refs,
54non-hiding NoBranchMatched diagnostics, zero absolute segments and three
ComObjectTable-no-CodeSegment refusals. Original packages, database and complete
runtime input manifests remained unchanged. This is not image/download success.
The temporary probe was saved privately and removed before implementation gates.

## Implemented locally

- Separate source-only RelativeSegment view in ProductDB: ID, explicit bounded
  offset/size/LSM metadata, Data/Mask, all other attributes; no absolute address.
- Shared base64/length decoder, no zero-buffer synthesis, explicit unsupported
  children/duplicate blobs/duplicate cross-kind IDs and malformed-field refusals.
- Legacy projection explicitly has no relative source records.
- Both existing absolute-image entry points reject any relative source record,
  including mixed input that could otherwise silently omit that source data.
- Ten source-reader tests observed compile RED (missing relative_segments), then
  GREEN. The mixed-source image regression produced the intended runtime RED
  (Ok image passed to unwrap_err), then passed after the early guard.
- Independent original-XML/strict-base64 oracle matches all three relative source
  metadata records and Data/Mask digests. The corrected probe confirms explicit
  image refusal and unchanged database/original files; saved then removed.
- Added reusable opt-in relative_source_corpus test with a bounded, strictly typed
  private case/oracle manifest. Its corrected-candidate acceptance remains pending.
- Separate in-session self-review found no critical/important/security issue;
  no subagents or independent approval claimed.

## Retained refusals and pending gates

First owning-crate Clippy rejected a collapsible conditional, now corrected.
A combined code:: filter ran41library tests but zero reader integration tests;
that target is not counted as reader coverage. Corrected explicit unfiltered
reader/private corpus/unit/Clippy/format and repository preflights must run.
Then freeze the complete candidate for full branch acceptance, commit, actual
non-FF integrated acceptance including selected isolated corpus, publication
readback and owned-artifact retirement. None is claimed complete in this entry.

## Corrected preflight acceptance

All twelve stages of preflight1 passed against matching2914path manifests:
reader10/0, ProductDB unit444/0 (including both image guards), explicit isolated
Release corpus1/0 covering3source cases in1package, owning-crate all-target
Clippy, formatting, documentation/five repository checks and whitespace.
Separate review hashes still match. Earlier failed/zero-scope attempts remain
retained separately; no old output is relabelled as a rerun.

Full frozen branch acceptance now passes all18stages: Rust4660/0/208ignored
in341targets, Vitest3626/263files and both explicit source regressions. All2914
source identities, external oracle/package commitments and Git fingerprints
match. Subsequent updates are documentation-only and runtime equality must be
checked before commit. Commit, actual merged acceptance, publication readback
and owned retirement remain pending.

## Boundaries

This is not a relative image builder, generated-table mapper, source
scope/substitution translator, executable plan or durable recovery. No schema,
transport/UI/dependency/public mask/hardware admission change. Raw source,
identifiers, hashes, original PDF bodies and detailed logs stay private.
Selected dirty root remains unsynchronized; never stage/reset its foreign work.

Canonical contract: docs/contracts/offline-relative-source-segments.md.
Aggregate evidence: docs/evidence/offline-relative-source-segments-2026-10-16.json.
Private receipts/oracles: profile cache/scratch/offline-download-20261015/p4b/.
