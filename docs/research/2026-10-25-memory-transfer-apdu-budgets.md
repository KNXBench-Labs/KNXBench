# Memory transfer APDU budgets (25 October 2026)

## Question

Does an `A_UserMemory_*` PDU carry as many data octets as an `A_Memory_*` PDU
of the same APDU length, and how far should an unproved route follow a
reported `PID_MAX_APDU_LENGTH`?

## Primary sources, checked directly

- Application Layer v02.01.01, 3.5.6.2 (printed page 119) and 3.5.6.3
  (printed page 122), with Figure 81 (printed page 120): the user-memory
  services put the count in its own four-bit field beside the address
  extension, so the header takes three octets where the ordinary memory
  services take two with the count inline. `[D]`
- Resources v01.10.01, 4.3.7.2 (printed page 46): the range and meaning of
  the maximum APDU length and the 15-octet default management frame. `[D]`
- The repository's own cEMI encoder (`crates/knx-net/src/cemi.rs`) shows the
  same header difference independently. Paraphrase only; no source text is
  reproduced here.

## Conclusions

- `[D]` User-memory data budget is the ordinary one minus one, capped at 15.
  The previous code applied only the cap, so a 12-octet limit produced a
  user-memory PDU one octet longer than the limit allows.
- `[A]` Holding hardware-kind transports to 12 data octets is a conservative
  repository policy for a route whose capacity is unproved. It is not derived
  from the Standard and not a measurement.

## Not established

Real interface or coupler capacity, extended-frame behaviour of any product,
and the throughput effect. The contract is
[memory transfer budget](../contracts/memory-transfer-budget.md).
