# Simulator gateway delivery

Observed: 26 October 2026.

## Delivered boundary

Published merge `f75820cad` integrates feature `03b6affcf` on main-first base
`5bb8d0766`. Independent whole-feature review and correction re-review approved;
a separate test-only follow-up also received APPROVE.

The UDP simulator front-end exercises the real cEMI codec, captures frames and
serves one tunnel plus one device-management connection. The encoder chooses
extended FT for `L > 15`; gateway lifecycle, reply failure isolation, ACK matching,
channel allocation and property-index error responses have regression coverage.
Documented simulator limits remain in [architecture](../ARCHITECTURE.md#simulator-gateway-front-end-25-october-2026-source).

## Acceptance

Both feature and actual merge passed the maintained change-scoped package gate
against `origin/main`: all 14 planned stages completed, input fingerprints stayed
frozen, and protected user-data touched/unchecked lists were empty.
The integrated Rust scope passed 3,305 tests with 135 ignored; selected private
corpus scope passed 91/91. This is not an execution of every ignored test.
Web/browser suites were not selected; the required web bundle was built.
Focused final gateway tests passed 20 cases and the example passed two.
Publication readback confirmed local, tracking and live main equality.

## Specification-only corrections

[Verify Mode audit](../spec-audits/2026-10-25-system2-verify-mode.md) records
connection-scoped PID 14 behavior, literal memory-write data including `FEh`,
and the absence of an established mask-0025 requirement ordering Verify Mode
relative to TaskSegment. No fixed-address identity mapping or hardware support
is inferred. The earlier source-mask/wire-data conflation is corrected.

## Open execution blocker

The existing `system2-validation` CLI asks its durable activity log to start kind
`system2Validation`, but the activity validator does not admit that kind. The
operation therefore refuses before sending. No activity guard was bypassed or
changed in this package. A complete application-download differential is not
established and requires a separately authorized journal-contract correction.
No real bus or hardware access was performed. This delivery grants no additional
family, recovery or compatibility admission.
