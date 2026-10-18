# Diagnostic export reads identity only

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-184)

<!-- BEGIN retained catalogue record -->

## Identity-only, simulator-verified export (17 October 2026)

`generic-identity-v1` reads Device Descriptor Type 0 and Device Object PID12 and
PID78. Configuration memory is never requested: no independently admitted safe
memory profile exists and product source offsets are not runtime addresses.
A zero-element property reply means denied access or an absent property; the
export records `refused-ambiguous` instead of guessing. Devices without property
services are expected to stay silent, ending the sequence with a timeout or a
released connection; this is an assumption, unverified on hardware. The
cross-process lease covers KNXBench processes of the same user only. Activity
history rows of kind `deviceDiagnostics` are refused by the history listing of
builds older than this feature (downgrade only). Lifting:
a separately authorised read-only hardware run, then a memory profile only with
its own evidence. [Contract](../contracts/device-diagnostic-export.md),
[ADR-0134](../adr/0134-generic-read-only-diagnostic-export.md).
<!-- END retained catalogue record -->

## First real-device run (18 October 2026)

One read-only export from a Siemens `MV-0025` presence detector (issue #22):
descriptor `0025`, PID12 `0001` and a six-octet PID78 all answered, nothing was
written and the disconnect attempt completed
([evidence](../research/2026-10-18-up258-device-diagnostics.md)).
The identity-only boundary is unchanged. A refusing or silent device, the
`refused-ambiguous` path and post-run device behaviour are still unobserved on
hardware.
