# Read-only check after a regular memory download

Outcome 3 of the [programming improvements goal](../goals/programming-improvements-2026-10-25.md),
branch `feat/post-download-observation-20261025`. Simulator-tested source; no
hardware acceptance. Source APPROVED after re-review of the corrections; not
delivered: integration acceptance (corpus, package gate, browser EN/DE) is pending.

## When it runs

- Only the regular memory download (`knx device download --gateway … --confirm …`
  and the server/web download worker) runs it. `knx device restore` does not.
- Only after `run_memory_download_with_backup` returned `Ok` for the plan
  (complete or partial, any `RestartOutcome`, including `Unconfirmed`), i.e.
  after the transfer's closing `Disconnect` and, when the plan restarts, its
  restart wait (`notInPlan` has none). A failed transfer gets no follow-up
  connection.
- The transfer's result (`written`, `restart`) is recorded in the activity
  history first. Programming flags settle from the transfer exactly as before
  (ADR-0129); the check never changes them.
- One call of `knx_net::commissioning::post_download::observe_after_download`
  on the same tunnel, while the download still owns it (the server keeps the
  gateway reservation until the check and tunnel cleanup end). Read-only
  session, `AuthorisationPlan::Skip`: no write, restart, reset,
  `A_Authorize_Request` or Verify Mode, and no second attempt. Transport Layer
  repetitions of a frame inside that one attempt are the session's own and are
  not a second attempt.

## Shared record

`knx_app::post_download_check::PostDownloadCheck`, one vocabulary for the CLI,
the server status, the tester's report and the activity history. Reach and the
load-state verdict come from the helper's own `reach()`/`load_verdict()`.

```json
{ "observation": { "status": "attempted", "reach": { "result": "reached",
    "identity": { "result": "match", "exemplar": "notVerified",
      "loadStates": [ { "machine": "addressTable", "expected": "loaded",
                        "observed": { "result": "observed", "loadState": "loaded" } } ],
      "planLoadVerdict": "matchesPlan" } } },
  "runtime": "unknown" }
```

| Level | Values |
| --- | --- |
| `observation.status` | `notChecked`, `unsupported` (`reason`: `unsupportedMask`/`applicationUnknown`), `notPerformed`, `interrupted`, `attempted` |
| `reach.result` | `notReached`, `unknown`, `reached` (only this one carries `identity`) |
| `identity.result` | `unavailable` (`part`), `mismatch` (`part`, `kind`: `differs`/`malformed`), `match` |
| `part.kind` | `mask`, `manufacturer`, `property` (`objectIndex`, `propertyId`), `application` |
| machine | `addressTable`, `associationTable`, `applicationProgram`, `peiProgram`; `expected` is a load state or `null` (not settled by the plan) |
| `observed.result` | `observed` (`loadState`), `unavailable` (the read failed), `notRead` (an earlier read failed) |
| `planLoadVerdict` | `matchesPlan`, `differsFromPlan`, `unknown` |
| `runtime` | always `unknown` |

- Nesting makes contradictions unrepresentable (no identity without reach, no
  load states without a matching identity). A stored verdict its machines
  contradict, a machine read after a failed one, unknown fields or a runtime
  other than `unknown` are refused on read (Rust and web). Both readers run the
  same case table (`apps/knx-web/src/postDownloadCheck.cases.json`, each case a
  fixture with one edit). The fixtures are hand-written: the Rust test proves
  each is accepted and re-serialized unchanged, not that a live run produced it.
- Earlier readings survive a later failed read (`unavailable`, then `notRead`).
- `match` is not the same physical exemplar: no serial number is read
  (`exemplar: notVerified`). `loaded` is a load state, not a run state.
- Payload-free: no address, serial number, raw identity octets, key or
  free-text error. A mismatch names the part, not what the device answered.
  The CLI additionally prints the helper's local detail line for the operator;
  it is never stored or reported.

## Where it appears

| Reader | Field | Absent |
| --- | --- | --- |
| Server status `GET /api/device-download/status` | `status.postDownloadCheck` on `finished`; event `postDownloadCheckStarted` while it runs (status stays `running`) | older server: not checked |
| Report `GET /api/device-download/report`, `knx device download --report` | top-level `postDownloadCheck` (failed runs: `notChecked`) | older report: not checked |
| Activity history | `downloadEvidence.postDownloadCheck`, only on `state: finished`, `written: yes`; never `notChecked` (absence says that); an explicit `null` is refused (Rust and web) | older record or no result: not checked |

Kind-specific validation: the field exists only inside `downloadEvidence`,
which only `deviceDownload` may carry; a `deviceCompare` (or any other kind)
row with it is refused, as is a `failed` download or one with `written:
partially` that carries one.

**Report format stays 1.** The field is additive and optional; no existing
field changes meaning (the `REPORT_FORMAT` rule). It identifies no
installation, so the report privacy rule needs no ADR. The shareable summary
(issue body, mail) adds "not proven the same physical device" to a matching
identity, as the CLI and the web view do.

**History storage format stays 2; additive in one direction only.** This
reader reads every older row (absent field: not checked). The published
format-2 reader (`07fe2e67f`, Rust and web) refuses unknown fields: once it
pages a history holding one checked row (the server's history view), the whole
page fails, the history reads `unavailable` and that process refuses further
bus writes ("activity history is unavailable; not sent"). Nothing is rewritten
or lost; use this or a newer version to read that history. A safe refusal, not a seamless downgrade; no format bump or
migration was made for it (`an_older_reader_refuses_a_checked_row_without_losing_it`
pins the published row shape and the refusal path).

## Failure and cancellation

- Not reached, reach unknown, identity unavailable or mismatch: the transfer
  stays `finished`/`written: yes`; the check result says what it saw. No
  automatic resend or repair is offered; the independent function checklist
  (`DownloadVerification`) is unchanged.
- Server worker stopped during the check, or a panic unwinding the CLI: status
  `finished` with the transfer's facts and `observation.status: interrupted`;
  history keeps `finished`/`written: yes`, `cleanup: unknown`, check
  `interrupted`.
- CLI killed (signal, `kill`) or server process crashed during the check: no
  destructor runs. The download path installs no SIGINT handler, so no graceful
  interruption is claimed. The history keeps `finished`/`written: yes` with
  cleanup `pending`, which a later process reads as `cleanup: unknown` with
  `interrupted: true`, and no check result, which reads as not checked although
  read frames may have been sent.
- A history that cannot record the check leaves the transfer's record as is;
  the check is still shown and reported, and the CLI exits non-zero, as for any
  other unrecorded fact.
- CLI exit status does not depend on the check's result. `main.rs` returns
  `device_download::finish_regular_download`, the composition the CLI tests
  call: cleanup record, report with the check, exit.

## Evidence and the gate still to run

Simulator-only. CLI and server unit regressions run the real runner and
composition; no hardware. The HTTP parent path (route status, event, report,
history) is asserted only in corpus tests that need the gitignored
`OriginalData/` and are `#[ignore]`d in a plain run, so they are pending, not
passed, until the corpus gate runs them:

- `http_device_download::a_verified_download_says_so_and_keeps_a_backup_first`:
  `status.postDownloadCheck` is checked (runtime `unknown`), the last event is
  `postDownloadCheckStarted`, the report's `postDownloadCheck` equals it.
- `http_device_download::download_terminal_receipt_survives_worker_completion`:
  the history row's `downloadEvidence` has five fields, its check equals the
  route's `status.postDownloadCheck`, no frame after the join.

Run with `cargo test -p knx-server --test it http_device_download -- --ignored`
(or `tools/run_corpus_tests.py`, see [verification](../VERIFICATION.md)).
