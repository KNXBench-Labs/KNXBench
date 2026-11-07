# Captured saved-instance priority — implementation and evidence

## Delivery state

The bounded implementation is independently reviewed, has focused executable
proofs and is software-accepted and published on main as `ed0cf7737`
(main-first parents `196e1f53f` and feature `7943905f4`). Exact remote,
tracking and live-tree readback was verified. See
[Integrated execution](#integrated-execution-and-selective-fixture-regression).
Broader WP-06 remains open.

## Implemented boundary

Fresh, uniquely owned, non-module schema-23 imports capture the saved instance's
own priority declaration independently of product-query priority. Declared raw
text, attribute omission, instance-element omission and unavailable observations
remain distinct. Empty, whitespace, unknown and XML-decoded literals survive
without trimming, interpretation or fallback. Source-member identity, original
owner/occurrence counts and retained attribute bytes/hash must corroborate a
capture; ambiguity or failed admission withholds metadata, not existing raw data.

Boxed captured payloads and deterministic count/string/index/work budgets bound
the extra observation. Resource refusal covers the whole object population,
including orphaned or miskeyed records. These are application observation limits,
not KNX format limits or measured RSS/latency guarantees.

Native schema **17** adds an exact validated overlay. Native16 and older history
receive **NotCaptured**, never an inferred opaque backfill or invented absence.
An empty overlay preserves previous semantic identity; captured facts participate
in new identity. Baseline, working state, undo/redo and named versions preserve
facts and retained context. Rebinding preserves the stored fact while the
read-only projection withholds its raw/source payload as SourceBindingMismatch.

The existing device-detail GET has a required tagged `saved_instance_priority`
field with matching generated TypeScript types. It is a captured observation,
not currently reverified source, live device state, selected product priority,
effective inheritance or wire encoding. No new priority UI/editor, report/diff,
source-schema/module support, programming policy or hardware authority is added.

## Genuine predecessor and provenance

The object-bearing V16 witness was written using unchanged production sources
from `7cba23ff2`, then closed and reopened through the old APIs. It contains a
root baseline, divergent working state, both command stacks, a named version and
retained opaque/context facts. The committed synthetic inputs, exact synthetic
producer source and portable provenance accompany the fixture:

`8246f92ca2610c8f8a58295bb026a6310134ff06acae5550e36a2cd9e62992b5`

The separate store-lib schema-subtraction test is only an empty-overlay/shape
control, not an authentic predecessor. Original producer attempts and final
writer/executable/input identities are retained privately. The successful old
producer had no real-user-home before/after observer; that limitation is not
retroactively repaired or credited as a migration result.

## Focused execution and review

- The web bundle and seven owning Rust test targets compile.
- All 27 named new contract tests have actual passing results. Their retained
  source/executable epochs remain separate: original binaries are not relabelled
  as built from later test or generated-binding edits.
- The first core test failed because its helper omitted topology membership.
  Reusing the existing valid fixture, without weakening preservation assertions,
  made the freshly rebuilt regression pass. Original failure evidence remains.
- Actual V16/history/rollback, importer, store, App, projection and mounted GET
  paths ran. Seven application/fixture TypeScript projects passed; 18 generated
  dependency bindings match after the established trailing-whitespace normalization.
- Whole-feature static review found one P2 assertion gap, not a production defect:
  the GET test used its production projector as the unavailable-reason oracle.
  Seven independent literal reason/status objects now check actual responses both
  with and without catalogue overlays, including absence of raw/source payload.
  A fresh server regression passed and independent delta review closed F1.
- Focused proof receipts bind frozen source and retained executables, offline
  loopback-only unprivileged namespaces, scratch HOME/XDG, ordered shared leases
  and untouched/checked KNX user data. No numeric resource trace or executed
  hypothetical mapping mutant is claimed. The reviewer executed no product tests.

## Integrated execution and selective-fixture regression

Integrated acceptance (7 November 2026) exposed a selective-import regression
and outdated historical test setups. Both were corrected without relabelling
the earlier execution epochs:

- `97780b1b5` keeps saved-priority provenance aboard selective imports: the
  selective path now remaps the saved-source provenance instead of dropping it.
  An independent review of this fix returned APPROVE.
- `7943905f4` restores the historical tests' intended table shapes instead of
  merely lowering a version number while retaining the v17 overlay table.
  These setup corrections are not newly authenticated historical producers.
  The parent reviewed them; no production store writer, migration or binary
  fixture changed.

Seven targeted tests were added. The combined run covers 51 named tests in 12
stages including `fmt`, all passing. Source stayed frozen, head stable and KNX
user data was checked untouched.

**Full maintained gate.** The selected 17-stage gate ran in full and passed; the
web stages were executed fresh, not composed or reused. The nested fresh 55 web
stages all passed with per-stage `proofValid`; the complete web receipt records
`inputsFrozen`, `planCompleted` and `success`.
The historical 55-web result from the first epoch is not reused, because the
selective production logic changed after it. Workspace Rust: 8,341 passed, 248
ignored. The private corpus ran separately: 155/155 in 9 targets.

**Build.** The actual server build exited 0 and reports
`knx-server 0.1.0-alpha.19+ged0cf7737`. The retained executed ELF is 175,280,728
bytes, SHA-256
`7d6074ee155b0f4940ba50a3f44411075a387e1d00f7f1960f4560fa8f85eb80`.

**Isolation.** Offline loopback, original uid, a zero-capability parent,
maintained per-stage scratch homes and ordered shared leases were verified.
The outer plain/font environment kept its real home; it is not credited as a
scratch-home execution. Source was frozen and KNX user data fully checked
untouched.

**Retained failure.** Original attempt 06 failed with 34 Rust failures; all raw
and 35 logged executables stay retained unchanged. Nothing was relabelled as a
pass. The earlier limits above stand: the original 27-test epochs, the GET F1
assertion gap and the authentic-V16 fixture limitations (no real-home observer
on the successful old producer).

**Scope.** The native17 fact is a captured schema-23 saved source observation,
not effective, product or wire priority. UI, editing, report, comparison and
broader WP-06 stay open. No source-format, hardware, release or deployment claim.
Private evidence stays under `<profile>/evidence/2026-11-07-wp06-wp11-continuation/`.
The core archive is fully byte/hash-verified; its separate storage failure is
retained without relabelling. Final documentation retention and retirement are
tracked by the root-local checkpoint.

## Compatibility and remaining work

Keep originals and independent backups: older applications can refuse newly
written schema17 files. Read-only native opening uses a migrated memory copy and
leaves original file bytes unchanged; it does not enrich historical metadata.
Other source schemas, module authority, effective/product resolution, editing,
comparison/report/UI presentation and full WP-06 are separate work. Released
Alpha 19 source/format cutoffs are unchanged. No ETS equivalence, native-shell,
physical hardware, release or deployment acceptance follows.

[Contract](../contracts/saved-instance-priority.md) ·
[Source admission](../research/2026-11-06-saved-instance-priority-admission.md) ·
[ADR-0158](../adr/0158-object-priority-provenance.md) ·
[Remaining WP-06](../open-work/WP-06-communication-object-organisation.md).
