# S2-01 — System-2: width-aware PID 7 evidence and safe diagnostic projection

**Priority:** P1. **Status:** Private diagnostic delivered (`5f1ad454e`);
exact-profile width correction implemented locally; final acceptance/publication pending
([record](../status/2026-10-26-up258-pid7-width.md)). Safe structured
status/report projection remains a separate open slice.
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

The candidate is limited to a new revision of the already exact-UP258 configured
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
paths are exercised in the local candidate; final maintained acceptance is pending.
The small typed, non-raw Net refusal seam is implemented for the next slice;
it is not by itself durable status/report projection.

## Remaining safe projection

The private failure journal retains the bounded raw diagnostic. Do not expose
that diagnostic, arbitrary exception text or raw bytes to HTTP/shareable output.
A follow-up must carry a typed allowlist (closed reason, object, property and
counts) from the read through a strictly versioned checkpoint/reopen contract,
DTO, report and EN/DE UI. Legacy absence must remain unknown; never parse old
free text to manufacture structured evidence. Coordinate its shared journal/error
surfaces with the width owner before editing.

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
