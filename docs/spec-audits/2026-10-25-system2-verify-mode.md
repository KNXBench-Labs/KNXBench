# System-2 Verify Mode and literal memory-write data

Date: 2026-10-25.

## Scope

Specification audit of the existing management session and simulated device for
mask `0025h`. This is not hardware validation or a new compatibility claim.

## Literal write data

Application Layer `03_03_07` v02.01.01 §3.5.4, printed pp111–113,
defines the payload as the octets to write. No payload-octet escape is defined:
`FEh` is literal data, not an in-band mask. Source/image masks determine which
bytes are omitted before transmission and must not reinterpret transmitted data.
The [earlier procedure note](../research/2026-10-22-system2-ets-download-procedure.md)
is corrected accordingly. The simulator's `memory_write` already stores literal
bytes; no runtime change is warranted for this distinction.

## PID 14 and verified writes

Resources `03_05_01` v01.10.01 §4.2.14, Table 11, printed p34,
assigns Device Control bit 2 to Verify Mode. §§4.2.14.7.1–4,
printed pp35–36, define connection-scoped setup: default off, enable only
with a Transport Layer connection, actively set after a new connection,
automatically clear on disconnection. The simulator initially returns `00h`,
accepts `04h` with its default supported-mode configuration, and clears the bit
when the connection closes.

Application Layer §3.5.4, printed pp112–113, requires no application response
while Verify Mode is inactive. When active, the response carries the memory
explicitly read back after writing; a failed write is represented by zero count
and no data. It is not merely an unconditional echo of the request. The existing
simulator models stored-byte readback and its corruption fault injection
separately. Management Procedures `03_05_02` v02.01.02 §3.16.3,
printed pp101–102, defines the corresponding client verification procedure.

## Verify Mode versus TaskSegment

Resources §4.2.14.7.1 limits Verify Mode's effect to memory-write services.
AllocAbsTaskSeg is a property-based load-control record (Management Procedures
§3.31.3.4, printed p140), not a memory-write operation. No defining clause
inspected requires a particular relative placement of those two operations for
mask `0025h`. Setting Verify Mode before the first load-control record therefore
is not a demonstrated defect. Configuration Procedures `03_05_03`
v02.01.01 §3.9.2.2.2, printed pp67–68, is a `5705h` example and must not
be promoted into a `0025h` procedure contract.

## Explicit unresolved boundaries

- Profiles v02.01.01 §4.2, printed p37, marks Verify Mode mandatory for
  System 2 / BCU 2; footnote 8's optional-off case belongs to System 1 / BCU 1.
  Resources §4.2.14.7.2, printed p36, omits `0025h` from its usage list.
  This cross-volume discrepancy remains unresolved. An injected unsupported-mode
  simulator case does not establish that a conforming `0025h` device may omit it.
- AllocAbsTaskSeg carries a five-octet application/table identity, but its
  definition does not assign that identity to memory `0103h`–`0107h` or `0109h`.
  Do not add fixed-address writes or implicit memory side effects from that
  similarity alone. Resources §4.2.13, printed pp33–34, defines Program Version
  separately and makes its configuration use profile-dependent.
- This bounded audit does not certify every simulator service, device profile,
  malformed count case, or hardware effect. Existing safety/recovery gates stay.
