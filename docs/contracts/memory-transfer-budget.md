# Memory transfer data budget

## Rule

`knx_core::commissioning::memory::write_limit` keeps interpreting the reported
`PID_MAX_APDU_LENGTH` unchanged: 12 to 63 data octets for ordinary memory.
Two rules sit on top of it.

1. **Service budget** `[D]`. `WriteLimit::max_octets_for(service)` is the data
   count one PDU may carry. Ordinary memory keeps `max_octets()`. User memory
   carries one octet less, because its header is three octets against two for
   ordinary memory (the count is not inline), capped at 15:
   `min(15, max_octets - 1)`. At the standard frame that is 12 and 11; the
   user budget reaches 14 at a reported length of 18 and 15 from 19 on.
   Core `chunks` and the regular read path use this one helper and keep the
   single whole-region service choice (`base + length`, exclusive).
2. **Unproved route** `[A]`. `ManagementSession::write_limit` lowers a limit
   above 12 to 12 (`WriteLimit::capped_to_standard_frame`, reason
   `CappedToStandardFrame`) when the transport reports
   `TargetKind::Hardware`, which is also the default of a transport that says
   nothing. The reported value is read first, preserving existing property
   refusal/fallback handling and malformed-reply errors before the cap.
   A limit already at 12 keeps its
   value and reason. Simulator transports keep their modelled capacity.

The cap is a conservative policy for a route whose capacity has not been
established. It is not a measured interface or coupler capacity, not a rule
that extended frames must be avoided, and not an authorisation: `TargetKind`
never permits a write.

## Invariant

Every production constructor of `WriteLimit` yields 12 to 63, so
`max_octets - 1` cannot underflow and no chunk or read step is zero. The unit
tests check every reported 16-bit value for all source kinds.

## Consequences

Throughput over real transports may fall, since transfers use smaller
requests; no factor is claimed. Reads and writes that cross `FFFFh` now split
11 plus 5 (standard frame) where 12 plus 4 was used before. No new extended
path or route capability is introduced, and family admission, registry and
recovery behaviour are unchanged. Existing consumers: region writes,
backup/compare reads, byte-order probe reads and property recovery reads.

The connect step's progress note labels its figure as ordinary-memory data
octets per PDU, because user-memory requests carry one octet less.

## Evidence boundary

Synthetic simulator tests, including a hardware-kind wrapper around an
in-process simulated device, check exact request schedules, byte patterns
and the encoded length field (15 for 12 ordinary or 11 user-memory data
octets) through the public encoder. No real device, interface or coupler was
contacted; nothing here supports a hardware compatibility claim. See the
[research note](../research/2026-10-25-memory-transfer-apdu-budgets.md).
