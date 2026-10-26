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

## Execution blocker (resolved 26 October 2026)

At delivery, the existing `system2-validation` CLI asked its durable activity log
to start kind `system2Validation`, but the activity validator did not admit that
kind, so the operation refused before sending. A separately authorized,
test-first correction now admits that kind with the unchanged property-write
states and `writeEvidence` rules; the format-2 storage, strict reader, reopen
projection and Web admission treat it exactly like `serviceControlWrite`
([contract](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md#coverage-and-outcomes)).
Inconsistent rows of the new kind stay refused and latch `unavailable`.

The correction exposed two lifecycle defects. A failure before any send
evidence (for example an unreachable gateway) claimed `effectUnverified`, so the
write guard panicked and left `unknown`; the CLI now records `notSent` there.
Independent review then found the CLI marked backup and send evidence before
connecting, so a refusal before any recovery or mutation would have been stored
as `effectUnverified` with a recorded backup. The operation store now runs that
journal hook at the executor's recovery barrier, immediately before Verify Mode;
download and recovery replay are covered through the simulator gateway.

With both corrections the built CLI completed its simulated application download
through the simulator gateway, isolated with the client in a loopback-only
network namespace, at device `MAX_APDU_LENGTH` 55 and 15: outcome `finished`,
journal `verified` with backup and send evidence, reopened by a fresh strict
reader. The tunnelled path caps memory writes to standard frames (12 octets) at
both lengths. This is simulator evidence only: no real bus or hardware access
was performed, gateway PID 56/83 values remain assumptions, and this grants no
additional family, recovery or compatibility admission.
