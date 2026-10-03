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

## Integrated and published result

Feature16c9d774688c2e7e9a0c0b6bf436c788a7ce79e1 was integrated onto freshly fetched
upstream5540dcac5ce380630771de36ed6aa28366d7de6c as merge3d03aea5ee2905a98450ee193bdc1246d0f93c99.
Complete upstream handover suffix and both status prefixes were verified exactly.
Actual merged-result gates pass: Web1739/98, build,12 production/16 reference
Chromium groups, Clippy workspace/all-targets/-D warnings, Rust3009 passed/166
ignored across153 result blocks, fmt and4 fresh worktree-root-bound xtask gates.
All admitted commands exit0. Ordinary workspace scope excludes the ignored
private/hardware tests; no full corpus/native/Orca/WCAG acceptance is claimed.
Merged native-frame screenshot inspected: selected first row, light on1/0/9,
readable inspector and unobstructed footer. No code/config delta after this gate;
closure metadata/evidence is separately checked. Publication readback completed:
feature16c9d774 matches its live branch; published main d5c1080efb0cf66b5ea51d912e233130414125cd
matches local HEAD/fetched/live main and contains the design and feature ancestors.
Concurrent upstream80a5500d ZIP-measurement changes were documentation-only.
Reconciled full upstream handover/status archives, proved no code/config delta
against gated3d03aea5, and repeated4 repository metadata gates before push.

Cleanup confirmed: port4173 has no listener; own node_modules/dist in both feature
and integration checkouts, both fresh gate targets/log directories and named
publication scratch removed, retaining permanent source/PNG/receipts/log. Root
HEAD/index remained unchanged. A concurrent storytelling track updated its brief
and handover; those live changes were preserved rather than overwritten/staged.
The final source-identical Markdown acknowledgment is separately committed/pushed;
current refs/latest handover identify that bookkeeping tip, not a self-referential
hash invented inside its own commit. Closure timestamp is2026-10-03 22:39 UTC.

Main source advanced in CLI/productdb since the design baseline; no
frontend source overlap. Only handover/status documents overlap. Preserve the
complete authoritative upstream handover archive during conflict reconciliation.
Do not update the checked-out dirty root main ref or stage its foreign changes.
The final handover/closure receipt will replace this pending state with measured
commit IDs/gate results after execution, not before.
