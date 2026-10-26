# S2-01 — System-2: width-aware PID 7 evidence and safe diagnostic projection

**Priority:** P1. **Status:** Private diagnostic delivered (`5f1ad454e`);
exact-profile width correction published as `54b052885`, reviewed and integrated-gate accepted
([record](../status/2026-10-26-up258-pid7-width.md)). The safe structured
status/report projection is implemented as a candidate in the working tree,
**pending independent review, full gates and parent acceptance**; it is neither
published nor accepted, so this item stays open
([dossier](../status/2026-10-26-system2-safe-diagnostic-projection.md)).
Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30.

## Current evidence

The [diagnostic acceptance](../status/2026-10-25-system2-pid7-diagnostic.md)
records the delivered private raw-prefix diagnostic. The 26 October reporter
reply now supplies object 1, one element, two octets, `0116`, with no mutation
reported. The build contains both diagnostic fixes. Objects 2/3 and the device's
property description remain unmeasured.

An independently retrieved, digest-pinned public manufacturer package declares
PID 7 datatypes of two and four octets and two-octet table-pointer resources on
objects 1/2/3 for mask `0025h`. The
[research record](../research/2026-10-26-up258-pid7-width.md) separates that
metadata from the observed reply and the standard's general four-octet rule.
The earlier "raw bytes unknown" blocker no longer applies to object 1. The
Alpha 16 byte comparison could itself have mistaken width for value difference;
its original numeric-mismatch explanation was never a measured finding.

## Width correction acceptance boundary

The published correction is limited to a new revision of the already exact-UP258 configured
hardware record. Its explicit, hash-bound policy admits exactly one element of
two or four octets on objects 1/2/3. Object 1 has one field observation; objects
2/3 remain source-declared experimental hypotheses, not verified firmware support.
Old format-1/2 bundles and format-3 revision-1 operations keep their original
four-octet interpretation, including recovery of retained operations.

The fix must preserve typed raw two/four-octet values through acquisition,
in-procedure/final observations, private persistence, reopening and explicit
recovery. Numeric source-address equality must not manufacture padded raw data.
A representation change from the captured prestate must survive as an
observation even when its numeric address agrees. Unknown widths, invalid new-
policy counts, malformed records and unbound policies still refuse. An older
reader must reject new policy/representation data rather than discard it.

A usable software path requires original semantic RED, complete simulated
execution, disk reopening and explicit recovery, legacy byte/digest preservation,
malformed/tamper cases and the existing hard final checks. Those focused software
paths passed on the published source, including the complete 15-stage integrated gate.
The small typed, non-raw Net refusal seam is implemented for the next slice;
it is not by itself durable status/report projection.

## Remaining safe projection (implementation pending acceptance)

The private failure journal retains the bounded raw diagnostic. Raw text, arbitrary
exception text and raw bytes stay out of HTTP/shareable output. A candidate
implementation now carries a closed six-key typed diagnostic `{run, reason, object,
propertyId, elements, octets}` from the read through a strictly versioned
checkpoint/reopen contract, DTO, shareable report and EN/DE panel
([contract](../contracts/system2-safe-failure-diagnostic.md)). Reasons are closed
(`zeroElementsWithData`, `elementCount`, `unsupportedWidth`); `object`,
`propertyId` and `elements` are u8, `octets` is u32, and counts may be 0. Conversion
is typed only: legacy and untyped failures (including legacy-policy refusals) and old
journals stay without a diagnostic and are never backfilled from free text.

Open until accepted: independent review, the maintained full gates and the parent's
acceptance decision. Until then no count, revision or delivery is claimed. A
diagnostic is a bounded observation, not a hardware-support grant, a retry or a
recovery route. The shared journal/error surfaces were coordinated with the width
owner (already published as `54b052885`).

## Retained boundaries

No live-bus operation, reporter retry, vendor contact or hardware support claim.
Writes use plan-absolute addresses, not observed pointers. Different/unreadable
prestate can still make original-state recovery unavailable. Complete allocation
memory, final Loaded state and written program identity remain hard checks.
Masked-byte survival across Unload and complete physical recovery remain open
([validation boundaries](../contracts/up258-system2-validation.md)).

Issue #30 remains open until its actual requested outcome is established.
[S2-02](S2-02-gateway-busy-after-failed-runs.md) remains a separate investigation;
a start that succeeds on a newer build does not explain its earlier failure.
