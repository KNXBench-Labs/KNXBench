# Property-based application run-state observation — 29 October 2026

## Scope and evidence

This is the source contract for a bounded PRG-01 software increment, not
hardware acceptance, an additional download family or a complete programming
correctness claim. Original KNX Standard PDFs from the authorised specification
corpus were inspected; relevant bodies, tables and printed footers were checked
independently. Licensed extracts and rendered pages remain private.

| Original source | Inspected clause / printed page | Established fact |
| --- | --- | --- |
| `03_05_01 Resources v01.10.01 AS.pdf` | §4.1.2.2 Table 7, p25 | `0700h`, `0701h` and `0705h` are listed BIM M112 masks. This does not admit every `070nh` implementation. |
| Same | §4.2.1, p30; §4.2.6, p31 | PID 1 identifies the hosting object's type; reading PID 6 observes that object's run state, whereas writing it delivers an event. |
| Same | §4.24.2, p298, footnote 94 | The property realization includes existing `0701h` implementations with `PDT_UNSIGNED_CHAR`; newer implementations use `PDT_CONTROL`. A mandatory description-type filter would incorrectly exclude the legacy case. |
| Same | §4.24.2.3.1 Table 95, p299 | Complete property-state octets are `00` Halted, `01` Running, `02` Ready, `03` Terminated, `04` Starting, `05` Shutting down. Higher values have no defined meaning in this table. |
| `03_05_02 Management Procedures v02.01.02 AS.pdf` | §3.34.3.4, p153; §3.36.3, pp157–158 | The property observation reads one 8-bit value, not the multi-octet write-control format, from the corresponding Interface Object at PID 6, start index 1, element count 1. |
| Same | §3.28.2, pp127–128 | Object scanning is a separate, conditionally enabled procedure. The property observation does not require exhaustive enumeration of every object. |
| `06 Profiles v02.01.01.pdf` | §4.2, p37; Annex A.2.6, p146 | Property-based run-state realization and PID 6 are required for the selected `0701h` / `0705h` columns. The Applicationprogram object has type `0003h`; this does not define its object index. Recommended access levels do not guarantee access in the current unauthorised connection. |

The new decoder interprets a complete property-state octet. It does not mask
upper bits, interpret an event as a state, or collapse Ready/Terminated into
Running/Halted. An undefined octet remains undefined, not a source-declared
reserved value or a guessed state.

## Bounded object location: an explicit interpretation

**Interpretation, not a universal fixed-index rule:** positively validating one
candidate's object type and exact plan application identity is sufficient for
an observation of that specific hosting object. This is not full discovery,
proof of uniqueness, selection of numbered management state machine zero, or
proof that a particular memory load-control machine maps to that object.

The existing regular post-download identity path already uses candidate index
3. The bounded continuation validates this candidate again: PID 1 must name
Applicationprogram type `0003h`, then a strict PID 13 reread must match the plan's
exact five-octet application identity. Only then may PID 6 be attributed to
this plan-matching hosting object. Type `0003h` is never converted into index 3.
A different layout remains unsupported by this continuation; no scan selects
another candidate arbitrarily. Other objects may exist without invalidating
this limited observation or creating a global uniqueness claim.

## Correlation and preservation

All three new reads use the existing strict raw property response matcher:
object index, PID and start index must correlate. Each returned element count
and width is checked separately. A correlated zero-element response is a
refusal; another correlated count or width is malformed. Wrong echoed
object/PID/start is ignored by that matcher: if no valid response follows,
there is no correlated answer, not an observed malformed reply.

An unavailable observation does not escalate authorisation or trigger a
memory read, restart, reset or resend. Earlier identity and load observations,
transfer facts and programming settlement remain unchanged. A read load state
that differs from the plan does not prevent the independent run-state read;
a failed or skipped load read does.

## Memory-path stop line

Management Procedures §3.36.2, printed p157, describes the `070nh`
connection-oriented memory reader at `0101h` for the application and `0102h`
for PEI state, one octet. It does not establish `0105h` as that observation.
Resources §4.24.3, printed p302, leaves the Type 2 memory realization
unspecified in this edition. Table 95 belongs to property-based Type 1;
its values cannot simply be borrowed for the memory octet.

Named missing evidence: `MISSING_MASK070NH_MEMORY_RUNSTATE_ENCODING`.
No memory-state decoder or fallback is admitted by this increment.

## Acceptance boundary

The [implementation contract](../contracts/post-download-run-state.md) owns
wire shape, failure distinctions and required tests. Actual executed software
acceptance and publication must be recorded separately. Even a reported
Running state is a point-in-time device observation, not installation function,
whole-device health, successful recovery or verified physical-exemplar identity.
