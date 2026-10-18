# ADR 0143: A ProductDB-declared, read-only memory evidence profile

Date: 2026-10-18
Status: Accepted (offline/simulator scope; no hardware run)
Session: UP258 stage 0
Supersedes in part: [ADR-0134](0134-generic-read-only-diagnostic-export.md)
decision 4 and its rejected alternative *bounded memory reads from product
source segments*, for the one profile named here only.

## Context

Stage 0 of the [UP 258D12 write ladder](../goals/up258-programming-2026-10-18.md)
needs the device's ETS-programmed memory so it can be compared offline with
KNXBench's interpretation of the source. ADR-0134 rejected memory reads because
source offsets are not proven runtime addresses and configuration memory can be
sensitive. Both facts still hold. What changes is the scope of the question:
the comparison needs only the ranges the application's own load procedure
declares, read from the device it was written for.

Verified facts this relies on:

- The public package of `M-0001_A-9205-04-9270` declares four
  `LdCtrlAbsSegment`s (`0116h`/330, `0260h`/416, `0400h`/512, `3C00h`/256 =
  1514 octets) and one Device Object PID 78 comparison
  (`system2_public_package.rs`, digest-pinned).
- `system2::project_tool_metadata` already accepts exactly this one-procedure
  System-2 shape and refuses every other one.
- `A_Memory_Read` answers a failure with zero octets (AL §3.5.3); a standard
  frame carries at most 12 data octets (RES §4.3.7.2.1, 15 − 3).
- The owner/lead decided the profile's shape on 18 October 2026 (goal doc).

Assumption, not verified: a System-2 device answers `A_Memory_Read` in these
ranges without side effects. No hardware has exercised this profile.

## Decision

1. A second diagnostic profile, `system2-declared-segments-v1`, exists beside
   the unchanged `generic-identity-v1`. Its ranges are derived only from
   ProductDB data (`knx_productdb::declared_segments`): the precondition is
   `project_tool_metadata`, then every `AbsSegment` in source order and the one
   Device Object PID 78 `CompareProp`. No product is named in code.
2. The derivation refuses whole — no partial profile — on another shape, a
   missing or repeated PID 78 comparison, empty inline data, a zero size, a
   range reaching `FFFFh` (where CP §3.5.2 switches to the user-memory
   service), overlapping ranges, more than 8 ranges or more than 2048 octets
   (`knx_core::commissioning::declared_memory`).
3. Memory is read only after the device shows all three asserted identity
   values exactly: descriptor = declared mask, PID 12 = manufacturer, PID 78
   `property_matches` the comparison. Mask and manufacturer assertions are
   required; without them the collector refuses before connecting. A mismatch
   or an inexact identity read stops with a closed token and sends no
   `A_Memory_Read`.
4. Reads are `A_Memory_Read` of 12 octets (the session's negotiated limit
   capped at a standard frame, which is always 12; no `PID_MAX_APDU_LENGTH`
   read is needed), in a read-only session without authorisation, one item per
   chunk. A refused chunk ends its range (`refused-ambiguous`, rest
   `not-read`); another length is `partial` with its bytes kept and ends its
   range; timeout, connection loss, cancellation or the 180 s deadline end the
   run with earlier chunks kept. `T_Disconnect` is always attempted.
5. The archive becomes format 2 for this profile only: a fifth fixed member
   `memory.json` (provenance, chunks, per-range coverage) and plain notices
   that the ranges are candidate locations, not a backup and not a restore
   basis, and private. Format 1 stays byte-identical. The reader recomputes
   the chunk plan and coverage and refuses evidence that does not follow from
   them, memory read without the recorded identity matching the assertions,
   chunk orders the collector cannot produce, and duplicate ZIP entries
   (checked in the raw central directory, which must be the one the `zip`
   crate reads: no ZIP comment, same offsets). Observer snapshots with the
   collector's one in-flight chunk stay exportable. Only the ProductDB derivation can
   build the acquisition plan.
6. Entry point: the CLI only (`--profile`, `--product-db` required, preview
   without `--confirm-read`). The web/HTTP entry still offers only the
   identity profile.

## Alternatives considered

- **Generic dump or caller-supplied ranges.** Rejected: unbounded scope, and
  no product data justifying any address.
- **Read the `Static/Code` segments instead of the procedure's `AbsSegment`s.**
  Rejected: the procedure is what ETS executes; static segment placement is a
  separate, unproven mapping.
- **Read `PID_MAX_APDU_LENGTH` to size chunks.** Rejected: capped at a
  standard frame the result is always 12, so the read adds a bus exchange
  without changing anything.
- **Admit the profile to the web UI now.** Deferred: UI work has another owner.

## Consequences

- A reporter can produce one bounded, read-only offline-comparison artefact
  for the exact device; the comparison itself is separate work.
- Format-2 archives are readable only through `read_export`; `read_bundle`
  refuses them rather than dropping the memory evidence.
- A hardware claim still needs a separately consented run on a named device.
  Nothing here admits a write, a download or a restore.
