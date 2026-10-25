# S2-02 — System-2: "gateway busy" after two failed operations

**Priority:** P1. **Status:** Planned (scheduled by the owner, 25 October 2026); investigation first.
Listed in [OPEN_WORK](../OPEN_WORK.md). Origin: GitHub issue #30. After two operations that
failed at the pre-mutation recovery acquisition, every recovery track answered "Gateway belegt"
while the bus monitor still connected, and no new operation journal was created.

## Facts and unknowns

- Verified from code: a refusal before an operation exists comes from the in-server reservation
  (`worker::reserve`, `gatewayBusy`), the cross-process lease file, or the System-2 coordinator
  (`system2Busy` while a settlement is `Running`/`Uncertain`).
- A failed teardown deliberately leaves the coordinator uncertain ("an error quarantines the
  coordinator"); a confirmed teardown settles it.
- Not known: which of these produced the reporter's message, whether either failed run left a
  session, lease or unconfirmed teardown behind, and the exact message text and panel.
- Possibly relevant, unverified: the System-2 refusal `gatewayBusy` carries no `holder`, while
  the web layer's structured-refusal admission requires one, so the ordinary holder wording would
  not appear for it.

## Deliverables

1. **Reproduce offline** with the simulator: run a System-2 start that fails at the recovery
   acquisition (also with the new wrong-width answer from [S2-01](S2-01-pid7-raw-response.md)),
   then check reservation, lease, coordinator settlement and the activity/status endpoints.
2. **Fix a real leak** if found (teardown, lease or settlement not released on that failure path),
   with a regression test. If none is found, record that and which paths were checked.
3. **Say which holder is busy.** A System-2 busy refusal names its cause (in-server holder,
   lease, uncertain teardown), reusing the existing holder wording; no generic "busy".
4. **Document the recovery route** for a stuck state (what the activity and status endpoints
   show, when a server restart is the only route) without inventing a clear-state control.

## Boundaries

No live-bus operation and no force-disconnect of a tunnel; external clients are not visible to the
in-server checks. Needs the reporter's exact message and the read-only endpoint outputs from #30
to close the "which path" question. Shared code: coordinate with [PRG-01](PRG-01-programming-gaps.md).
