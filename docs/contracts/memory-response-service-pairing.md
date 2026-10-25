# Memory response service pairing

## Rule

A memory read, and a memory write under active Verify Mode, are answered only by a response of
the service the request used, at the requested address. The standard-memory service
(`A_Memory_Read`, `A_Memory_Write`) takes `A_Memory_Response`; the user-memory service
(`A_UserMemory_Read`, `A_UserMemory_Write`) takes `A_UserMemory_Response`. The caller chooses the
service: the region paths take it from `service_for`, the fixed load-state read uses standard
memory, and the recovery and download paths pass their own.

A response of the other service is not the answer: it is acknowledged at the transport layer
like any numbered frame, then ignored, and the wait continues under the unchanged answer ladder.
With no matching response the call ends as `NoAnswer`. This holds when the other service carries
the same numeric address, octets or no octets. A response of the requested service at another
address is no answer either and ends the same way. An empty response of the requested service is
unchanged: it is `MemoryRefused` at that address, and a same-service octet mismatch is still
`ReadBackMismatch`. Without active Verify Mode the write is read back, and that read-back
follows the read rule: a response of the other service ends the write as `NoAnswer` after one
write and one read-back.

Response count, retry ladder, acknowledgement and sequence handling, write retry policy and the
`service_for` policy are unchanged. No smaller-service fallback is added.

## References

AL v02.01.01 maps a read to the response PDU of its own service in §3.5.3 (pp. 109-111) and
§3.5.6.2 (pp. 117-119), and an active-Verify write to the response PDU of its own service in
§3.5.4 (p. 112) and §3.5.6.3 (p. 120). Within one service a single response PDU serves both read
and write. No clause found there makes one service's response the confirmation of the other's.
The two have different APCI codes (Table 1, p. 9). The paraphrase is ours; no text is copied.

## Evidence boundary

This is robustness against a synthetic, nonconforming peer. No real device has been observed
answering with the other service, and no aliasing, data-loss or address-space claim follows.
The tests wrap the simulator, rewrite its memory response into the other service through the
actual cEMI encoder and decoder before the session sees it, and check the frames, acknowledgements
and exact request schedules on the device: one read, one active-Verify write, or one write
followed by one read-back without active Verify Mode. Both standard and user memory are covered.
They establish neither hardware nor interface-coupler compatibility, nor any recovery behaviour
beyond the `NoAnswer` the unchanged ladder gives.

## Harness limits

The wrapper remaps the sequence of a transport acknowledgement it swallows or forwards. It does
not remap a negative acknowledgement, and no case injects one, so none is tested here. A future
case that makes the session send one needs that mapping first.
