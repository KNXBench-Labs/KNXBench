# PRG-01 — Close non-Secure programming gaps

**Status:** In progress. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

**Owner:** programming-correctness track; see the restart snapshot
[`.ai/CURRENT_STATE.md`](../../.ai/CURRENT_STATE.md) before touching shared
commissioning code.

## Task

The owner authorised (25 October 2026) closing existing non-Secure gaps in the
regular download/restore path, one bounded increment at a time, without live
bus or device operations.

## Software-accepted bounded increment — descriptor-qualified exact 0700 property state

The [admitted contract](../contracts/post-download-run-state-0700.md) and
[original-source investigation](../research/2026-10-29-property-description-run-state-0700.md)
allow one optional-property observation on exact `0700h`, after the existing
identity/load gates and strict candidate binding. Independent descriptor
metadata must establish PDT_CONTROL before one PID 6 read; a one-byte value
alone is not type evidence. Implementation `5f075cf99` has genuine original
CLI transfer/check RED → GREEN and independent whole-feature static review
with no open findings. Both maintained 15-stage branch/main-first gates and
actual isolated build/smoke passed on runtime `0af68438f`
([software acceptance](../status/2026-10-30-descriptor-qualified-0700-run-state.md)).
Historical baseline `8d14de9ca` returned unsupported mask. The existing
0701/0705 path, other masks, family/authorisation gates, retries/waits and
memory-refusal boundary stay unchanged. The package must include actual
CLI/HTTP/history/report/web consumers and semantic RED before GREEN, not stop
at a codec primitive. Physical function and overall PRG-01 remain open.

## Software-accepted bounded increment — descriptor-qualified exact 0702 property state

The [admitted contract](../contracts/post-download-run-state-0702.md), the
[original-source investigation](../research/2026-11-20-property-run-state-observation-0702.md)
and [ADR-0180](../adr/0180-exact-0702h-run-state-observation.md) extend the
bounded optional-property observation to exact `0702h` with the same
shape as exact `0700h`: Resources footnote 94 grants the legacy
`PDT_UNSIGNED_CHAR` datatype only to masks `0020h`/`0701h`, so `0702h` is
`PDT_CONTROL` and needs the same independent qualifying PID 6 property
description before one `PID_RUN_STATE_CONTROL` read. Schedule, gates and
serialized bytes for `0700h`/`0701h`/`0705h` are unchanged; other `070nh`
masks still report `UnsupportedMask`.

## Software-accepted bounded increment — regular restore exemplar binding

The [source-reviewed contract](../contracts/regular-restore-exemplar-binding.md)
binds a regular restore to the pre-mutation captured serial, with a precondition
embedded by the public core derivation rather than only a CLI check. Format 1/2
backups remain readable but lose restore eligibility, including partial backups;
no serial is fabricated and no bypass is added. This compatibility narrowing was
recorded before implementation. Runtime integration `0ddfe4953` implements the
core/executor guard, format 3 and closed consumer projections. Additional
CLI/HTTP/history/web proofs exposed a backup-phase diagnostic-admission defect;
its original semantic RED and the targeted Rust/web correction are retained.
Independent correction/whole-feature review accepted the candidate with no open
findings. Both maintained 17-stage gates and the actual isolated server
build/smoke passed ([software acceptance](../status/2026-10-29-regular-restore-exemplar.md)).
This is not hardware acceptance, absolute physical-identity proof or complete
mutation/recovery. The overall PRG-01 assignment remains open.

## Software-accepted bounded increment — 29 October 2026

The [property run-state continuation](../contracts/post-download-run-state.md)
implements an independently specified observation for exact masks `0701h` and
`0705h`, limited to one type-and-plan-identity-validated hosting object. The
[original-source review](../research/2026-10-29-property-run-state-observation.md)
separates the property contract from the still-unproved memory-state encoding.
Independent reviews, both 17-stage package gates and isolated real-server smoke
passed on runtime integration `161010078`
([software acceptance](../status/2026-10-29-property-run-state-observation.md)).
This is not hardware acceptance or closure of PRG-01.
Physical-exemplar proof, other object layouts, parameter policies and complete
mutation/recovery remain independent open requirements.

