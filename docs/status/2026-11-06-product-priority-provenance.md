# Product-query priority provenance — software acceptance, 6 November 2026

## Delivered boundary

Source `c376f0347`, published as tree-identical main-first `edd41cd25`, adds
`ComObjectView.priority_layer: Option<ValueLayer>` beside the existing literal.
ProgramRef wins by presence, including equal, empty, whitespace and unknown
values; Program is reported only for a present program declaration; absence
asserts no origin. Selected text, catalogue schema and image policies do not
change. [ADR-0158](../adr/0158-object-priority-provenance.md) separates raw
source, product selection, future project-effective priority and wire priority.

Existing project-attribute opaque retention is not new functionality. Typed
project priority, migrations, commands, API/UI/report/comparison, real-export
compatibility and the wider WP-06 task remain open. High remains refused and
the existing Alert inference is unchanged. No new device operation is admitted.

## Focused execution and independent review

Five production program-reader/query tests cover fifteen literal matrix entries,
separate SQL declarations, single/batch agreement, repeated IDs across programs,
translation isolation, query nonmutation and current-schema native read-only
catalogue reopen with unchanged closed-file bytes. These are synthetic product
reader witnesses, not full original-package or project-import acceptance.

The first author run passed four tests and failed one at test-fixture setup:
its translation insert used an obsolete column. The corrected test uses the
production translation API; the retained successor passes all five. It is not
an original missing-feature RED; implementation and acceptance rely on the
executed source-bound successor.

Independent whole-feature review approved the authenticated eight-path feature
and the separately bound research clarification. In its isolated archive copy:

| Attempt | Passed | Failed | Exit |
| --- | --- | --- | --- |
| Original control | 5 | 0 | 0 |
| Every present literal attributed to Program | 2 | 3 | 101 |
| Program origin invented for absence | 2 | 3 | 101 |
| Equal reference literal attributed to Program | 2 | 3 | 101 |
| Restored original control | 5 | 0 | 0 |

These are compiled discrimination mutants, not pre-feature semantic REDs.
Five executable copies with four distinct hashes, raw logs, source manifests
and namespace/user-data results were retained. Mutant failure stops some matrix
loops early and is not complete matrix execution. The parent rehashed these
artifacts and checked the actual named test outcomes before accepting review.

## Maintained main-first acceptance

One complete selected fourteen-stage package gate ran on `edd41cd25`, whose tree
is exactly the reviewed feature tree. No distinct branch-gate execution is
claimed. All selected stages completed with direct exit zero and valid proofs:
format, documentation, browser-lane policy, five repository checks, tool tests,
desktop build cleanup, warning-denied Clippy, affected Rust tests, selected
private corpus and diff check.

- Rust: **5,500 passed, zero failed, 177 ignored** across the gate's selected
  packages and dependents. The receipt's 24 result summaries include doctests;
  they are not asserted as 24 executable test binaries.
- Tooling: **517 registered tests, six existing optional skips**.
- Separate private corpus: **97/97 passed in six targets**. These regressions
  are not a complete compatibility census or new real-export witness.
- `npm ci` and the actual TypeScript/Vite production build passed as environment
  preparation. No additional application browser, bindings, website or story
  lane was selected for this Rust query metadata change. Tool tests may launch
  their own browser fixtures; that is not application UI acceptance.

Execution used an offline loopback network namespace, nested unprivileged user,
scratch test data/config homes, a task-owned Cargo target and the maintained
shared execution lease. Complete ordered stages, source freeze and both empty
user-data touched/unchecked lists were verified against native receipts.
Review controls, focused tests, Rust tests and corpus counts overlap and are
not added into a unique test total. No real bus or device was contacted.

## Publication and evidence

Local candidate, freshly fetched tracking ref and live remote main all matched
`edd41cd2501a5624f5d409a46f207c94cd3a06d4` after the normal fast-forward push;
all nine owned file blobs were read back exactly. The intentionally old/dirty
selected root was not synchronized or reset. No release, CI activation, native
UI run or deployment occurred.

Raw source, original failure, independent review, controls, executables and
maintained gate receipts are private under
`<profile-scratch>/next-task-20261106/` pending final retention under
`<profile>/evidence/2026-11-06-product-priority-provenance/`. Documentation-only
closeout is a separately gated successor; runtime results stay bound to the
original executed commit rather than being relabelled as a new Rust run.
