# Offline download implementation P1 — 15 October 2026

Owner continuation explicitly lifted the pause for maximal evidence-based offline
implementation; target-specific field writes remain separate. Isolated branch
feat/offline-download-20261015, baseline3a9fd92f. Initial claim/goal commitd5698c15
was pushed to the feature branch and read back exactly.

Implemented pure absolute property-control payloads, independent legacy Type1/2
descriptor rewriting and strict System2 metadata projection retaining every source
position. Existing public download admission is unchanged. Original PDF page
images inspected directly; no extract as normative evidence. In particular the
property checksum-disable bit cannot reuse memory checksum-enable semantics.

Test-first receipts: missing API REDs, actual ambiguous TaskCtrl2 acceptance RED,
wrong three-byte-header diagnostic RED, then GREEN. Realistic start/length swap
mutant killed by byte assertions; canonical source restored exactly and re-tested.
Separate source/security self-review approved the bounded codec slice, not the
whole goal; no independent review/subagents.

Actual branch acceptance:11/11 stages; Rust4612 passed/0 failed/207 ignored in334
result blocks; Vitest3626/0 in263 files; warnings-denied all-target workspace Clippy,
workspace build, tooling and all five repository checks pass.2882 selected source
paths equal at start/end/current. Earlier gate1 stopped because frontend dist was
missing; original failed receipt retained. Offline npm ci and genuine frontend
build fixed setup; distinct successor gate2 performed every declared step.
Explicit ignored Release System2 source fixture:1 passed/0 failed/0 ignored.

Actual integrated acceptance on ad5f0e49:12/12 stages, Rust4612/0/207ignored,
Vitest3626, workspace Clippy/build/tools/five gates, and isolated OriginalData
152/152 in33targets.2884path start/end/current equality checked. Normal main
push verified local integration HEAD=tracking=live remote, with selected root
untouched. No runtime certification/field compatibility,
release/deploy, proprietary executable invocation or selected-root synchronisation.
Private evidence in profile cache/scratch/offline-download-20261015; final archive
and owned cleanup occur at package closure. Next work follows P2+ in the contract.
