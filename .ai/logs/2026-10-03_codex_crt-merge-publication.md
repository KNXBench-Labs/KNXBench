# CRT commit/main integration/publication — 2026-10-03

Authorization: user “dann bitte” after the explicit list of animation commit,
main integration with repeated gates, and push. The existing design branch
is included as an ancestor of the animation branch. Shared dirty/stale root
main is protected; integration/publication uses a separately owned checkout.

## Pre-commit in-session review

No subagents used, per standing user preference. This is not an independent review.
Complete new controller/hook/tests/verifier and tracked UI/bootstrap/motion diff
were reread against ADR-0022, native event ownership and the palette-v1 boundary.
No CRITICAL or IMPORTANT finding. Cheap MINOR findings corrected before commit:

- New test files lacked ADR-0018 headers: added one-sentence purpose/SPDX comments.
- crtInteractions.ts:1 contained two sentences. The first repository header gate
  correctly refused it; reduced it to one sentence and repeated the whole branch
  gate. Original rejected gate evidence was retained until closure.
- IMPLEMENTATION_STATUS browser wording overclaimed explicit dirty-state evidence.
  Browser tests assert Save refusal/error and single Enter request. Unchanged
  dirty-state behavior is established by source trace, not actual persistence.
  Corrected the statement; no fabricated store or native acceptance claim.

Selection and native keyboard actions are neither consumed nor synthesized;
manual-save presentation cannot acknowledge persistence or throttle actual calls.
Palette-v1, settings/project schemas, API contracts and KNX protocol are untouched.
No secret/dynamic-code/persistence path is introduced by the presentation controller.

## Measured branch gates

Repeated full branch gate: Web1739 tests/98 files; TypeScript/Vite build;12 real-App
Chromium groups and16 reference/theme-manager browser groups; cargo fmt; fresh
worktree-root-bound xtask layering/headers/anchors/corpus-registration; whitespace.
Every admitted stage exit0. No real backend/project/settings/KNX operation.
Final native-frame screenshot visually inspected: selected first row, light at
1/0/9, readable inspector and unobstructed footer. Timing tested separately.

## Pending publication

Feature commit, conflict-preserving integration onto the freshly fetched remote
main, merged-result gates and live push readback are still pending at this log
creation. Main source advanced in CLI/productdb since the design baseline; no
frontend source overlap. Only handover/status documents overlap. Preserve the
complete authoritative upstream handover archive during conflict reconciliation.
Do not update the checked-out dirty root main ref or stage its foreign changes.
The final handover/closure receipt will replace this pending state with measured
commit IDs/gate results after execution, not before.