## Delivered so far

- Hardware identity response width: Device Object PID 78 needs exactly six
  response octets before backup and mutation, published as `07fe2e67f`
  ([delivery](../status/2026-10-25-hardware-identity-width.md),
  [contract](../contracts/hardware-identity-comparison.md)).
- Partial-application identity response width: the existing object 3/PID 13
  precondition requires exactly five response octets before backup and mutation,
  published as `02c437110` ([contract](../contracts/partial-identity-comparison.md)).
- Scope advice across core, app, CLI, HTTP and web chooses the concrete pending
  scope or prepares nothing, published as `78fd9d780`
  ([contract](../contracts/device-programming-support-guidance.md)).

[Integrated acceptance and publication](../status/2026-10-25-programming-scope-and-identity.md)
record the boundaries separately; these increments do not close PRG-01.

- Post-download read-only observation and its CLI, server, web, report and
  history consumers, published as `8b5edad7f`
  ([acceptance](../status/2026-10-25-post-download-check.md),
  [contract](../contracts/post-download-check.md)). That original increment kept
  runtime unknown; the 29 October property continuation above adds bounded
  observations. Physical-exemplar identity is still not verified.

- Backup prior-application identity: a backed-up run records the pre-write object 3/PID 13
  answer (backup format 2) and `restore_plan` refuses a plan of another or unrecorded
  application; legacy partial backups keep their precondition route
  ([contract](../contracts/backup-application-identity.md)). Published as `08db292f3`
  after independent reviews and the maintained 16-stage integrated package gate
  ([acceptance](../status/2026-10-25-backup-application-identity.md)); it does not
  prove whole-device recovery.

- Source-bound programming settlement replans the original request at completion
  and withholds written-memory credit after a selected-plan source change
  ([contract](../contracts/download-settlement-source-freshness.md)). Source
  reviews and all 14 selected integrated package stages passed; published as
  `fa4c18213` ([acceptance](../status/2026-10-25-download-source-freshness.md)).
  Other parts and later catalogue edits are not
  monitored; this does not close PRG-01.

- Region-bound memory reads use one selected service and respect the user-memory
  count limit for backup, comparison and byte-order probes
  ([contract](../contracts/memory-region-read-service.md)). Independent review and
  all 14 integrated package stages passed; published as `a4a4ebcf1`
  ([acceptance](../status/2026-10-25-memory-region-reads.md)).
  This adds no hardware or interface/coupler budget claim.

- Property load-control response-state handling preserves an actual LoadCompleting
  observation and rejects invalid responses before polling
  ([contract](../contracts/load-control-response-state.md)). Independent review
  and all 14 integrated package stages passed; published as `e4f5e7c6f`
  ([acceptance](../status/2026-10-25-load-control-response.md)).
  No inferred silent state, new retry, common timeout or hardware support.

- Memory request/response service pairing now requires the requested service as
  well as its address, without changing the answer ladder or retry policy
  ([contract](../contracts/memory-response-service-pairing.md)). Independent reviews
  and all 14 integrated package stages passed; published as `53400189e`
  ([acceptance](../status/2026-10-25-memory-response-pairing.md)). Synthetic
  nonconforming-peer evidence is not hardware acceptance.

- Declared TypeNumber bound admission refuses unreadable limits instead of silently
  dropping them, without altering stored text or encoding
  ([contract](../contracts/parameter-number-bound-admission.md)). Independent review,
  correction re-review and all 14 integrated stages passed; published as `87d71d4aa`
  ([acceptance](../status/2026-10-25-parameter-number-bounds.md)). Selected corpus
  regression is not an exhaustive compatibility census; parameter policies remain open.

- Ambiguous allocated data-bearing image starts are refused rather than silently
  selecting one payload ([contract](../contracts/duplicate-image-segment-start.md)).
  Independent review and all 14 integrated stages passed; published as `d73a2413b`
  ([acceptance](../status/2026-10-25-duplicate-segment-start.md)). Payload-free
  allocations remain valid; no real-product fault or new family support is claimed.

