# System-2 download result, report and device-list projection (issue #31)

## Scope

A System-2 run can end with the transfer complete but its final verification not
finished (field report: 26 of 27 steps, the last step `Disconnect` failed with
a full-allocation mismatch). The device list used to show only the project's
plain "pending" marks. This contract defines the typed, journal-derived facts that
explain such a run, the shareable report, and how both reach the device list. It
settles the project's programming belief through the existing ADR-0129 mechanism, see
[Settlement](#settlement).

## Source of the facts

`OperationProgress.download_result` (App, `DownloadResult`) is derived only from the
validated checkpoint chain and the exact plan inside the durable recovery record. It
reads no message, no free text and no step count taken from a label:

| Value | Meaning |
|---|---|
| `inProgress` | The run has not ended. |
| `nothingWritten` | Ended and the journal never reached the durable backup barrier: no write could have been sent. |
| `transferIncomplete` | A write may have been sent and fewer than all non-final steps completed. |
| `transferCompleteVerificationNotStarted` | Every step before the last completed; the last step never started. |
| `transferCompleteVerificationUnfinished` | Every step before the last completed; the last step started and did not finish. |
| `verified` | The `Finished` checkpoint, which requires every step including the last. |
| `unknown` | The journal cannot prove any of the above. |

The last plan step must be `Disconnect`; that step hosts the read-only full-allocation
verification, the final Loaded-state and program-version reads, and the last write
(restoring Verify Mode). A failure while it is active is therefore **not** proof of a
verification *mismatch*: a read timeout or the final write can end it the same way.
The value claims exactly "transfer steps completed, final verification step did not
finish". It never claims a loaded device. All steps finished without a `Finished`
checkpoint is `unknown`, never `verified`. The value is frozen when the download
ends (or when a restore starts) so a later restore does not rewrite it, and it
is recomputed identically when a journal is reopened. A journal that predates the
fields still replays; facts it cannot prove stay `unknown`.

The server publishes `DownloadResult::settled`: `verified` only beside a finished
outcome, never `nothingWritten` beside a possible write, and `inProgress` while a
task is live whatever the journal already says.

## Download versus recovery

`OperationProgress.download` (`DownloadFacts`: result, outcome, completed steps, total
steps) is frozen when the download ends, or when a recovery starts on a download that
never ended. A recovery does not rewrite it: after `RestoreStarted` the current
`outcome`/`completed_steps` are the recovery's, and `restore_attempted` is true. A
reopened journal reproduces both. Nothing shows the historical download as a current
success after any recovery attempt: the device then holds the recovery's outcome, never
the requested configuration.

## Status schema 3

`GET /api/device-system2/status` (and every route returning the same view) moves to
`schemaVersion: 3` and adds `downloadResult`, `restoreAttempted`,
`downloadCompletedSteps`, `downloadTotalSteps` and `projectSettlement`. When
`restoreAttempted` is true, `state`, `completedSteps` and `totalSteps` are the recovery's;
the download keeps its own counters. The web client accepts exact versions 1, 2 and 3;
older wires normalize to `downloadResult: "unknown"`, no recovery, `notRecorded`. A
`downloadResult` key on a version-2 body is refused, and a failed or cancelled run
claiming `verified` is refused. `projectSettlement` is `notRecorded`, `noMutation`,
`persisted`, `sessionOnly`, `persistenceFailed`, `noProject`, `otherProject`,
`deviceMissing` or `unavailable`: what this process's terminal settlement did, volatile,
`notRecorded` after a restart, select or reopen. Nothing is ever replayed.

## Settlement

The worker settles once, on a blocking thread, before the terminal status is published,
through `domain::settle_programming` (ADR-0129): outside undo, invalidating older
programmed claims in undo and redo history, persisting before publishing, bound to the
project incarnation the run started in. Never an undoable command; GET, select, report
and reopen never settle.

- **No mutation possible** (journal never reached the durable backup barrier, or a
  recovery that never started): nothing changes. Receipt `noMutation`.
- **Mutation possible, not verified** (failed, cancelled, interrupted, unknown): the
  memory parts Application, Parameters and Group addresses become pending. Address is
  not touched.
- **Verified download** (`Finished`, final verification finished, no recovery): under
  the project lock, `admit_current_project_hardware` rebuilds the retained-source plan
  from the current project and requires the same normalized plan and full source
  preparation hash. Only then are all four parts marked programmed (the device
  answered at the project's address for the whole run) and the download time recorded.
  This comparison is on the full snapshot: **any** edit that changes the preparation,
  even to an unrelated parameter, leaves the memory parts pending. That is the
  documented conservative boundary; there is no per-part equivalence and no hash alone is
  used as per-part proof.
- **Recovery** that started: memory parts pending, never programmed, whatever its
  verification said.
- The operation's server incarnation must equal the running server's; a historical operation
  (recovery after a restart) may act on its acknowledged hardware but its numeric project id
  proves nothing about a new session (`otherProject`, nothing touched).
- The bound device must exist and still carry the recorded address. Another project
  (`otherProject`), no project, a missing device or an address reused by another device
  id (`deviceMissing`) changes nothing.
- A persistence failure applies nothing and is reported as `persistenceFailed`; flags,
  history and revision stay as they were.

**Unexpected worker end.** The retained service handle is a supervisor: it ends only after the
worker and, when the worker panicked or was aborted inside this process, after a conservative
terminal settlement computed from the durable journal alone (never a verified claim; no journal
or observed record counts as a possible mutation) and the published end (`interrupted`,
refusal `system2WorkerEnded`, `downloadResult` never `verified`). A process crash or runtime
shutdown cannot run it and promises nothing; GET, select, report and reopen never settle.

This is the project's belief, not a hardware-compatibility claim and not a device read.
A process crash before the settlement leaves the belief as it was and the receipt
absent; the retained journal remains the evidence.

## Report

`GET /api/device-system2/report?operationId=<id>` returns the distinct kind
`knxbench-system2-download-summary`, format 2: `format`, `kind`, `support`, `evidence`
(`local-journal-only`), `download` (`outcome`, `result`, `completedSteps`, `totalSteps`),
`recovery` (`null`, or `outcome`, `completedSteps`, `totalSteps`), `mutationPossible`,
`prestateDiffers`, `failureDiagnostics` (the typed list of the
[safe failure diagnostic](system2-safe-failure-diagnostic.md)), `executionBuild` and
`exporterBuild`. The ordinary `knxbench-property-validation-summary` is not reused: it says
nothing of the restart or per-block verification the run did or did not do. No address,
device name, parameters, backup bytes, paths, operation id, binding, phrases or journal text
is in it.

`executionBuild` is `{state: "recorded", version}` only for operations that wrote the
operation-bound sidecar `execution-build.json` (format 1, the operation id, the version
string; written create-new, before any device contact, bounded to 512 bytes). Every older
operation, and any missing, oversized, foreign or malformed sidecar, reports
`{state: "unknown"}`. `exporterBuild` is the build that wrote this report and is never a
claim about the execution. A version is only the closed build vocabulary of `GET /api/version`,
`MAJOR.MINOR.PATCH[-prerelease][+g<hex sha>]`, checked identically by the server (sidecar and
exporter) and the client; a path, free text or any other value makes the record unknown and is
never sanitized.

Admission is read-only: the report refuses (HTTP 409 `system2Busy`) only while a task is
actually live or another System-2 command holds the service. A quarantined coordinator (a
teardown that failed) does not lock the retained evidence away. Other refusals:
`system2OperationUnknown`, `system2ReportUnavailable` (unreadable or inconsistent
evidence; no result is guessed). A malformed or missing id is HTTP 400. A retained
operation with neither a journal nor an observed record cannot prove that nothing was
written: it reports `mutationPossible: true` and result `unknown`.

The panel saves the report as a local file and tells the user to review it and email
it to issue@knxbench.com. The report is experimental evidence, not a compatibility claim.

## Device list

The Devices table and the System-2 panel share one status lifetime (`system2Status.ts`),
owned at application level by `System2Lifecycle` (mounted at the root, so a tab or view switch
that unmounts the Download panel does not end it). The owner polls once a second **only while a
known operation is running or restoring**, never when idle and never per row; the panel's own
answers and every table read are published to the same store, and a read that began before a newer
publication is dropped. When a settled terminal status becomes visible, the store enters
`refreshing` in the same step, so a clean mark is never shown beside it; the owner reloads the
project once, hands it to the editor, and only then acknowledges (`current`). A failed reload
leaves an explicit `stale` belief with a **Reload marks** button (one explicit retry, never an
automatic loop); a late answer for a superseded end is dropped. The table and the panel show the
belief notice until it is acknowledged. A null status after a known operation, or a failed read,
withdraws the authority: the last known result is shown as "status unavailable" rather than as
success, and the evidence stays separate.

A result is shown only when the status binds to the open project: same server
incarnation, same project incarnation, the device id still present **and** still carrying
the recorded address. The same address on another device id shows nothing. A changed
revision keeps the chip and marks it edited. Running and restoring operations show
nothing in the table. After a recovery attempt the chip shows the recovery outcome
(`recoveryRestored`, `recoveryFailed`, `recoveryUnfinished`), never the historical download.
The chip is neutral, labelled in EN and DE, sits beside the project's pending marks and
never replaces them. Its visible text is searchable and ties of the programming sort are
broken by it, without changing what "pending only" means. The explorer dot is unchanged.

## Limits

- Retained operations from before a server restart carry another server incarnation: they
  show in the System-2 panel and report, not in the device table.
- Only the currently selected operation feeds the table (one status read).
- The mismatch/timeout split inside the final step needs a typed Net failure and a
  journal field; not done. #30's actual allocation mismatch stays unresolved.
- A verified run clears the memory beliefs only when the whole preparation is still equal;
  otherwise they stay pending (conservative, documented above).
- No hardware evidence. Tests use journals built on the public synthetic fixture,
  mounted hardware-kind simulator workers with the digest-pinned public package, and
  intercepted HTTP.
