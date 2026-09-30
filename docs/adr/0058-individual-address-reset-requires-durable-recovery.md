# ADR-0058: Public individual-address reset fails closed without durable recovery

- Status: Accepted (2026-09-30)
- Scope: production `knx device reset-address` (MP §2.18), offline change only

## Evidence and problem

The CLI's confirmed reset previously passed a valid target-bound phrase to
`individual_address_reset` and opened a KNXnet/IP tunnel. Its first broadcast
read checked that exactly the operator-named devices were in programming mode,
but neither that check nor the phrase durably backed up their affected storage.
The generic `device_backup` file covers regions selected by a *download plan*,
not every manufacturer-specific effect of an address reset. ADR-0057 and
RESEARCH §22 explain why a previous address or an arbitrary memory dump cannot
prove a complete backup. No access key may be guessed.

A specifically authorized earlier run on `1.1.67` restored its address and
observed an unchanged application dump (RESEARCH §19.16). That is evidence for
that run, not proof that the public CLI automatically saves a complete durable
pre-write recovery record for every future device or simultaneous pressed
receiver. Its unevaluated restart did not end programming mode. No HTTP reset
route exists; a past prototype was not published (KNOWN_LIMITATIONS §140).

## Decision

1. After address and phrase validation, but before runtime construction and
   tunnel opening, the public confirmed CLI reset invokes an application-layer
   recovery precondition. While complete affected-storage scope, durable
   persistence/readback and a deliberate recovery/abort plan are absent, it
   always fails with a clear no-write message. No flag or environment bypass.
2. Plan-only mode remains usable and explicitly says that confirmed writes are
   blocked. Invalid addresses and confirmation phrases still fail validation
   first. The MP §2.18 protocol implementation and direct simulator tests stay
   intact; they test telegram behavior, not public hardware authorization.
3. Do not publish a reset HTTP/UI route or imply that a prior live approval,
   programming-button observation, a successful tunnel send or an address
   reprogramming command satisfies this recovery gate. Low-level Rust callers
   of `knx-net` are outside the public CLI guarantee.

## Reopening criteria (not authorization)

For each supported device class/mask, establish which storage can change and a
read-only way to capture *all* affected values, with operator-supplied keys if
required. Identify every pressed device without ambiguity; durably write and
read back a versioned, target-bound pre-send record for each; refuse incomplete
or unverified captures before broadcast. Test cancellation, transport failure,
multiple pressed devices, and a separately authorized recovery path. Only
then reconsider the public write with a fresh device-specific go. Existing
application-memory backup or an observed unchanged dump is not a substitute.

## Verification boundary

A simulated CLI integration test binds a local UDP listener and proves the
confirmed command exits with a missing-backup error without sending a datagram;
plan and validation regression tests remain. No gateway, device, UI or live
write is touched by this decision.
