# Issue #30: patient experimental System-2 waits

## Scope

The owner requested substantially larger bounded timeouts and a useful next
source build for an optional reporter retest. This package changes patience,
not compatibility admission or the definition of download success.

[Contract and values](../contracts/system2-patient-waits.md): gateway ACK
1 → 10 seconds per attempt, post-ACK management answer slices 3 → 15 seconds and
load-state transitions 30 → 120 seconds. The device transport ACK/repetition wait
remains three seconds; device connection, poll, heartbeat,
programming/restart timers and repetition limits retain their existing values.
Ordinary download, monitoring and scan paths retain their standard timings.

The shared server/desktop System-2 worker selects the longer profile for its
hardware observation, download and explicit recovery; the experimental hardware
CLI uses the same policy. No arbitrary timeout configuration or whole-operation
retry is introduced. The receiver/teardown correction `1ab4ad6a5` is already
part of the source base. Historical Alpha 17 artifacts do not acquire either
change by restart alone.

## Evidence and limits

The field evidence is the public issue #30 thread, not a privately inspected
journal or our own device measurement. The latest reporter capture adds timing,
CPU/storage and exclusive-client information for the **same** earlier attempt
on `ae47331`, which predates `1ab4ad6a5`; it is not a failed retest of that fix.
The reporter confirms normal detector function and no new attempt.

The 30.040-second checkpoint gap includes hardware checks, device prestate reads,
snapshot/model validation and persistence. It is not an fsync duration or proof
of a silent 30-second connection. The faster-host facts weaken the earlier
slow-host explanation; the original cause remains unknown. The public
acknowledgement `6454382575` corrects that inference and describes the bounded
change. No new bus attempt was requested before publication.

Original RED, corrected development runs and final acceptance must remain
separate. The actual delayed-ACK controls exercise the UDP client, while
synthetic device/virtual-time cases exercise management answer and transition
waits. Constants-only checks are supplementary. Failed setup or wrong-fixture
runs are not the missing-behaviour RED. Repeated focused and package executions
overlap and are not additive unique coverage.

## Review corrections

The initial whole-feature review found no fundamental blocker but required
separating device transport ACK timing from the longer answer wait. The final
policy keeps the three-second pre-ACK repetition cadence; only the first
positive ACK starts a 15-second answer slice. A follow-up found that duplicate
ACKs renewed that deadline. Its regression failed on the candidate with 140
seconds elapsed instead of the bounded 60-second answer ladder; the correction
ignores duplicate ACKs for deadline renewal and the six-case timing suite passes.
This is candidate-correction evidence, not a reproduction of the field failure.
A RouteBack connector wire regression also rejects a client-construction mutant
that would lose the configured return path. Standard profiles retain the prior
answer timing through an absent optional override.

## Acceptance boundary

Published source: main-first merge `3583ab8e0`, containing feature `6f7d22228`
and the ordinary-server persistence follow-up `ca940e2ed`. Final independent
confirmation is APPROVE. Branch and integrated maintained package gates each
passed all 14 selected stages with frozen inputs and no touched or unchecked
user data. Branch: 3,472 Rust tests passed, 140 ignored; integrated: 3,479 passed,
140 ignored. Each selected corpus run passed 91/91. These overlapping executions
are not additive unique coverage. No frontend test, browser execution or bindings
stage was selected; the web bundle was built as a desktop dependency.

The integrated server was built and run with isolated state, using loopback HTTP
only: `/healthz` and `/` returned 200, version
`knx-server 0.1.0-alpha.17+g3583ab8e0`. No device endpoint was called.
Local HEAD, tracking and live remote refs matched at publication. On 28 October
2026, issue comment `6455873678` requested one optional separately confirmed
retest on this source or a descendant, with evidence/data backup, ETS fallback
and stop-on-failure boundaries; its exact body was read back. No release, updated
image, deployment, hardware outcome or issue closure is claimed.

The first branch controller failed during private runner import before product
checks because a helper shadowed Python's `inspect` module. That setup refusal
is retained separately; the corrected runner's branch gate is the accepted run.

## Preserved safety and follow-up

Target/source admission, actual prestate backup and its durable readback remain
before setup writes. Final memory/identity/Loaded verification remains hard.
A matching transport ACK cannot cause extra mutating application retransmissions;
an explicit peer close remains terminal even inside the larger wait. A missing
answer remains failure, and an incompatible prestate can still make original-
state recovery unavailable. No automatic retry, restore or restart is added.

The [operator guide](../manual/user-guide/05-devices-and-products.md#more-patient-experimental-waits-source-builds-after-alpha-17)
explains the longer waits. After exact-source publication, a retest request must
name the minimum tested commit, source-build availability, one separately
confirmed attempt, the existing ETS fallback and stop-on-failure boundary.
Issues #30/#22 and [S2-04](../open-work/S2-04-tunnel-loss-after-checkpoint.md)
remain open for actual device outcome. No real bus, gateway, detector, vendor
contact, release, registry publication or deployment was used for this package.
