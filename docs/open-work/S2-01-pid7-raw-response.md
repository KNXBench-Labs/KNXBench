# S2-01 — System-2: keep the raw PID 7 answer and model its width

**Priority:** P1. **Status:** In progress. Deliverable 1 is implemented on a branch (the refusal names object, element
count, octet count and the first 16 raw octets, simulator-tested); deliverables 2 (width model) and 3
(research record) wait for a real observed answer.
Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30, field report of a
refusal `resource pointer response is not four octets` on a real UP 258D12 (two operations,
`mutation_may_have_started: false`).

## Problem

The pre-write recovery acquisition reads `PID_TABLE_REFERENCE` (PID 7) per resource and
requires exactly four octets (`read_pointer` in `crates/knx-net/src/commissioning/property_recovery.rs`).
Any other length aborts the run before the first write and discards the answer. The report
therefore proves only "not four octets": the observed element count and bytes are lost, and the
open wire-width question ([up258 validation, row 2](../contracts/up258-system2-validation.md))
stays open although a real device has now answered it with a different width.

The earlier Alpha 16 refusal ("current resource pointer differs from original source allocation
shape") compared raw bytes against a four-octet expectation, so it may have been a width mismatch
too. This is a reading of the code, not a measurement; the "pointer = last allocated segment start"
explanation given in #30 is withdrawn until raw bytes exist.

## Deliverables

1. **Raw observation (done in source).** `read_pointer` reads with `read_property_counted` (same
   start-index matching as before) and the refusal carries element count and raw bytes (no device
   configuration), so the failure text in journal and status shows them. A structured CLI/web
   field is not added: the failure text is the only carrier today. A width other than four is an observation with its true bytes, never padded
   or truncated, and never labelled as a pointer value.
2. **Wrong-width rule: the refusal stays until the width is understood.** An observation-only
   rule is rejected for now, because a backup cannot cover this case: the recovery snapshot holds
   a four-octet pointer per resource, and `recovery_plan` already refuses original-state recovery
   whenever the prestate differs from the source shape. A run that continues past a wrong-width
   answer would therefore have no original-state recovery, only a fresh ETS download as the way
   back. Independently, the masked 338 bytes cannot be shown to survive Unload
   ([up258 validation, row 1](../contracts/up258-system2-validation.md)), so even a clean backup
   would not prove a way back. Writes use the plan's absolute addresses, not the read pointer, but
   a device that answers PID 7 in another width shows that the firmware's resource model differs
   from ours in a way we cannot yet quantify.
   The fix is to model the width once the raw bytes are known: a width-aware pointer in the
   snapshot and a recovery path for it, admitted only with evidence for that firmware, so that a
   usable backup exists. Until then the refusal before the first write stays, now carrying the
   element count and the bytes. The post-download strict checks (memory read-back, Loaded, written
   program version) stay hard in every variant.
3. **Research record** in `docs/research/`: the observed width as a field fact (one device, one
   firmware, unverified for others), kept apart from the standard's four-octet definition.

## Boundaries

No live-bus operation, no retry on the reporter's device, no vendor contact. Simulator and
software tests only; add tests for 0, 1, 2, 3, 5 octet answers and a mismatched element count.
Does not claim hardware support or recovery. Shared commissioning code: read the restart snapshot
and coordinate with [PRG-01](PRG-01-programming-gaps.md) before editing.

## Done when

The raw answer survives into journal and report, the wrong-width behaviour (refusal or evidence-backed width model) is documented in
the System-2 contract and limitations, `docs/IMPLEMENTATION_STATUS.md` is updated, and the package gate passes.
