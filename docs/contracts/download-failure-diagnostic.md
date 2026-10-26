# Regular-download failure diagnostic (DIAG-02)

Status: implemented on a feature branch (26 October 2026); software-tested
only, no hardware run. Origin and acceptance: [DIAG-02](../open-work/DIAG-02-download-failure-evidence.md).

## Problem

A regular memory download that stopped partway left only the programming-flag
settlement in the session log (`info`, no location, no detail), and the report
carried `written` and `stoppedInStep` without any cause. The executor's
terminal error existed only as free text in the volatile server status. The
yellow colour came from the *written: partially* line; nothing marked the run as
failed. This contract records what is now retained. It says nothing about the
cause of the original field failure, which remains unknown.

## One value, built once

`knx_app::download_failure::DownloadFailure` is built by the server worker from
its own terminal result, exactly once per run, and every reader carries that same
value:

| Reader | Where | Retention |
|---|---|---|
| Server status | `GET /api/device-download/status` → `status.failure` (failed only) | until the next download replaces the session or the server stops |
| Session log | one `DeviceDownload` entry, `error` (or `warning` for a stop on request), `detail: "download <id>"` | this server run; cleared when another project is opened (`SessionLog::reset`) |
| Activity history | `downloadEvidence.failure` on the durable `failed` row | persistent across server restarts and project reopen |
| Tester's report | `report.failure` (format 1, additive) | in the saved file |

Closing and reopening the download panel reads the server-held session again
(no new download); the bus activity history shows the durable copy. The
settlement entry that follows a failure keeps `info` and carries the same
`detail` correlation; it reports the programming flags, never the cause.

## Fields

```json
{
  "category": "timeout",
  "code": "noAnswer",
  "phase": "inStep",
  "step": { "number": 8, "of": 16, "kind": "writeMemory", "summary": "write memory 4000h..4003h, 4 octets" },
  "restart": "notSent",
  "readBackOctets": 0
}
```

- `code`: one of 45 closed tokens, one per executor/session error variant
  (`FailureCode::of_error`, exhaustive: a new variant does not compile until
  classified), plus `stopped` and `taskEnded`. `category` (12 tokens) follows
  from `code` alone; readers refuse a disagreeing pair.
- `phase` + `step`: `inStep` names the step that ran (the error's own step
  index, else the last started step); `backupBeforeStep` and
  `stoppedBeforeStep` name the step that did **not** start; `beforeSteps`
  (plan/session refusal, connection setup) has no step. A failure before a plan
  step never gets an invented step. `stoppedInStep` keeps its old meaning (last
  step that started), so it can differ from `step.number` outside `inStep`.
- `step.summary`: built from the plan step without its octets (addresses and
  counts only, at most 160 octets); `kind` is the plan-step kind.
- `restart`: `notInPlan`; `notSent` when the run ended before the restart step
  (the executor runs steps in order); `unknown` in or after the restart step and
  for every unwitnessed end. Never "sent".
- `readBackOctets`: data octets written **and read back unchanged** before the
  end. Load records and the restart are not counted; a write after them may
  have reached the device partly. `written` (`no`/`partially`) stays the
  attempted-write classification; progress alone never implies success.

## Severity and presentation

A failed run is an `error` log entry and a `role="alert"` block headed
*Download failed*; a stop the user asked for is a `warning` entry and a
`role="status"` block headed *Download stopped on request*; success and an
unconfirmed restart carry no failure. The status line reads *Download failed in
step N* or *stopped on request before step N*. Colour (red/amber border) only
supports the heading and role.

## Privacy

Report and history never carry the executor's error text, memory or property
octets, keys, gateway, backup path or project data. The UI shows the server's
own error text only as a collapsed *Technical detail* in the live panel; it is
not part of the report, the history or the log entry.

## Compatibility

- Reports: format stays 1; `failure` is optional like `postDownloadCheck`. A
  stopped run without it (an older report, the CLI) reads as *cause not
  recorded*, never as success.
- History: `failure` is optional; an explicit `null`, a cause on a non-failed
  row, a stop with written octets, or any inconsistent value refuses the whole
  page in both strict readers (Rust and web, shared cases in
  `apps/knx-web/src/downloadFailure.cases.json`). As with `postDownloadCheck`,
  an older KNXBench that reads a history containing a recorded cause refuses
  that page (safe refusal, not a seamless downgrade).
- A task that ends without a terminal result keeps its history row `unknown`
  without a cause; status, log and report say `taskEnded`.

## Not covered

- The CLI (`knx device download`) and the System-2 worker still record failed
  rows without `failure`; their rows read as *cause not recorded*.
- No retry, restart, restore, timeout change or protocol change. The original
  hardware failure is not reproduced or explained.