- Server shutdown can prevent a regular download's first mutation through early
  run registration and atomic stop/commit ordering
  ([contract](../contracts/regular-download-prewrite-stop.md)). Published as
  `6c9ca2d7d` after independent reviews and all 14 integrated stages
  ([acceptance](../status/2026-10-25-regular-prewrite-stop.md)). After commitment,
  the run continues unchanged; no mid-procedure stop or full recovery is admitted.

- Run-scoped preparation and settlement receipts in regular-download history
  ([contract](../contracts/download-settlement-receipt.md)) distinguish applied
  project parts from computed, not-applied parts and separate read-only checks.
  Independent reviews and all 15 integrated stages passed on `3431aabbc`
  ([acceptance](../status/2026-10-25-download-settlement-receipt.md)); final
  main-first documentation integration preserves runtime bytes. These display
  receipts are not device baselines and do not close effects/recovery work.

## Delivered bounded source admission

The regular planner conservatively refuses declared `MergeId` fragments rather
than silently omitting them ([contract](../contracts/regular-merge-fragment-admission.md)).
Stored source and fragment-free plans remain unchanged; shared read-only
comparison/source/restore preparation inherits the restriction. The original semantic regression, final focused tests, independent review and
all 14 selected maintained integrated stages passed; source is published as
`be2687796` ([acceptance](../status/2026-10-25-regular-merge-fragment-admission.md)). This is
not merged-procedure execution or closure of PRG-01.

## Delivered service-aware memory budgets

The [service-aware memory budget](../contracts/memory-transfer-budget.md)
reserves the extra user-memory header octet and applies a conservative standard
cap to unproved hardware-kind/default routes. Standard data budgets are 12
ordinary or 11 user-memory octets; actual interface/coupler and extended-path
capacity acquisition is not implemented. Original semantic regressions and
independent initial/correction reviews passed; all 16 selected maintained
integrated stages passed, published as `1e6229cea3`
([record](../status/2026-10-26-memory-transfer-budget.md)). No new family,
authorisation, recovery behavior or physical outcome is claimed.

## Delivered durable tunnel runtime for regular CLI operations

`device download`, `device restore` and `device service-control` use the existing
multi-thread tunnel runtime so receive/heartbeat tasks can continue during
synchronous persistence. Published as `a8004cb2e`, independently reviewed APPROVE,
13/13 merged package stages green (521 Rust tests, selected corpus 17/17).
[Acceptance and test limits](../status/2026-10-26-cli-durable-tunnel-runtime.md)
separate native loopback ACK characterization from lexical call-site checks;
there is no CLI end-to-end, new shutdown, recovery or hardware guarantee.

<a id="implemented-software-acceptance-pending--customer-adjustable-write-refusal"></a>

## Software-accepted bounded increment — customer-adjustable write refusal

The [bounded contract](../contracts/customer-adjustable-write-admission.md) and
[primary-source investigation](../research/2026-10-30-customer-adjustable-write-admission.md)
admit a stricter KNXBench refusal policy, implemented at `d6918027b` with
independent corrected whole-feature review and no open findings. Maintained
branch/main-first gates each passed 15/15 stages; actual isolated server build/smoke
passed on runtime `e990fd956`
([software acceptance and limits](../status/2026-10-30-customer-adjustable-write-admission.md)).
Runtime acceptance is separate from publication/readback.
The historical baseline `17e45dc62` did not consume the admitted unqualified
markers before otherwise supported regular write preparation.

Only uninterpreted presence in explicitly bounded root/Union/module contexts
and the own root option may refuse, with kind/plugin precedence and unchanged
read-only preparation. Module-local Options and other excluded contexts are
named matcher limits, not claims of harmlessness or universal flag detection.
The XML value domains and actual ETS calibration-preservation algorithm remain
unproved here; no positive true/Sync classes, status bypass or per-byte skip.
Original semantic REDs and focused consumer, valid restore/source-freshness,
privacy and retained-source tests are preserved. A separate compiled resource RED
exposed multiplied deep paths; flat shared prefixes close it without changing
marker semantics. On the final corrected code, nine independent semantic mutants
failed their intended assertions, two inert controls passed, and the copied
3,905-file input set was restored exactly. Those populations overlap; neither
static review nor calibration replaces integrated delivery. This closes neither
the broader parameter rules nor PRG-01.

