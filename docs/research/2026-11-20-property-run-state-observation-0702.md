# Property run-state observation for exact `0702h` — 20 November 2026

## Scope and evidence

This note picks the smallest next PRG-01-N1 gap after the accepted
`0701h`/`0705h` masks and the descriptor-qualified exact `0700h`, and records
what the original sources say about the next mask in the `070nh` family.
Original KNX Standard v3.0.0 PDFs from the authorised local corpus were
inspected again; printed footers were checked. Licensed extracts stay private.

| Original source | Inspected clause / printed page | Established fact |
| --- | --- | --- |
| `03_05_01 Resources v01.10.01 AS.pdf` | §4.1.2.2 Table 7, printed p. 25 | Only `0700h`, `0701h` and `0705h` are listed as approved BIM M112 mask versions. `0702h` is **not listed anywhere in Table 7**; the family is `070nh` by the low nibble, but approval is per exact mask. |
| Same | §4.24.2.3.1 Table 95, printed p. 299, with footnote 94 on p. 298 | Footnote 94 names exactly two legacy exceptions to `PDT_CONTROL`: mask `0020h` and mask `0701h` (`PDT_UNSIGNED_CHAR`). `0702h` is not among them: a `0702h` Type-1 run-state property is `PDT_CONTROL` like any non-legacy mask. The six Table 95 state octets apply to the property-based realization regardless of mask. |
| Same | §4.2.6, printed p. 31 | PID 6 reads the hosting Interface Object's run state for every object, not only `0701h`. |
| `06 Profiles v02.01.01.pdf` | §4.2, printed p. 37 | The System 7 columns list `0700h`, `5705h`, `0705h`, `0701h`. `0702h` has no System 7 column: PROF does not profile it. Its Applicationprogram object table (Annex A.2.6, p. 146) lists `0700h`/`0701h` (and `0705h`) rows only. |
| `03_05_02 Management Procedures v02.01.02 AS.pdf` | §3.31.2, printed p. 134 | The memory-mapped load-control procedure is restricted to "device model for mask version `070nh` (BIM M112)" — the `n` is the low nibble, so `0702h` is inside the memory family the existing download path already admits (`loads_through_memory`). |
| Same | §3.35.2/§3.35.3, printed pp. 154–155 | The property-based `DM_RunStateMachineVerify` (`DMP_RunStateMachineVerify_RCo_IO`) reads `PID_RUN_STATE_CONTROL`, start 1, count 1, from the searched Interface Object, with a conditional description step. It names no mask and prescribes no fixed object index; §3.28 (pp. 127–128) keeps object scanning a separate, conditionally enabled procedure. |

## What this establishes, and what it does not

1. **The `070nh` memory-load family is one thing; the approved BIM M112 mask
   list is another.** `loads_through_memory` admits every `070nh` mask for the
   *download*, but Resources Table 7 approves only `0700h`/`0701h`/`0705h` as
   BIM M112 device masks, and PROF profiles those three (plus `5705h`) as
   System 7. `0702h` sits inside the `070nh` procedure family without being an
   approved or profiled BIM M112 mask in this edition. KNXBench therefore treats
   it as an unproven-but-not-forbidden member: the download path is unchanged,
   and the run-state *observation* may be attempted under the same bounded
   candidate rules, reported as exactly `0702h`.
2. **No legacy datatype exception applies to `0702h`.** Footnote 94 limits
   `PDT_UNSIGNED_CHAR` to `0020h`/`0701h`. So for exact `0702h` the stricter
   rule is source-backed rather than merely conservative: the PID 6 property,
   if present, is `PDT_CONTROL`, and — exactly as for `0700h` — independent
   description evidence must qualify it before a one-byte value read. A
   one-byte value alone is never datatype evidence.
3. **The observation stays bounded exactly as before**: one existing fresh
   read-only session, identity match, every load state read, candidate object
   index 3 validated by PID 1 = `0003h` and a strict PID 13 reread equal to the
   plan's five-octet application identity, then (new for `0702h`) one
   `A_PropertyDescription_Read` of PID 6, then — only when qualified — one
   PID 6 value read. No scan, no second index, no memory fallback, no new
   session, retry or wait.

## Residual boundaries (unchanged, named)

- `MISSING_MASK070NH_MEMORY_RUNSTATE_ENCODING`: Resources §4.24.3 (printed
  p. 302) leaves the memory-mapped Type-2 realization unspecified, and MP
  §3.36.2 (printed p. 157) names `0101h`/`0102h` for the *memory* run state —
  that is a different realization and remains out of scope. No memory decoder
  is admitted.
- Generic object discovery (§3.28) remains out of scope; candidate index 3 is
  the documented default layout, not a universal rule.
- PROF has no `0702h` column, so no profile-level capability (mandatory PID 6,
  access level) is claimed for it; the observation is optional-property
  evidence only, like exact `0700h`.
- Physical function, whole-device health, physical-exemplar identity and
  complete mutation/recovery remain unproved. Simulator/loopback evidence
  only; no hardware claim.
