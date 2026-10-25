# S2-01 — System-2: keep the raw PID 7 answer and model its width

**Priority:** P1. **Status:** Partially delivered on `main` as `5f1ad454e`; remaining work blocked/open.
Independent review APPROVE; integrated package gate 14/14 passed.
[Diagnostic acceptance](../status/2026-10-25-system2-pid7-diagnostic.md) records the exact scope.
The private failure journal retains object, element count, actual octet count and up to 16 raw
octets. HTTP status and the shareable report do not expose these details. The width model and
research record remain blocked on real response evidence; no supported width is inferred.
Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30, field report of a
refusal `resource pointer response is not four octets` on a real UP 258D12 (two operations,
`mutation_may_have_started: false`).

## Problem

The pre-write recovery acquisition reads `PID_TABLE_REFERENCE` (PID 7) per resource and
requires exactly four octets (`read_pointer` in `crates/knx-net/src/commissioning/property_recovery.rs`).
In the reporter's build, any other length aborted the run before the first write and discarded
the answer. The field report therefore proves only "not four octets": its element count and bytes
are unknown, and the open wire-width question ([up258 validation, row 2](../contracts/up258-system2-validation.md))
stays open although a real device has now answered it with a different width.

The earlier Alpha 16 refusal ("current resource pointer differs from original source allocation
shape") compared raw bytes against a four-octet expectation, so it may have been a width mismatch
too. This is a reading of the code, not a measurement; the "pointer = last allocated segment start"
explanation given in #30 is withdrawn until raw bytes exist.

## Deliverables

1. **Private raw diagnostic (implemented).** `read_pointer` uses `read_property_counted` with
   unchanged start-index matching. A wrong-width answer remains a failure, not a non-blocking
   observation. Its element count, actual byte count and first 16 raw octets (or `(none)`) survive
   in the private journal's `Failed.local_diagnostic` and in the CLI error result. The bytes are
   never padded or decoded as a pointer. Longer payloads are explicitly abbreviated with `...`;
   this is a diagnostic prefix, not a complete capture. HTTP status and the shareable report
   continue to omit the runtime failure text. A structured status/report/UI projection is still
   open; this subdelivery does not implement it.
2. **Wrong-width rule: the refusal stays until the width is understood.** An observation-only
   rule is rejected for now, because a backup cannot cover this case: the recovery snapshot holds
   a four-octet pointer per resource, and `recovery_plan` already refuses original-state recovery
   whenever the prestate differs from the source shape. A run that continues past a wrong-width
   answer would therefore have no original-state recovery, only a fresh ETS download as the way
   back. Independently, the masked 338 bytes cannot be shown to survive Unload
   ([up258 validation, row 1](../contracts/up258-system2-validation.md)), so even a clean backup
   would not prove a way back. Writes use the plan's absolute addresses, not the read pointer.
   A wrong-width reply alone does not identify its cause or establish the firmware's resource
   representation. Do not infer either merely from a byte count.
   The fix is to model the width once the raw bytes are known: a width-aware pointer in the
   snapshot and a recovery path for it, admitted only with evidence for that firmware, so that a
   usable backup exists. Until then the refusal before the first write stays, now carrying the
   element count and the bytes. The post-download strict checks (memory read-back, Loaded, written
   program version) stay hard in every variant.
3. **Research record** in `docs/research/`: the observed width as a field fact (one device, one
   firmware, unverified for others), kept apart from the standard's four-octet definition.

## Boundaries

No live-bus operation, no retry on the reporter's device, no vendor contact. Simulator and
software tests only. Coverage includes positive-element answers of 0, 1, 2, 3 and 5 octets,
zero-element refusal, true element-count retention, and exact 16/17-byte display boundaries.
A mismatched element count with exactly four bytes remains existing behaviour, not newly approved
wire semantics; decide its admission with the evidence-backed width model.
Does not claim hardware support or recovery. Shared commissioning code: read the restart snapshot
and coordinate with [PRG-01](PRG-01-programming-gaps.md) before editing.

## Done when

The raw answer survives into journal and report, the wrong-width behaviour (refusal or evidence-backed width model) is documented in
the System-2 contract and limitations, `docs/IMPLEMENTATION_STATUS.md` is updated, and the package gate passes.
