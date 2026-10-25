# ADR 0153: A regular download may be stopped until its first change, and not after

Date: 2026-10-25
Status: Accepted; bounded software acceptance published `6c9ca2d7d`.
Hardware outcomes, actual HTTP-start execution and mid-procedure stop remain unverified.
[Acceptance](../status/2026-10-25-regular-prewrite-stop.md) records the exact scope.
Session: 7 (integration/hardening)

## Context

[ADR-0045](0045-device-download-route-demands-phrase-and-plan.md) §5 gave the web download no cancel, because
a download stopped between load-state steps leaves the device partly loaded. That is still
true after the first change. Before it, there is a boundary where stopping is defined: the
identity checks and the backup are reads, and the first load record or memory write has not
been sent. The server's graceful stop did not use it: a download still connecting or reading its
backup went on to write while the process waited out its grace period.

## Decision

1. A mask-`070nh` regular download has one **stop token** (`DownloadStop`) per run. It is claimed
   by exactly one executor, atomically, before any frame. It cannot be reset, retargeted or
   reused, whatever the first run did (finished, failed, stopped, or had no changing step).
2. States: `Fresh`, `FreshStopRequested`, `Running`, `RunningStopRequested`, `Committed`,
   `Ended`, `EndedAfterCommit`. A request answers `WillStop` before the commit, `TooLate` after
   it and `Ended` once a claimed executor is over. A token refused before ownership, or whose
   server session could not be constructed, stays unclaimed; `WillStop` on that token does not
   establish that a live run exists. Requests and the one commit serialise on a single
   atomic: an accepted stop wins the commit, and a commit that won makes every later request
   `TooLate`. No lock, no `await`, no dropped future.
3. The executor checks the token only between complete exchanges: before Connect, after each
   read-only step, between the backup's reads (also between region chunks), before the backup
   is handed to the caller, and once more after the caller kept it. The **commit** is the last
   of these, synchronously before the first changing step starts. After it nothing is checked:
   verify, readback, load polls, restart and disconnect run as before, with no resend, restore,
   rollback or reset.
4. `WillStop` promises one thing: this run sends no memory write, user-memory write, load
   record, property write or restart. It does not promise silence (connect, key exchange, reads
   and disconnect may have gone out; only a stop before Connect sends nothing), and it does not
   promise a `Stopped` result: an error found first stays the run's typed error.
5. The server's `StopAdmission` holds a weak handle to the registered download token under the
   same short synchronous mutex that `close` takes. The route registers while it holds the
   download reservation, after its busy check and before history, tunnel and spawn. A close
   therefore either finds the registration (and requests that token, without the async lock) or
   precedes it (and the route is refused with 503 before anything starts). `drain_device_writes`
   asks once more and reports the answer; that second request is a diagnostic, not the guarantee.
6. A stopped run is stored as `failed`, `written: no`. When the stop lands after the caller kept
   the backup, the row also says `sendPossible: true` with its backup, which the existing
   history validation accepts. No schema or migration changes. A crash still reads
   `interrupted/unknown`; nothing manufactures a stopped checkpoint. No automatic resume: a new
   run needs a fresh confirmation, plan and backup.
7. The stoppable entry point validates before any frame: plan shape, a memory-mapped mask, a
   session authorised for `WriteScope::Download`, no connection already held and an unclaimed
   token. A refusal sends nothing, not even a disconnect, and leaves the caller's session alone.
   This composition keeps Verify Mode, property and key writes out of the run; it is not a new
   device admission. Legacy entry points are unchanged.

ADR-0045 §5 stands for everything after the commit.

## Not decided here

Stopping after the first change, a per-step journal, restore, retry or reset, a stop route or
button, a CLI interrupt, and System-2 downloads. None of them is admitted by this decision.

## Consequences

- The graceful stop can prevent a download that has not changed the device yet.
- `WriteDrain` gains `download_stop`; the server's shutdown report records the request outcome.
  `WillStop` promises no mutation, not necessarily a `Stopped` terminal result.
- The contract is [regular-download-prewrite-stop](../contracts/regular-download-prewrite-stop.md).
