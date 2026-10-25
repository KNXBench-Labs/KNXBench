# S2-02 — System-2: misleading busy hint and missing refusal reason

**Priority:** P1. **Status:** In progress: own-run hint/refusal diagnostic slice;
actual reporter-side start failure remains unresolved.
Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30.

## Facts and unknowns (25 October 2026)

The reporter supplied the exact `gatewayRefusal.busy.connecting.managementOperation`
text, an empty activity snapshot before/after the attempt, and the previous failed
selected operation after a server restart. The banner also occurs after restart;
that observation does not identify a leaked session or lease.

Source tracing shows that the gateway hint mistakes the System-2 worker's own
address-programming reservation for a foreign management operation. The worker
reserves download, programming, monitor and scan together. The hint previously
excluded only the ordinary download lock. The banner itself does not stop a start
and is not the original failure reason.

Before creating a new operation journal, the worker can refuse shared reservation,
the process-wide gateway lease, activity recording, source recheck or journal
creation. Previously the worker discarded the `ApiError` kind. The old selected
operation after restart therefore does not identify which early step refused.
The reported short HTTP stall and the actual refused step remain unresolved.

## Diagnostic slice

- `GET /api/bus/activity` names an admitted running coordinator as
  `system2Operation` before the existing busy locks. The individual lock/session
  evidence remains present. The snapshot stays partial and non-atomic.
- The download hint treats those shared locks as this page's own System-2 run,
  rather than announcing a foreign management operation. A real foreign holder
  without that marker is still shown. Other clients' gateway occupancy remains
  outside this endpoint's coverage.
- System-2 status exposes nullable `refusal`: only the stable API error kind that
  ended this in-process attempt, never a message, path or raw exception. It is
  volatile: selecting/reloading historical evidence does not invent a reason.
- The panel shows that reason and a structured `4xx` start refusal's kind.
  Transport loss, malformed replies and server errors retain unknown-outcome
  handling. No automatic retry, force-disconnect or clear-state control is added.

Diagnostic slice delivered in `aa2f39f1b` (main `a15b8f0eb`); reporter re-test pending. See the source/HTTP contract in
[alpha16-system2-http-mount](../contracts/alpha16-system2-http-mount.md).

## Remaining investigation and closure

1. Review and exercise this diagnostic slice with synthetic software/loopback
   fixtures, including zero device access for pre-connect refusals.
2. Publish the accepted source revision before requesting a new reporter attempt.
   A prior build cannot expose the newly retained reason. Preserve the original
   private journals; do not post them publicly.
3. Use the new reason to investigate the actual start failure. Shared-lock
   conflicts and failure to acquire the process lease currently share
   `gatewayBusy`, including lease-file/runtime I/O errors; the token alone does
   not distinguish these mechanisms or prove contention or a leak.
4. Fix a real leak only if reproduced, with its own regression. An uncertain
   coordinator after unconfirmed teardown is a deliberate safety boundary, not
   permission to clear evidence. Confirmed teardown settles it; restart does not
   prove device recovery or undo writes.

No live-bus access, forced tunnel release, firmware/protocol relaxation or vendor
contact is authorized. Coordinate shared source with
[PRG-01](PRG-01-programming-gaps.md). This diagnostic slice does not close issue #30.
