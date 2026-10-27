# Experimental System-2 patient wait profile

## Scope and intent

Issue #30 reached the hardware-check/setup operation and failed with a tunnel
acknowledgement timeout on build `ae47331`. That build predates the receiver
liveness and response-aware teardown fix `1ab4ad6a5`. The owner requested a
substantially more patient next build, without another prerequisite data request.
This profile is an explicit experimental client policy, **not** a new KNX
specification timing or proof of the original failure's cause.

The experimental System-2 hardware download and explicit recovery paths select
the profile in the shared server/desktop service and the corresponding CLI
command. Ordinary downloads, bus monitoring, scans, discovery and other tunnel
users keep their existing defaults. Simulator-only and offline preparation paths
do not acquire new hardware permissions.

## Timing contract

| Wait | Existing default | Experimental System-2 |
| --- | --- | --- |
| Matching gateway `TUNNELLING_ACK`, each attempt | 1 s | 10 s |
| Device transport acknowledgement before repetition | 3 s | unchanged |
| Management answer slice after positive device transport ACK | 3 s | 15 s |
| Load-state transition budget | 30 s | 120 s |
| Device connection confirmation | 6 s | unchanged |
| Load-state poll interval | 3 s | unchanged |
| Heartbeat interval / response budget | 60 s / 10 s | unchanged |

The tunnel retains the original request plus **one** repetition, with the same
sequence and payload. Silence therefore consumes up to approximately 20 seconds
of acknowledgement waits, plus scheduling and I/O; there is no third request.
A matching reply can complete either wait early. Explicit gateway closure or
known socket failure must terminate a pending wait without consuming the larger
silence budget. Incoming requests remain acknowledged and heartbeat tasks remain
independent while an outgoing request waits.

The extended management figure applies **after a positive device transport ACK**,
not to the wait for that ACK. Before acknowledgement, the existing three-second
repetition cadence remains, so a single lost request or ACK can still recover
inside the device's six-second idle window. A positive acknowledgement switches
to the longer answer slices without retransmitting that application request.
The existing bounded repetition/answer ladder remains; the extended slices are
not a total operation deadline and can accumulate in addition to lower-layer
send waits and cleanup. Exact response matching and refusals are unchanged.

No whole-download retry, restore or restart is added. The transition budget
changes patience, not accepted final states or readback requirements. Peer-owned
deadlines do not change: waiting longer locally cannot keep a device connection
alive after the peer has closed it, nor compensate for a stalled receiver.

## Preserved boundaries

- Hardware identity, source/plan admission and target-bound confirmations stay.
- Actual prestate backup, durable persistence and readback precede setup writes.
- Uncertain or failed operations remain uncertain or failed; time passing cannot
  turn a missing response into success.
- Final memory, application identity and load-state checks remain mandatory.
- Captured backup availability does not promise original-state recovery when
  the captured prestate differs from the source model.
- No format migration, journal rewrite, raw diagnostic publication, new hardware
  family or compatibility admission follows from this timing policy.

## Evidence interpretation and retest

The reporter's checkpoint 3-to-4 interval of 30.040 seconds spans hardware
checks, device prestate reads, snapshot/model validation and persistence. File
mtime is not completion of fsync, HEAD update or readback. It must not be labelled
as one 30-second disk write or an equally long idle connection. The reported
CPU/storage facts weaken the specific slow-host explanation; no cause is proven.

Acceptance must exercise delayed replies through real client/executor paths,
not only compare constants. Require delayed matching ACK admission without an
early repetition, bounded unchanged-sequence silence, prompt explicit closure,
continued inbound reception, and no mutating application retransmission after
positive transport ACK. Keep standard defaults separately characterized.

A future reporter retest must use a build containing **both** this profile and
`1ab4ad6a5`; an existing Alpha 17 image does not update itself. Publish the exact
tested revision before requesting one optional, explicitly confirmed attempt
with the existing ETS fallback. On failure preserve the operation and stop;
there is no automatic retry. Software/simulator acceptance alone does not prove
UP258 programming, recovery or normal application function.

[Existing tunnel analysis and remaining work](../open-work/S2-04-tunnel-loss-after-checkpoint.md)
remain separate from the timing change. The earlier
[management timing research](../research/knxnet-ip-and-bus/8-5-line-scan-bus-side-device-discovery-t17-spike-2026-09-12-shipped.md)
and the cited source comments retain their normative defaults; the extended
values above are deliberately labelled experimental policy.
