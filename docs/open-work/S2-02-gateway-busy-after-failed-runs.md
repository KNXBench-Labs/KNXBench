# S2-02 — System-2: misleading busy hint and missing refusal reason

**Priority:** P1. **Status:** **Closed 15 November 2026** (removed from
[OPEN_WORK](../OPEN_WORK.md); not reproduced, cause unproven). The own-run
hint/refusal diagnostic slice ships in Alpha 17; the original start failure did not
recur, and the 8 November 2026 reporter run started and finished verified (see
[field outcome](#field-outcome-8-november-2026)). Issue #30 is closed. Origin: GitHub issue #30.

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

Diagnostic slice delivered in `aa2f39f1b` (hash mapping 2026-11-22: alpha-22 history; current-main counterpart `b036331fe`) (main `a15b8f0eb` (hash mapping 2026-11-22: alpha-22 history; current-main counterpart `7accddfab`)) and published in Alpha 17
(tag commit `a9ea32d72`). At that delivery no reporter re-test had been requested or run;
for the later runs see the closure section below. See the source/HTTP contract in
[alpha16-system2-http-mount](../contracts/alpha16-system2-http-mount.md).

## Investigation and closure plan (historical, through 8 November 2026)

1. Review and exercise this diagnostic slice with synthetic software/loopback
   fixtures, including zero device access for pre-connect refusals.
2. Published in Alpha 17. The reporter's later attempts on Alpha 17 source builds
   started without a refusal (27 October, comment `6438334297`; 28 October,
   comment `6468817418`), so the original start failure did not recur; its cause
   stays unproven. A combined optional retest on a current `main` build was
   requested on 8 November 2026 ([#30](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6718382084)). Preserve the original private
   journals; do not post them publicly.
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

## Field outcome, 8 November 2026

The reporter's one requested attempt (reported build `0.1.0-alpha.19+gac8f005`) was first
refused with `system2ConnectFailed` ("nothing was written";
[report](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6719267699)); that refusal happens before any device contact or
operation journal ([reply](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6719331526)). The reporter then found that the
interface address used in the first attempt was wrong
([comment](https://github.com/KNXBench-Labs/KNXBench/issues/30#issuecomment-6719332657)), so this refusal was an environment
input error, not a recurrence of the original failure. With the correct address one
new operation started without a refusal and finished verified, 27/27 steps (see
[S2-05](S2-05-result-and-settlement.md#field-run-8-november-2026)).

## Closure, 15 November 2026

Closed without a reproduced cause. The original reporter-side start failure did not
recur on the 27 and 28 October runs or on 8 November, so no leak was reproduced and no
fix beyond the delivered diagnostic is admitted (item 4 above). Still true and
documented as a boundary, not open work: `gatewayBusy` covers shared-lock conflicts,
process-lease failures and lease-file/runtime I/O errors alike, and does not identify
which one happened. A new report with that refusal is tracked as a new issue.