<a id="implemented-package-acceptance-pending--inactive-whole-byte-write-selection"></a>

## Historical software acceptance — inactive whole-byte write selection

The [bounded contract](../contracts/inactive-byte-write-selection.md) now has a
positive regular-App-to-executor implementation, not merely a declaration guard.
It omits only proved exclusive inactive whole bytes with source-default topology,
retains source Mask/literals/active Union winners and read-only behavior, and binds
new format-4 backups to an explicit typed restore generator. Historical restores
keep their exact captured shape and existing application/exemplar checks.
Since the 31 October byte-precise fallback (owner decision) a failed proof never
refuses: unproved bytes are written as before and program-wide failures keep the
regular plan; the amendment passed a second independent review.

Original real-writer RED → GREEN, genuine old-backup and reopened-catalogue
consumer proof, 412 distinct passing regression bodies and final independent
source review with no open findings are recorded in the
[focused result](../status/2026-10-30-inactive-byte-write-selection.md). The
[research](../research/2026-10-30-inactive-byte-write-selection.md) separates raw
option/opaque-attribute/expanded-namespace provenance and actual scoped-plan
historical evidence reuse. The
[4 November historical reconciliation](../status/2026-10-30-inactive-byte-write-selection.md#historical-acceptance-reconciliation--4-november-2026)
establishes one completed 14-stage main-first gate on `2bec7dd8a`, contained in
main, and attributes the original build/smoke observations to their receipts and
logs. No separate final branch-green or original final-smoke executable-byte
binding is claimed. This is not fresh current-main or hardware acceptance.

`regular-070n-exclusive-inactive-storage-survival` stays OPEN: omitted transfers
do not prove retention through Unload, allocation, task control or Restart.
Broader inactive/default/Union/customer-write rules and PRG-01 remain open.

## Bounded increment — PRG-01-N3: module-local Options admission (2026-11-20)

The [N3 contract](../contracts/customer-adjustable-module-options.md), the
[evidence note](../research/2026-11-20-module-local-options-admission.md) and
[ADR-0181](../adr/0181-module-local-options-customer-adjustable.md) admit
`CustomerAdjustableParameters` on a module definition's own `Static/Options`
(new closed context class `ModuleOptions`); the parser walk is unchanged. The public writer refusal category
`customer-adjustable-parameters` gains a `module Options` count; read-only
preparation is unchanged. Acceptance record:
[2026-11-20 module-local-options admission](../status/2026-11-20-module-local-options-admission.md).
Wrapper, `Dynamic`/`Extension`, extra-`Static` and foreign-prefixed shapes stay
excluded matcher limits. This is not hardware acceptance, not ETS
customer-write semantics and not a closure of PRG-01.

## Scoping record — PRG-01-N4 (21 November 2026)

Remaining 070nh masks, other object layouts and memory run-state encoding have no
source evidence; only their refusal is simulator-verifiable and is now pinned
([research](../research/2026-11-21-run-state-remaining-masks-and-layouts.md),
[contract](../contracts/post-download-run-state-unadmitted-masks.md),
[ADR-0182](../adr/0182-unadmitted-run-state-masks-stay-refused.md),
[record](../status/2026-11-21-unadmitted-run-state-masks.md)).

## Open

- Broader target/exemplar identity and runtime-state observation beyond the
  bounded property contracts (`0701h`/`0705h`, plus descriptor-qualified exact
  `0700h` and `0702h` whose bounded software acceptance is recorded above).
  Other object layouts, memory encoding and physical function remain unproved.
- Parameter policies beyond the bounded whole-byte transfer contract: physical
  inactive-byte survival, broader inactive/default semantics, editable Dynamic,
  `Union` and customer-write rules remain unresolved (module-local Options are
  now an admitted refusal context per PRG-01-N3; the remaining excluded
  customer-adjustable shapes stay matcher limits).
- Complete mutation effects and whole-mutation recovery contracts.

PRG-01-N5 (21 November 2026) inventoried these remaining parameter-policy items
and found none simulator-verifiable without inventing ETS semantics; it names the
missing evidence per item ([research](../research/2026-11-21-prg01-n5-remaining-parameter-policy-evidence.md),
[scope contract](../contracts/prg01-remaining-parameter-policy-scope.md),
[ADR-0183](../adr/0183-prg01-remaining-parameter-policies-need-evidence.md)).
No code or runtime claim changed.

PRG-01-N6 (21 November 2026) examined the mutation-effects axis (post-restart memory
comparison, further declared forms, other layouts) and found none simulator-verifiable
without inventing semantics; the post-download check deliberately reads no memory
because a restart may rewrite octets. It names the missing evidence
([research](../research/2026-11-21-prg01-n6-mutation-effects-observation.md),
[scope contract](../contracts/prg01-mutation-effects-observation-scope.md),
[ADR-0184](../adr/0184-prg01-mutation-effects-observation-scope.md)). No code changed.

PRG-01-N8 (21 November 2026) reconciled this file, the OPEN_WORK row and the
ledger with the increments still queued on PR branches; see the
[reconciliation](../status/2026-11-21-prg01-n8-queue-reconciliation.md). Docs only.

PRG-01-N9 (21 November 2026) audited the post-download check across CLI, history,
web and report: one drift fixed (the report summary now keeps the run-state reason)
and the report file pinned against the shared admission table
([ADR-0186](../adr/0186-report-summary-keeps-the-run-state-reason.md),
[record](../status/2026-11-21-prg01-n9-report-consumer-consistency.md)).
Simulator/fixture evidence only.

PRG-01-N10 (21 November 2026) pinned malformed choose/when input for inactive
whole-byte selection ([acceptance](../status/2026-11-21-prg01-n10-malformed-choose-when-pins.md)).
Test-only; no behaviour change, no new policy.

PRG-01-N11 (21 November 2026) hardened the backup file reader: signed or padded hex text
(for example region octets `+5`) is refused as malformed instead of being read as a number
([acceptance](../status/2026-11-21-prg01-n11-backup-hex-refusal.md)). Offline pin only; no
format, consumer or hardware change.

PRG-01-N12 (21 November 2026) pinned consumer consistency: the web's per-category
reason and action tables must name exactly the server's refusal categories, so a
new category cannot silently fall back to a generic web reason
([ADR-0187](../adr/0187-download-refusal-categories-have-a-drift-pin.md),
[acceptance](../status/2026-11-21-prg01-n12-category-drift-pin.md)). Test-only.

## Full-product coverage pin debt resolved — 8 November 2026

The owner authorised a new documented characterisation baseline rather than
further reconstruction of the historical 105-package selection. The ignored
`download_coverage_corpus` test now pins **122 packages / 758 programs**, plus an
input-content and installation-order commitment. The private input manifest
names every file and its hash; [research §19.22](../research/commissioning/19-22-product-coverage-characterisation-baseline.md)
attributes every changed category against the 9 October code on the same inputs.
A small refusal-precedence correction keeps non-memory masks classified as such
even when their code cannot be read; memory-mapped malformed code still refuses
as `product-data`. No parser guard was relaxed and no hardware support added.

This resolves only the pin-maintenance debt. The original selection is still
unreconstructed; the new baseline is explicitly not an independent semantic
oracle. The broader PRG-01 requirements above and KL-006 remain open.

## Refusal reason codes pinned — 21 November 2026 (N7)

`plan_category`/`image_category` are now exhaustive and every plan refusal's
code is pinned ([contract](../contracts/download-refusal-reason-codes.md),
[ADR-0185](../adr/0185-download-refusal-classifiers-are-exhaustive.md),
[status](../status/2026-11-21-prg01-n7-refusal-code-pin.md)). No behaviour change.

## Boundaries

KNX Secure is excluded. Existing family and recovery gates stay in force; a
partial download never escalates to a full one implicitly. No hardware
support is inferred from software increments.

## Done when (per increment)

Independent review, the maintained integrated package gate and publication
are recorded; hardware acceptance needs its own operation-specific consent
([WP-12](WP-12-field-evidence-and-owner-acceptance.md)).
