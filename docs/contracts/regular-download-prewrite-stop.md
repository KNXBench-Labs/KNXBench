# Regular download: stop until the first change

Bounded server and executor contract of 2026-10-25, [ADR-0153](../adr/0153-regular-download-stop-ends-at-first-change.md).
It narrows [ADR-0045](../adr/0045-device-download-route-demands-phrase-and-plan.md) §5 and nothing else.

## What a stop does

| Request arrives | Answer | Effect |
|---|---|---|
| before the first change is committed | `WillStop` | this run sends no memory write, user-memory write, load record, property write or restart |
| after the commit | `TooLate` | the run goes on unchanged: no resend, restore, rollback or reset |
| after a claimed executor ended | `Ended` | nothing |
| repeated while pending | `WillStop` | idempotent |

The commit is taken synchronously after the backup is kept and before the first changing step
starts. Stop points are between complete exchanges only. Stops are never delivered by dropping
a future. A previously unused token rejected by pre-claim validation (or whose server session
construction fails) remains unclaimed; its later request can still answer `WillStop`, without
proving a live run exists. A reuse refusal does not unclaim another executor's token.
Only an executor that actually claimed the token settles it to `Ended` on return or unwind.

## What it does not promise

- Not silence. Connect, key exchange, reads and disconnect may be sent before the stop lands.
  Only a stop before Connect sends nothing.
- Not a `Stopped` result: a genuine error met first is the result. A plan with no changing step
  finishes normally.
- Not safety after the commit. A forced exit still ends a committed download wherever it is.
- Not a resume: a stopped run is over; a new run needs its own confirmation, plan and backup.

## Surface

- `knx_net::commissioning::memory_download::run_memory_download_with_backup_stoppable(..,
  &DownloadStop)`: validates shape, memory-mapped mask, `WriteScope::Download`, no held
  connection and an unclaimed token before any frame; otherwise `StopRefused`.
- `MemoryDownloadError::Stopped { before_step, intent_recorded }`. Not a locked-device hint.
  `intent_recorded` means the caller's keeper accepted the backup; only the server keeper also
  guarantees its durable `sendPossible` record. `before_step` and its Display are zero-based;
  the existing server `stoppedInStep` field is one-based.
- Server: `StopAdmission::register_download` (503 once stopping) and `close` request the
  registered token under one synchronous mutex; the worker owns the token before it spawns;
  `drain_device_writes` reports `WriteDrain::download_stop`.
- A stopped run is `failed`, `written: no`, with no post-download check. After the backup was
  kept the history row also reads `sendPossible: true` with its backup.

## Limits

Public stop route, UI, CLI interrupt, System-2 downloads and any stop after the first change
are not part of this contract. Tested against the simulator only; no bus access.
The actual HTTP start route's registration order was source-reviewed and compiled, not exercised
through HTTP by the focused synthetic tests. Those tests use the real worker and a preregistered
handle, including a held async reservation and close before session installation.
