# Changed-parts download readiness and future test plan — GAP-T30-04

Prepared 2026-10-11 against source `aeb5a386`. **Preparation only; no protocol
implementation, live connection, read, download or write performed.** The
[ledger](../status/LEDGER.md) keeps GAP-T30-04 `LATER / RECORDED_SCOPE`.
[Powerline preparation](powerline-readiness.md) is a different undertaking and
requires a different explicit Go. Neither approval implies the other.

## Existing boundary, not a missing generic download switch

The [historical CP §3.5.3 audit](../spec-audits/2026-09-19-cp-3_5_3-partial-download.md)
corrects the former claim that differential download is undocumented. In its
GAP-T30-04 correction it identifies standard-defined prerequisites and client
baseline state while distinguishing the implementation-defined diff/chunk
strategy from unspecified ETS internals. Its early issue list and later
answers must not be read as an uncorrected current defect list.

Current source already carries distinct CRC outcomes and
`DownloadCounterCheck` in the [sequencer](../../crates/knx-net/src/commissioning/download.rs),
and five partial variants in the [core model](../../crates/knx-core/src/commissioning/partial_download_variant.rs).
These observations do **not** prove that changed byte ranges are selected,
written or validated on a real device. A counter/CRC observation alone is not
proof of an exact previous image, complete recovery coverage or an unchanged
application. Uninterpreted imported `LoadedImage`/`CheckSums` are not automatically
trusted live baselines or a permission to write.

A future design must distinguish three operations explicitly:

- **Complete download:** existing validated plan remains the unchanged fallback.
- **Partial part/segment reload:** an existing scoped procedure is not thereby a
  byte-differential algorithm.
- **Changed-range/differential write:** new target-specific scope, including
  exact baseline identity, permitted regions, framing, verification and recovery.

This package neither adds the third operation nor silently changes the first.
A refused optimization may recommend rebuilding/reconfirming an existing full
plan; it must never turn consent for a changed-range write into an automatic
full download, unload or reset.

## Required design evidence before implementation

1. Identify an exact device/product/mask/application version and the separately
   authorized test topology. Resolve each applicable primary CP/RES/profile and
   manufacturer clause. Document which properties and segment variants apply;
   missing/unsupported properties are not presumed zero.
2. Define a versioned, immutable last-successful image receipt bound to stable
   device identity, mask/application version, part/region, payload bytes/hash,
   observed counter/checksums and acquisition source. Imported intent and
   simulator state must remain distinct from live observations.
3. Require exact, readable pre-write baseline evidence and complete recoverable
   mutation scope. Counter equality, a checksum match or an existing image file
   is not enough without matching identity and coverage. Changed, absent,
   unavailable, stale or ambiguous evidence must fail closed.
4. Define deterministic diff selection over **verified writable regions** only,
   with bounded ranges, alignment/framing, overlap handling, atomicity limits,
   cancellation, partial progress and read-back. Do not infer EEPROM/layout or
   manufacturer flag semantics from generic field names or another product.
5. Persist, sync and read back a complete recovery receipt before any potentially
   state-changing setup or write. Include indirect state (load state, allocation,
   version/counter/access policy) where authoritative evidence says it changes.
   Do not re-label a plan-scoped dump as a full-device backup.
6. Keep original source/payload evidence exact. Publish a new successful baseline
   only after required device readbacks and functional verification succeed.
   Interrupted or partially acknowledged operations cannot advance it.

## Future phases and exit criteria

| Phase | Proposed activity after its own authorization | Required exit evidence |
| --- | --- | --- |
| D0 — offline design | Review original normative/manufacturer evidence, baseline/recovery format and smallest supported device/part scope. | Traceable evidence and applicability matrix; unknown regions/effects remain refused. |
| D1 — offline implementation | Synthetic unchanged/single-range/multiple-range/full-change fixtures plus a simulator that models stated device restrictions. Exercise stale/absent counter, wrong identity/version, mismatched/ambiguous baseline, forbidden region, budget/alignment errors, cancellation and interrupted writes. | Named RED/GREEN tests and guard-negative controls; exact preview ranges; baseline advances only after successful verification. |
| D2 — native persistence | Save/reopen baseline and recovery receipts; test truncated/corrupt/future formats, transaction/persistence/readback failure and concurrent changes. | Exact byte/provenance equality and atomic refusal; confirmed request without durable proof refuses before connection activity. |
| D3 — separately authorized read-only bench | Identify the nominated device and collect only approved baseline facts, after checking setup-side effects. | Exact identity/version/region/observations agree; backup completeness and restoration method are proven. |
| D4 — separately authorized full baseline | If a live verified baseline must be established by full download, obtain its own operation-specific Go and satisfy existing full-download/recovery gates. | Full state and function read back; immutable successful baseline stored/read back. This is not implicitly authorized by D3. |
| D5 — separately authorized differential trial | One small, reviewed change on the isolated device, fresh pre-send baseline/recovery check and explicit change overview. | Allowed ranges only; unchanged regions remain unchanged; required readbacks/function pass; truthful outcome and new baseline. |
| D6 — separately authorized recovery | Only reviewed interrupted-write/recovery cases with complete pre-write proof. | Covered state restored exactly and function checked; failed/unknown recovery prevents further mutation. |

## No-go and acceptance

No present task-specific hardware Go exists in this Session2 scope. Do not
perform D3–D6, run ignored/live tests, enable a connector, send discovery probes,
implement a differential sequencer or modify complete-download fallback here.
Stop on identity/version drift, another configurator's change, uncertain CRC
validity, absent baseline facts, unproven persistence, unknown region effects,
ambiguous targets, unexpected load state, errors/timeouts or operator cancellation.
No automatic retry, fallback mutation or escalation beyond the confirmed scope.

Simulator and native receipt tests establish only their own evidence axes.
Hardware compatibility, ETS chunking equivalence and end-to-end commissioning
claims require separately approved real-device evidence. This preparation can
be complete while GAP-T30-04 remains later, unimplemented and unverified.

## Re-check 17 October 2026

No new primary source. The KNX help article on differential download stays
unreachable to automated retrieval (HTTP 403, API 401, no archive snapshot).
xknx-editor's new *preflight* reads device bytes and lists changed ranges before
a full download; that is a read-only comparison like KNXBench's compare plan, not
a changed-parts write, and an independent implementation is never a defining
source. The prerequisites above are unchanged
([review](../status/2026-10-17-ledger-residue-review.md#phase-2--evidence-re-check-no-new-primary-source)).
